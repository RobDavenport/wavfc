# wavfc

A generic, dimension-agnostic Wave Function Collapse library for Rust.

[![no_std](https://img.shields.io/badge/no__std-compatible-green.svg)](https://doc.rust-lang.org/reference/names/preludes.html#the-no_std-attribute)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)

**[Live Demo](https://robdavenport.github.io/wavfc/)** -- try it in your browser (compiled to WebAssembly)

## Overview

**wavfc** implements the [Wave Function Collapse](https://github.com/mxgmn/WaveFunctionCollapse)
algorithm as a generic Rust library. Instead of being locked to a specific grid
type, the solver works with *any* topology you can describe: 2D grids, 3D voxels,
hex maps, spherical grids, or arbitrary graphs.

Key capabilities:

- **Dimension-agnostic** -- works with 2D grids, 3D voxels, hex maps, spherical grids, arbitrary graphs
- **Dual propagation engines** -- AC-3 (low memory) and AC-4 (fast)
- **Configurable backtracking** -- none, restart, or chronological
- **Cell selection heuristics** -- minimum count or Shannon entropy
- **Global constraint hooks** for advanced non-local constraint logic
- **Observable solve process** via callbacks
- **`no_std` compatible** -- only requires `alloc`

## Quick Start

Add `wavfc` to your `Cargo.toml`:

```toml
[dependencies]
wavfc = "0.1"
```

Here is a minimal example that solves a 1D chain with alternating states:

```rust
use wavfc::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

// Define a simple 1D chain topology
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Dir { Left, Right }

struct Chain { len: usize }

impl Topology for Chain {
    type Direction = Dir;
    fn num_cells(&self) -> usize { self.len }
    fn directions(&self) -> &[Dir] { &[Dir::Left, Dir::Right] }
    fn neighbors(&self, cell: usize) -> impl Iterator<Item = (usize, Dir)> {
        let mut result = Vec::new();
        if cell > 0 { result.push((cell - 1, Dir::Left)); }
        if cell + 1 < self.len { result.push((cell + 1, Dir::Right)); }
        result.into_iter()
    }
    fn opposite(&self, dir: Dir) -> Dir {
        match dir { Dir::Left => Dir::Right, Dir::Right => Dir::Left }
    }
    fn direction_index(&self, dir: Dir) -> usize {
        match dir { Dir::Left => 0, Dir::Right => 1 }
    }
}

// Define simple rules: 2 states that alternate
struct AlternatingRules;

impl AdjacencyRules for AlternatingRules {
    type Direction = Dir;
    fn num_states(&self) -> usize { 2 }
    fn compatible(&self, a: usize, b: usize, _dir: Dir) -> bool { a != b }
}

// Solve!
let config = SolverConfig::new();
let mut solver = WfcSolver::new(Chain { len: 10 }, &AlternatingRules, config).unwrap();
let mut rng = StdRng::seed_from_u64(42);
let result = solver.solve(&mut rng).unwrap();

// Every adjacent pair alternates
for i in 0..9 {
    assert_ne!(result.state(i), result.state(i + 1));
}
```

## Features

- **Topology-agnostic**: Implement the `Topology` trait for any graph structure.
  Cells are plain `usize` indices connected by typed directions.

- **Dual propagation**: Choose AC-3 (low memory overhead, recomputes support sets)
  or AC-4 (maintains support counters for O(1) removal) via `PropagatorKind`.

- **Backtracking**: `BacktrackStrategy::None` fails immediately on contradiction.
  `BacktrackStrategy::Restart { max_restarts }` re-initializes and retries.
  `BacktrackStrategy::Chronological { max_depth }` saves snapshots and backtracks
  to try alternate choices.

- **Heuristics**: `Heuristic::MinCount` selects the cell with the fewest remaining
  states (fast, good results). `Heuristic::ShannonEntropy` uses weighted Shannon
  entropy for better distribution at slightly higher cost.

- **Pre-seeding**: Pin specific cells to known states before solving via
  `SolverConfig::seed(cell, state)`.

- **Global constraints**: Inject custom non-local constraint logic by implementing
  the `GlobalConstraint` trait. Constraints run after each propagation pass and
  can further restrict cell possibilities.

- **Observation**: Monitor solve progress by implementing the `Observer` trait.
  Receive callbacks on collapse, propagation, contradiction, and backtrack events.

## Feature Flags

```toml
[features]
default = []
std = []  # Future: enables std-dependent features
```

The library is `no_std` by default and only requires `alloc`. Enable the `std`
feature if you need standard-library-dependent functionality (reserved for future use).

## Performance

Target benchmarks from the project specification:

| Metric | Target |
|--------|--------|
| 64x64 2D grid, 16 states | < 5 ms |
| 128x128 2D grid, 64 states | < 50 ms |
| 32x32x32 3D grid, 32 states | < 200 ms |
| Propagation throughput | > 10M cell-state-removals/sec |

Run benchmarks locally with:

```sh
cargo bench
```

## API Overview

| Type | Role |
|------|------|
| `Topology` | Trait -- define your world graph (cells + directions) |
| `AdjacencyRules` | Trait -- define which states may be adjacent |
| `SolverConfig` | Builder -- configure propagator, heuristic, backtracking, seeds |
| `WfcSolver` | Core solver -- call `solve(&mut rng)` or `step(&mut rng)` |
| `SolveResult` | Output -- collapsed state per cell + iteration count |
| `PropagatorKind` | Enum -- `Ac3` or `Ac4` |
| `BacktrackStrategy` | Enum -- `None`, `Restart`, `Chronological` |
| `Heuristic` | Enum -- `MinCount` or `ShannonEntropy` |
| `Observer` | Trait -- receive solve-progress callbacks |
| `GlobalConstraint` | Trait -- inject non-local constraints |
| `BitSet128` | 128-bit fixed bitset (up to 128 states) |
| `WfcError` | Error enum -- `Contradiction`, `TooManyStates`, `InvalidPin` |

## License

Licensed under either of

- [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
- [MIT License](http://opensource.org/licenses/MIT)

at your option.
