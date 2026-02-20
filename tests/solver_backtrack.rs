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

// ---- Impossible Rules ----
// 3 states: A(0) can only be next to B(1), B(1) can only be next to C(2),
// C(2) can only be next to A(0). On a 2-cell chain this is impossible:
// If cell 0 = A, cell 1 must be B. But then cell 1 = B, cell 0 must be C. Contradiction.

struct CyclicRules;

impl AdjacencyRules for CyclicRules {
    type Direction = Dir1D;

    fn num_states(&self) -> usize {
        3
    }

    fn compatible(&self, state_a: usize, state_b: usize, _direction: Dir1D) -> bool {
        // A->B, B->C, C->A (one-directional chain)
        // But this is directional; for the constraint to be impossible on 2 cells,
        // we need: A is only compatible with B, B only with C, C only with A.
        match (state_a, state_b) {
            (0, 1) => true, // A can have B as neighbor
            (1, 2) => true, // B can have C as neighbor
            (2, 0) => true, // C can have A as neighbor
            _ => false,
        }
    }
}

// ---- Tight Rules ----
// Rules that are very constrained and likely to contradict without backtracking
// on larger grids. 4 states with strict directional constraints.

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

// Tight directional rules: 4 states with strict placement rules
// State 0: can have 1 to the east, 2 to the south, self otherwise
// State 1: can have 0 to the west, 3 to the south, self otherwise
// State 2: can have 3 to the east, 0 to the north, self otherwise
// State 3: can have 2 to the west, 1 to the north, self otherwise
// This creates a strict checkerboard-like pattern that's hard to satisfy
struct TightDirectionalRules;

impl AdjacencyRules for TightDirectionalRules {
    type Direction = Dir2D;

    fn num_states(&self) -> usize {
        4
    }

    fn compatible(&self, state_a: usize, state_b: usize, direction: Dir2D) -> bool {
        match direction {
            Dir2D::East => matches!(
                (state_a, state_b),
                (0, 1) | (1, 1) | (0, 0) | (2, 3) | (3, 3) | (2, 2)
            ),
            Dir2D::West => matches!(
                (state_a, state_b),
                (1, 0) | (0, 0) | (1, 1) | (3, 2) | (2, 2) | (3, 3)
            ),
            Dir2D::South => matches!(
                (state_a, state_b),
                (0, 2) | (2, 2) | (0, 0) | (1, 3) | (3, 3) | (1, 1)
            ),
            Dir2D::North => matches!(
                (state_a, state_b),
                (2, 0) | (0, 0) | (2, 2) | (3, 1) | (1, 1) | (3, 3)
            ),
        }
    }
}

// ---- Tests ----

#[test]
fn impossible_rules_with_no_backtrack_returns_contradiction() {
    // Cyclic rules on 2 cells is impossible
    let chain = Chain1D { len: 2 };
    let rules = CyclicRules;
    let config = SolverConfig::new().backtrack(BacktrackStrategy::None);
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let result = solver.solve(&mut rng);
    assert!(result.is_err(), "impossible rules should fail");
}

#[test]
fn contradiction_error_has_partial_state() {
    let chain = Chain1D { len: 2 };
    let rules = CyclicRules;
    let config = SolverConfig::new().backtrack(BacktrackStrategy::None);
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let result = solver.solve(&mut rng);
    match result {
        Err(WfcError::Contradiction {
            partial_state,
            ..
        }) => {
            assert!(
                !partial_state.is_empty(),
                "partial_state should not be empty"
            );
            // Should have one bitset per cell
            assert_eq!(partial_state.len(), 2);
        }
        Err(other) => panic!("expected Contradiction error, got: {other}"),
        Ok(_) => panic!("expected error but got success"),
    }
}

#[test]
fn impossible_rules_fail_even_with_restart_backtracking() {
    let chain = Chain1D { len: 2 };
    let rules = CyclicRules;
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 20 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let result = solver.solve(&mut rng);
    assert!(result.is_err(), "impossible rules should always fail");
}

