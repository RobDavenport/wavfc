# wavfc — Product Requirements Document

**Project:** `wavfc` — A generic, dimension-agnostic Wave Function Collapse library for Rust
**Author:** Zerve / Nethercore Systems
**Status:** Draft v1
**Date:** 2026-02-20

---

## 1. Overview

`wavfc` is a high-performance, `no_std`-compatible Rust library implementing the Wave Function Collapse (WFC) algorithm over arbitrary graph topologies. The library enforces zero opinions about dimensionality, tile semantics, or world structure — users define their own topology, adjacency rules, and constraints via traits.

The crate ships as a pure solver library with no built-in topologies or tile definitions. A companion WASM-based demo application validates the library and showcases 2D and 3D generation in the browser.

---

## 2. Goals

- **Dimension-agnostic:** Operate on abstract graph topologies. 2D grids, 3D voxel spaces, hex maps, spherical grids, 4D+ hypercubes, and arbitrary graphs are all valid use cases — the solver doesn't know or care.
- **Fast:** Bitset-driven constraint propagation, precomputed adjacency tables, minimal allocations on the hot path.
- **`no_std` compatible:** Core library requires only `alloc`, not `std`. Users can run WFC on embedded targets or in WASM without pulling in the standard library.
- **Ergonomic trait-based API:** Users implement small, focused traits. The library handles the rest.
- **Extensible constraint model:** Support local adjacency constraints (core WFC), pre-seeding/pinning, post-validation, and optional global constraint hooks for advanced use cases.

---

## 3. Non-Goals (v1)

- No built-in topologies (Grid2D, Grid3D, HexGrid, etc.) in the core crate
- No parallelism or multithreading (future feature flag)
- No `serde` serialization (future feature flag)
- No overlapping-model WFC (tile/simple model only)
- No GUI, rendering, or visualization in the core crate

---

## 4. Architecture

### 4.1 Core Concept: Graph, Not Grid

WFC operates on a set of **cells**, each with a set of **possible states**. Cells have **neighbor relationships** in named **directions**. The algorithm enforces **pairwise adjacency constraints** — "can state A be adjacent to state B in direction D?" — via iterative constraint propagation.

This naturally maps to a directed labeled graph. A 2D grid is just a graph where each cell has 4 neighbors (N/S/E/W). A wrapping torus is the same graph with edges connecting opposite boundaries. A sphere is a geodesic mesh where cells have 5–6 neighbors. The solver is agnostic to all of this.

### 4.2 Trait Surface

#### `Topology`

Defines the world's structure.

```rust
pub trait Topology {
    type Direction: Copy + Eq + core::fmt::Debug;

    /// Total number of cells in the world.
    fn num_cells(&self) -> usize;

    /// All valid direction values. Used to precompute adjacency tables.
    fn directions(&self) -> &[Self::Direction];

    /// Returns an iterator of (neighbor_cell_index, direction_from_self_to_neighbor)
    /// for a given cell.
    fn neighbors(&self, cell: usize) -> impl Iterator<Item = (usize, Self::Direction)>;

    /// The reverse direction. If A is North of B, then B is South of A.
    fn opposite(&self, dir: Self::Direction) -> Self::Direction;

    /// Map a direction to a dense index in 0..directions().len().
    /// Used for indexing into precomputed adjacency tables.
    fn direction_index(&self, dir: Self::Direction) -> usize;
}
```

**Design note:** This trait fully supports wrapping worlds, toroidal grids, spherical topologies, Möbius strips, and any exotic geometry. The solver never assumes boundaries exist — if a cell has no neighbor in a direction, it simply doesn't yield that neighbor from `neighbors()`.

#### `AdjacencyRules`

Defines which states can be adjacent.

```rust
pub trait AdjacencyRules {
    type Direction: Copy + Eq;

    /// Number of distinct tile/state types.
    fn num_states(&self) -> usize;

    /// Can `state_a` exist in a cell when the neighbor in `direction` has `state_b`?
    fn compatible(&self, state_a: usize, state_b: usize, direction: Self::Direction) -> bool;

    /// Optional: relative weight/frequency of a state. Higher = more likely to be chosen.
    /// Default implementation returns 1.0 for all states (uniform).
    fn weight(&self, state: usize) -> f64 { let _ = state; 1.0 }
}
```

