use wavfc::*;
use wavfc::path_constraint::UnionFind;
use rand::SeedableRng;
use rand::rngs::StdRng;

// ---- 2D Grid Topology (shared test helper) ----

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

// ---- Unconstrained Rules (all states compatible in every direction) ----

struct UnconstrainedRules2D {
    num_states: usize,
}

impl AdjacencyRules for UnconstrainedRules2D {
    type Direction = Dir2D;

    fn num_states(&self) -> usize {
        self.num_states
    }

    fn compatible(&self, _a: usize, _b: usize, _d: Dir2D) -> bool {
        true
    }
}

struct UnconstrainedRules1D {
    num_states: usize,
}

impl AdjacencyRules for UnconstrainedRules1D {
    type Direction = Dir1D;

    fn num_states(&self) -> usize {
        self.num_states
    }

    fn compatible(&self, _a: usize, _b: usize, _d: Dir1D) -> bool {
        true
    }
}

// ---- Terrain Rules for integration test ----
// 3 states: 0=grass, 1=water, 2=sand
// grass<->grass, grass<->sand, sand<->water, water<->water, sand<->sand
// NOT: grass<->water

struct TerrainRules;

impl AdjacencyRules for TerrainRules {
    type Direction = Dir2D;

    fn num_states(&self) -> usize {
        3
    }

    fn compatible(&self, state_a: usize, state_b: usize, _direction: Dir2D) -> bool {
        match (state_a, state_b) {
            (0, 0) => true,
            (0, 2) | (2, 0) => true,
            (2, 1) | (1, 2) => true,
            (1, 1) => true,
            (2, 2) => true,
            _ => false,
        }
    }
}

// ============================
// UnionFind basic tests
// ============================

#[test]
fn union_find_new_creates_disjoint_sets() {
    let mut uf = UnionFind::new(5);
    // Each element is its own representative.
    for i in 0..5 {
        assert_eq!(uf.find(i), i);
    }
}

#[test]
fn union_find_find_returns_self_for_singleton() {
    let mut uf = UnionFind::new(3);
    assert_eq!(uf.find(0), 0);
    assert_eq!(uf.find(1), 1);
    assert_eq!(uf.find(2), 2);
}

#[test]
fn union_find_union_connects_elements() {
    let mut uf = UnionFind::new(5);
    uf.union(0, 1);
    assert!(uf.connected(0, 1));
    assert!(!uf.connected(0, 2));

    uf.union(2, 3);
    assert!(uf.connected(2, 3));
    assert!(!uf.connected(0, 2));

    uf.union(1, 3);
    assert!(uf.connected(0, 3));
    assert!(uf.connected(0, 2));
    assert!(uf.connected(1, 2));
}

#[test]
fn union_find_connected_reflexive() {
    let mut uf = UnionFind::new(3);
    assert!(uf.connected(0, 0));
    assert!(uf.connected(1, 1));
    assert!(uf.connected(2, 2));
}

#[test]
fn union_find_path_compression() {
    let mut uf = UnionFind::new(10);
    // Build a chain: 0-1-2-3-4-5-6-7-8-9
    for i in 0..9 {
        uf.union(i, i + 1);
    }
    // After find with path compression, all should point to same root
    let root = uf.find(0);
    for i in 1..10 {
        assert_eq!(uf.find(i), root);
    }
}

// ============================
// PathConstraint unit tests
// ============================

#[test]
fn required_cell_collapsed_to_non_path_state_is_contradiction() {
    // Setup: 1D chain with 3 cells, 2 states (0=path, 1=non-path).
    // Pin cell 1 to state 1 (non-path), then require cell 1 to be path.
    let chain = Chain1D { len: 3 };
    let rules = UnconstrainedRules1D { num_states: 2 };
    let config = SolverConfig::new()
        .seed(1, 1); // Pin cell 1 to state 1 (non-path)

    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    // path_states: only state 0 is a path state
    let mut path_states = BitSet128::new();
    path_states.set(0);
    let constraint = PathConstraint::new(path_states, vec![1]);

    let mut observer = NoOpObserver;
    let result = solver.solve_with(&mut rng, &mut observer, &[&constraint]);

    assert!(result.is_err(), "required cell pinned to non-path state should cause contradiction");
}

