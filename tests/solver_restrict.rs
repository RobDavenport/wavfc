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

// ---- Alternating Rules: A-B must alternate ----

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

// ---- Terrain Rules for 2D grid ----
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

// ---- Tests ----

/// Restrict a cell to 2 states, verify only those remain.
#[test]
fn restrict_cell_to_two_states() {
    let chain = Chain1D { len: 5 };
    let rules = UnconstrainedRules { num_states: 4 };
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();

    // Allow only states 1 and 3 for cell 2
    let mut allowed = BitSet128::new();
    allowed.set(1);
    allowed.set(3);
    solver.restrict(2, &allowed).unwrap();

    // Solve and check that cell 2 is either state 1 or state 3
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();
    let s = result.state(2);
    assert!(
        s == 1 || s == 3,
        "cell 2 should be state 1 or 3 after restrict, got {s}"
    );
}

/// Restrict a cell to 0 states, verify Contradiction error.
#[test]
fn restrict_to_empty_yields_contradiction() {
    let chain = Chain1D { len: 3 };
    let rules = UnconstrainedRules { num_states: 3 };
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();

    // Allow nothing for cell 1
    let empty = BitSet128::new();
    let result = solver.restrict(1, &empty);
    match result {
        Err(WfcError::Contradiction { contradiction, .. }) => {
            assert_eq!(contradiction.cell, 1, "contradiction should be at cell 1");
        }
        other => panic!("expected Contradiction error, got {other:?}"),
    }
}

/// Restrict to current possibilities (no-op), verify Ok(()).
#[test]
fn restrict_noop_when_superset() {
    let chain = Chain1D { len: 3 };
    let rules = UnconstrainedRules { num_states: 3 };
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();

    // Allow all 3 states (superset of current possibilities which are {0,1,2})
    let mut allowed = BitSet128::new();
    allowed.set(0);
    allowed.set(1);
    allowed.set(2);
    // Also set extra bits beyond num_states -- should still be a superset
    allowed.set(3);

    let result = solver.restrict(0, &allowed);
    assert!(result.is_ok(), "restricting to superset should be a no-op");

    // Verify solve still works
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();
    assert_eq!(result.states().len(), 3);
}

/// Restrict triggers propagation: 2x2 grid, restrict one cell, verify neighbors updated.
#[test]
fn restrict_triggers_propagation_on_grid() {
    // 2x2 grid with alternating-like terrain rules.
    // Use AlternatingRules on a 1D chain for a simple propagation test:
    // If we restrict cell 0 to only state 0, alternating rules force cell 1 = 1.
    let chain = Chain1D { len: 4 };
    let rules = AlternatingRules;
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();

    // Restrict cell 0 to only state 0
    let mut allowed = BitSet128::new();
    allowed.set(0);
    solver.restrict(0, &allowed).unwrap();

    // After propagation, the entire chain should be determined:
    // cell 0=0, cell 1=1, cell 2=0, cell 3=1
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    assert_eq!(result.state(0), 0, "cell 0 restricted to 0");
    assert_eq!(result.state(1), 1, "cell 1 forced to 1 by propagation");
    assert_eq!(result.state(2), 0, "cell 2 forced to 0 by propagation");
    assert_eq!(result.state(3), 1, "cell 3 forced to 1 by propagation");
}

/// Restrict triggers propagation on a 2x2 grid with terrain rules.
#[test]
fn restrict_propagates_on_2x2_grid() {
    // 2x2 grid with terrain rules (grass/water/sand).
    // Restrict cell 0 to grass only (state 0). Since grass is not compatible
    // with water directly, neighbors should lose water as an option.
    let grid = Grid2D { width: 2, height: 2 };
    let rules = TerrainRules;
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(grid, &rules, config).unwrap();

    // Restrict cell 0 to only grass (state 0)
    let mut allowed = BitSet128::new();
    allowed.set(0);
    solver.restrict(0, &allowed).unwrap();

    // Solve and verify all adjacency constraints hold
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    assert_eq!(result.state(0), 0, "cell 0 should be grass");
    // Cell 1 (east neighbor) should be grass(0) or sand(2), not water(1)
    let c1 = result.state(1);
    assert!(
        c1 == 0 || c1 == 2,
        "cell 1 (east of grass) should be grass or sand, got {c1}"
    );
    // Cell 2 (south neighbor) should be grass(0) or sand(2), not water(1)
    let c2 = result.state(2);
    assert!(
        c2 == 0 || c2 == 2,
        "cell 2 (south of grass) should be grass or sand, got {c2}"
    );
}