**Design note:** The `compatible` method is called once per (state, state, direction) triple during precomputation — never during the hot solve loop. The solver converts this into precomputed bitmask tables at construction time.

#### `Observer` (Optional)

Callback for monitoring solve progress.

```rust
pub trait Observer<T: Topology> {
    /// Called after a cell is collapsed to a single state.
    fn on_collapse(&mut self, cell: usize, state: usize);

    /// Called after propagation completes for a step.
    fn on_propagation_complete(&mut self);

    /// Called when a contradiction is detected.
    fn on_contradiction(&mut self, cell: usize);

    /// Called when backtracking occurs.
    fn on_backtrack(&mut self, depth: usize);
}
```

#### `GlobalConstraint` (Optional, Advanced)

Allows users to inject non-local constraint logic into the solve loop.

```rust
pub trait GlobalConstraint<T: Topology> {
    /// Called after each propagation pass. May further restrict cell possibilities.
    /// Return Err(Contradiction) to trigger backtracking.
    fn enforce(&self, state: &mut SolverState<T>) -> Result<(), Contradiction>;
}
```

**Warning:** This is a power-user API. Incorrect implementations can cause unsolvable states or violate solver invariants. Documentation must clearly explain the contract.

### 4.3 Internal Representation

#### States

States are represented as `usize` indices in `0..num_states()`. Users map their domain-specific tile types to/from indices externally. This enables bitset-based storage and avoids generic type parameters on the hot path.

#### Bitset

Fixed-size inline bitset supporting up to 128 states (v1):

```
BitSet128 = [u64; 2]   // 128 bits, 16 bytes, fits in a register pair
```

Operations required:
- `set(bit)`, `clear(bit)`, `test(bit)`
- `count_ones()` (popcount)
- `bitor_assign`, `bitand_assign`
- `is_empty()`, `is_singleton()`
- `iter_ones()` — iterate set bits
- `clear_all()`, `set_all(n)` — reset to empty / full with n bits

All operations must be branchless or SIMD-friendly where possible.

#### Precomputed Adjacency Table

At solver construction, the `AdjacencyRules` trait is evaluated exhaustively to build:

```
compatibility: Vec<BitSet128>
// Indexed as: compatibility[state * num_directions + dir_index]
// Value: bitset of all states compatible with `state` when looking in direction `dir`
```

This table is used by both propagation engines:
- **AC-3:** Propagation reduces to bulk bitwise OR/AND operations over these masks.
- **AC-4:** Used to compute initial support counters at construction time.

#### AC-4 Support Counters (optional, AC-4 only)

When the AC-4 propagator is selected, the solver additionally maintains:

```
support_count: Vec<u16>
// Indexed as: support_count[cell * num_states * num_dirs + state * num_dirs + dir]
// Value: number of states in the neighboring cell (in direction dir) that support `state`
```

Initialized at construction by scanning the compatibility table against each cell's initial possibilities. During propagation, counters are decremented in O(1) per removal. When a counter hits 0, the corresponding state is dead and removed.

This array is **not allocated** when using the AC-3 propagator.

#### Solver State

```rust
pub struct SolverState<T: Topology> {
    /// Per-cell bitset of remaining possible states
    possibilities: Vec<BitSet128>,

    /// Per-cell count of remaining possibilities (avoids popcount on hot path)
    entropy_cache: Vec<u32>,

    /// AC-4 only: support counters per (cell, state, direction).
    /// None when using AC-3 propagation.
    /// Indexed as: support_count[cell * num_states * num_dirs + state * num_dirs + dir]
    support_count: Option<Vec<u16>>,

    /// Propagation stack: (cell_index, removed_state)
    propagation_stack: Vec<(usize, usize)>,

    /// Number of fully collapsed cells
    collapsed_count: usize,

    // ... topology reference, compatibility table, etc.
}
```

