"""Project-owned GPT-SoVITS code mirror; no upstream cwd/cache/config writes."""
from pathlib import Path
import os
import re
import shutil
import sys
import sysconfig
import json

GPT = "s1v3.ckpt"
SOVITS = "v2Pro/s2Gv2ProPlus.pth"
DISCRIMINATOR = "v2Pro/s2Dv2ProPlus.pth"
SV = "sv/pretrained_eres2netv2w24s4ep4.ckpt"


def link_inference_assets(destination):
    """Refresh fixed upstream auxiliary paths, including packages downloaded later."""
    destination = Path(destination)
    roots = [Path(root) for root in json.loads((destination / "model-sources.json").read_text())]
    for name in (SV, "s2Gv3.pth", "gsv-v4-pretrained/s2Gv4.pth", "gsv-v4-pretrained/vocoder.pth",
                 "models--nvidia--bigvgan_v2_24khz_100band_256x/bigvgan_generator.pt",
                 "models--nvidia--bigvgan_v2_24khz_100band_256x/config.json"):
        target = destination / "GPT_SoVITS/pretrained_models" / name
        if target.exists():
            continue
        for root in roots:
            source = root / "GPT_SoVITS/pretrained_models" / name
            if source.is_file():
                target.parent.mkdir(parents=True, exist_ok=True)
                if target.is_symlink():
                    target.unlink()
                target.symlink_to(source.resolve())
                break


def assets(engine, models_root=None, version="v2ProPlus"):
    root = Path(models_root or engine).resolve(strict=True) / "GPT_SoVITS/pretrained_models"
    versions = {
        "v2": ("gsv-v2final-pretrained/s1bert25hz-5kh-longer-epoch=12-step=369668.ckpt", "gsv-v2final-pretrained/s2G2333k.pth", "gsv-v2final-pretrained/s2D2333k.pth"),
        "v2Pro": (GPT, "v2Pro/s2Gv2Pro.pth", "v2Pro/s2Dv2Pro.pth"),
        "v2ProPlus": (GPT, SOVITS, DISCRIMINATOR),
        "v3": (GPT, "s2Gv3.pth"),
        "v4": (GPT, "gsv-v4-pretrained/s2Gv4.pth"),
    }
    if version not in versions:
        raise ValueError("尚未接入此模型版本")
    result = dict(zip(("gpt", "sovits", "discriminator"), (root / name for name in versions[version])))
    if version == "v3":
        result["vocoder"] = root / "models--nvidia--bigvgan_v2_24khz_100band_256x/bigvgan_generator.pt"
        result["vocoder_config"] = result["vocoder"].parent / "config.json"
    elif version == "v4":
        result["vocoder"] = root / "gsv-v4-pretrained/vocoder.pth"
    result.update(bert=root / "chinese-roberta-wwm-ext-large", hubert=root / "chinese-hubert-base",
                  language=root / "fast_langdetect/lid.176.bin")
    if version in {"v2Pro", "v2ProPlus"}:
        result["sv"] = root / SV
    for value in result.values():
        if not value.exists():
            raise ValueError(f"本地 {version} 预训练文件缺失：{value.name}")
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
    project = Path(__file__).resolve().parents[1]
    libraries = {model_source.parent, project / "data/models", project / "models"}
    sources = [model_source, engine]
    for library in sorted(libraries):
        sources.extend(library / ("gpt-sovits-" + version) for version in ("v2", "v2pro", "v2proplus", "v3", "v4"))
    (destination / "model-sources.json").write_text(json.dumps([str(root) for root in sources]))
    shutil.copyfile(Path(__file__), destination / "_meowlive_engine_workspace.py")
    link_inference_assets(destination)
    # Text disambiguation need not occupy CUDA alongside the two TTS models.
    onnx = destination / "GPT_SoVITS/text/g2pw/onnx_api.py"
    code = onnx.read_text()
    marker = 'providers=["CUDAExecutionProvider", "CPUExecutionProvider"]'
    if marker in code:
        onnx.write_text(code.replace(marker, 'providers=["CPUExecutionProvider"]'))
    elif 'providers=["CPUExecutionProvider"]' not in code:
        raise ValueError("上游中文推理入口已变化，请检查低显存配置")
    return destination


def environment(work, models, performance=None, version="v2ProPlus"):
    performance = performance or {
        "cpu_threads": 2,
        "gpu_index": 0,
    }
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
               CUDA_VISIBLE_DEVICES=str(performance["gpu_index"]),
               _CUDA_VISIBLE_DEVICES=str(performance["gpu_index"]),
               OMP_NUM_THREADS=str(performance["cpu_threads"]),
               version=version, hz="25hz", bert_path=str(models["bert"]), GRADIO_ANALYTICS_ENABLED="False")
    return env


def configure_low_memory(work, enabled=True):
    """Change only optimizer implementation in the owned copy; upstream stays read-only."""
    if not enabled:
        return
    path = work / "GPT_SoVITS/s2_train.py"
    original = path.read_text()
    marker = "        eps=hps.train.eps,\n"
    replacement = marker + "        foreach=False,  # MeowLive: avoid Adam tensor-list peak on 6GB GPUs\n"
    if "MeowLive: avoid Adam" in original:
        return
    if original.count(marker) != 2:
        raise ValueError("上游优化器入口已变化，请检查训练兼容性")
    path.write_text(original.replace(marker, replacement))


