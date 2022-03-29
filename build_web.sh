# wasm-pack will run wasm-opt automatically if it's added to PATH
# wasm-pack: https://rustwasm.github.io/wasm-pack/installer/
# wasm-opt:  https://github.com/WebAssembly/binaryen/releases
wasm-pack build web/ --release --target web --out-name crafty