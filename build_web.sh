# seems like wasm-pack runs wasm-opt automatically if it's added to PATH
# wasm-pack: https://rustwasm.github.io/wasm-pack/installer/
# wasm-opt:  https://github.com/WebAssembly/binaryen/releases
wasm-pack build web/ --release --target web