### 4.4 Algorithm

```
1. INITIALIZE
   - Build precomputed adjacency bitmask table from AdjacencyRules
   - Set all cells to "all states possible"
   - Apply any pre-seeded/pinned cells → propagate

2. OBSERVE
   - Find the uncollapsed cell with lowest entropy (fewest remaining states)
   - Break ties randomly (via user-provided RNG)
   - If no uncollapsed cells remain → DONE (success)

3. COLLAPSE
   - Choose one state from the cell's remaining possibilities
   - Selection is weighted by AdjacencyRules::weight()
   - Remove all other states from the cell

4. PROPAGATE (two engines, user-selectable via SolverConfig)
   
   The library ships two propagation engines behind a shared `Propagator` trait.
   They produce identical results but have very different resource profiles:
   
   **AC-3 (compute-optimized) — low memory, higher CPU per removal:**
   When a state is removed from a cell, recompute the full support set for each 
   neighbor via bitwise OR over compatibility bitmasks. 
   
   - Push all removed (cell, state) pairs onto the propagation stack
   - While stack is non-empty:
     - Pop (cell, removed_state)
     - For each neighbor (neighbor_cell, direction):
       - Recompute support: OR of `compatibility[s * num_dirs + dir]` for all s 
         still possible in cell
       - AND neighbor's possibilities with support set
       - If any states were removed from neighbor, push them onto the stack
       - If neighbor has 0 remaining states → CONTRADICTION
   
   Cost per removal: O(remaining_states) per neighbor (bitwise ops, very fast in practice).
   Memory overhead: O(states × directions) for the compatibility table only (~8KB typical).
   **Best for:** memory-constrained targets (fantasy consoles, embedded, WASM with 
   tight budgets), small-to-medium grids, tilesets with few states.
   
   **AC-4 (memory-optimized for speed) — high memory, O(1) per removal:**
   Maintain a support counter array: `support_count[cell][state][dir]` = number of 
   states in the neighboring cell that are compatible with this state.
   
   - Push all removed (cell, state) pairs onto the propagation stack
   - While stack is non-empty:
     - Pop (cell, removed_state)
     - For each neighbor (neighbor_cell, direction):
       - For each state `s` still possible in neighbor_cell:
         - If `compatible(s, removed_state, opposite(direction))`:
           - Decrement `support_count[neighbor_cell][s][dir]`
           - If count reaches 0: remove `s` from neighbor_cell, push onto stack
       - If neighbor has 0 remaining states → CONTRADICTION
   
   Cost per removal: O(1) after initialization.
   Memory overhead: O(cells × states × directions × 2 bytes).
   **Best for:** large grids, many states, desktop/server targets with ample RAM.
   
   **Memory comparison (128×128 grid, 128 states, 4 directions):**
   
   | Component | AC-3 | AC-4 |
   |-----------|------|------|
   | Compatibility table | 8 KB | 8 KB |
   | Possibilities array | 256 KB | 256 KB |
   | Support counters | — | **16 MB** |
   | **Total** | **~264 KB** | **~16.3 MB** |
   
   Both propagators must produce identical results — this invariant is enforced 
   by the test suite (run same inputs through both, assert identical outputs).

5. HANDLE CONTRADICTION
   - If backtracking enabled: restore snapshot, try different state
   - If backtracking disabled: return Err with partial state + diagnostic info

6. GLOBAL CONSTRAINTS (if any registered)
   - After propagation settles, call each GlobalConstraint::enforce()
   - If any constraint removes states, return to propagation (step 4)

7. REPEAT from step 2
```

### 4.5 Backtracking

Three modes, user-selectable:

| Mode | Behavior | Use Case |
|------|----------|----------|
| `None` | Fail immediately on contradiction, return partial state | Tilesets known to be contradiction-free, or user wants max speed |
| `Restart` | Re-initialize solver with new RNG seed, retry up to N times | Simple, works for most tilesets |
| `Chronological` | Snapshot state before each collapse, restore on contradiction, try next state | Most robust, higher memory cost |

