"""Model lifetime tests use controlled objects and never load GPU/model weights."""
from pathlib import Path
from contextlib import asynccontextmanager, redirect_stderr, redirect_stdout
import asyncio
import gc
import importlib.util
import io
import os
import tempfile
import threading
import types
import sys
import unittest
import weakref
from unittest.mock import patch

import model_runtime


class Pipeline:
    def speak(self):
        return "audio"


class ModelRuntimeTests(unittest.TestCase):
    @unittest.skipUnless(importlib.util.find_spec("fastapi"), "受管推理环境的 HTTP 集成测试")
    def test_launcher_autoloads_models_and_manual_unload_stays_unloaded(self):
        from fastapi import FastAPI
        from fastapi.testclient import TestClient
        lifecycle = []
        @asynccontextmanager
        async def lifespan(app):
            lifecycle.append("startup")
            yield {"upstream": True}
            lifecycle.append("shutdown")
        app = FastAPI(lifespan=lifespan)
        with patch.dict(os.environ, {"MEOWLIVE_AUTO_ENABLE_MODELS": "1"}):
            runtime = model_runtime.install_model_runtime(app, lambda config: Pipeline(), {})
        @app.post("/tts")
        def speak():
            return {"audio": runtime.speak()}
        with TestClient(app) as client:
            self.assertEqual(client.get("/meowlive/models").json()["state"], "loaded")
            self.assertEqual(client.post("/tts").json(), {"audio": "audio"})
            client.post("/meowlive/models", json={"enabled": False})
            self.assertEqual(client.get("/meowlive/models").json()["state"], "unloaded")
            self.assertEqual(client.post("/tts").status_code, 409)
        self.assertEqual(lifecycle, ["startup", "shutdown"])

    @unittest.skipUnless(importlib.util.find_spec("fastapi"), "受管推理环境的 HTTP 集成测试")
    def test_automatic_model_load_failure_remains_visible_and_can_be_retried(self):
        from fastapi import FastAPI
        from fastapi.testclient import TestClient
        app = FastAPI()
        attempts = []
        def factory(config):
            attempts.append(True)
            if len(attempts) == 1:
                raise ValueError("controlled model load failure")
            return Pipeline()
        with patch.dict(os.environ, {"MEOWLIVE_AUTO_ENABLE_MODELS": "1"}):
            model_runtime.install_model_runtime(app, factory, {})
        with redirect_stderr(io.StringIO()), TestClient(app) as client:
            self.assertEqual(client.get("/meowlive/models").json()["state"], "failed")
            self.assertEqual(client.post("/meowlive/models", json={"enabled": True}).json()["state"], "loaded")

    def test_managed_entrypoint_passes_auto_load_flag_through_private_environment(self):
        spec = importlib.util.spec_from_file_location("managed_inference", Path(__file__).with_name("start-managed-inference.py"))
        launcher = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(launcher)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            models = {key: root / key for key in ("gpt", "sovits", "bert", "hubert")}
            for flag in ("1", "0"):
                with (patch.dict(os.environ, {"MEOWLIVE_AUTO_ENABLE_MODELS": flag, "PRIVATE_API_KEY": "not-forwarded"}),
                      patch.object(sys, "argv", ["inference", "--engine-root", str(root), "--data-dir", str(root)]),
                      patch.object(launcher, "assets", return_value=models),
                      patch.object(launcher, "workspace", return_value=root),
                      patch.object(launcher, "configure_inference_memory"),
                      patch.object(launcher, "prepare_runtime"),
                      patch.object(os, "chdir"), patch.object(os, "execve") as execute,
                      redirect_stdout(io.StringIO())):
                    launcher.main()
                child_env = execute.call_args.args[2]
                self.assertEqual(child_env["MEOWLIVE_AUTO_ENABLE_MODELS"], flag)
                self.assertNotIn("PRIVATE_API_KEY", child_env)

    @unittest.skipUnless(importlib.util.find_spec("fastapi"), "受管推理环境的 HTTP 集成测试")
    def test_http_starts_idle_and_requires_explicit_enable(self):
        from fastapi import FastAPI
        from fastapi.testclient import TestClient
        app = FastAPI()
        created = []
        configurations = []
        config = {"weights": "default"}
        def factory(config):
            configurations.append(config["weights"])
            config["weights"] = "previous-audition-deleted"
            pipeline = Pipeline()
            created.append(weakref.ref(pipeline))
            return pipeline
        with patch.object(model_runtime, "release_model_memory", gc.collect):
            runtime = model_runtime.install_model_runtime(app, factory, config)
        @app.post("/tts")
        def speak():
            return {"audio": runtime.speak()}
        with TestClient(app) as client:
            self.assertEqual(client.get("/openapi.json").status_code, 200)
            self.assertEqual(created, [])
            self.assertEqual(client.get("/meowlive/models").json()["state"], "unloaded")
            self.assertEqual(client.post("/tts").status_code, 409)
            self.assertEqual(client.post("/meowlive/models", json={"enabled": "yes"}).status_code, 400)
            self.assertEqual(client.post("/meowlive/models", json={"enabled": True}).json()["state"], "loaded")
            self.assertEqual(client.post("/tts").json(), {"audio": "audio"})
            self.assertEqual(client.post("/meowlive/models", json={"enabled": False}).json()["state"], "unloaded")
            self.assertIsNone(created[0]())
            self.assertEqual(client.post("/tts").status_code, 409)
            self.assertEqual(client.post("/meowlive/models", json={"enabled": True}).status_code, 200)
            self.assertEqual(configurations, ["default", "default"])
            self.assertEqual(config["weights"], "default")
            client.post("/meowlive/models", json={"enabled": False})

    def test_start_is_unloaded_and_disable_releases_the_pipeline(self):
        created = []
        def create():
            pipeline = Pipeline()
            created.append(weakref.ref(pipeline))
            return pipeline
        runtime = model_runtime.ModelRuntime(create, gc.collect)
        self.assertEqual(runtime.snapshot()["state"], "unloaded")
        self.assertEqual(created, [])
        with self.assertRaisesRegex(model_runtime.RuntimeUnavailable, "启用"):
            runtime.acquire()
        runtime.set_enabled(True)
        runtime.set_enabled(True)
        self.assertEqual(len(created), 1)
        runtime.acquire()
        try:
            self.assertEqual(runtime.speak(), "audio")
            with self.assertRaises(model_runtime.RuntimeUnavailable):
                runtime.set_enabled(False)
        finally:
            runtime.release()
        runtime.set_enabled(False)

        self.assertIsNone(created[0]())
        self.assertEqual(runtime.snapshot()["state"], "unloaded")
        runtime.set_enabled(True)
        self.assertEqual(len(created), 2)
        runtime.set_enabled(False)

    def test_unload_clears_global_helper_models_and_tensor_caches(self):
        modules = {name: types.ModuleType(name) for name in (
            "text", "text.chinese2", "fast_langdetect.infer", "module.mel_processing", "GPT_SoVITS.TTS_infer_pack.TTS", "torch")}
        modules["text"].chinese2 = modules["text.chinese2"]
        modules["text.chinese2"].g2pw = Pipeline()
        model = weakref.ref(modules["text.chinese2"].g2pw)
        modules["fast_langdetect.infer"]._default_detector = types.SimpleNamespace(_models={"full": Pipeline()})
        modules["module.mel_processing"].mel_basis = {"cuda": Pipeline()}
        modules["module.mel_processing"].hann_window = {"cuda": Pipeline()}
        modules["GPT_SoVITS.TTS_infer_pack.TTS"].resample_transform_dict = {"cuda": Pipeline()}
        cleared = []
        modules["torch"].cuda = types.SimpleNamespace(is_initialized=lambda: True, empty_cache=lambda: cleared.append(True))
        with patch.dict(sys.modules, modules):
            model_runtime.release_model_memory()
            self.assertIsNone(model())
            self.assertNotIn("text.chinese2", sys.modules)
            self.assertFalse(hasattr(modules["text"], "chinese2"))
            self.assertEqual(modules["fast_langdetect.infer"]._default_detector._models, {})
            self.assertEqual(modules["module.mel_processing"].mel_basis, {})
            self.assertEqual(modules["module.mel_processing"].hann_window, {})
            self.assertEqual(modules["GPT_SoVITS.TTS_infer_pack.TTS"].resample_transform_dict, {})
            self.assertEqual(cleared, [True])
    def test_load_failure_is_reported_and_retry_is_possible(self):
        attempts = []
        allocated = []
        def create():
            attempts.append(True)
            pipeline = Pipeline()
            allocated.append(weakref.ref(pipeline))
            if len(attempts) == 1:
                raise ValueError("controlled failure")
            return pipeline
        runtime = model_runtime.ModelRuntime(create, gc.collect)
        with self.assertRaisesRegex(model_runtime.RuntimeUnavailable, "加载失败") as failure:
            runtime.set_enabled(True)
        self.assertIsNotNone(failure.exception)
        self.assertIsNone(allocated[0](), "加载失败的异常回溯不应保留部分模型")
        self.assertEqual(runtime.snapshot()["state"], "failed")
        runtime.set_enabled(True)
        self.assertEqual(runtime.snapshot()["state"], "loaded")
        runtime.set_enabled(False)

    def test_private_api_install_is_idempotent_and_rejects_unknown_entry(self):
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            api = work / "api_v2.py"
            api.write_text("tts_pipeline = TTS(tts_config)\n\nAPP = FastAPI()\n")
            model_runtime.prepare_runtime(work)
            first = api.read_text()
            compile(first, str(api), "exec")
            self.assertTrue((work / "_meowlive_model_runtime.py").is_file())
            model_runtime.prepare_runtime(work)
            self.assertEqual(api.read_text(), first)
            api.write_text("unknown upstream entry")
            with self.assertRaises(ValueError):
                model_runtime.prepare_runtime(work)


