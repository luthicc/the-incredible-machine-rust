#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
bash src/web/build.sh
printf 'Open http://127.0.0.1:8765 and supply the original game ZIP.\n'
python3 -m http.server 8765 --bind 127.0.0.1 --directory web
