#!/bin/bash
set -euo pipefail
umask 022
export DEBIAN_FRONTEND=noninteractive
ROOT=/opt/meowlive-voice
REVISION=48b1a0169a28582a8984402f82cf438d3bfa6aca
mkdir -p "$ROOT/runtime"
python3 - "$ROOT/runtime/operation.pid" "$$" <<'PYJOB'
import json,pathlib,sys
pid=int(sys.argv[2]); start=pathlib.Path(f"/proc/{pid}/stat").read_text().rsplit(")",1)[1].split()[19]
pathlib.Path(sys.argv[1]).write_text(json.dumps({"pid":pid,"start":start,"boot":pathlib.Path("/proc/sys/kernel/random/boot_id").read_text().strip()}))
PYJOB
# apt changes only OS prerequisites; all Python dependencies are isolated in venv.
apt-get update
apt-get install -y python3 python3-venv python3-dev git ffmpeg build-essential libsndfile1 cmake
python3 -c 'import sys; assert (3,10) <= sys.version_info[:2] <= (3,12), "需要 Python 3.10–3.12 的 Ubuntu 发行版"'
if [ -e "$ROOT/engine" ]; then
  test -d "$ROOT/engine/.git" || { echo '专用引擎目录含非受管内容，拒绝覆盖'; exit 1; }
  test "$(git -C "$ROOT/engine" remote get-url origin)" = https://github.com/RVC-Boss/GPT-SoVITS.git
  test -z "$(git -C "$ROOT/engine" status --porcelain)" || { echo '专用引擎目录有本地修改，拒绝覆盖'; exit 1; }
else
  git clone --no-checkout https://github.com/RVC-Boss/GPT-SoVITS.git "$ROOT/engine"
fi
git -C "$ROOT/engine" fetch --depth 1 origin "$REVISION"
git -C "$ROOT/engine" checkout --detach "$REVISION"
python3 -m venv "$ROOT/venv"
"$ROOT/venv/bin/python" -m pip install --upgrade pip wheel setuptools
"$ROOT/venv/bin/python" -m pip install torch==2.6.0 torchaudio==2.6.0 --index-url https://download.pytorch.org/whl/cu124
"$ROOT/venv/bin/python" -m pip install -r "$ROOT/engine/requirements.txt" faster-whisper huggingface-hub
"$ROOT/venv/bin/python" -m nltk.downloader -e -d "$ROOT/venv/nltk_data" averaged_perceptron_tagger averaged_perceptron_tagger_eng cmudict
"$ROOT/venv/bin/python" -c 'import torch, faster_whisper, transformers, fastapi, librosa, soundfile'
printf '{"schema":1,"revision":"%s"}\n' "$REVISION" > "$ROOT/backend.json"
