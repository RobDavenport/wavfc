//! Criterion benchmarks for wavfc propagation and solve performance.

use criterion::{criterion_group, criterion_main, Criterion};
use rand::rngs::StdRng;
use rand::SeedableRng;
use wavfc::*;

// ---------------------------------------------------------------------------
// Benchmark topology: 2D grid
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Dir2D {
    North,
    South,
    East,
    West,
}

struct BenchGrid2D {
    width: usize,
    height: usize,
}

impl Topology for BenchGrid2D {
    type Direction = Dir2D;

    fn num_cells(&self) -> usize {
        self.width * self.height
    }

    fn directions(&self) -> &[Dir2D] {
        &[Dir2D::North, Dir2D::South, Dir2D::East, Dir2D::West]
    }

    fn neighbors(&self, cell: usize) -> impl Iterator<Item = (usize, Dir2D)> {
        let x = cell % self.width;
        let y = cell / self.width;
        let mut result = Vec::new();
        if y > 0 {
            result.push((cell - self.width, Dir2D::North));
        }
        if y + 1 < self.height {
            result.push((cell + self.width, Dir2D::South));
        }
        if x + 1 < self.width {
            result.push((cell + 1, Dir2D::East));
        }
        if x > 0 {
            result.push((cell - 1, Dir2D::West));
        }
        result.into_iter()
    }

    fn opposite(&self, dir: Dir2D) -> Dir2D {
        match dir {
            Dir2D::North => Dir2D::South,
            Dir2D::South => Dir2D::North,
            Dir2D::East => Dir2D::West,
            Dir2D::West => Dir2D::East,
        }
    }

    fn direction_index(&self, dir: Dir2D) -> usize {
        match dir {
            Dir2D::North => 0,
            Dir2D::South => 1,
            Dir2D::East => 2,
            Dir2D::West => 3,
        }
    }
}

// ---------------------------------------------------------------------------
// Benchmark rules: all states compatible with all states
// ---------------------------------------------------------------------------

struct BenchRules {
    num_states: usize,
}

impl AdjacencyRules for BenchRules {
    type Direction = Dir2D;

    fn num_states(&self) -> usize {
        self.num_states
    }

    fn compatible(&self, _a: usize, _b: usize, _dir: Dir2D) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// Benchmark functions
// ---------------------------------------------------------------------------

fn benchmarks(c: &mut Criterion) {
    // 1. Small grid: 32x32, 16 states, AC-3
    c.bench_function("solve_32x32_16states_ac3", |b| {
        b.iter(|| {
            let grid = BenchGrid2D {
                width: 32,
                height: 32,
            };
            let rules = BenchRules { num_states: 16 };
            let config = SolverConfig::new().propagator(PropagatorKind::Ac3);
            let mut solver = WfcSolver::new(grid, &rules, config).unwrap();
            let mut rng = StdRng::seed_from_u64(42);
            solver.solve(&mut rng).unwrap();
        })
    });

    // 2. Medium grid: 128x128, 64 states, AC-3
    c.bench_function("solve_128x128_64states_ac3", |b| {
        b.iter(|| {
            let grid = BenchGrid2D {
                width: 128,
                height: 128,
            };
            let rules = BenchRules { num_states: 64 };
            let config = SolverConfig::new().propagator(PropagatorKind::Ac3);
            let mut solver = WfcSolver::new(grid, &rules, config).unwrap();
            let mut rng = StdRng::seed_from_u64(42);
            solver.solve(&mut rng).unwrap();
        })
    });

    // 3. AC-3 vs AC-4 comparison on small grid
    c.bench_function("solve_32x32_16states_ac4", |b| {
        b.iter(|| {
            let grid = BenchGrid2D {
                width: 32,
                height: 32,
            };
            let rules = BenchRules { num_states: 16 };
            let config = SolverConfig::new().propagator(PropagatorKind::Ac4);
            let mut solver = WfcSolver::new(grid, &rules, config).unwrap();
            let mut rng = StdRng::seed_from_u64(42);
            solver.solve(&mut rng).unwrap();
        })
    });

    // 4. Small grid with Restart backtracking
    c.bench_function("solve_32x32_16states_restart", |b| {
        b.iter(|| {
            let grid = BenchGrid2D {
                width: 32,
                height: 32,
            };
            let rules = BenchRules { num_states: 16 };
            let config = SolverConfig::new()
                .propagator(PropagatorKind::Ac3)
                .backtrack(BacktrackStrategy::Restart { max_restarts: 5 });
            let mut solver = WfcSolver::new(grid, &rules, config).unwrap();
            let mut rng = StdRng::seed_from_u64(42);
            solver.solve(&mut rng).unwrap();
        })
    });
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
