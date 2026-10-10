#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
cargo build --locked --release --target wasm32-unknown-unknown --bin tim-bevy
mkdir -p web/pkg
cp src/web/index.html src/web/icon.svg web/
wasm-bindgen --target web --no-typescript --out-name tim_bevy --out-dir web/pkg \
  "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/tim-bevy.wasm"
python3 src/web/bundle.py web web/TheIncredibleMachine.html
