#!/usr/bin/env python3
"""Emit one JSON transcript from a local WAV; diagnostics go to stderr."""
import argparse
import json
from pathlib import Path
import sys

from training_transcription import LANGUAGES, transcribe_audio_files


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--audio", type=Path, required=True)
    parser.add_argument("--language", choices=sorted(LANGUAGES), required=True)
    parser.add_argument("--engine-root", type=Path, required=True)
    parser.add_argument("--model", type=Path)
    args = parser.parse_args()
    if not args.engine_root.is_absolute():
        raise ValueError("引擎目录须使用绝对路径")
    result = transcribe_audio_files([(args.audio, args.language)], args.engine_root, args.audio.parent, args.model)
    print(json.dumps(result[0], ensure_ascii=False), flush=True)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"自动转写失败：{error}", file=sys.stderr, flush=True)
        sys.exit(1)