Chronological backtracking stores snapshots of `possibilities` and `entropy_cache`. A configurable depth limit prevents unbounded memory growth.

### 4.6 Cell Selection Heuristics

| Heuristic | Description | Performance |
|-----------|-------------|-------------|
| `MinCount` (default) | Cell with fewest remaining states, random tie-break | Fast, good results |
| `ShannonEntropy` | Cell with lowest Shannon entropy (accounts for weights) | Better distribution, ~2x slower |

User-selectable via solver configuration.

---

## 5. API Surface

### 5.1 Solver Construction

```rust
let solver = WfcSolver::new(topology, rules, config);
```

Where `config` includes:
- Backtracking strategy + depth limit
- Cell selection heuristic
- Optional: pre-seeded cells `Vec<(cell_index, state)>`
- Optional: `Observer` implementation
- Optional: `GlobalConstraint` implementations

### 5.2 Solve

```rust
let result: Result<SolveResult, WfcError> = solver.solve(&mut rng);
```

`SolveResult` provides:
- `state(cell) -> usize` — the collapsed state for each cell
- Iteration / step count statistics

`WfcError` provides:
- The contradiction location (cell index)
- The partial solver state for inspection
- Backtrack depth reached (if applicable)

### 5.3 Incremental / Partial Solve

```rust
// Solve one step at a time (for visualization or interleaved logic)
let step_result = solver.step(&mut rng);
// Returns: Collapsed(cell, state) | Propagated | Contradiction(cell) | Complete
```

### 5.4 Pre-Seeding

```rust
solver.pin(cell_index, state)?;
// Immediately propagates constraints from the pinned cell.
// Returns Err if the pin causes a contradiction.
```

### 5.5 Post-Validation

```rust
let result = solver.solve_with_validation(&mut rng, |state| {
    // Return true if the completed output satisfies global requirements
    has_entrance(state) && has_exit(state) && is_connected(state)
}, max_retries);
```

---

## 6. Crate Structure

```
wavfc/
├── Cargo.toml
├── src/
│   ├── lib.rs              # #![no_std], extern crate alloc, public API re-exports
│   ├── bitset.rs           # BitSet128 implementation
│   ├── topology.rs         # Topology trait definition
│   ├── rules.rs            # AdjacencyRules trait + precomputation logic
│   ├── constraint.rs       # GlobalConstraint trait
│   ├── observer.rs         # Observer trait
│   ├── solver.rs           # Core WFC solver (observe/collapse/propagate loop)
│   ├── propagator.rs       # AC-3 and AC-4 propagation engines + Propagator trait
│   ├── backtrack.rs        # Backtracking strategies + snapshot management
│   ├── entropy.rs          # Cell selection heuristics (MinCount, Shannon)
│   ├── config.rs           # SolverConfig builder
│   └── error.rs            # WfcError, Contradiction types
├── tests/
│   ├── bitset_tests.rs
│   ├── solver_basic.rs     # Simple 1D chain, small grids
│   ├── solver_wrapping.rs  # Toroidal / wrapping topologies
│   ├── solver_backtrack.rs # Contradiction + recovery tests
│   ├── solver_propagator.rs # AC-3 vs AC-4 equivalence tests
│   ├── solver_weighted.rs  # Weighted tile selection
│   ├── solver_pinning.rs   # Pre-seeding / partial solve
│   └── solver_global.rs   # GlobalConstraint integration
├── benches/
│   └── propagation.rs      # Criterion benchmarks for propagation throughput
└── README.md
```

### Feature Flags

```toml
[features]
default = []
std = []                  # Enables std-dependent features (future: threading)
serde = ["dep:serde"]     # Serialization of solver state, rules
```

### Dependencies (minimal)

```toml
[dependencies]
rand_core = { version = "0.6", default-features = false }  # RNG trait, no_std

[dev-dependencies]
rand = "0.8"
criterion = "0.5"
```

---

