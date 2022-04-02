# crafty

A FFXIV crafting experiment.

## Debugging

Running benchmarks:

```sh
cargo bench
```

Flamegraphs can be generated and viewed by following the steps in this [blog post](https://www.jibbow.com/posts/criterion-flamegraphs).

```sh
cargo bench --profile release --bench benchmark -- --profile-time=10
```

## Compiling to WASM

`wasm-pack` is used to generate an ES6 module for consumption in Node.js or browsers; it can be found [here](https://rustwasm.github.io/wasm-pack/installer). Javascript bindings can be generated from the `web` crate:

```sh
wasm-pack build web --release --target web --out-name crafty
```

Note that `wasm-pack` will run `wasm-opt` (a [wasm optimizer](https://github.com/WebAssembly/binaryen/releases)) automatically if it is added to PATH. For this project we aim to optimize mainly for performance, and not code size.

