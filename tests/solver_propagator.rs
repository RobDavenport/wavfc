use wavfc::*;
use rand::SeedableRng;
use rand::rngs::StdRng;

// ---- 1D Chain Topology ----

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Dir1D {
    Left,
    Right,
}

const DIRS_1D: [Dir1D; 2] = [Dir1D::Left, Dir1D::Right];

struct Chain1D {
    len: usize,
}

impl Topology for Chain1D {
    type Direction = Dir1D;

    fn num_cells(&self) -> usize {
        self.len
    }

    fn directions(&self) -> &[Dir1D] {
        &DIRS_1D
    }

    fn neighbors(&self, cell: usize) -> impl Iterator<Item = (usize, Dir1D)> {
        let mut result = Vec::new();
        if cell > 0 {
            result.push((cell - 1, Dir1D::Left));
        }
        if cell + 1 < self.len {
            result.push((cell + 1, Dir1D::Right));
        }
        result.into_iter()
    }

    fn opposite(&self, dir: Dir1D) -> Dir1D {
        match dir {
            Dir1D::Left => Dir1D::Right,
            Dir1D::Right => Dir1D::Left,
        }
    }

    fn direction_index(&self, dir: Dir1D) -> usize {
        match dir {
            Dir1D::Left => 0,
            Dir1D::Right => 1,
        }
    }
}

// ---- 2D Grid Topology ----

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Dir2D {
    North,
    South,
    East,
    West,
}

const DIRS_2D: [Dir2D; 4] = [Dir2D::North, Dir2D::South, Dir2D::East, Dir2D::West];

struct Grid2D {
    width: usize,
    height: usize,
}

impl Topology for Grid2D {
    type Direction = Dir2D;

    fn num_cells(&self) -> usize {
        self.width * self.height
    }

    fn directions(&self) -> &[Dir2D] {
        &DIRS_2D
    }

