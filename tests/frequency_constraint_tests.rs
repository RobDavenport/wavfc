use wavfc::*;
use wavfc::frequency_constraint::FrequencyConstraint;
use rand::SeedableRng;
use rand::rngs::StdRng;

// ---- 1D Chain Topology (replicated from solver_global.rs) ----

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
fn max_caps_state_frequency() {
    let chain = Chain1D { len: 8 };
    let rules = UnconstrainedRules { num_states: 3 };
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 100 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint = FrequencyConstraint::builder()
        .max(0, 2)
        .build();
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint])
        .unwrap();

    let count_0 = result.states().iter().filter(|&&s| s == 0).count();
    assert!(
        count_0 <= 2,
        "expected at most 2 cells with state 0, got {count_0}"
    );
}

#[test]
fn max_zero_bans_state() {
    let chain = Chain1D { len: 4 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 50 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint = FrequencyConstraint::builder()
        .max(1, 0)
        .build();
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint])
        .unwrap();

    for cell in 0..4 {
        assert_ne!(
            result.state(cell),
            1,
            "cell {cell} should not have state 1 when max(1, 0)"
        );
    }
}

#[test]
fn max_equals_grid_is_noop() {
    let chain = Chain1D { len: 4 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint = FrequencyConstraint::builder()
        .max(0, 4)
        .build();
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint]);

    assert!(result.is_ok(), "max equal to grid size should succeed");
}

#[test]
fn min_ensures_minimum() {
    let chain = Chain1D { len: 6 };
    let rules = UnconstrainedRules { num_states: 3 };
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 100 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint = FrequencyConstraint::builder()
        .min(2, 3)
        .build();
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint])
        .unwrap();

    let count_2 = result.states().iter().filter(|&&s| s == 2).count();
    assert!(
        count_2 >= 3,
        "expected at least 3 cells with state 2, got {count_2}"
    );
}

#[test]
fn min_impossible_contradicts() {
    let chain = Chain1D { len: 4 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 10 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint = FrequencyConstraint::builder()
        .min(0, 5)
        .build();
    let mut observer = NoOpObserver;
    let result = solver.solve_with(&mut rng, &mut observer, &[&constraint]);

    assert!(
        result.is_err(),
        "min(0, 5) on 4 cells should be impossible"
    );
}

#[test]
fn min_equals_grid_forces_all() {
    let chain = Chain1D { len: 4 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 50 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint = FrequencyConstraint::builder()
        .min(0, 4)
        .build();
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint])
        .unwrap();

    for cell in 0..4 {
        assert_eq!(
            result.state(cell),
            0,
            "all cells should be state 0 when min(0, 4) on 4 cells"
        );
    }
}

#[test]
fn bound_min_max_combined() {
    let chain = Chain1D { len: 8 };
    let rules = UnconstrainedRules { num_states: 3 };
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 200 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint = FrequencyConstraint::builder()
        .bound(1, 2, 4)
        .build();
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint])
        .unwrap();

    let count_1 = result.states().iter().filter(|&&s| s == 1).count();
    assert!(
        count_1 >= 2 && count_1 <= 4,
        "expected 2 <= count(1) <= 4, got {count_1}"
    );
}

#[test]
fn conflicting_bounds_contradiction() {
    let chain = Chain1D { len: 4 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 10 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    // min(0,3) + min(1,3) on 4 cells: needs 3+3=6 but only 4 cells
    let constraint = FrequencyConstraint::builder()
        .min(0, 3)
        .min(1, 3)
        .build();
    let mut observer = NoOpObserver;
    let result = solver.solve_with(&mut rng, &mut observer, &[&constraint]);

    assert!(
        result.is_err(),
        "conflicting bounds (min(0,3) + min(1,3) on 4 cells) should contradict"
    );
}

#[test]
fn max_fraction_basic() {
    let chain = Chain1D { len: 100 };
    let rules = UnconstrainedRules { num_states: 3 };
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 100 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint = FrequencyConstraint::builder()
        .max_fraction(0, 0.1, 100)
        .build();
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint])
        .unwrap();

    let count_0 = result.states().iter().filter(|&&s| s == 0).count();
    assert!(
        count_0 <= 10,
        "expected at most 10 cells with state 0 (10%), got {count_0}"
    );
}

#[test]
fn max_fraction_clamps_to_one() {
    // 0.001 * 10 = 0.01, floor = 0, but should clamp to 1
    let chain = Chain1D { len: 10 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 50 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint = FrequencyConstraint::builder()
        .max_fraction(0, 0.001, 10)
        .build();
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint])
        .unwrap();

    let count_0 = result.states().iter().filter(|&&s| s == 0).count();
    // max should be 1 (clamped from 0), so at most 1 occurrence
    assert!(
        count_0 <= 1,
        "expected max=1 (clamped), so count(0) <= 1, got {count_0}"
    );
}

#[test]
fn frequency_with_pinning() {
    let chain = Chain1D { len: 4 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new()
        .seed(0, 1)
        .backtrack(BacktrackStrategy::Restart { max_restarts: 50 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint = FrequencyConstraint::builder()
        .max(1, 1)
        .build();
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint])
        .unwrap();

    assert_eq!(result.state(0), 1, "pinned cell should remain state 1");
    let count_1 = result.states().iter().filter(|&&s| s == 1).count();
    assert_eq!(
        count_1, 1,
        "only the pinned cell should have state 1, got {count_1}"
    );
}

#[test]
fn frequency_with_backtracking() {
    let chain = Chain1D { len: 6 };
    let rules = UnconstrainedRules { num_states: 3 };
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 200 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(12345);

    // Tight bounds: state 0 exactly 2, state 1 exactly 2 (leaves 2 for state 2)
    let constraint = FrequencyConstraint::builder()
        .bound(0, 2, 2)
        .bound(1, 2, 2)
        .build();
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint])
        .unwrap();

    let count_0 = result.states().iter().filter(|&&s| s == 0).count();
    let count_1 = result.states().iter().filter(|&&s| s == 1).count();
    assert_eq!(count_0, 2, "expected exactly 2 of state 0, got {count_0}");
    assert_eq!(count_1, 2, "expected exactly 2 of state 1, got {count_1}");
}

#[test]
fn empty_bounds_is_noop() {
    let chain = Chain1D { len: 4 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint = FrequencyConstraint::builder().build();
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint]);

    assert!(result.is_ok(), "empty bounds should succeed without affecting solve");
}