class ModelMiddlewareTests(unittest.IsolatedAsyncioTestCase):
    async def test_prepared_inference_does_not_block_status_or_repeat_synthesis(self):
        entered = threading.Event()
        release = threading.Event()
        calls = []
        def synthesize(request):
            calls.append(request)
            entered.set()
            release.wait(1)
            return "audio"
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            api = work / "api_v2.py"
            api.write_text("tts_pipeline = TTS(tts_config)\n\nAPP = FastAPI()\n"
                           "async def tts_handle(req: dict):\n    return synthesize(req)\n")
            model_runtime.prepare_runtime(work)
            source = api.read_text()
            # Avoid loading real FastAPI, CUDA and weights; execute the prepared handler.
            source = source[source.index("async def tts_handle"): ] if "@run_blocking" not in source else source[source.index("@run_blocking"): ]
            namespace = {"synthesize": synthesize, "run_blocking": getattr(model_runtime, "run_blocking", None)}
            exec(compile(source, str(api), "exec"), namespace)
            task = asyncio.create_task(namespace["tts_handle"]({"text": "一条弹幕"}))
            await asyncio.to_thread(entered.wait, 1)
            try:
                self.assertTrue(entered.is_set())
                self.assertFalse(task.done(), "状态轮询必须能在合成结束前运行")
                self.assertEqual(len(calls), 1)
            finally:
                release.set()
                self.assertEqual(await task, "audio")
            self.assertEqual(len(calls), 1)

    async def test_cancelled_stream_releases_pipeline_lease(self):
        runtime = model_runtime.ModelRuntime(Pipeline, gc.collect)
        runtime.set_enabled(True)
        entered = threading.Event()
        finish = threading.Event()
        def generator_work():
            entered.set()
            finish.wait(3)
        async def app(scope, receive, send):
            await asyncio.to_thread(generator_work)
        middleware = model_runtime.ModelLeaseMiddleware(app, runtime)
        task = asyncio.create_task(middleware({"type": "http", "path": "/tts"}, None, None))
        await asyncio.to_thread(entered.wait, 1)
        try:
            self.assertTrue(entered.is_set())
            task.cancel()
            with self.assertRaises(asyncio.CancelledError):
                await task
            with self.assertRaises(model_runtime.RuntimeUnavailable):
                runtime.set_enabled(False)
        finally:
            finish.set()
        for _ in range(100):
            await asyncio.sleep(0.01)
            if not runtime._lock.locked():
                break
        self.assertEqual(runtime.set_enabled(False)["state"], "unloaded")


if __name__ == "__main__":
    unittest.main()
