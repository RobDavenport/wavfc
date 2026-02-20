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

// ---- Alternating Rules ----

struct AlternatingRules;

impl AdjacencyRules for AlternatingRules {
    type Direction = Dir1D;

    fn num_states(&self) -> usize {
        2
    }

    fn compatible(&self, state_a: usize, state_b: usize, _direction: Dir1D) -> bool {
        state_a != state_b
    }
}

// ---- Unconstrained Rules ----

struct UnconstrainedRules {
    num_states: usize,
}

impl AdjacencyRules for UnconstrainedRules {
    type Direction = Dir1D;

    fn num_states(&self) -> usize {
        self.num_states
    }

    fn compatible(&self, _a: usize, _b: usize, _d: Dir1D) -> bool {
        true
    }
}

// ---- Tests ----

#[test]
fn pin_cell_zero_to_state_zero_and_solve() {
    let chain = Chain1D { len: 5 };
    let rules = UnconstrainedRules { num_states: 3 };
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    solver.pin(0, 0).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    assert_eq!(result.state(0), 0, "pinned cell 0 should be state 0");
}

#[test]
fn pin_multiple_cells_all_respected() {
    let chain = Chain1D { len: 6 };
    let rules = UnconstrainedRules { num_states: 4 };
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    solver.pin(0, 1).unwrap();
    solver.pin(2, 3).unwrap();
    solver.pin(5, 0).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    assert_eq!(result.state(0), 1, "cell 0 should be pinned to state 1");
    assert_eq!(result.state(2), 3, "cell 2 should be pinned to state 3");
    assert_eq!(result.state(5), 0, "cell 5 should be pinned to state 0");
}

#[test]
fn pin_invalid_cell_returns_error() {
    let chain = Chain1D { len: 3 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();

    let result = solver.pin(10, 0); // cell 10 out of range
    match result {
        Err(WfcError::InvalidPin { cell, state }) => {
            assert_eq!(cell, 10);
            assert_eq!(state, 0);
        }
        other => panic!("expected InvalidPin error, got {other:?}"),
    }
}

#[test]
fn pin_invalid_state_returns_error() {
    let chain = Chain1D { len: 3 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();

    let result = solver.pin(0, 5); // state 5 out of range
    match result {
        Err(WfcError::InvalidPin { cell, state }) => {
            assert_eq!(cell, 0);
            assert_eq!(state, 5);
        }
        other => panic!("expected InvalidPin error, got {other:?}"),
    }
}

#[test]
fn pin_with_alternating_rules_propagates_correctly() {
    // Pin cell 0 to state 0, alternating rules force:
    // cell 0 = 0, cell 1 = 1, cell 2 = 0, cell 3 = 1, ...
    let chain = Chain1D { len: 6 };
    let rules = AlternatingRules;
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    solver.pin(0, 0).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    for i in 0..6 {
        let expected = i % 2;
        assert_eq!(
            result.state(i),
            expected,
            "cell {i} should be {expected} due to propagation from pin"
        );
    }
}

#[test]
fn pin_last_cell_with_alternating_rules() {
    // Pin last cell and verify propagation backward
    let chain = Chain1D { len: 5 };
    let rules = AlternatingRules;
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    solver.pin(4, 1).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    // cell 4 = 1, cell 3 = 0, cell 2 = 1, cell 1 = 0, cell 0 = 1
    for i in 0..5 {
        let expected = if (4 - i) % 2 == 0 { 1 } else { 0 };
        assert_eq!(
            result.state(i),
            expected,
            "cell {i} should be {expected} (backward propagation from pin at cell 4)"
        );
    }
}

#[test]
fn seed_config_pins_cell_before_solve() {
    let chain = Chain1D { len: 5 };
    let rules = UnconstrainedRules { num_states: 3 };
    let config = SolverConfig::new().seed(0, 2);
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    assert_eq!(result.state(0), 2, "seeded cell 0 should be state 2");
}

#[test]
fn seeds_config_pins_multiple_cells() {
    let chain = Chain1D { len: 5 };
    let rules = UnconstrainedRules { num_states: 4 };
    let config = SolverConfig::new().seeds(vec![(0, 1), (2, 3), (4, 0)]);
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    assert_eq!(result.state(0), 1);
    assert_eq!(result.state(2), 3);
    assert_eq!(result.state(4), 0);
}

#[test]
fn seed_config_with_alternating_rules_forces_pattern() {
    let chain = Chain1D { len: 4 };
    let rules = AlternatingRules;
    let config = SolverConfig::new().seed(0, 1);
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    // 1, 0, 1, 0
    assert_eq!(result.state(0), 1);
    assert_eq!(result.state(1), 0);
    assert_eq!(result.state(2), 1);
    assert_eq!(result.state(3), 0);
}

#[test]
fn pin_already_collapsed_cell_is_ok() {
    // Pin cell 0 to state 0, then try to pin it to 0 again (already collapsed, but same state)
    // This should be fine since the state is still "possible" (it's the only one)
    let chain = Chain1D { len: 3 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    solver.pin(0, 0).unwrap();
    // Pin again to same state should succeed
    let result = solver.pin(0, 0);
    assert!(result.is_ok(), "re-pinning to same state should be ok");
}

#[test]
fn pin_collapsed_cell_to_different_state_fails() {
    // Pin cell 0 to state 0, then try to pin it to state 1
    let chain = Chain1D { len: 3 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    solver.pin(0, 0).unwrap();
    let result = solver.pin(0, 1);
    match result {
        Err(WfcError::InvalidPin { cell, state }) => {
            assert_eq!(cell, 0);
            assert_eq!(state, 1);
        }
        other => panic!("expected InvalidPin, got {other:?}"),
    }
}