/// pin() still works after refactor (basic pin + solve).
#[test]
fn pin_still_works_after_refactor() {
    let chain = Chain1D { len: 6 };
    let rules = AlternatingRules;
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
    solver.pin(0, 0).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    // With alternating rules and cell 0 pinned to 0:
    // 0, 1, 0, 1, 0, 1
    for i in 0..6 {
        let expected = i % 2;
        assert_eq!(
            result.state(i),
            expected,
            "cell {i} should be {expected} after pin(0, 0)"
        );
    }
}

/// pin() error cases still work after refactor.
#[test]
fn pin_invalid_cell_still_errors() {
    let chain = Chain1D { len: 3 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();

    let result = solver.pin(10, 0);
    match result {
        Err(WfcError::InvalidPin { cell, state }) => {
            assert_eq!(cell, 10);
            assert_eq!(state, 0);
        }
        other => panic!("expected InvalidPin error, got {other:?}"),
    }
}

/// pin() error for invalid state still works.
#[test]
fn pin_invalid_state_still_errors() {
    let chain = Chain1D { len: 3 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();

    let result = solver.pin(0, 5);
    match result {
        Err(WfcError::InvalidPin { cell, state }) => {
            assert_eq!(cell, 0);
            assert_eq!(state, 5);
        }
        other => panic!("expected InvalidPin error, got {other:?}"),
    }
}

/// Restrict is idempotent: calling twice with same mask is fine.
#[test]
fn restrict_idempotent() {
    let chain = Chain1D { len: 5 };
    let rules = UnconstrainedRules { num_states: 4 };
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();

    let mut allowed = BitSet128::new();
    allowed.set(0);
    allowed.set(2);

    // First restrict
    solver.restrict(2, &allowed).unwrap();
    // Second restrict with the same mask -- should be a no-op
    solver.restrict(2, &allowed).unwrap();

    // Solve and verify cell 2 is either 0 or 2
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();
    let s = result.state(2);
    assert!(
        s == 0 || s == 2,
        "cell 2 should be 0 or 2 after double restrict, got {s}"
    );
}

/// Restrict out-of-bounds cell returns error.
#[test]
fn restrict_out_of_bounds_cell() {
    let chain = Chain1D { len: 3 };
    let rules = UnconstrainedRules { num_states: 2 };
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();

    let mut allowed = BitSet128::new();
    allowed.set(0);
    let result = solver.restrict(10, &allowed);
    match result {
        Err(WfcError::InvalidPin { cell, state }) => {
            assert_eq!(cell, 10);
            assert_eq!(state, 0);
        }
        other => panic!("expected InvalidPin error, got {other:?}"),
    }
}

/// Restrict then pin still works.
#[test]
fn restrict_then_pin() {
    let chain = Chain1D { len: 5 };
    let rules = UnconstrainedRules { num_states: 4 };
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();

    // Restrict cell 0 to states {1, 2, 3}
    let mut allowed = BitSet128::new();
    allowed.set(1);
    allowed.set(2);
    allowed.set(3);
    solver.restrict(0, &allowed).unwrap();

    // Then pin cell 0 to state 2 (which is still allowed)
    solver.pin(0, 2).unwrap();

    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();
    assert_eq!(result.state(0), 2, "cell 0 should be pinned to 2");
}

/// Restrict progressively narrows possibilities.
#[test]
fn restrict_progressive_narrowing() {
    let chain = Chain1D { len: 3 };
    let rules = UnconstrainedRules { num_states: 5 };
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(chain, &rules, config).unwrap();

    // First restrict: allow {0, 1, 2, 3}
    let mut allowed1 = BitSet128::new();
    allowed1.set(0);
    allowed1.set(1);
    allowed1.set(2);
    allowed1.set(3);
    solver.restrict(1, &allowed1).unwrap();

    // Second restrict: allow {1, 3, 4} -- intersection with current {0,1,2,3} = {1, 3}
    let mut allowed2 = BitSet128::new();
    allowed2.set(1);
    allowed2.set(3);
    allowed2.set(4);
    solver.restrict(1, &allowed2).unwrap();

    // Solve and verify cell 1 is either 1 or 3
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();
    let s = result.state(1);
    assert!(
        s == 1 || s == 3,
        "cell 1 should be 1 or 3 after progressive restrict, got {s}"
    );
}
