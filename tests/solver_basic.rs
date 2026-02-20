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

// ---- Alternating Rules: A-B must alternate ----
// 2 states: 0 (A) and 1 (B)
// A can only be next to B, B can only be next to A

struct AlternatingRules;

impl AdjacencyRules for AlternatingRules {
    type Direction = Dir1D;

    fn num_states(&self) -> usize {
        2
    }

    fn compatible(&self, state_a: usize, state_b: usize, _direction: Dir1D) -> bool {
        // A(0) is compatible with B(1) and vice versa
        state_a != state_b
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

// ---- Terrain Rules ----
// 3 states: 0=grass, 1=water, 2=sand
// Adjacency: grass<->grass, grass<->sand, sand<->water, water<->water
// NOT allowed: grass<->water (must have sand between)

struct TerrainRules;

impl AdjacencyRules for TerrainRules {
    type Direction = Dir2D;

    fn num_states(&self) -> usize {
        3
    }

    fn compatible(&self, state_a: usize, state_b: usize, _direction: Dir2D) -> bool {
        match (state_a, state_b) {
            (0, 0) => true, // grass <-> grass
            (0, 2) | (2, 0) => true, // grass <-> sand
            (2, 1) | (1, 2) => true, // sand <-> water
            (1, 1) => true, // water <-> water
            (2, 2) => true, // sand <-> sand
            _ => false, // grass <-> water NOT allowed
        }
    }
}

// ---- Helper: verify adjacency constraints ----

fn verify_1d_chain_result(chain: &Chain1D, rules: &AlternatingRules, result: &SolveResult) {
    for cell in 0..chain.len {
        let state = result.state(cell);
        assert!(state < rules.num_states(), "state {state} out of range for cell {cell}");

        for (neighbor, dir) in chain.neighbors(cell) {
            let nb_state = result.state(neighbor);
            assert!(
                rules.compatible(state, nb_state, dir),
                "Adjacency violation: cell {cell} (state {state}) and cell {neighbor} (state {nb_state}) in direction {dir:?}"
            );
        }
    }
}

fn verify_2d_grid_result(grid: &Grid2D, rules: &TerrainRules, result: &SolveResult) {
    for cell in 0..grid.num_cells() {
        let state = result.state(cell);
        assert!(state < rules.num_states(), "state {state} out of range for cell {cell}");

        for (neighbor, dir) in grid.neighbors(cell) {
            let nb_state = result.state(neighbor);
            assert!(
                rules.compatible(state, nb_state, dir),
                "Adjacency violation: cell {cell} (state {state}) and cell {neighbor} (state {nb_state}) in direction {dir:?}"
            );
        }
    }
}

// ---- Tests ----

#[test]
fn chain_alternating_solves_successfully() {
    let chain = Chain1D { len: 10 };
    let rules = AlternatingRules;
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    assert_eq!(result.states().len(), 10);
    assert!(result.iterations() > 0);
}

#[test]
fn chain_alternating_result_satisfies_constraints() {
    let chain = Chain1D { len: 20 };
    let rules = AlternatingRules;
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(123);
    let result = solver.solve(&mut rng).unwrap();

    let chain_for_verify = Chain1D { len: 20 };
    let rules_for_verify = AlternatingRules;
    verify_1d_chain_result(&chain_for_verify, &rules_for_verify, &result);
}

#[test]
fn chain_alternating_pattern_is_correct() {
    // With alternating rules, the result must be 0,1,0,1,... or 1,0,1,0,...
    let chain = Chain1D { len: 8 };
    let rules = AlternatingRules;
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    let first = result.state(0);
    for i in 0..8 {
        let expected = if i % 2 == 0 { first } else { 1 - first };
        assert_eq!(
            result.state(i),
            expected,
            "cell {i}: expected {expected} but got {}",
            result.state(i)
        );
    }
}

#[test]
fn grid_terrain_solves_successfully() {
    let grid = Grid2D {
        width: 4,
        height: 4,
    };
    let rules = TerrainRules;
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 5 });
    let mut solver = WfcSolver::new(grid, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    assert_eq!(result.states().len(), 16);
}

#[test]
fn grid_terrain_result_satisfies_constraints() {
    let grid = Grid2D {
        width: 4,
        height: 4,
    };
    let rules = TerrainRules;
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 10 });
    let mut solver = WfcSolver::new(grid, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    let grid_for_verify = Grid2D {
        width: 4,
        height: 4,
    };
    let rules_for_verify = TerrainRules;
    verify_2d_grid_result(&grid_for_verify, &rules_for_verify, &result);
}

#[test]
fn chain_single_cell_solves() {
    let chain = Chain1D { len: 1 };
    let rules = AlternatingRules;
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    assert_eq!(result.states().len(), 1);
    assert!(result.state(0) < 2);
}

#[test]
fn chain_two_cells_alternates() {
    let chain = Chain1D { len: 2 };
    let rules = AlternatingRules;
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    assert_ne!(result.state(0), result.state(1));
}