## 7. Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| 64×64 2D grid, 16 states | < 5ms | Typical small map |
| 128×128 2D grid, 64 states | < 50ms | Medium map |
| 32×32×32 3D grid, 32 states | < 200ms | Voxel world chunk |
| Propagation throughput | > 10M cell-state-removals/sec | Core bottleneck |

Benchmarks measured on a modern x86_64 CPU, single-threaded, release mode.

---

## 8. Demo Application: WASM Showcase

### 8.1 Purpose

A browser-based interactive demo that validates the library works end-to-end and showcases WFC generation in both 2D and 3D. This is a separate crate/workspace member, not part of the core library.

### 8.2 Structure

```
demo-wasm/
├── Cargo.toml             # Depends on wavfc, wasm-bindgen, web-sys
├── src/
│   ├── lib.rs             # WASM entry points
│   ├── topology_2d.rs     # Grid2D topology impl (flat + wrapping)
│   ├── topology_3d.rs     # Grid3D topology impl (voxel)
│   ├── tilesets/
│   │   ├── terrain.rs     # 2D terrain tileset (grass, water, sand, forest, etc.)
│   │   └── dungeon.rs     # 2D dungeon tileset (rooms, corridors, walls, doors)
│   ├── renderer_2d.rs     # Canvas2D tile rendering
│   └── renderer_3d.rs     # WebGL / simple 3D voxel rendering
├── www/
│   ├── index.html
│   ├── style.css
│   └── main.js            # UI controls, calls into WASM
└── README.md
```

### 8.3 Features

**2D Demo:**
- Configurable grid size (up to 128×128)
- Toggle wrapping (flat vs toroidal)
- Multiple tilesets: terrain, dungeon
- Weighted tile frequency sliders
- Step-through mode (watch collapse in real-time)
- Regenerate with new seed

**3D Demo:**
- Configurable voxel grid (up to 32×32×32)
- Simple WebGL renderer with orbit camera
- Layer-by-layer slice view
- Voxel tileset: stone, air, ore, cave

**UI Controls:**
- Seed input (reproducible results)
- Backtracking toggle + depth limit
- Heuristic selector (MinCount / Shannon)
- Performance stats overlay (solve time, iterations, backtracks)

### 8.4 Tech Stack

- `wasm-bindgen` + `web-sys` for browser interop
- Canvas 2D API for 2D rendering
- WebGL (raw, no framework) for 3D rendering
- `wasm-pack` for build tooling
- Hosted as static files (GitHub Pages or similar)

---

## 9. Open Questions

The following decisions should be resolved during implementation. They are listed here so Claude Code agent teams can discuss and resolve them as they encounter them.

### OQ-1: Associated Types vs Generics for `Direction`

**Context:** The `Topology` trait currently uses an associated type for `Direction`. This means one topology type = one direction type.

**Alternative:** Make `Direction` a generic parameter on the trait: `Topology<D: Direction>`. This allows a single topology struct to support multiple direction schemes.

**Leaning:** Associated types. One topology naturally has one direction scheme. Generics would add noise to every type signature that touches the solver.

**Resolution criteria:** If we find a concrete use case during implementation where associated types are limiting, switch to generics.

### OQ-2: Contradiction Return Semantics

**Context:** When backtracking is disabled and a contradiction occurs, should the solver return the partial state (all `possibilities` bitsets as they were at the moment of contradiction) or just an error code?

**Leaning:** Return the partial state. It's invaluable for debugging tilesets — users can inspect which cells were already collapsed, which cell contradicted, and what the remaining possibilities looked like. The memory cost is negligible since the state already exists.

**Resolution criteria:** Implement partial state return. If profiling shows the clone/copy is a bottleneck in tight retry loops, add an option to skip it.

### OQ-3: Snapshot Strategy for Chronological Backtracking

**Context:** Chronological backtracking requires saving solver state before each collapse so it can be restored on contradiction. Options:

- **Full snapshot:** Clone entire `possibilities` + `entropy_cache` vectors. Simple, O(num_cells) per snapshot.
- **Delta/diff:** Only record what changed since last snapshot. More complex, lower memory.
- **Copy-on-write:** Use a CoW wrapper around the possibilities array. Amortized cost.