#[test]
fn all_required_cells_uncollapsed_succeeds() {
    // Setup: 3 cells, 2 states, no pins. The constraint should not fire
    // an error when required cells are not yet collapsed.
    // We test this indirectly: if the solver succeeds, the constraint
    // didn't erroneously contradict during solving.
    let chain = Chain1D { len: 3 };
    let rules = UnconstrainedRules1D { num_states: 2 };
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 50 });

    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    // Both states are path states, so any collapse is a path state.
    let mut path_states = BitSet128::new();
    path_states.set(0);
    path_states.set(1);
    let constraint = PathConstraint::new(path_states, vec![0, 1, 2]);

    let mut observer = NoOpObserver;
    let result = solver.solve_with(&mut rng, &mut observer, &[&constraint]);

    assert!(result.is_ok(), "all cells as path states should always succeed");
}

#[test]
fn path_constraint_with_no_required_cells_always_succeeds() {
    let chain = Chain1D { len: 5 };
    let rules = UnconstrainedRules1D { num_states: 3 };
    let config = SolverConfig::new();

    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    // No required cells -- should always succeed.
    let mut path_states = BitSet128::new();
    path_states.set(0);
    let constraint = PathConstraint::new(path_states, vec![]);

    let mut observer = NoOpObserver;
    let result = solver.solve_with(&mut rng, &mut observer, &[&constraint]);

    assert!(result.is_ok(), "no required cells should trivially succeed");
}

#[test]
fn disconnected_required_cells_causes_contradiction() {
    // Setup: a 1D chain of 5 cells with 2 states (0=path, 1=wall).
    // Pin cell 0 to path, cell 4 to path, and cell 2 to wall.
    // Require cells 0 and 4 to be connected.
    // Since cell 2 is a wall, cells 0 and 4 cannot be connected through it.
    // However, in a 1D chain, cell 2 is the only path between them,
    // so this should cause a contradiction.
    let chain = Chain1D { len: 5 };
    let rules = UnconstrainedRules1D { num_states: 2 };
    let config = SolverConfig::new()
        .seed(0, 0)  // cell 0 = path
        .seed(2, 1)  // cell 2 = wall (blocks connectivity)
        .seed(4, 0)  // cell 4 = path
        .backtrack(BacktrackStrategy::Restart { max_restarts: 50 });

    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let mut path_states = BitSet128::new();
    path_states.set(0); // only state 0 is path

    let constraint = PathConstraint::new(path_states, vec![0, 4]);
    let mut observer = NoOpObserver;
    let result = solver.solve_with(&mut rng, &mut observer, &[&constraint]);

    // The solver should fail: cells 0 and 4 can never be connected in a 1D chain
    // when cell 2 is a wall, because the only route is 0-1-2-3-4.
    // Even if cells 1 and 3 become path tiles, cell 2 is a wall breaking the chain.
    assert!(result.is_err(), "disconnected required cells should cause contradiction");
}

// ============================
// Integration test
// ============================