def configure_training_loaders(work):
    """Make both upstream training loaders honor bounded job-owned settings."""
    sovits = work / "GPT_SoVITS/s2_train.py"
    code = sovits.read_text()
    markers = [
        ("        num_workers=5,\n", "        num_workers=hps.data.num_workers,\n"),
        ("        persistent_workers=True,\n        prefetch_factor=3,\n",
         "        persistent_workers=hps.data.persistent_workers,\n        prefetch_factor=hps.data.prefetch_factor,\n"),
    ]
    for original, replacement in markers:
        if replacement in code:
            continue
        if code.count(original) != 1:
            raise ValueError("上游 SoVITS 数据加载入口已变化，请检查训练兼容性")
        code = code.replace(original, replacement)
    sovits.write_text(code)

    gpt = work / "GPT_SoVITS/AR/data/data_module.py"
    code = gpt.read_text()
    original = ("            num_workers=self.num_workers,\n"
                "            persistent_workers=True,\n"
                "            prefetch_factor=16,\n")
    replacement = ("            num_workers=self.num_workers,\n"
                   "            persistent_workers=self.config[\"data\"][\"persistent_workers\"],\n"
                   "            prefetch_factor=self.config[\"data\"][\"prefetch_factor\"],\n")
    if replacement not in code:
        if code.count(original) != 1:
            raise ValueError("上游 GPT 数据加载入口已变化，请检查训练兼容性")
        code = code.replace(original, replacement)
    validation_original = ("            num_workers=max(self.num_workers, 12),\n"
                           "            persistent_workers=True,\n"
                           "            prefetch_factor=16,\n")
    validation_replacement = ("            num_workers=self.num_workers,\n"
                              "            persistent_workers=self.config[\"data\"][\"persistent_workers\"],\n"
                              "            prefetch_factor=self.config[\"data\"][\"prefetch_factor\"],\n")
    if validation_original in code:
        code = code.replace(validation_original, validation_replacement)
    gpt.write_text(code)


def configure_inference_memory(work, mode):
    """Select the upstream pypinyin fallback only in our owned inference copy."""
    if mode not in {"low", "standard"}:
        raise ValueError("未知推理内存模式")
    path = work / "GPT_SoVITS/text/chinese2.py"
    original = path.read_text()
    updated, count = re.subn(r"(?m)^is_g2pw = (?:True|False)(?=\s*(?:#|$))",
                             "is_g2pw = " + str(mode == "standard"), original)
    if count != 1:
        raise ValueError("上游中文推理入口已变化，无法应用内存模式；请检查引擎兼容性")
    if updated != original:
        path.write_text(updated)


def configure_inference_audio(work):
    """Avoid librosa's first-request JIT in the owned reference-audio reader."""
    path = Path(work) / "GPT_SoVITS/TTS_infer_pack/TTS.py"
    source = path.read_text()
    marker = "            wav16k, sr = librosa.load(ref_wav_path, sr=16000)"
    replacement = (
        "            # MeowLive: reuse the already loaded Torch audio frontend.\n"
        "            ref_wave, ref_sr = torchaudio.load(ref_wav_path)\n"
        "            ref_wave = ref_wave.float().mean(dim=0)\n"
        "            if ref_sr != 16000:\n"
        "                ref_wave = torchaudio.functional.resample(ref_wave, ref_sr, 16000)\n"
        "            wav16k, sr = ref_wave.numpy(), 16000"
    )
    if source.count(marker) == 1:
        path.write_text(source.replace(marker, replacement))
    elif replacement not in source:
        raise ValueError("上游参考音频入口已变化，无法配置低延迟读取")


def configure_lora_training(work):
    """Load the immutable previous adapter after wrapping the base flow model."""
    path = work / "GPT_SoVITS/s2_train_v3_lora.py"
    code = path.read_text()
    marker = "        net_g.cfm = get_peft_model(net_g.cfm, lora_config)\n"
    replacement = marker + (
        "        # MeowLive: job-owned LoRA continuation\n"
        "        if getattr(hps.train, 'pretrained_lora', ''):\n"
        "            previous = torch.load(hps.train.pretrained_lora, map_location='cpu', weights_only=True)\n"
        "            if previous.get('lora_rank') != hps.train.lora_rank:\n"
        "                raise ValueError('LoRA continuation rank mismatch')\n"
        "            result = net_g.load_state_dict(previous['weight'], strict=False)\n"
        "            if result.unexpected_keys:\n"
        "                raise ValueError('LoRA continuation keys mismatch')\n"
    )
    if "MeowLive: job-owned LoRA continuation" not in code:
        if code.count(marker) != 2:
            raise ValueError("上游 LoRA 入口已变化，请检查训练兼容性")
        # Only the fresh initialization branch; automatic resumes have their own checkpoint.
        pos = code.rindex(marker)
        code = code[:pos] + code[pos:].replace(marker, replacement, 1)
    code, count = re.subn(r'(?m)^    worker_count = .+$', '    worker_count = hps.data.num_workers', code)
    if count != 1:
        raise ValueError("上游 LoRA 数据加载入口已变化")
    code = code.replace('loader_kwargs["persistent_workers"] = True', 'loader_kwargs["persistent_workers"] = False')
    code = code.replace('loader_kwargs["prefetch_factor"] = 2 if os.name == "nt" else 3', 'loader_kwargs["prefetch_factor"] = 2')
    path.write_text(code)