**Leaning:** Full snapshot with a depth limit (default 32 or 64). The vectors are small in practice (128×128 grid × 16 bytes per cell = 256KB per snapshot). Delta tracking adds complexity that may not be worth it for v1.

**Resolution criteria:** Start with full snapshot. If benchmarks show backtracking memory is a problem for large grids, implement delta compression.

### OQ-4: Should `BitSet128` Be Const-Generic?

**Context:** Currently spec'd as a fixed 128-bit set (`[u64; 2]`). Could instead be `BitSet<const N: usize>` where N is the number of u64 words, allowing `BitSet<1>` (64 states), `BitSet<2>` (128), `BitSet<4>` (256), etc.

**Leaning:** Start with fixed `BitSet128`. If users request >128 states, generalize to const-generic. The fixed version is simpler and avoids const-generic complexity bleeding into the solver's type signatures.

**Resolution criteria:** Ship v1 with `BitSet128`. Track requests for >128 states. If demand exists, make it generic in v2.

### OQ-5: ~~Propagation Strategy~~ RESOLVED → Dual Engine, User-Selectable

**Decision:** Both AC-3 and AC-4 are first-class propagation engines, selected via `SolverConfig`. Neither is a "fallback." They serve different resource profiles.

**Rationale:** AC-4's O(1) per-removal is a significant speed win, validated by `ghx_proc_gen`. However, its memory cost is prohibitive for constrained targets. For a 128×128 grid with 128 states and 4 directions, AC-4 needs ~16MB for support counters alone vs AC-3's ~264KB total. Fantasy consoles, embedded targets, and memory-budgeted WASM builds cannot afford this.

AC-3's bitwise OR propagation is still very fast in practice — scanning 128 states over a `[u64; 2]` bitset is 2 OR instructions per state, and the entire operation fits in L1 cache. For tilesets with fewer states (<32), the AC-3 overhead is negligible.

**API:** `SolverConfig` exposes a `propagator` field:
```rust
enum PropagatorKind {
    /// Low memory (~KB), O(remaining_states) per removal. 
    /// Best for constrained targets.
    Ac3,
    /// High memory (~MB), O(1) per removal. 
    /// Best for large grids on desktop/server.
    Ac4,
}
```

**Testability:** Both produce identical results. The test suite runs every integration test through both propagators and asserts equivalence.

---

## 10. Competitive Analysis

The following existing Rust WFC crates were evaluated:

### `wfc` (gridbugs) — crates.io: `wfc`
- 162 stars, ~45K downloads, **last updated ~3 years ago**
- Overlapping model focused (image synthesis), grid-oriented API
- Not `no_std`, not topology-agnostic, no backtracking, restart on contradiction only

### `wave-function-collapse` (AustinHellerRepo) — crates.io: `wave-function-collapse`
- 23 stars, node-based graph model (closest to our approach conceptually)
- Supports multiple solver algorithms including backtracking
- Not `no_std`, not bitset-optimized, no observer/callback, no global constraints, small community

### `ghx_proc_gen` (Henauxg) — crates.io: `ghx_proc_gen`
- 128 stars, **actively maintained**, AC-4 solver, Bevy integration
- Supports Cartesian2D and Cartesian3D with axis looping, observer pattern, pre-seeding
- Not `no_std`, **not topology-agnostic** (hardcoded to Cartesian grid types — no hex, sphere, graph, 4D+), no backtracking (restart only), no global constraint hooks, Bevy-flavored API

### `simple_tiled_wfc` — crates.io: `simple_tiled_wfc`
- 2D grid only, basic implementation, stale

### `microwfc` — crates.io: `microwfc`
- Const-generic N dimensions on a grid, callback support
- Still grid-based (not arbitrary graph), not `no_std`, basic implementation, stale

### Gap Analysis

No existing crate satisfies all of:
- `no_std` + `alloc`
- Truly arbitrary graph topology (user-defined via trait)
- Bitset-optimized AC-4 propagation
- Configurable backtracking (none / restart / chronological)
- Weighted tile selection
- Pre-seeding / pinning
- Global constraint hooks
- Observer callbacks

