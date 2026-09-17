"""Explicit lifetime for the project-owned GPT-SoVITS inference pipeline."""
import gc
import asyncio
import copy
from pathlib import Path
import shutil
import sys
import threading
import traceback


class RuntimeUnavailable(RuntimeError):
    pass


class ModelRuntime:
    def __init__(self, factory, cleanup):
        self._factory = factory
        self._cleanup = cleanup
        self._pipeline = None
        self._lock = threading.Lock()
        self._state = "unloaded"

    def snapshot(self):
        messages = {"unloaded": "模型已关闭，启用后才能播报或试听", "loaded": "模型已启用",
                    "loading": "正在加载模型", "unloading": "正在释放模型内存", "failed": "模型加载失败，请检查引擎日志后重试"}
        state = self._state
        return {"supported": True, "state": state, "message": messages[state]}

    def acquire(self):
        if not self._lock.acquire(blocking=False):
            raise RuntimeUnavailable("模型正在加载、卸载或合成，请稍后重试")
        if self._pipeline is None:
            self._lock.release()
            raise RuntimeUnavailable("模型未启用，请先启用语音模型")

    def release(self):
        self._lock.release()

    def __getattr__(self, name):
        pipeline = self._pipeline
        if pipeline is None:
            raise RuntimeUnavailable("模型未启用，请先启用语音模型")
        return getattr(pipeline, name)

    def set_enabled(self, enabled):
        if type(enabled) is not bool:
            raise ValueError("enabled 必须为布尔值")
        if not self._lock.acquire(blocking=False):
            raise RuntimeUnavailable("模型正在加载、卸载或合成，请稍后重试")
        try:
            if enabled and self._pipeline is None:
                self._state = "loading"
                try:
                    self._pipeline = self._factory()
                except Exception as error:
                    self._pipeline = None
                    self._state = "failed"
                    traceback.clear_frames(error.__traceback__)
                    self._cleanup()
                    raise RuntimeUnavailable("模型加载失败，请检查引擎日志后重试") from error
                self._state = "loaded"
            elif not enabled:
                self._state = "unloading"
                self._pipeline = None
                self._cleanup()
                self._state = "unloaded"
            return self.snapshot()
        finally:
            self._lock.release()


def release_model_memory():
    # Upstream keeps these helpers outside TTS. Release them as well as the
    # pipeline; clean_text imports chinese2 again when it is next needed.
    for name in ("text.chinese2", "GPT_SoVITS.text.chinese2"):
        module = sys.modules.pop(name, None)
        if module is not None:
            if hasattr(module, "g2pw"):
                module.g2pw = None
            parent_name, attribute = name.rsplit(".", 1)
            parent = sys.modules.get(parent_name)
            if parent is not None and getattr(parent, attribute, None) is module:
                delattr(parent, attribute)
    detector_module = sys.modules.get("fast_langdetect.infer")
    detector = getattr(detector_module, "_default_detector", None)
    models = getattr(detector, "_models", None)
    if isinstance(models, dict):
        models.clear()
    for name, attributes in (
        ("module.mel_processing", ("mel_basis", "hann_window")),
        ("GPT_SoVITS.module.mel_processing", ("mel_basis", "hann_window")),
        ("GPT_SoVITS.TTS_infer_pack.TTS", ("resample_transform_dict",)),
        ("TTS_infer_pack.TTS", ("resample_transform_dict",)),
    ):
        module = sys.modules.get(name)
        for attribute in attributes:
            cache = getattr(module, attribute, None)
            if isinstance(cache, dict):
                cache.clear()
    gc.collect()
    torch = sys.modules.get("torch")
    if torch is not None and torch.cuda.is_initialized():
        torch.cuda.empty_cache()


class ModelLeaseMiddleware:
    def __init__(self, app, runtime):
        self.app = app
        self.runtime = runtime
        self._responses = set()

    async def __call__(self, scope, receive, send):
        if scope["type"] != "http" or scope["path"] not in {
                "/tts", "/set_gpt_weights", "/set_sovits_weights", "/set_refer_audio"}:
            return await self.app(scope, receive, send)
        try:
            self.runtime.acquire()
        except RuntimeUnavailable as error:
            from starlette.responses import JSONResponse
            return await JSONResponse({"message": str(error)}, status_code=409)(scope, receive, send)
        async def respond():
            try:
                await self.app(scope, receive, send)
            finally:
                self.runtime.release()
        # Cancelling an ASGI task cannot stop a synchronous CUDA generator in
        # Starlette's thread pool. Keep ownership until that worker really ends.
        task = asyncio.create_task(respond())
        self._responses.add(task)
        def finished(response):
            self._responses.discard(response)
            if not response.cancelled():
                response.exception()
        task.add_done_callback(finished)
        await asyncio.shield(task)


def install_model_runtime(app, pipeline_type, config):
    from starlette.responses import JSONResponse
    # TTS mutates its config when switching weights and stores device tensors in
    # it. Keep the startup configuration isolated across enable/disable cycles.
    startup_config = copy.deepcopy(config)
    runtime = ModelRuntime(lambda: pipeline_type(copy.deepcopy(startup_config)), release_model_memory)

    @app.get("/meowlive/models")
    def model_status():
        return runtime.snapshot()

    @app.post("/meowlive/models")
    def model_control(body: dict):
        if set(body) != {"enabled"} or type(body["enabled"]) is not bool:
            return JSONResponse({"message": "需要 enabled 布尔值"}, status_code=400)
        try:
            return runtime.set_enabled(body["enabled"])
        except RuntimeUnavailable as error:
            if runtime.snapshot()["state"] == "failed":
                traceback.print_exc()
            return JSONResponse({"message": str(error)}, status_code=409)

    app.add_middleware(ModelLeaseMiddleware, runtime=runtime)
    return runtime


def prepare_runtime(work):
    work = Path(work)
    api = work / "api_v2.py"
    source = api.read_text()
    marker = "tts_pipeline = TTS(tts_config)\n\nAPP = FastAPI()"
    replacement = ('APP = FastAPI()\nfrom _meowlive_model_runtime import install_model_runtime\n'
                   'tts_pipeline = install_model_runtime(APP, TTS, tts_config)')
    if source.count(marker) == 1:
        api.write_text(source.replace(marker, replacement))
    elif replacement not in source:
        raise ValueError("上游推理入口已变化，无法配置模型启停")
    shutil.copyfile(Path(__file__), work / "_meowlive_model_runtime.py")