#[test]
fn integration_grid_with_path_constraint_solves() {
    // 3x3 grid, 2 states: 0=path, 1=grass. All compatible with all (unconstrained).
    // Require cells at corners (0, 2, 6, 8) to be connected via path tiles.
    // The solver should produce a result where those corners are reachable
    // through adjacent path cells.
    let grid = Grid2D { width: 3, height: 3 };
    let rules = UnconstrainedRules2D { num_states: 2 };
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 200 });

    let mut solver = WfcSolver::new(grid, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let mut path_states = BitSet128::new();
    path_states.set(0); // state 0 is "path"

    // Require all four corners to be connected via path
    let constraint = PathConstraint::new(path_states, vec![0, 2, 6, 8]);
    let mut observer = NoOpObserver;
    let result = solver.solve_with(&mut rng, &mut observer, &[&constraint]);

    match result {
        Ok(solve_result) => {
            let states = solve_result.states();

            // All required cells must be path (state 0)
            for &req in &[0, 2, 6, 8] {
                assert_eq!(
                    states[req], 0,
                    "required cell {req} should be path (state 0), got {}",
                    states[req]
                );
            }

            // Verify connectivity: BFS from cell 0 through path cells should reach 2, 6, 8
            let grid = Grid2D { width: 3, height: 3 };
            let mut visited = vec![false; 9];
            let mut queue = Vec::new();
            queue.push(0);
            visited[0] = true;
            while let Some(cell) = queue.pop() {
                for (neighbor, _dir) in grid.neighbors(cell) {
                    if !visited[neighbor] && states[neighbor] == 0 {
                        visited[neighbor] = true;
                        queue.push(neighbor);
                    }
                }
            }
            for &req in &[2, 6, 8] {
                assert!(
                    visited[req],
                    "required cell {req} should be reachable from cell 0 via path tiles"
                );
            }
        }
        Err(e) => {
            panic!("expected solver to find a solution with path constraint, got error: {e}");
        }
    }
}

#[test]
fn integration_terrain_grid_with_path_constraint() {
    // 4x4 grid with terrain rules (grass/water/sand).
    // Path states: grass(0) and sand(2) -- require corners 0 and 15 connected.
    let grid = Grid2D { width: 4, height: 4 };
    let rules = TerrainRules;
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 200 });

    let mut solver = WfcSolver::new(grid, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(123);

    let mut path_states = BitSet128::new();
    path_states.set(0); // grass
    path_states.set(2); // sand

    // Require top-left (0) and bottom-right (15) to be connected via land (grass/sand).
    let constraint = PathConstraint::new(path_states, vec![0, 15]);
    let mut observer = NoOpObserver;
    let result = solver.solve_with(&mut rng, &mut observer, &[&constraint]);

    match result {
        Ok(solve_result) => {
            let states = solve_result.states();

            // Both required cells must be path states (grass or sand)
            assert!(
                states[0] == 0 || states[0] == 2,
                "cell 0 should be grass(0) or sand(2), got {}",
                states[0]
            );
            assert!(
                states[15] == 0 || states[15] == 2,
                "cell 15 should be grass(0) or sand(2), got {}",
                states[15]
            );

            // Verify connectivity via BFS
            let grid = Grid2D { width: 4, height: 4 };
            let mut visited = vec![false; 16];
            let mut queue = Vec::new();
            queue.push(0);
            visited[0] = true;
            while let Some(cell) = queue.pop() {
                for (neighbor, _dir) in grid.neighbors(cell) {
                    if !visited[neighbor] && (states[neighbor] == 0 || states[neighbor] == 2) {
                        visited[neighbor] = true;
                        queue.push(neighbor);
                    }
                }
            }
            assert!(
                visited[15],
                "cell 15 should be reachable from cell 0 via grass/sand path tiles"
            );
        }
        Err(e) => {
            panic!("expected solver to find a solution with terrain path constraint, got error: {e}");
        }
    }
}

#[test]
fn single_required_cell_always_succeeds() {
    // A single required cell can always satisfy the constraint as long as
    // it collapses to a path state.
    let chain = Chain1D { len: 3 };
    let rules = UnconstrainedRules1D { num_states: 2 };
    let config = SolverConfig::new()
        .seed(1, 0)  // Pin middle cell to path
        .backtrack(BacktrackStrategy::Restart { max_restarts: 20 });

    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let mut path_states = BitSet128::new();
    path_states.set(0);
    let constraint = PathConstraint::new(path_states, vec![1]);

    let mut observer = NoOpObserver;
    let result = solver.solve_with(&mut rng, &mut observer, &[&constraint]);

    assert!(result.is_ok(), "single required path cell pinned to path should succeed");
    assert_eq!(result.unwrap().state(1), 0, "cell 1 should be path state 0");
}