`ghx_proc_gen` is the closest competitor and its AC-4 implementation validates our architectural choice. The key differentiators of this project are topology-agnosticism and `no_std` support.

---

## 10. Test Strategy

### Unit Tests

- `BitSet128`: all operations, edge cases (bit 0, bit 127, empty, full)
- Precomputation: verify bitmask tables match brute-force `compatible()` calls
- Propagation: small hand-crafted graphs with known outcomes

### Integration Tests

- **1D chain:** Verify WFC on a linear sequence produces valid adjacency everywhere
- **2D grid (small):** 8×8 with known tileset, verify all adjacency constraints hold
- **Wrapping topology:** Toroidal grid, verify edge cells wrap correctly
- **Weighted selection:** Over many runs, verify state frequencies approximate weights
- **Pinning:** Pin cells, verify solver respects them and propagates correctly
- **Contradiction detection:** Tilesets designed to always contradict, verify error returned
- **AC-3/AC-4 equivalence:** Run identical inputs through both propagation algorithms, assert identical collapsed outputs (critical correctness invariant)
- **Backtracking:** Tilesets that require backtracking, verify success with backtracking enabled and failure without
- **Global constraints:** Custom constraint that forces at least one instance of a specific state

### Property Tests (optional, nice-to-have)

- For any completed solve: every adjacent cell pair satisfies `compatible()`
- Pinned cells retain their pinned state in the output
- Weighted generation over N runs: chi-squared test against expected frequencies

### Benchmarks (Criterion)

- `propagation_throughput`: cells processed per second during pure propagation
- `ac3_vs_ac4`: same tileset solved with both propagators, compare wall-clock time
- `solve_2d_small`: end-to-end 32×32, 16 states
- `solve_2d_medium`: end-to-end 128×128, 64 states
- `solve_3d`: end-to-end 32×32×32, 32 states
- `backtracking_overhead`: same tileset with and without backtracking

---

## 11. Implementation Order

Suggested phasing for Claude Code agent teams:

### Phase 1: Foundation
1. `bitset.rs` — `BitSet128` with full test coverage
2. `topology.rs` — trait definition
3. `rules.rs` — trait definition + precomputation
4. `error.rs` — error types

### Phase 2: Core Solver
5. `entropy.rs` — MinCount heuristic
6. `propagator.rs` — Propagator trait + AC-3 engine (simpler, implement first)
7. `solver.rs` — core loop (observe → collapse → propagate), no backtracking
8. Integration tests with simple topologies defined in test helpers

### Phase 3: Features
9. AC-4 propagation engine + AC-3/AC-4 equivalence tests
10. `backtrack.rs` — Restart + Chronological strategies
11. `config.rs` — SolverConfig builder (including PropagatorKind selection)
12. `observer.rs` — Observer trait
13. `constraint.rs` — GlobalConstraint trait
14. Weighted selection, Shannon entropy heuristic
15. Pinning API

### Phase 4: Polish
16. Criterion benchmarks (including AC-3 vs AC-4 comparison)
17. Documentation (rustdoc on all public items)
18. README with examples
19. `no_std` validation (build for a bare-metal target to confirm)

### Phase 5: Demo
20. `demo-wasm/` — 2D topology, terrain tileset, canvas renderer
21. 2D dungeon tileset
22. 3D topology + voxel renderer
23. UI controls + step-through mode
24. GitHub Pages deployment

---

## 12. Success Criteria

- [ ] Core library compiles with `#![no_std]` on `thumbv7em-none-eabihf` (ARM Cortex-M4)
- [ ] All performance targets met (Section 7)
- [ ] 100% of public API items have rustdoc
- [ ] Zero `unsafe` in the core crate (allowed in bitset if needed for SIMD, must be documented)
- [ ] WASM demo loads in browser, generates and renders 2D + 3D worlds
- [ ] Demo includes step-through visualization
- [ ] All integration tests pass
- [ ] Benchmarks run and are tracked in CI