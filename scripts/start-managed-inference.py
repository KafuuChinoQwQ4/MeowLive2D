#!/usr/bin/env python3
"""Launch GPT-SoVITS with a private working copy and mutable inference config."""
import argparse
import json
import os
from pathlib import Path
import sys
from engine_workspace import assets, environment, workspace


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine-root", type=Path, required=True)
    parser.add_argument("--data-dir", type=Path, required=True)
    parser.add_argument("--model-root", type=Path)
    parser.add_argument("--port", type=int, default=9880)
    parser.add_argument("--device", choices=["cuda", "cpu"], default="cuda")
    parser.add_argument("--prepare-only", action="store_true")
    args = parser.parse_args()
    if not 1024 <= args.port <= 65535:
        parser.error("port must be between 1024 and 65535")
    models = assets(args.engine_root, args.model_root)
    root = args.data_dir.resolve()
    work = workspace(args.engine_root, root / "work", args.model_root)
    env = environment(work, models)
    config = root / "tts-infer.yaml"
    # JSON is valid YAML. The upstream API will update only this private config.
    config.write_text(json.dumps({"custom": {
        "version": "v2", "device": args.device, "is_half": args.device == "cuda",
        "bert_base_path": str(models["bert"]), "cnhuhbert_base_path": str(models["hubert"]),
        "t2s_weights_path": str(models["gpt"]), "vits_weights_path": str(models["sovits"]),
    }}, indent=2))
    print(f"受管推理配置：{config}", flush=True)
    if args.prepare_only:
        return
    os.chdir(work)
    os.execve(sys.executable, [sys.executable, "-s", str(work / "api_v2.py"), "-a", "127.0.0.1",
                              "-p", str(args.port), "-c", str(config)], env)


if __name__ == "__main__":
    main()
