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

// ---- MinStateCount Global Constraint ----

struct MinStateCount {
    required_state: usize,
    min_count: usize,
}

impl<T: Topology> GlobalConstraint<T> for MinStateCount {
    fn enforce(&self, state: &mut SolverState<'_, T>) -> Result<(), Contradiction> {
        // Count how many cells CAN still have the required state
        let mut possible_count = 0;
        // Count how many cells are already committed to the required state
        let mut committed_count = 0;

        for cell in 0..state.num_cells() {
            let poss = state.possibilities(cell);
            if poss.test(self.required_state) {
                possible_count += 1;
                if poss.is_singleton() {
                    committed_count += 1;
                }
            }
        }

        // If even the maximum possible count is less than min_count, contradiction
        if possible_count < self.min_count {
            // Find a cell to blame for the contradiction
            for cell in 0..state.num_cells() {
                if !state.possibilities(cell).test(self.required_state)
                    && state.possibilities(cell).count_ones() > 0
                {
                    return Err(Contradiction { cell });
                }
            }
            return Err(Contradiction { cell: 0 });
        }

        // If exactly min_count cells can have the required state,
        // and we haven't committed enough yet, force them
        if possible_count == self.min_count && committed_count < self.min_count {
            for cell in 0..state.num_cells() {
                let poss = *state.possibilities(cell);
                if poss.test(self.required_state) && !poss.is_singleton() {
                    // Force this cell to the required state
                    let mut only = BitSet128::new();
                    only.set(self.required_state);
                    *state.possibilities_mut(cell) = only;
                    state.set_entropy(cell, 1);
                }
            }
        }

        Ok(())
    }
}

// ---- Tests ----

#[test]
fn global_constraint_ensures_minimum_state_count() {
    let chain = Chain1D { len: 4 };
    let rules = UnconstrainedRules { num_states: 3 };
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 50 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint = MinStateCount {
        required_state: 0,
        min_count: 2,
    };
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint])
        .unwrap();

    // Count cells with state 0
    let count_state_0 = result.states().iter().filter(|&&s| s == 0).count();
    assert!(
        count_state_0 >= 2,
        "expected at least 2 cells with state 0, got {count_state_0}"
    );
}

#[test]
fn global_constraint_impossible_returns_error() {
    // Require 10 cells with state 0, but only 4 cells exist
    let chain = Chain1D { len: 4 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 10 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint = MinStateCount {
        required_state: 0,
        min_count: 10,
    };
    let mut observer = NoOpObserver;
    let result = solver.solve_with(&mut rng, &mut observer, &[&constraint]);

    assert!(result.is_err(), "impossible constraint should cause error");
}

#[test]
fn global_constraint_min_count_equals_grid_size_forces_all_same() {
    // Require ALL 4 cells to be state 0
    let chain = Chain1D { len: 4 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 50 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint = MinStateCount {
        required_state: 0,
        min_count: 4,
    };
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint])
        .unwrap();

    for cell in 0..4 {
        assert_eq!(
            result.state(cell),
            0,
            "all cells should be state 0 when min_count equals grid size"
        );
    }
}

#[test]
fn global_constraint_min_count_zero_is_noop() {
    // A min count of 0 is trivially satisfied
    let chain = Chain1D { len: 4 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint = MinStateCount {
        required_state: 0,
        min_count: 0,
    };
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint])
        .unwrap();

    // Should succeed regardless
    assert_eq!(result.states().len(), 4);
}

#[test]
fn multiple_global_constraints_both_enforced() {
    // Require at least 1 cell of state 0 AND at least 1 cell of state 1
    let chain = Chain1D { len: 4 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 50 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint_0 = MinStateCount {
        required_state: 0,
        min_count: 1,
    };
    let constraint_1 = MinStateCount {
        required_state: 1,
        min_count: 1,
    };
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint_0, &constraint_1])
        .unwrap();

    let count_0 = result.states().iter().filter(|&&s| s == 0).count();
    let count_1 = result.states().iter().filter(|&&s| s == 1).count();

    assert!(count_0 >= 1, "should have at least 1 cell with state 0");
    assert!(count_1 >= 1, "should have at least 1 cell with state 1");
}

#[test]
fn global_constraint_with_pinning() {
    // Pin cell 0 to state 1, require at least 2 cells with state 0
    let chain = Chain1D { len: 4 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new()
        .seed(0, 1)
        .backtrack(BacktrackStrategy::Restart { max_restarts: 50 });
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);

    let constraint = MinStateCount {
        required_state: 0,
        min_count: 2,
    };
    let mut observer = NoOpObserver;
    let result = solver
        .solve_with(&mut rng, &mut observer, &[&constraint])
        .unwrap();

    assert_eq!(result.state(0), 1, "pinned cell should remain state 1");
    let count_0 = result.states().iter().filter(|&&s| s == 0).count();
    assert!(
        count_0 >= 2,
        "should have at least 2 cells with state 0, got {count_0}"
    );
}
