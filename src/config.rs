//! Solver configuration builder.
//!
//! Use [`SolverConfig`] to configure the WFC solver before creating it.
//! Supports a builder pattern for ergonomic configuration.

use alloc::vec::Vec;

use crate::backtrack::BacktrackStrategy;
use crate::entropy::Heuristic;
use crate::propagator::PropagatorKind;

/// Configuration for the WFC solver.
///
/// Use the builder methods to configure, then pass to
/// [`WfcSolver::new`](crate::solver::WfcSolver::new).
#[derive(Debug, Clone)]
pub struct SolverConfig {
    /// The backtracking strategy to use.
    pub(crate) backtrack: BacktrackStrategy,
    /// The cell selection heuristic.
    pub(crate) heuristic: Heuristic,
    /// The propagation engine to use.
    pub(crate) propagator: PropagatorKind,
    /// Pre-seeded cells: `(cell_index, state_index)` pairs.
    pub(crate) seeds: Vec<(usize, usize)>,
}

impl SolverConfig {
    /// Creates a new solver configuration with defaults.
    ///
    /// Defaults:
    /// - Backtracking: `None` (fail immediately on contradiction)
    /// - Heuristic: `MinCount`
    /// - Propagator: `Ac3`
    /// - No seeds
    pub fn new() -> Self {
        Self {
            backtrack: BacktrackStrategy::default(),
            heuristic: Heuristic::default(),
            propagator: PropagatorKind::default(),
            seeds: Vec::new(),
        }
    }

    /// Sets the backtracking strategy.
    pub fn backtrack(mut self, strategy: BacktrackStrategy) -> Self {
        self.backtrack = strategy;
        self
    }

    /// Sets the cell selection heuristic.
    pub fn heuristic(mut self, heuristic: Heuristic) -> Self {
        self.heuristic = heuristic;
        self
    }

    /// Sets the propagation engine.
    pub fn propagator(mut self, propagator: PropagatorKind) -> Self {
        self.propagator = propagator;
        self
    }

    /// Adds a single pre-seeded cell.
    ///
    /// The cell at `cell` will be pinned to `state` before solving begins.
    pub fn seed(mut self, cell: usize, state: usize) -> Self {
        self.seeds.push((cell, state));
        self
    }

    /// Sets all pre-seeded cells at once.
    ///
    /// Each tuple is `(cell_index, state_index)`.
    pub fn seeds(mut self, seeds: Vec<(usize, usize)>) -> Self {
        self.seeds = seeds;
        self
    }
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self::new()
    }
}
