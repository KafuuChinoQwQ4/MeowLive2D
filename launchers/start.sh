#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
if ! command -v node >/dev/null || ! command -v npm >/dev/null; then
  echo '请先在 Linux / WSL 安装 Node.js 22.12+ 和 npm 10+，再运行 ./launchers/start.sh。' >&2
  exit 1
fi
if [[ ! -d node_modules/vite ]]; then
  echo '首次启动：正在安装控制面板依赖，请稍候…'
  npm ci
fi
exec npm start -- "$@"
