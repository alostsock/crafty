# crafty

A very experimental crafting rotation solver for FFXIV.

## How it works

The simulator implemented for this project was roughly rewritten from [the Teamcraft simulator](https://github.com/ffxiv-teamcraft/simulator) and is reasonably accurate, but has some rounding errors compared to actual in-game crafts.

The solver uses a basic [Monte Carlo tree search](https://en.wikipedia.org/wiki/Monte_Carlo_tree_search) (MCTS) algorithm with some hand-coded crafting rotation logic to better guide the search. Usually this method is used for two-player games like chess or go -- this solver is a single-agent variant.

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

`wasm-pack` is used to generate an ES6 module for usage in Node.js or browsers; it can be found [here](https://rustwasm.github.io/wasm-pack/installer). Javascript bindings can be generated from the `web` crate:

```sh
wasm-pack build web --release --target web --out-name crafty
```

Note that `wasm-pack` will automatically run [`wasm-opt`](https://github.com/WebAssembly/binaryen/releases) if it is installed and added to `$PATH`. For this project we aim to optimize mainly for performance, and not code size.

## Relevant work

[Schadd, Maarten PD, et al. "Single-player Monte-Carlo tree search for SameGame." Knowledge-Based Systems 34 (2012): 3-11.](http://www.schadd.com/Papers/2012SameGame.pdf)

[Browne, Cameron B., et al. "A survey of monte carlo tree search methods." IEEE Transactions on Computational Intelligence and AI in games 4.1 (2012): 1-43.](http://repository.essex.ac.uk/4117/1/MCTS-Survey.pdf)

[Chaslot, Guillaume MJ-B., Mark HM Winands, and H. J. V. D. Herik. "Parallel monte-carlo tree search." International Conference on Computers and Games. Springer, Berlin, Heidelberg, 2008.](https://dke.maastrichtuniversity.nl/m.winands/documents/multithreadedMCTS2.pdf)

[Cazenave, Tristan, and Nicolas Jouandeau. "On the parallelization of UCT." Computer games workshop. 2007.](https://hal.archives-ouvertes.fr/hal-02310186/document)

[Gelly, Sylvain, and David Silver. "Monte-Carlo tree search and rapid action value estimation in computer Go." Artificial Intelligence 175.11 (2011): 1856-1875.](https://www.sciencedirect.com/science/article/pii/S000437021100052X)