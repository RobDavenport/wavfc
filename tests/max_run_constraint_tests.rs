use wavfc::*;
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

// ---- Helper: check no run of `state` exceeds `max_run` ----

fn max_run_length(states: &[usize], target: usize) -> usize {
    let mut max = 0;
    let mut current = 0;
    for &s in states {
        if s == target {
            current += 1;
            if current > max {
                max = current;
            }
        } else {
            current = 0;
        }
    }
    max
}

// ---- Tests ----

#[test]
fn max_run_prevents_long_sequences() {
    let chain = Chain1D { len: 8 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 200 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint = MaxRunConstraint::builder()
        .max_run(0, 3)
        .build(2);
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint])
        .unwrap();

    let run_0 = max_run_length(result.states(), 0);
    assert!(
        run_0 <= 3,
        "expected max run of state 0 <= 3, got {run_0}; states: {:?}",
        result.states()
    );
}

#[test]
fn max_run_one_forces_alternation() {
    let chain = Chain1D { len: 4 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 200 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint = MaxRunConstraint::builder()
        .max_run(0, 1)
        .max_run(1, 1)
        .build(2);
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint])
        .unwrap();

    let states = result.states();
    // Every adjacent pair must differ
    for i in 0..states.len() - 1 {
        assert_ne!(
            states[i], states[i + 1],
            "cells {} and {} should alternate, got {:?}",
            i, i + 1, states
        );
    }
}

#[test]
fn max_run_equals_grid_is_noop() {
    let chain = Chain1D { len: 4 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint = MaxRunConstraint::builder()
        .max_run(0, 4)
        .build(2);
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint]);

    assert!(result.is_ok(), "max_run equal to grid size should succeed");
}

#[test]
fn max_run_impossible_contradicts() {
    // 6 cells, 2 states, seeds create sandwiched uncollapsed cells.
    // Cells 0=0, 2=1, 3=0, 5=1. Cells 1 and 4 are uncollapsed.
    // Cell 1 is between 0(=0) and 2(=1): state 0 extends run from cell 0, state 1
    // extends run from cell 2. Both removed -> contradiction.
    // Cell 4 is between 3(=0) and 5(=1): same situation.
    // No restart can resolve this.
    let chain = Chain1D { len: 6 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new()
        .seed(0, 0)
        .seed(2, 1)
        .seed(3, 0)
        .seed(5, 1)
        .backtrack(BacktrackStrategy::Restart { max_restarts: 10 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint = MaxRunConstraint::builder()
        .max_run(0, 1)
        .max_run(1, 1)
        .build(2);
    let mut observer = NoOpObserver;
    let result = solver.solve_with(&mut rng, &mut observer, &[&constraint]);

    assert!(
        result.is_err(),
        "sandwiched cells with max_run(0,1) + max_run(1,1) should contradict"
    );
}

#[test]
fn default_max_run_applies_to_all() {
    let chain = Chain1D { len: 6 };
    let rules = UnconstrainedRules { num_states: 3 };
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 200 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint = MaxRunConstraint::builder()
        .default_max_run(2)
        .build(3);
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint])
        .unwrap();

    for s in 0..3 {
        let run = max_run_length(result.states(), s);
        assert!(
            run <= 2,
            "expected max run of state {s} <= 2, got {run}; states: {:?}",
            result.states()
        );
    }
}

#[test]
fn per_state_overrides_default() {
    let chain = Chain1D { len: 6 };
    let rules = UnconstrainedRules { num_states: 3 };
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 200 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint = MaxRunConstraint::builder()
        .default_max_run(2)
        .max_run(0, 4)
        .build(3);
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint])
        .unwrap();

    // State 0 can run up to 4, others max 2
    let run_0 = max_run_length(result.states(), 0);
    assert!(
        run_0 <= 4,
        "state 0 should allow run up to 4, got {run_0}; states: {:?}",
        result.states()
    );

    for s in 1..3 {
        let run = max_run_length(result.states(), s);
        assert!(
            run <= 2,
            "expected max run of state {s} <= 2, got {run}; states: {:?}",
            result.states()
        );
    }
}

#[test]
fn max_run_with_backtracking() {
    let chain = Chain1D { len: 8 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 500 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(99);

    // Tight limits: no more than 2 consecutive of either state
    let constraint = MaxRunConstraint::builder()
        .max_run(0, 2)
        .max_run(1, 2)
        .build(2);
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint])
        .unwrap();

    for s in 0..2 {
        let run = max_run_length(result.states(), s);
        assert!(
            run <= 2,
            "expected max run of state {s} <= 2, got {run}; states: {:?}",
            result.states()
        );
    }
}

#[test]
fn max_run_empty_is_noop() {
    let chain = Chain1D { len: 4 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    // No limits configured at all
    let constraint = MaxRunConstraint::builder().build(2);
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint]);

    assert!(
        result.is_ok(),
        "empty max_run constraint should succeed without affecting solve"
    );
}

