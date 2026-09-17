"""Offline ASR shared by clip review and audio-only training; upstream stays read-only."""
from contextlib import contextmanager, redirect_stdout
import gc
import os
from pathlib import Path
import sys
import unicodedata

LANGUAGES = {"zh", "en", "ja", "ko", "yue"}


def validate_text(text):
    if (not isinstance(text, str) or not text.strip() or len(text) > 500
            or any(unicodedata.category(c) == "Cc" or c == "|" for c in text)):
        raise ValueError("转写文本为空、超过 500 字或包含无效字符，请手动输入并校对")
    return text.strip()


def local_model(engine_root, model=None):
    configured = model or os.environ.get("MEOWLIVE_ASR_MODEL")
    path = Path(configured) if configured else Path(engine_root) / "tools/asr/models/faster-whisper-large-v3"
    if not path.is_absolute():
        raise ValueError("本地 ASR 模型须使用绝对目录路径")
    # A missing tokenizer makes faster-whisper fall back to a remote tokenizer,
    # even when its model constructor receives local_files_only=True.
    # Without its preprocessor config, large-v3 silently gets the 80-band
    # extractor default instead of the 128 bands expected by its encoder.
    for filename in ("model.bin", "config.json", "tokenizer.json", "preprocessor_config.json"):
        if not (path / filename).is_file() or (path / filename).stat().st_size == 0:
            raise ValueError(f"本地 ASR 模型缺少 {filename}：{path}；请配置 MEOWLIVE_ASR_MODEL，运行器不会下载模型")
    return path.resolve(strict=True)


@contextmanager
def asr_environment(work):
    work = Path(work)
    if not work.is_absolute() or work.resolve(strict=True) != work:
        raise ValueError("ASR 工作目录须为真实绝对目录，不能使用符号链接")
    cache = work / ".asr-cache"
    if cache.is_symlink():
        raise ValueError("ASR 缓存目录不能是符号链接")
    cache.mkdir(exist_ok=True)
    values = {"HF_HUB_OFFLINE": "1", "TRANSFORMERS_OFFLINE": "1", "HF_DATASETS_OFFLINE": "1",
              "TOKENIZERS_PARALLELISM": "false", "PYTHONDONTWRITEBYTECODE": "1"}
    for key in ("HF_HOME", "TRANSFORMERS_CACHE", "XDG_CACHE_HOME", "TORCH_HOME", "TMPDIR"):
        target = cache / key.lower()
        if target.is_symlink():
            raise ValueError("ASR 缓存目录不能是符号链接")
        target.mkdir(exist_ok=True)
        values[key] = str(target)
    previous = {key: os.environ.get(key) for key in values}
    bytecode = sys.dont_write_bytecode
    try:
        os.environ.update(values)
        sys.dont_write_bytecode = True
        yield
    finally:
        sys.dont_write_bytecode = bytecode
        for key, value in previous.items():
            if value is None:
                os.environ.pop(key, None)
            else:
                os.environ[key] = value


def transcribe_audio_files(requests, engine_root, work, model=None):
    """Load one CPU model for the batch and release it before training starts."""
    for audio, language in requests:
        audio = Path(audio)
        if (not audio.is_absolute() or audio.is_symlink() or audio.resolve(strict=True) != audio
                or not audio.is_file() or not 0 < audio.stat().st_size <= 2 * 1024 * 1024):
            raise ValueError("转写音频路径无效、文件缺失或超过 2 MiB")
        if language not in LANGUAGES:
            raise ValueError("转写语言无效")
    model_path = local_model(engine_root, model)
    recognizer = None
    with asr_environment(work), redirect_stdout(sys.stderr):
        try:
            try:
                from faster_whisper import WhisperModel
            except ImportError as error:
                raise RuntimeError("本地 Python 环境缺少 faster-whisper，请安装离线语音识别依赖") from error
            recognizer = WhisperModel(str(model_path), device="cpu", compute_type="int8",
                                      cpu_threads=2, num_workers=1, local_files_only=True)
            results = []
            for audio, language in requests:
                if (language not in recognizer.supported_languages
                        or recognizer.hf_tokenizer.token_to_id(f"<|{language}|>") is None):
                    label = "粤语 (yue)" if language == "yue" else language
                    raise ValueError(f"当前本地 ASR 模型不支持{label}，请手动填写文本或配置支持该语言的模型")
                segments, info = recognizer.transcribe(str(audio), language=language, beam_size=5,
                                                      vad_filter=True, condition_on_previous_text=False)
                text = validate_text("".join(segment.text for segment in segments))
                if info.language != language:
                    raise ValueError("转写结果语言与请求不一致，请手动校对")
                results.append({"text": text, "language": language})
            return results
        finally:
            if recognizer is not None:
                recognizer.model.unload_model()
                del recognizer
            gc.collect()
