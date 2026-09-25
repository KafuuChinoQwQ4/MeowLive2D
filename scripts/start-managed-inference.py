#!/usr/bin/env python3
"""Launch GPT-SoVITS with a private working copy and mutable inference config."""
import argparse
import json
import os
from pathlib import Path
import sys
from engine_workspace import assets, configure_inference_memory, environment, workspace
from model_runtime import prepare_runtime


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine-root", type=Path, required=True)
    parser.add_argument("--data-dir", type=Path, required=True)
    parser.add_argument("--model-root", type=Path)
    parser.add_argument("--port", type=int, default=9880)
    parser.add_argument("--device", choices=["cuda", "cpu"], default="cuda")
    parser.add_argument("--memory-mode", choices=["low", "standard"], default="low")
    parser.add_argument("--prepare-only", action="store_true")
    args = parser.parse_args()
    if not 1024 <= args.port <= 65535:
        parser.error("port must be between 1024 and 65535")
    models = assets(args.engine_root, args.model_root)
    root = args.data_dir.resolve()
    work = workspace(args.engine_root, root / "work", args.model_root)
    configure_inference_memory(work, args.memory_mode)
    prepare_runtime(work)
    env = environment(work, models)
    # Forward only this explicit startup flag through the inference environment allowlist.
    env["MEOWLIVE_AUTO_ENABLE_MODELS"] = "1" if os.environ.get("MEOWLIVE_AUTO_ENABLE_MODELS") == "1" else "0"
    config = root / "tts-infer.yaml"
    # JSON is valid YAML. The upstream API will update only this private config.
    config.write_text(json.dumps({"custom": {
        "version": "v2", "device": args.device, "is_half": args.device == "cuda",
        "bert_base_path": str(models["bert"]), "cnhuhbert_base_path": str(models["hubert"]),
        "t2s_weights_path": str(models["gpt"]), "vits_weights_path": str(models["sovits"]),
    }}, indent=2))
    print(f"受管推理配置：{config}", flush=True)
    print("自动加载：服务启动时加载所选语音模型；训练前可在面板关闭模型。"
          if env.get("MEOWLIVE_AUTO_ENABLE_MODELS") == "1"
          else "模型待机：服务启动后请在面板启用语音模型；关闭模型可释放权重内存。", flush=True)
    print("低内存推理：使用轻量中文注音，不加载 G2PW 多音字模型。" if args.memory_mode == "low"
          else "标准推理：加载 G2PW 多音字模型，需要更多内存。", flush=True)
    if args.prepare_only:
        return
    os.chdir(work)
    os.execve(sys.executable, [sys.executable, "-s", str(work / "api_v2.py"), "-a", "127.0.0.1",
                              "-p", str(args.port), "-c", str(config)], env)


if __name__ == "__main__":
    main()
