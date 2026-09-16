"""Project-owned GPT-SoVITS code mirror; no upstream cwd/cache/config writes."""
from pathlib import Path
import os
import shutil
import sys
import sysconfig

GPT = "gsv-v2final-pretrained/s1bert25hz-5kh-longer-epoch=12-step=369668.ckpt"
SOVITS = "gsv-v2final-pretrained/s2G2333k.pth"
DISCRIMINATOR = "gsv-v2final-pretrained/s2D2333k.pth"


def assets(engine, models_root=None):
    root = Path(models_root or engine).resolve(strict=True) / "GPT_SoVITS/pretrained_models"
    result = {"gpt": root / GPT, "sovits": root / SOVITS, "discriminator": root / DISCRIMINATOR,
              "bert": root / "chinese-roberta-wwm-ext-large", "hubert": root / "chinese-hubert-base", "language": root / "fast_langdetect/lid.176.bin"}
    for value in result.values():
        if not value.exists():
            raise ValueError("本地 v2 预训练文件缺失")
    return result


def workspace(engine, destination, models_root=None):
    engine, destination = Path(engine).resolve(strict=True), Path(destination).resolve()
    model_source = Path(models_root or engine).resolve(strict=True)
    if destination == engine or engine in destination.parents:
        raise ValueError("运行目录必须独立于引擎安装")
    destination.mkdir(parents=True, exist_ok=True)
    # Each directory is real. Large immutable model files alone are symlinked;
    # Python modules/dictionaries are copied so upstream caches stay in this mirror.
    for name in ("GPT_SoVITS", "tools"):
        for folder, dirs, files in os.walk(engine / name, followlinks=False):
            dirs[:] = [d for d in dirs if d not in {"pretrained_models", "__pycache__", ".git", "asr", "uvr5", "logs"}]
            if models_root:
                dirs[:] = [d for d in dirs if d != "G2PWModel"]
            relative = Path(folder).relative_to(engine)
            target = destination / relative
            target.mkdir(parents=True, exist_ok=True)
            for filename in files:
                source, out = Path(folder) / filename, target / filename
                if filename.endswith((".pyc", ".log")) or out.exists():
                    continue
                if source.stat().st_size > 8 * 1024 * 1024 and source.suffix in {".onnx", ".bin", ".pth"}:
                    out.symlink_to(source.resolve())
                else:
                    shutil.copyfile(source, out)
    for filename in ("api_v2.py", "config.py"):
        source = engine / filename
        if source.is_file() and not (destination / filename).exists():
            shutil.copyfile(source, destination / filename)
    if models_root:
        g2pw_source = model_source / "GPT_SoVITS/text/G2PWModel"
        g2pw_target = destination / "GPT_SoVITS/text/G2PWModel"
        g2pw_target.mkdir(parents=True, exist_ok=True)
        for source in g2pw_source.iterdir():
            out = g2pw_target / source.name
            if source.is_file() and not out.exists():
                if source.suffix == ".onnx":
                    out.symlink_to(source.resolve())
                else:
                    shutil.copyfile(source, out)
    if not (destination / "GPT_SoVITS/text/G2PWModel/g2pW.onnx").exists():
        raise ValueError("本地 G2PW 中文模型缺失，请预先安装；运行器不下载模型")
    language_source = model_source / "GPT_SoVITS/pretrained_models/fast_langdetect/lid.176.bin"
    if not language_source.is_file():
        raise ValueError("缺少本地语言识别模型，运行器不下载模型")
    language_target = destination / "GPT_SoVITS/pretrained_models/fast_langdetect/lid.176.bin"
    language_target.parent.mkdir(parents=True, exist_ok=True)
    if not language_target.exists():
        language_target.symlink_to(language_source.resolve())
    # Text disambiguation need not occupy CUDA alongside the two TTS models.
    onnx = destination / "GPT_SoVITS/text/g2pw/onnx_api.py"
    code = onnx.read_text()
    marker = 'providers=["CUDAExecutionProvider", "CPUExecutionProvider"]'
    if marker in code:
        onnx.write_text(code.replace(marker, 'providers=["CPUExecutionProvider"]'))
    elif 'providers=["CPUExecutionProvider"]' not in code:
        raise ValueError("上游中文推理入口已变化，请检查低显存配置")
    return destination


def environment(work, models):
    env = {key: value for key, value in os.environ.items() if key in {
        "PATH", "LANG", "LC_ALL", "LD_LIBRARY_PATH", "SYSTEMROOT", "WINDIR", "HOME", "USERPROFILE",
        "CUDA_HOME", "CUDA_PATH", "SSL_CERT_FILE", "REQUESTS_CA_BUNDLE"
    }}
    for key in ("HF_HOME", "TRANSFORMERS_CACHE", "NUMBA_CACHE_DIR", "TMPDIR", "XDG_CACHE_HOME", "MPLCONFIGDIR", "TORCH_HOME"):
        path = work / "cache" / key.lower()
        path.mkdir(parents=True, exist_ok=True)
        env[key] = str(path)
    libraries = [Path(sys.prefix) / "lib", Path(sysconfig.get_paths()["purelib"]) / "torch/lib"]
    libraries.extend(sorted((Path(sysconfig.get_paths()["purelib"]) / "nvidia").glob("*/lib")))
    env["LD_LIBRARY_PATH"] = os.pathsep.join(str(path) for path in libraries if path.is_dir())
    env.update(PYTHONPATH=os.pathsep.join([str(work), str(work / "GPT_SoVITS")]),
               PYTHONDONTWRITEBYTECODE="1", PYTHONUNBUFFERED="1", HF_HUB_OFFLINE="1",
               TRANSFORMERS_OFFLINE="1", HF_DATASETS_OFFLINE="1", TOKENIZERS_PARALLELISM="false",
               CUDA_VISIBLE_DEVICES="0", _CUDA_VISIBLE_DEVICES="0", OMP_NUM_THREADS="2",
               version="v2", hz="25hz", bert_path=str(models["bert"]), GRADIO_ANALYTICS_ENABLED="False")
    return env


def configure_low_memory(work):
    """Change only optimizer implementation in the owned copy; upstream stays read-only."""
    path = work / "GPT_SoVITS/s2_train.py"
    original = path.read_text()
    marker = "        eps=hps.train.eps,\n"
    replacement = marker + "        foreach=False,  # MeowLive: avoid Adam tensor-list peak on 6GB GPUs\n"
    if "MeowLive: avoid Adam" in original:
        return
    if original.count(marker) != 2:
        raise ValueError("上游优化器入口已变化，请检查训练兼容性")
    path.write_text(original.replace(marker, replacement))
