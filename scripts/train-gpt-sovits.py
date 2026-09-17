#!/usr/bin/env python3
"""Run installed upstream v2 stages in a job-owned workspace; JSONL is progress only."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import traceback
import unicodedata
import uuid
import wave
from engine_workspace import assets, environment, workspace, configure_low_memory, configure_training_loaders
from training_transcription import transcribe_audio_files


def emit(state, progress):
    print(json.dumps({"state": state, "progress": progress}), flush=True)


def owned_file(root, relative, maximum):
    path = root / relative
    if Path(relative).is_absolute() or ".." in Path(relative).parts or path.is_symlink():
        raise ValueError("任务文件路径无效")
    if root not in path.resolve(strict=True).parents or not path.is_file() or not 0 < path.stat().st_size <= maximum:
        raise ValueError("任务文件缺失或超限")
    return path


def load_job(path):
    root = path.resolve(strict=True).parent
    if path.is_symlink() or path.stat().st_size > 512 * 1024:
        raise ValueError("任务清单无效")
    job = json.loads(path.read_text())
    if job.get("schema") != 1 or job.get("model_version") != "v2" or str(uuid.UUID(job["id"], version=4)) != job["id"]:
        raise ValueError("仅接受 v2 任务清单")
    if job.get("artifacts") != {"gpt": "artifacts/gpt.ckpt", "sovits": "artifacts/sovits.pth"}:
        raise ValueError("模型必须保存到固定成对路径")
    if job.get("fp16") is not True:
        raise ValueError("GPU 训练必须使用 fp16=true")
    performance = job.get("performance")
    if performance is None:
        if job.get("batch_size") != 1:
            raise ValueError("旧训练清单的 batch_size 必须为 1")
        performance = {
            "batch_size": 1,
            "data_workers": 1,
            "cpu_threads": 2,
            "gpu_index": 0,
            "low_memory": True,
        }
    if not isinstance(performance, dict) or set(performance) != {
        "batch_size", "data_workers", "cpu_threads", "gpu_index", "low_memory"
    }:
        raise ValueError("训练性能参数格式无效")
    bounded = {
        "batch_size": (1, 16),
        "data_workers": (0, 8),
        "cpu_threads": (1, 16),
        "gpu_index": (0, 15),
    }
    for key, (minimum, maximum) in bounded.items():
        if type(performance[key]) is not int or not minimum <= performance[key] <= maximum:
            raise ValueError("训练性能参数超出范围")
    if type(performance["low_memory"]) is not bool:
        raise ValueError("低显存模式必须为布尔值")
    job["performance"] = performance.copy()
    for key in ("gpt_epochs", "sovits_epochs"):
        if type(job[key]) is not int or not 1 <= job[key] <= 20:
            raise ValueError("训练轮次须为 1–20")
    if not 2 <= len(job["clips"]) <= 32:
        raise ValueError("需要 2–32 个音频片段")
    text_mode = job.get("text_mode", "reviewed_text")
    if text_mode not in {"audio_only", "reviewed_text"}:
        raise ValueError("训练文本模式无效")
    total, paths = 0, set()
    for index, clip in enumerate(job["clips"]):
        if clip["path"] != f"clips/{index:03}.wav" or clip["path"] in paths:
            raise ValueError("训练片段路径或顺序无效")
        paths.add(clip["path"])
        audio = owned_file(root, clip["path"], 2 * 1024 * 1024)
        total += audio.stat().st_size
        text = clip["text"]
        if not isinstance(text, str) or len(text) > 500 or any(unicodedata.category(c) == "Cc" or c == "|" for c in text):
            raise ValueError("训练文本无效")
        if (text_mode == "audio_only" and text != "") or (text_mode == "reviewed_text" and not text.strip()):
            raise ValueError("训练文本与所选模式不一致")
        if clip["language"] not in {"zh", "en", "ja", "ko", "yue"}:
            raise ValueError("训练语言无效")
        with wave.open(str(audio)) as wav:
            if wav.getsampwidth() != 2 or wav.getnchannels() not in {1, 2} or not 8000 <= wav.getframerate() <= 48000 or not 3 <= wav.getnframes() / wav.getframerate() <= 10:
                raise ValueError("片段须为 3–10 秒 PCM16 WAV")
            if not any(wav.readframes(wav.getnframes())):
                raise ValueError("片段不能静音")
    if total > 32 * 1024 * 1024:
        raise ValueError("训练素材总量超过 32 MiB")
    training_models(root, job, {})
    return root, job


def training_models(root, job, defaults):
    """Only trainer initialization changes; semantic preprocessing keeps its base encoder."""
    base = job.get("base")
    if base is None:
        return defaults
    if (not isinstance(base, dict) or base.get("voice_id") != job.get("voice_id")
            or not isinstance(base.get("voice_id"), str) or not base["voice_id"]
            or base.get("job_id") == job["id"]):
        raise ValueError("续训版本必须属于同一参考音色")
    try:
        parent = uuid.UUID(base["job_id"])
        if parent.version != 4 or str(parent) != base["job_id"]:
            raise ValueError("续训版本标识无效")
    except (KeyError, TypeError, AttributeError, ValueError) as error:
        raise ValueError("续训版本标识无效") from error
    directory = root / "base"
    if directory.is_symlink() or not directory.is_dir():
        raise ValueError("续训权重目录缺失或无效")
    result = dict(defaults)
    hashes = base.get("sha256")
    if not isinstance(hashes, dict):
        raise ValueError("续训权重缺少校验信息")
    for key, filename in (("gpt", "gpt.ckpt"), ("sovits", "sovits.pth")):
        if base.get(key) != f"base/{filename}":
            raise ValueError("续训权重必须位于本次任务目录")
        path = owned_file(root, base[key], 8 * 1024**3)
        digest = hashlib.sha256()
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(chunk)
        actual = digest.hexdigest()
        if hashes.get(key) != actual:
            raise ValueError("续训权重校验失败，请检查上一版本的模型文件")
        result[key] = path
    return result


def complete_transcripts(root, job, engine_root):
    if job.get("text_mode", "reviewed_text") != "audio_only":
        return
    if any(clip["text"] != "" for clip in job["clips"]):
        raise ValueError("纯音频模式不能覆盖已有文本")
    requests = [(owned_file(root, clip["path"], 2 * 1024 * 1024), clip["language"]) for clip in job["clips"]]
    results = transcribe_audio_files(requests, engine_root, root)
    for clip, result in zip(job["clips"], results):
        clip["text"] = result["text"]


def run_stage(script, args, work, env):
    # Python 3.10 multiprocessing appends /pymp-XXXXXXXX/listener-XXXXXXXX.
    # A UUID job's work/cache/tmpdir already exceeds the Unix socket budget.
    temporary_root = Path(__file__).resolve().parents[1] / "data/tmp"
    temporary_root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="gsv-", dir=temporary_root) as temporary:
        if len(os.fsencode(temporary)) > 70:
            raise ValueError("项目路径过长，训练进程通信需要更短的项目路径")
        stage_env = {**env, "TMPDIR": temporary}
        # Rust owns this process group. Children intentionally inherit the group.
        with subprocess.Popen([sys.executable, "-s", str(script), *args], cwd=work, env=stage_env,
                              stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                              stderr=subprocess.STDOUT) as process:
            with (work.parent / "stages.log").open("ab") as log:
                while data := process.stdout.read1(8192):
                    remaining = max(0, 256 * 1024 - log.tell())
                    log.write(data[:remaining])
                    log.flush()
            code = process.wait()
    if code:
        raise RuntimeError(f"训练阶段失败：{script.name}，退出码 {code}；参见任务 stages.log")


def validate_prepared(root, job):
    import torch
    prepared = root / "prepared"
    text_file = owned_file(root, "prepared/2-name2text-0.txt", 1024 * 1024)
    semantic_file = owned_file(root, "prepared/6-name2semantic-0.tsv", 1024 * 1024)
    text = text_file.read_text().strip().splitlines()
    semantic = semantic_file.read_text().strip().splitlines()
    expected = {Path(c["path"]).name for c in job["clips"]}
    if {line.split("\t")[0] for line in text} != expected or {line.split("\t")[0] for line in semantic} != expected:
        raise ValueError("预处理没有产生全部片段，不能开始训练")
    for line in semantic:
        columns = line.split("\t")
        if len(columns) != 2 or not columns[1].split() or any(not token.isdigit() or not 0 <= int(token) < 1024 for token in columns[1].split()):
            raise ValueError("语义 token 无效")
    for clip in job["clips"]:
        name = Path(clip["path"]).name
        files = [f"prepared/4-cnhubert/{name}.pt"]
        if clip["language"] == "zh":
            files.append(f"prepared/3-bert/{name}.pt")
        for filename in files:
            tensor = torch.load(owned_file(root, filename, 64 * 1024 * 1024), map_location="cpu", weights_only=True)
            if not isinstance(tensor, torch.Tensor) or not tensor.numel() or not torch.isfinite(tensor).all():
                raise ValueError("预处理张量无效")
        owned_file(root, f"prepared/5-wav32k/{name}", 2 * 1024 * 1024)
    (prepared / "2-name2text.txt").write_text("\n".join(text) + "\n")
    (prepared / "6-name2semantic.tsv").write_text("item_name\tsemantic_audio\n" + "\n".join(semantic) + "\n")


def configurations(root, job, work, models):
    import yaml
    models = training_models(root, job, models)
    prepared = root / "prepared"
    s2 = json.loads((work / "GPT_SoVITS/configs/s2.json").read_text())
    performance = job["performance"]
    s2["train"].update(batch_size=performance["batch_size"], epochs=job["sovits_epochs"], fp16_run=True, segment_size=10240,
                       pretrained_s2G=str(models["sovits"]), pretrained_s2D=str(models["discriminator"]),
                       if_save_latest=True, if_save_every_weights=True, save_every_epoch=1,
                       gpu_numbers=str(performance["gpu_index"]), text_low_lr_rate=0.4, grad_ckpt=False)
    s2["model"]["version"] = s2["version"] = "v2"
    s2["data"]["exp_dir"] = str(prepared)
    s2["data"].update(num_workers=performance["data_workers"], persistent_workers=False,
                      prefetch_factor=None)
    s2.update(s2_ckpt_dir=str(prepared), save_weight_dir=str(root / "weights-sovits"), name=job["id"])
    (prepared / "logs_s2_v2").mkdir(parents=True, exist_ok=True)
    s1 = yaml.safe_load((work / "GPT_SoVITS/configs/s1longer-v2.yaml").read_text())
    s1["train"].update(batch_size=performance["batch_size"], epochs=job["gpt_epochs"], precision="16-mixed", save_every_n_epoch=1,
                       if_save_every_weights=True, if_save_latest=True, if_dpo=False,
                       half_weights_save_dir=str(root / "weights-gpt"), exp_name=job["id"])
    s1.update(pretrained_s1=str(models["gpt"]), train_semantic_path=str(prepared / "6-name2semantic.tsv"),
              train_phoneme_path=str(prepared / "2-name2text.txt"), output_dir=str(root / "logs_s1_v2"))
    s1["data"].update(num_workers=performance["data_workers"], persistent_workers=False,
                      prefetch_factor=None)
    (root / "weights-sovits").mkdir(exist_ok=True)
    (root / "weights-gpt").mkdir(exist_ok=True)
    (root / "s2.json").write_text(json.dumps(s2, ensure_ascii=False, indent=2))
    (root / "s1.yaml").write_text(yaml.safe_dump(s1, allow_unicode=True))


def finalize(root, job, work):
    import torch
    sys.path.insert(0, str(work / "GPT_SoVITS"))
    from utils import HParams
    outputs = {"gpt": root / "weights-gpt" / f"{job['id']}-e{job['gpt_epochs']}.ckpt"}
    candidates = list((root / "weights-sovits").glob(f"{job['id']}_e{job['sovits_epochs']}_s*.pth"))
    if len(candidates) != 1:
        raise ValueError("缺少唯一的最终 SoVITS 权重")
    outputs["sovits"] = candidates[0]
    hashes = {}
    for name, source in outputs.items():
        owned_file(root, str(source.relative_to(root)), 2 * 1024**3)
        with torch.serialization.safe_globals([HParams]):
            checkpoint = torch.load(source, weights_only=True, map_location="cpu")
        weights = checkpoint.get("weight")
        if not isinstance(weights, dict) or not weights or not all(isinstance(t, torch.Tensor) and t.numel() and torch.isfinite(t).all() for t in weights.values()):
            raise ValueError("训练权重为空或包含非有限值")
        if name == "sovits" and tuple(weights["enc_p.text_embedding.weight"].shape) != (732, 192):
            raise ValueError("SoVITS 权重与 v2 不兼容")
        config = plain(checkpoint["config"])
        if name == "gpt" and config["model"]["phoneme_vocab_size"] != 732:
            raise ValueError("GPT 权重与 v2 不兼容")
        checkpoint["config"] = config
        destination = root / job["artifacts"][name]
        with destination.open("xb") as handle:
            torch.save(checkpoint, handle)
        hashes[name] = hashlib.sha256(destination.read_bytes()).hexdigest()
    with (root / "artifacts/manifest.json").open("x") as handle:
        json.dump({"job_id": job["id"], "model_version": "v2", "sha256": hashes}, handle, indent=2)


def plain(value):
    if hasattr(value, "items"):
        return {key: plain(item) for key, item in value.items()}
    if isinstance(value, (tuple, list)):
        return [plain(item) for item in value]
    return value


def main():
    # Rust cancels the entire process group. Unwind the runner so stage scratch
    # directories are removed after Popen has reaped the terminated child.
    signal.signal(signal.SIGTERM, lambda number, _frame: sys.exit(128 + number))
    parser = argparse.ArgumentParser()
    parser.add_argument("--job", type=Path, required=True)
    parser.add_argument("--engine-root", type=Path, default=os.environ.get("MEOWLIVE_GPT_SOVITS_ROOT"))
    args = parser.parse_args()
    if not args.engine_root:
        raise ValueError("缺少 GPT-SoVITS 安装路径")
    root, job = load_job(args.job)
    emit("preparing", 1)
    try:
        complete_transcripts(root, job, args.engine_root)
    except Exception:
        print(json.dumps({"state": "failed", "error": "transcription_failed", "progress": 1}), flush=True)
        raise
    models = assets(args.engine_root)
    work = workspace(args.engine_root, root / "work")
    configure_training_loaders(work)
    configure_low_memory(work, job["performance"]["low_memory"])
    env = environment(work, models, job["performance"])
    os.environ.update({k: v for k, v in env.items() if k in {"HF_HUB_OFFLINE", "TRANSFORMERS_OFFLINE", "PYTHONDONTWRITEBYTECODE"}})
    import torch
    if not torch.cuda.is_available():
        raise RuntimeError("此训练预设需要可用 CUDA GPU")
    emit("preparing", 5)
    dataset = root / "dataset.list"
    dataset.write_text("\n".join(f"{root / c['path']}|speaker|{c['language']}|{c['text']}" for c in job["clips"]) + "\n")
    env.update(inp_text=str(dataset), inp_wav_dir=str(root / "clips"), exp_name=job["id"],
               opt_dir=str(root / "prepared"), i_part="0", all_parts="1", is_half="True",
               bert_pretrained_dir=str(models["bert"]), cnhubert_base_dir=str(models["hubert"]),
               pretrained_s2G=str(models["sovits"]), s2config_path=str(work / "GPT_SoVITS/configs/s2.json"))
    for index, name in enumerate(("1-get-text.py", "2-get-hubert-wav32k.py", "3-get-semantic.py")):
        run_stage(work / "GPT_SoVITS/prepare_datasets" / name, [], work, env)
        emit("preparing", 10 + index * 10)
    validate_prepared(root, job)
    configurations(root, job, work, models)
    emit("training", 40)
    run_stage(work / "GPT_SoVITS/s2_train.py", ["--config", str(root / "s2.json")], work, env)
    emit("training", 70)
    run_stage(work / "GPT_SoVITS/s1_train.py", ["--config_file", str(root / "s1.yaml")], work, env)
    emit("validating", 95)
    finalize(root, job, work)
    emit("succeeded", 100)


if __name__ == "__main__":
    try:
        main()
    except Exception:
        traceback.print_exc(file=sys.stderr)
        sys.exit(1)
