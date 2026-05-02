#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

rm -rf www/pkg

export RUSTFLAGS='--cfg getrandom_backend="wasm_js"'
wasm-pack build --target=web --out-dir=www/pkg