#[test]
fn grid_1x1_terrain_solves() {
    let grid = Grid2D {
        width: 1,
        height: 1,
    };
    let rules = TerrainRules;
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(grid, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    assert_eq!(result.states().len(), 1);
    assert!(result.state(0) < 3);
}

#[test]
fn solve_result_iterations_is_positive() {
    let chain = Chain1D { len: 5 };
    let rules = AlternatingRules;
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    assert!(result.iterations() > 0, "should have taken at least one iteration");
}

// ---- Impossible Rules: no state is compatible with any other ----

struct ImpossibleRules;

impl AdjacencyRules for ImpossibleRules {
    type Direction = Dir1D;

    fn num_states(&self) -> usize {
        2
    }

    fn compatible(&self, _state_a: usize, _state_b: usize, _direction: Dir1D) -> bool {
        false
    }
}

// ---- Oversized Rules: reports more than 128 states ----

struct OversizedRules;

impl AdjacencyRules for OversizedRules {
    type Direction = Dir1D;

    fn num_states(&self) -> usize {
        129
    }

    fn compatible(&self, _state_a: usize, _state_b: usize, _direction: Dir1D) -> bool {
        true
    }
}

// ---- solve_with_validation tests ----

#[test]
fn solve_with_validation_passes_on_first_try() {
    let chain = Chain1D { len: 5 };
    let rules = AlternatingRules;
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    // Validation always accepts
    let result = solver
        .solve_with_validation(&mut rng, |_states| true, 3)
        .unwrap();

    assert_eq!(result.states().len(), 5);
    // Verify the result still satisfies adjacency constraints
    let chain_v = Chain1D { len: 5 };
    let rules_v = AlternatingRules;
    verify_1d_chain_result(&chain_v, &rules_v, &result);
}

#[test]
fn solve_with_validation_retries_on_rejection() {
    let chain = Chain1D { len: 5 };
    let rules = AlternatingRules;
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    // Reject results where cell 0 == state 0.
    // Since AlternatingRules only has 2 states, ~50% of solves will have cell 0 == 1.
    // With enough retries we should find one.
    let result = solver
        .solve_with_validation(&mut rng, |states| states[0] != 0, 20)
        .unwrap();

    assert_eq!(result.state(0), 1, "validation should have ensured cell 0 != 0");
}

#[test]
fn solve_with_validation_exhausts_retries() {
    let chain = Chain1D { len: 5 };
    let rules = AlternatingRules;
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    // Validation always rejects
    let result = solver.solve_with_validation(&mut rng, |_states| false, 3);

    match result {
        Err(WfcError::Contradiction { .. }) => {} // expected
        Err(other) => panic!("expected Contradiction error, got: {other}"),
        Ok(_) => panic!("expected error after exhausting retries, but got Ok"),
    }
}

// ---- step() tests ----

#[test]
fn step_produces_collapsed_then_complete() {
    let chain = Chain1D { len: 5 };
    let rules = AlternatingRules;
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let mut collapsed_steps = Vec::new();
    let mut completed = false;

    for _ in 0..100 {
        match solver.step(&mut rng) {
            StepResult::Collapsed { cell, state } => {
                assert!(state < 2, "state {state} out of range for AlternatingRules");
                collapsed_steps.push((cell, state));
            }
            StepResult::Complete => {
                completed = true;
                break;
            }
            StepResult::Contradiction { cell } => {
                panic!("unexpected contradiction at cell {cell} for a solvable chain");
            }
        }
    }

    assert!(completed, "step() should eventually return Complete");
    // At least one explicit Collapsed step must occur before Complete.
    // (Propagation may collapse remaining cells automatically, so the count
    // can be less than num_cells.)
    assert!(
        !collapsed_steps.is_empty(),
        "expected at least one Collapsed step before Complete"
    );
    // Every collapsed state should be valid
    for &(_cell, state) in &collapsed_steps {
        assert!(state < 2, "collapsed state {state} out of range");
    }
}

#[test]
fn step_contradiction_on_impossible_rules() {
    // A chain of 2+ cells with rules that disallow all adjacencies must contradict
    let chain = Chain1D { len: 3 };
    let rules = ImpossibleRules;
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let mut found_contradiction = false;
    for _ in 0..100 {
        match solver.step(&mut rng) {
            StepResult::Contradiction { .. } => {
                found_contradiction = true;
                break;
            }
            StepResult::Complete => break,
            StepResult::Collapsed { .. } => {}
        }
    }

    assert!(
        found_contradiction,
        "impossible rules should produce a contradiction"
    );
}

// ---- TooManyStates test ----

#[test]
fn too_many_states_returns_error() {
    let chain = Chain1D { len: 2 };
    let rules = OversizedRules;
    let config = SolverConfig::new();
    let result = WfcSolver::new(chain, &rules, config);

    match result {
        Err(WfcError::TooManyStates { requested, max }) => {
            assert_eq!(requested, 129);
            assert_eq!(max, 128);
        }
        Err(other) => panic!("expected TooManyStates, got: {other}"),
        Ok(_) => panic!("expected TooManyStates error, but got Ok"),
    }
}