    fn neighbors(&self, cell: usize) -> impl Iterator<Item = (usize, Dir2D)> {
        let x = cell % self.width;
        let y = cell / self.width;
        let mut result = Vec::new();

        if y > 0 {
            result.push(((y - 1) * self.width + x, Dir2D::North));
        }
        if y + 1 < self.height {
            result.push(((y + 1) * self.width + x, Dir2D::South));
        }
        if x + 1 < self.width {
            result.push((y * self.width + x + 1, Dir2D::East));
        }
        if x > 0 {
            result.push((y * self.width + x - 1, Dir2D::West));
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

// ---- Wrapping 2D Grid Topology ----

struct WrappingGrid2D {
    width: usize,
    height: usize,
}

impl Topology for WrappingGrid2D {
    type Direction = Dir2D;

    fn num_cells(&self) -> usize {
        self.width * self.height
    }

    fn directions(&self) -> &[Dir2D] {
        &DIRS_2D
    }

    fn neighbors(&self, cell: usize) -> impl Iterator<Item = (usize, Dir2D)> {
        let x = cell % self.width;
        let y = cell / self.width;

        let north_y = if y == 0 { self.height - 1 } else { y - 1 };
        let south_y = if y + 1 == self.height { 0 } else { y + 1 };
        let east_x = if x + 1 == self.width { 0 } else { x + 1 };
        let west_x = if x == 0 { self.width - 1 } else { x - 1 };

        vec![
            (north_y * self.width + x, Dir2D::North),
            (south_y * self.width + x, Dir2D::South),
            (y * self.width + east_x, Dir2D::East),
            (y * self.width + west_x, Dir2D::West),
        ]
        .into_iter()
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

// ---- Alternating Rules (1D) ----

struct AlternatingRules1D;

impl AdjacencyRules for AlternatingRules1D {
    type Direction = Dir1D;

    fn num_states(&self) -> usize {
        2
    }

    fn compatible(&self, state_a: usize, state_b: usize, _direction: Dir1D) -> bool {
        state_a != state_b
    }
}

// ---- Terrain Rules (2D) ----

struct TerrainRules;

impl AdjacencyRules for TerrainRules {
    type Direction = Dir2D;

    fn num_states(&self) -> usize {
        3
    }

    fn compatible(&self, state_a: usize, state_b: usize, _direction: Dir2D) -> bool {
        matches!((state_a, state_b), (0, 0) | (0, 2) | (2, 0) | (2, 1) | (1, 2) | (1, 1) | (2, 2))
    }
}

// ---- Tests ----

#[test]
fn ac3_and_ac4_produce_same_result_on_1d_chain() {
    let seed = 42u64;

    // AC-3
    let chain_ac3 = Chain1D { len: 10 };
    let rules_ac3 = AlternatingRules1D;
    let config_ac3 = SolverConfig::new().propagator(PropagatorKind::Ac3);
    let mut solver_ac3 = WfcSolver::new(chain_ac3, &rules_ac3, config_ac3).unwrap();
    let mut rng_ac3 = StdRng::seed_from_u64(seed);
    let result_ac3 = solver_ac3.solve(&mut rng_ac3).unwrap();

    // AC-4
    let chain_ac4 = Chain1D { len: 10 };
    let rules_ac4 = AlternatingRules1D;
    let config_ac4 = SolverConfig::new().propagator(PropagatorKind::Ac4);
    let mut solver_ac4 = WfcSolver::new(chain_ac4, &rules_ac4, config_ac4).unwrap();
    let mut rng_ac4 = StdRng::seed_from_u64(seed);
    let result_ac4 = solver_ac4.solve(&mut rng_ac4).unwrap();

    assert_eq!(
        result_ac3.states(),
        result_ac4.states(),
        "AC-3 and AC-4 should produce identical results on 1D chain with same seed"
    );
}

#[test]
fn ac3_and_ac4_produce_same_result_on_2d_grid() {
    let seed = 42u64;

    // AC-3
    let grid_ac3 = Grid2D {
        width: 4,
        height: 4,
    };
    let rules_ac3 = TerrainRules;
    let config_ac3 = SolverConfig::new()
        .propagator(PropagatorKind::Ac3)
        .backtrack(BacktrackStrategy::Restart { max_restarts: 20 });
    let mut solver_ac3 = WfcSolver::new(grid_ac3, &rules_ac3, config_ac3).unwrap();
    let mut rng_ac3 = StdRng::seed_from_u64(seed);
    let result_ac3 = solver_ac3.solve(&mut rng_ac3).unwrap();

    // AC-4
    let grid_ac4 = Grid2D {
        width: 4,
        height: 4,
    };
    let rules_ac4 = TerrainRules;
    let config_ac4 = SolverConfig::new()
        .propagator(PropagatorKind::Ac4)
        .backtrack(BacktrackStrategy::Restart { max_restarts: 20 });
    let mut solver_ac4 = WfcSolver::new(grid_ac4, &rules_ac4, config_ac4).unwrap();
    let mut rng_ac4 = StdRng::seed_from_u64(seed);
    let result_ac4 = solver_ac4.solve(&mut rng_ac4).unwrap();

    assert_eq!(
        result_ac3.states(),
        result_ac4.states(),
        "AC-3 and AC-4 should produce identical results on 2D grid with same seed"
    );
}

#[test]
fn ac3_and_ac4_produce_same_result_on_wrapping_grid() {
    let seed = 42u64;

    // AC-3
    let grid_ac3 = WrappingGrid2D {
        width: 4,
        height: 4,
    };
    let rules_ac3 = TerrainRules;
    let config_ac3 = SolverConfig::new()
        .propagator(PropagatorKind::Ac3)
        .backtrack(BacktrackStrategy::Restart { max_restarts: 20 });
    let mut solver_ac3 = WfcSolver::new(grid_ac3, &rules_ac3, config_ac3).unwrap();
    let mut rng_ac3 = StdRng::seed_from_u64(seed);
    let result_ac3 = solver_ac3.solve(&mut rng_ac3).unwrap();

    // AC-4
    let grid_ac4 = WrappingGrid2D {
        width: 4,
        height: 4,
    };
    let rules_ac4 = TerrainRules;
    let config_ac4 = SolverConfig::new()
        .propagator(PropagatorKind::Ac4)
        .backtrack(BacktrackStrategy::Restart { max_restarts: 20 });
    let mut solver_ac4 = WfcSolver::new(grid_ac4, &rules_ac4, config_ac4).unwrap();
    let mut rng_ac4 = StdRng::seed_from_u64(seed);
    let result_ac4 = solver_ac4.solve(&mut rng_ac4).unwrap();

    assert_eq!(
        result_ac3.states(),
        result_ac4.states(),
        "AC-3 and AC-4 should produce identical results on wrapping grid with same seed"
    );
}

#[test]
fn ac3_and_ac4_same_result_multiple_seeds_1d() {
    // Test with several different seeds to increase confidence
    for seed in [1, 42, 100, 999, 12345] {
        let chain_ac3 = Chain1D { len: 8 };
        let rules_ac3 = AlternatingRules1D;
        let config_ac3 = SolverConfig::new().propagator(PropagatorKind::Ac3);
        let mut solver_ac3 = WfcSolver::new(chain_ac3, &rules_ac3, config_ac3).unwrap();
        let mut rng_ac3 = StdRng::seed_from_u64(seed);
        let result_ac3 = solver_ac3.solve(&mut rng_ac3).unwrap();

        let chain_ac4 = Chain1D { len: 8 };
        let rules_ac4 = AlternatingRules1D;
        let config_ac4 = SolverConfig::new().propagator(PropagatorKind::Ac4);
        let mut solver_ac4 = WfcSolver::new(chain_ac4, &rules_ac4, config_ac4).unwrap();
        let mut rng_ac4 = StdRng::seed_from_u64(seed);
        let result_ac4 = solver_ac4.solve(&mut rng_ac4).unwrap();

        assert_eq!(
            result_ac3.states(),
            result_ac4.states(),
            "AC-3 and AC-4 differ for seed {seed}"
        );
    }
}

#[test]
fn ac3_result_is_valid_on_2d_grid() {
    let grid = Grid2D {
        width: 4,
        height: 4,
    };
    let rules = TerrainRules;
    let config = SolverConfig::new()
        .propagator(PropagatorKind::Ac3)
        .backtrack(BacktrackStrategy::Restart { max_restarts: 20 });
    let mut solver = WfcSolver::new(grid, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    let grid_verify = Grid2D {
        width: 4,
        height: 4,
    };
    let rules_verify = TerrainRules;
    for cell in 0..grid_verify.num_cells() {
        let state = result.state(cell);
        for (neighbor, dir) in grid_verify.neighbors(cell) {
            let nb_state = result.state(neighbor);
            assert!(
                rules_verify.compatible(state, nb_state, dir),
                "AC-3 adjacency violation at cell {cell}"
            );
        }
    }
}

#[test]
fn ac4_result_is_valid_on_2d_grid() {
    let grid = Grid2D {
        width: 4,
        height: 4,
    };
    let rules = TerrainRules;
    let config = SolverConfig::new()
        .propagator(PropagatorKind::Ac4)
        .backtrack(BacktrackStrategy::Restart { max_restarts: 20 });
    let mut solver = WfcSolver::new(grid, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    let grid_verify = Grid2D {
        width: 4,
        height: 4,
    };
    let rules_verify = TerrainRules;
    for cell in 0..grid_verify.num_cells() {
        let state = result.state(cell);
        for (neighbor, dir) in grid_verify.neighbors(cell) {
            let nb_state = result.state(neighbor);
            assert!(
                rules_verify.compatible(state, nb_state, dir),
                "AC-4 adjacency violation at cell {cell}"
            );
        }
    }
}
