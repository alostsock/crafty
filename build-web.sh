#!/bin/sh
set -eu

wasm-pack build web --release --target web --out-name crafty
sed -i 's/"name": "web"/"name": "crafty"/g' web/pkg/package.json
