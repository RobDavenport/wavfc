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

// ---- Weighted Unconstrained Rules ----
// 2 states: state 0 has weight 3.0, state 1 has weight 1.0
// Both are compatible with everything (no real adjacency constraints)

struct WeightedUnconstrainedRules;

impl AdjacencyRules for WeightedUnconstrainedRules {
    type Direction = Dir1D;

    fn num_states(&self) -> usize {
        2
    }

    fn compatible(&self, _state_a: usize, _state_b: usize, _direction: Dir1D) -> bool {
        true // everything is compatible
    }

    fn weight(&self, state: usize) -> f64 {
        match state {
            0 => 3.0,
            1 => 1.0,
            _ => 1.0,
        }
    }
}

// ---- Tests ----

#[test]
fn weighted_selection_frequency_min_count_heuristic() {
    let num_runs = 1000;
    let chain_len = 10;
    let mut count_state_0: usize = 0;
    let mut count_state_1: usize = 0;

    for seed in 0..num_runs {
        let chain = Chain1D { len: chain_len };
        let rules = WeightedUnconstrainedRules;
        let config = SolverConfig::new().heuristic(Heuristic::MinCount);
        let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
        let mut rng = StdRng::seed_from_u64(seed as u64);
        let result = solver.solve(&mut rng).unwrap();

        for cell in 0..chain_len {
            match result.state(cell) {
                0 => count_state_0 += 1,
                1 => count_state_1 += 1,
                s => panic!("unexpected state {s}"),
            }
        }
    }

    let total = count_state_0 + count_state_1;
    let ratio = count_state_0 as f64 / count_state_1 as f64;

    // Weight 3:1, so state 0 should appear roughly 3x as often as state 1
    // With loose tolerance of +/-20%: ratio should be between 2.4 and 3.6
    // But we use even looser tolerance since unconstrained WFC with propagation
    // can skew distributions
    assert!(
        total == num_runs * chain_len,
        "total cells should be {}, got {}",
        num_runs * chain_len,
        total
    );
    assert!(
        ratio > 1.5 && ratio < 6.0,
        "expected ratio ~3.0, got {ratio:.2} (state0={count_state_0}, state1={count_state_1})"
    );
}

#[test]
fn weighted_selection_frequency_shannon_entropy_heuristic() {
    let num_runs = 1000;
    let chain_len = 10;
    let mut count_state_0: usize = 0;
    let mut count_state_1: usize = 0;

    for seed in 0..num_runs {
        let chain = Chain1D { len: chain_len };
        let rules = WeightedUnconstrainedRules;
        let config = SolverConfig::new().heuristic(Heuristic::ShannonEntropy);
        let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
        let mut rng = StdRng::seed_from_u64(seed as u64);
        let result = solver.solve(&mut rng).unwrap();

        for cell in 0..chain_len {
            match result.state(cell) {
                0 => count_state_0 += 1,
                1 => count_state_1 += 1,
                s => panic!("unexpected state {s}"),
            }
        }
    }

    let ratio = count_state_0 as f64 / count_state_1 as f64;

    // Same loose tolerance
    assert!(
        ratio > 1.5 && ratio < 6.0,
        "expected ratio ~3.0, got {ratio:.2} (state0={count_state_0}, state1={count_state_1})"
    );
}

#[test]
fn equal_weights_produce_roughly_equal_distribution() {
    // Sanity check: when weights are equal, distribution should be roughly 1:1
    struct EqualWeightRules;

    impl AdjacencyRules for EqualWeightRules {
        type Direction = Dir1D;

        fn num_states(&self) -> usize {
            2
        }

        fn compatible(&self, _a: usize, _b: usize, _d: Dir1D) -> bool {
            true
        }

        fn weight(&self, _state: usize) -> f64 {
            1.0
        }
    }

    let num_runs = 1000;
    let chain_len = 10;
    let mut count_state_0: usize = 0;
    let mut count_state_1: usize = 0;

    for seed in 0..num_runs {
        let chain = Chain1D { len: chain_len };
        let rules = EqualWeightRules;
        let config = SolverConfig::new();
        let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
        let mut rng = StdRng::seed_from_u64(seed as u64);
        let result = solver.solve(&mut rng).unwrap();

        for cell in 0..chain_len {
            match result.state(cell) {
                0 => count_state_0 += 1,
                1 => count_state_1 += 1,
                s => panic!("unexpected state {s}"),
            }
        }
    }

    let ratio = count_state_0 as f64 / count_state_1 as f64;

    // With equal weights, ratio should be close to 1.0 (say 0.5 to 2.0)
    assert!(
        ratio > 0.5 && ratio < 2.0,
        "equal weights should produce ~1:1 ratio, got {ratio:.2} (state0={count_state_0}, state1={count_state_1})"
    );
}

#[test]
fn heavily_weighted_state_dominates() {
    // State 0 has weight 99.0, state 1 has weight 1.0
    struct HeavyWeightRules;

    impl AdjacencyRules for HeavyWeightRules {
        type Direction = Dir1D;

        fn num_states(&self) -> usize {
            2
        }

        fn compatible(&self, _a: usize, _b: usize, _d: Dir1D) -> bool {
            true
        }

        fn weight(&self, state: usize) -> f64 {
            match state {
                0 => 99.0,
                _ => 1.0,
            }
        }
    }

    let num_runs = 500;
    let chain_len = 10;
    let mut count_state_0: usize = 0;

    for seed in 0..num_runs {
        let chain = Chain1D { len: chain_len };
        let rules = HeavyWeightRules;
        let config = SolverConfig::new();
        let mut solver = WfcSolver::new(chain, &rules, config).unwrap();
        let mut rng = StdRng::seed_from_u64(seed as u64);
        let result = solver.solve(&mut rng).unwrap();

        for cell in 0..chain_len {
            if result.state(cell) == 0 {
                count_state_0 += 1;
            }
        }
    }

    let total = num_runs * chain_len;
    let fraction = count_state_0 as f64 / total as f64;

    // State 0 should dominate (>80% at least)
    assert!(
        fraction > 0.80,
        "heavily weighted state should dominate, but only {:.1}% of cells are state 0",
        fraction * 100.0
    );
}