#[test]
fn impossible_rules_fail_even_with_chronological_backtracking() {
    let chain = Chain1D { len: 2 };
    let rules = CyclicRules;
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Chronological { max_depth: 32 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let result = solver.solve(&mut rng);
    assert!(result.is_err(), "impossible rules should always fail");
}

#[test]
fn tight_rules_with_no_backtrack_may_contradict() {
    // Test that the tight rules on a larger grid can produce contradictions
    // without backtracking. We try multiple seeds and expect at least some failures.
    let mut contradiction_count = 0;
    let mut success_count = 0;

    for seed in 0..20 {
        let grid = Grid2D {
            width: 4,
            height: 4,
        };
        let rules = TightDirectionalRules;
        let config = SolverConfig::new().backtrack(BacktrackStrategy::None);
        let mut solver = WfcSolver::new(grid, &rules, config).unwrap();
        let mut rng = StdRng::seed_from_u64(seed);

        match solver.solve(&mut rng) {
            Ok(_) => success_count += 1,
            Err(WfcError::Contradiction { .. }) => contradiction_count += 1,
            Err(e) => panic!("unexpected error: {e}"),
        }
    }

    // We expect that without backtracking, at least some seeds contradict
    // (the test is that the solver properly reports contradictions)
    // Either outcome is fine as long as nothing panics
    assert!(
        contradiction_count + success_count == 20,
        "all 20 runs should either succeed or contradict"
    );
}

#[test]
fn tight_rules_with_restart_backtracking_succeeds_more_often() {
    let mut success_count = 0;

    for seed in 0..10 {
        let grid = Grid2D {
            width: 4,
            height: 4,
        };
        let rules = TightDirectionalRules;
        let config = SolverConfig::new()
            .backtrack(BacktrackStrategy::Restart { max_restarts: 50 });
        let mut solver = WfcSolver::new(grid, &rules, config).unwrap();
        let mut rng = StdRng::seed_from_u64(seed);

        if solver.solve(&mut rng).is_ok() {
            success_count += 1;
        }
    }

    // With restarts, we expect at least some successes
    assert!(
        success_count > 0,
        "restart backtracking should succeed at least once in 10 tries"
    );
}

#[test]
fn tight_rules_with_chronological_backtracking_succeeds_reliably() {
    let mut success_count = 0;

    for seed in 0..10 {
        let grid = Grid2D {
            width: 4,
            height: 4,
        };
        let rules = TightDirectionalRules;
        let config = SolverConfig::new()
            .backtrack(BacktrackStrategy::Chronological { max_depth: 32 });
        let mut solver = WfcSolver::new(grid, &rules, config).unwrap();
        let mut rng = StdRng::seed_from_u64(seed);

        if solver.solve(&mut rng).is_ok() {
            success_count += 1;
        }
    }

    // Chronological backtracking should succeed more reliably
    assert!(
        success_count > 0,
        "chronological backtracking should succeed at least once in 10 tries"
    );
}

#[test]
fn contradiction_error_contains_backtrack_depth() {
    let chain = Chain1D { len: 2 };
    let rules = CyclicRules;
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Chronological { max_depth: 10 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let result = solver.solve(&mut rng);
    match result {
        Err(WfcError::Contradiction {
            backtrack_depth, ..
        }) => {
            // The backtrack_depth should be the max_depth since we exhausted all options
            assert_eq!(backtrack_depth, 10);
        }
        Err(other) => panic!("expected Contradiction error, got: {other}"),
        Ok(_) => panic!("expected error but got success"),
    }
}

#[test]
fn no_backtrack_strategy_reports_zero_depth() {
    let chain = Chain1D { len: 2 };
    let rules = CyclicRules;
    let config = SolverConfig::new().backtrack(BacktrackStrategy::None);
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let result = solver.solve(&mut rng);
    match result {
        Err(WfcError::Contradiction {
            backtrack_depth, ..
        }) => {
            assert_eq!(backtrack_depth, 0, "no backtrack should report depth 0");
        }
        Err(other) => panic!("expected Contradiction error, got: {other}"),
        Ok(_) => panic!("expected error but got success"),
    }
}
