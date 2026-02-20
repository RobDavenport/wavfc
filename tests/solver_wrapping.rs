use wavfc::*;
use rand::SeedableRng;
use rand::rngs::StdRng;

// ---- Wrapping 2D Grid Topology (Torus) ----

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Dir2D {
    North,
    South,
    East,
    West,
}

const DIRS_2D: [Dir2D; 4] = [Dir2D::North, Dir2D::South, Dir2D::East, Dir2D::West];

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

// ---- Simple rules: 2 states, both compatible with each other and themselves ----

struct UnconstrainedRules;

impl AdjacencyRules for UnconstrainedRules {
    type Direction = Dir2D;

    fn num_states(&self) -> usize {
        2
    }

    fn compatible(&self, _state_a: usize, _state_b: usize, _direction: Dir2D) -> bool {
        true
    }
}

// Terrain rules for wrapping grid
// 3 states: 0=grass, 1=water, 2=sand
// grass<->grass, grass<->sand, sand<->water, water<->water, sand<->sand
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
fn wrapping_grid_solves_successfully() {
    let grid = WrappingGrid2D {
        width: 4,
        height: 4,
    };
    let rules = UnconstrainedRules;
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(grid, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    assert_eq!(result.states().len(), 16);
}

#[test]
fn wrapping_grid_every_cell_has_four_neighbors() {
    let grid = WrappingGrid2D {
        width: 5,
        height: 5,
    };
    for cell in 0..grid.num_cells() {
        let neighbors: Vec<_> = grid.neighbors(cell).collect();
        assert_eq!(
            neighbors.len(),
            4,
            "cell {cell} should have exactly 4 neighbors, got {}",
            neighbors.len()
        );
    }
}

#[test]
fn wrapping_grid_edge_cells_wrap_correctly() {
    let grid = WrappingGrid2D {
        width: 5,
        height: 5,
    };

    // Cell (0,0) = index 0
    // West neighbor should be (4,0) = index 4
    // North neighbor should be (0,4) = index 20
    let neighbors: Vec<_> = grid.neighbors(0).collect();
    let west_neighbor = neighbors.iter().find(|&&(_, d)| d == Dir2D::West).unwrap();
    assert_eq!(west_neighbor.0, 4, "west wrap: expected cell 4, got {}", west_neighbor.0);

    let north_neighbor = neighbors.iter().find(|&&(_, d)| d == Dir2D::North).unwrap();
    assert_eq!(north_neighbor.0, 20, "north wrap: expected cell 20, got {}", north_neighbor.0);

    // Cell (4,4) = index 24
    // East neighbor should be (0,4) = index 20
    // South neighbor should be (4,0) = index 4
    let neighbors: Vec<_> = grid.neighbors(24).collect();
    let east_neighbor = neighbors.iter().find(|&&(_, d)| d == Dir2D::East).unwrap();
    assert_eq!(east_neighbor.0, 20, "east wrap: expected cell 20, got {}", east_neighbor.0);

    let south_neighbor = neighbors.iter().find(|&&(_, d)| d == Dir2D::South).unwrap();
    assert_eq!(south_neighbor.0, 4, "south wrap: expected cell 4, got {}", south_neighbor.0);
}

#[test]
fn wrapping_grid_terrain_result_satisfies_constraints() {
    let grid = WrappingGrid2D {
        width: 4,
        height: 4,
    };
    let rules = TerrainRules;
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 20 });
    let mut solver = WfcSolver::new(grid, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    // Verify all adjacencies including wrapped edges
    let grid_verify = WrappingGrid2D {
        width: 4,
        height: 4,
    };
    let rules_verify = TerrainRules;
    for cell in 0..grid_verify.num_cells() {
        let state = result.state(cell);
        assert!(state < rules_verify.num_states());

        for (neighbor, dir) in grid_verify.neighbors(cell) {
            let nb_state = result.state(neighbor);
            assert!(
                rules_verify.compatible(state, nb_state, dir),
                "Adjacency violation: cell {cell} (state {state}) and cell {neighbor} (state {nb_state}) in direction {dir:?}"
            );
        }
    }
}

#[test]
fn wrapping_grid_unconstrained_solves_to_all_valid_states() {
    let grid = WrappingGrid2D {
        width: 3,
        height: 3,
    };
    let rules = UnconstrainedRules;
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(grid, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    for cell in 0..9 {
        assert!(result.state(cell) < 2, "state should be 0 or 1");
    }
}

#[test]
fn wrapping_grid_larger_size_solves() {
    let grid = WrappingGrid2D {
        width: 8,
        height: 8,
    };
    let rules = TerrainRules;
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 30 });
    let mut solver = WfcSolver::new(grid, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(99);
    let result = solver.solve(&mut rng).unwrap();

    assert_eq!(result.states().len(), 64);

    // Verify all adjacencies
    let grid_verify = WrappingGrid2D {
        width: 8,
        height: 8,
    };
    let rules_verify = TerrainRules;
    for cell in 0..grid_verify.num_cells() {
        let state = result.state(cell);
        for (neighbor, dir) in grid_verify.neighbors(cell) {
            let nb_state = result.state(neighbor);
            assert!(
                rules_verify.compatible(state, nb_state, dir),
                "Adjacency violation at cell {cell}"
            );
        }
    }
}
