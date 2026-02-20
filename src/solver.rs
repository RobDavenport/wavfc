//! Core WFC solver implementation.
//!
//! The [`WfcSolver`] drives the observe-collapse-propagate loop. It is generic
//! over the [`Topology`] to support arbitrary dimensionalities and graph shapes.

use alloc::vec;
use alloc::vec::Vec;

use rand_core::RngCore;

use crate::backtrack::{BacktrackStrategy, SnapshotStack};
use crate::bitset::BitSet128;
use crate::config::SolverConfig;
use crate::constraint::{GlobalConstraint, SolverState};
use crate::entropy::{self, Heuristic};
use crate::error::{Contradiction, WfcError};
use crate::observer::{NoOpObserver, Observer};
use crate::propagator::Propagator;
use crate::rules::{AdjacencyRules, CompatibilityTable};
use crate::topology::Topology;

/// Result of a successful solve.
///
/// Contains the collapsed state for each cell and statistics about the solve.
pub struct SolveResult {
    states: Vec<usize>,
    iterations: usize,
}

impl SolveResult {
    /// Get the collapsed state for a cell.
    #[inline]
    pub fn state(&self, cell: usize) -> usize {
        self.states[cell]
    }

    /// Get all states as a slice.
    #[inline]
    pub fn states(&self) -> &[usize] {
        &self.states
    }

    /// Number of observe-collapse-propagate iterations.
    #[inline]
    pub fn iterations(&self) -> usize {
        self.iterations
    }
}

/// Result of a single solver step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepResult {
    /// A cell was collapsed to a state and propagation completed.
    Collapsed {
        /// The cell that was collapsed.
        cell: usize,
        /// The state it was collapsed to.
        state: usize,
    },
    /// A contradiction was detected.
    Contradiction {
        /// The cell where the contradiction occurred.
        cell: usize,
    },
    /// All cells are collapsed -- solve is complete.
    Complete,
}

/// The core Wave Function Collapse solver.
///
/// Generic over the [`Topology`] to support arbitrary world structures.
///
/// # Usage
///
/// ```ignore
/// let solver = WfcSolver::new(topology, &rules, config)?;
/// let result = solver.solve(&mut rng)?;
/// ```
pub struct WfcSolver<T: Topology> {
    topo: T,
    compat: CompatibilityTable,
    possibilities: Vec<BitSet128>,
    entropy_cache: Vec<u32>,
    collapsed_count: usize,
    propagator: Propagator,
    config: SolverConfig,
    snapshots: SnapshotStack,
    iterations: usize,
}

impl<T: Topology> WfcSolver<T> {
    /// Creates a new WFC solver.
    ///
    /// # Errors
    ///
    /// Returns [`WfcError::TooManyStates`] if `rules.num_states() > 128`.
    /// Returns [`WfcError::InvalidPin`] if any seed references an invalid cell or state.
    pub fn new<R: AdjacencyRules<Direction = T::Direction>>(
        topo: T,
        rules: &R,
        config: SolverConfig,
    ) -> Result<Self, WfcError> {
        let num_states = rules.num_states();
        if num_states > BitSet128::MAX_BITS {
            return Err(WfcError::TooManyStates {
                requested: num_states,
                max: BitSet128::MAX_BITS,
            });
        }

        let num_cells = topo.num_cells();
        let compat = CompatibilityTable::new(&topo, rules);
        let full = BitSet128::full(num_states);
        let possibilities = vec![full; num_cells];
        let entropy_cache = vec![num_states as u32; num_cells];

        let propagator = Propagator::new(config.propagator, &topo, &compat, &possibilities);
        let snapshots = SnapshotStack::new();

        let mut solver = Self {
            topo,
            compat,
            possibilities,
            entropy_cache,
            collapsed_count: 0,
            propagator,
            config,
            snapshots,
            iterations: 0,
        };

        // Apply seeds
        let seeds = solver.config.seeds.clone();
        for (cell, state) in seeds {
            solver.pin(cell, state)?;
        }

        Ok(solver)
    }

    /// Pins a cell to a single state.
    ///
    /// All other states are removed and the change is propagated.
    ///
    /// # Errors
    ///
    /// Returns [`WfcError::InvalidPin`] if the cell or state is out of range,
    /// or if the state is not currently possible for the cell.
    pub fn pin(&mut self, cell: usize, state: usize) -> Result<(), WfcError> {
        if cell >= self.topo.num_cells() || state >= self.compat.num_states() {
            return Err(WfcError::InvalidPin { cell, state });
        }

        if !self.possibilities[cell].test(state) {
            return Err(WfcError::InvalidPin { cell, state });
        }

        // Collect removed states
        let old = self.possibilities[cell];
        let mut only = BitSet128::new();
        only.set(state);
        self.possibilities[cell] = only;

        let was_collapsed = self.entropy_cache[cell] <= 1;
        self.entropy_cache[cell] = 1;
        if !was_collapsed {
            self.collapsed_count += 1;
        }

        // Build removal list
        let removed = old.and_not(&only);
        let removals: Vec<(usize, usize)> = removed.iter_ones().map(|s| (cell, s)).collect();

        // Propagate
        self.propagate_from(&removals).map_err(|c| WfcError::Contradiction {
            contradiction: c,
            partial_state: self.possibilities.clone(),
            backtrack_depth: 0,
        })
    }

    /// Runs the full solve loop with no observer and no global constraints.
    ///
    /// This is the primary solve method matching the PRD signature `solver.solve(&mut rng)`.
    ///
    /// # Errors
    ///
    /// Returns [`WfcError::Contradiction`] if the puzzle cannot be solved.
    pub fn solve(&mut self, rng: &mut impl RngCore) -> Result<SolveResult, WfcError> {
        let mut observer = NoOpObserver;
        let constraints: &[&NoConstraint<T>] = &[];
        self.solve_with(rng, &mut observer, constraints)
    }

    /// Runs the full solve loop with an observer and global constraints.
    ///
    /// Repeatedly observes, collapses, propagates, and enforces global constraints
    /// until all cells are collapsed or an unrecoverable contradiction is reached.
    ///
    /// # Errors
    ///
    /// Returns [`WfcError::Contradiction`] if the puzzle cannot be solved within
    /// the configured backtracking limits.
    pub fn solve_with<O: Observer<T>, G: GlobalConstraint<T>>(
        &mut self,
        rng: &mut impl RngCore,
        observer: &mut O,
        constraints: &[&G],
    ) -> Result<SolveResult, WfcError> {
        match self.config.backtrack {
            BacktrackStrategy::None => self.solve_no_backtrack(rng, observer, constraints),
            BacktrackStrategy::Restart { max_restarts } => {
                self.solve_with_restarts(rng, observer, constraints, max_restarts)
            }
            BacktrackStrategy::Chronological { max_depth } => {
                self.solve_chronological(rng, observer, constraints, max_depth)
            }
        }
    }

    /// Performs a single step of the solve loop.
    ///
    /// Returns the result of the step without handling backtracking.
    pub fn step(&mut self, rng: &mut impl RngCore) -> StepResult {
        // Observe: select the next cell to collapse
        let cell = match self.observe(rng) {
            Some(c) => c,
            None => return StepResult::Complete,
        };

        // Collapse: pick a state for the cell
        let state = self.collapse(cell, rng);

        // Propagate
        let old = self.possibilities[cell];
        let mut only = BitSet128::new();
        only.set(state);
        self.possibilities[cell] = only;
        self.entropy_cache[cell] = 1;
        self.collapsed_count += 1;

        let removed = old.and_not(&only);
        let removals: Vec<(usize, usize)> = removed.iter_ones().map(|s| (cell, s)).collect();

        match self.propagate_from(&removals) {
            Ok(()) => {
                self.iterations += 1;
                StepResult::Collapsed { cell, state }
            }
            Err(c) => StepResult::Contradiction { cell: c.cell },
        }
    }

    /// Solves with a post-solve validation function.
    ///
    /// After each successful solve, the validation function is called. If it
    /// returns `false`, the solver is reset and retries up to `max_retries` times.
    ///
    /// # Errors
    ///
    /// Returns [`WfcError::Contradiction`] if no valid solution is found within
    /// the retry limit.
    pub fn solve_with_validation(
        &mut self,
        rng: &mut impl RngCore,
        validate: impl Fn(&[usize]) -> bool,
        max_retries: usize,
    ) -> Result<SolveResult, WfcError> {
        let num_states = self.compat.num_states();
        for _ in 0..=max_retries {
            self.reset(num_states);
            // Re-apply seeds
            let seeds = self.config.seeds.clone();
            for (cell, state) in seeds {
                self.pin(cell, state)?;
            }

            match self.solve(rng) {
                Ok(result) => {
                    if validate(result.states()) {
                        return Ok(result);
                    }
                    // Validation failed, retry
                }
                Err(_) => {
                    // Contradiction, retry
                }
            }
        }

        Err(WfcError::Contradiction {
            contradiction: Contradiction { cell: 0 },
            partial_state: self.possibilities.clone(),
            backtrack_depth: 0,
        })
    }

    // ---- Private helpers ----

    /// Selects the next cell to collapse using the configured heuristic.
    fn observe(&self, rng: &mut impl RngCore) -> Option<usize> {
        match self.config.heuristic {
            Heuristic::MinCount => entropy::select_min_count(&self.entropy_cache, rng),
            Heuristic::ShannonEntropy => {
                entropy::select_shannon(&self.possibilities, self.compat.weights(), rng)
            }
        }
    }

    /// Collapses a cell to a single state using weighted random selection.
    fn collapse(&self, cell: usize, rng: &mut impl RngCore) -> usize {
        let poss = &self.possibilities[cell];
        let weights = self.compat.weights();

        // Compute total weight of remaining states
        let mut total_weight = 0.0;
        for s in poss.iter_ones() {
            total_weight += weights[s];
        }

        if total_weight <= 0.0 {
            // Fallback: return first available state
            return poss.first_one().unwrap_or(0);
        }

        // Generate a random value in [0, total_weight)
        let r = random_f64(rng) * total_weight;

        let mut cumulative = 0.0;
        let mut last_state = 0;
        for s in poss.iter_ones() {
            cumulative += weights[s];
            last_state = s;
            if cumulative > r {
                return s;
            }
        }

        // Floating point edge case: return the last state
        last_state
    }

    /// Propagates removals through the constraint network.
    fn propagate_from(&mut self, removals: &[(usize, usize)]) -> Result<(), Contradiction> {
        self.propagator.propagate(
            &mut self.possibilities,
            &mut self.entropy_cache,
            &self.topo,
            &self.compat,
            removals,
        )
    }

    /// Resets the solver state for restart backtracking.
    fn reset(&mut self, num_states: usize) {
        let num_cells = self.topo.num_cells();
        let full = BitSet128::full(num_states);
        for p in self.possibilities.iter_mut() {
            *p = full;
        }
        for e in self.entropy_cache.iter_mut() {
            *e = num_states as u32;
        }
        self.collapsed_count = 0;
        self.iterations = 0;
        self.snapshots.clear();
        self.propagator
            .reinitialize(&self.topo, &self.compat, &self.possibilities);
        let _ = num_cells;
    }

    /// Solve loop without backtracking.
    fn solve_no_backtrack<O: Observer<T>, G: GlobalConstraint<T>>(
        &mut self,
        rng: &mut impl RngCore,
        observer: &mut O,
        constraints: &[&G],
    ) -> Result<SolveResult, WfcError> {
        loop {
            // Observe
            let cell = match self.observe(rng) {
                Some(c) => c,
                None => return Ok(self.build_result()),
            };

            // Collapse
            let state = self.collapse(cell, rng);
            let old = self.possibilities[cell];
            let mut only = BitSet128::new();
            only.set(state);
            self.possibilities[cell] = only;
            self.entropy_cache[cell] = 1;
            self.collapsed_count += 1;
            self.iterations += 1;
            observer.on_collapse(cell, state);

            // Propagate
            let removed = old.and_not(&only);
            let removals: Vec<(usize, usize)> = removed.iter_ones().map(|s| (cell, s)).collect();

            if let Err(c) = self.propagate_from(&removals) {
                observer.on_contradiction(c.cell);
                return Err(WfcError::Contradiction {
                    contradiction: c,
                    partial_state: self.possibilities.clone(),
                    backtrack_depth: 0,
                });
            }

            observer.on_propagation_complete();

            // Global constraints (with re-propagation loop)
            if let Err(c) = self.enforce_constraints(constraints) {
                observer.on_contradiction(c.cell);
                return Err(WfcError::Contradiction {
                    contradiction: c,
                    partial_state: self.possibilities.clone(),
                    backtrack_depth: 0,
                });
            }
        }
    }

    /// Solve loop with restart backtracking.
    fn solve_with_restarts<O: Observer<T>, G: GlobalConstraint<T>>(
        &mut self,
        rng: &mut impl RngCore,
        observer: &mut O,
        constraints: &[&G],
        max_restarts: usize,
    ) -> Result<SolveResult, WfcError> {
        let num_states = self.compat.num_states();
        let mut last_error = None;

        for restart in 0..=max_restarts {
            if restart > 0 {
                self.reset(num_states);
                // Re-apply seeds
                let seeds = self.config.seeds.clone();
                for (cell, state) in seeds {
                    self.pin(cell, state)?;
                }
            }

            match self.solve_no_backtrack(rng, observer, constraints) {
                Ok(result) => return Ok(result),
                Err(e) => {
                    observer.on_backtrack(restart);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| WfcError::Contradiction {
            contradiction: Contradiction { cell: 0 },
            partial_state: self.possibilities.clone(),
            backtrack_depth: max_restarts,
        }))
    }

    /// Solve loop with chronological backtracking.
    fn solve_chronological<O: Observer<T>, G: GlobalConstraint<T>>(
        &mut self,
        rng: &mut impl RngCore,
        observer: &mut O,
        constraints: &[&G],
        max_depth: usize,
    ) -> Result<SolveResult, WfcError> {
        loop {
            // Observe
            let cell = match self.observe(rng) {
                Some(c) => c,
                None => return Ok(self.build_result()),
            };

            // Save snapshot before collapsing
            let mut tried = BitSet128::new();
            let state = self.collapse(cell, rng);
            tried.set(state);

            if self.snapshots.depth() < max_depth {
                self.snapshots.push(
                    &self.possibilities,
                    &self.entropy_cache,
                    cell,
                    tried,
                    self.collapsed_count,
                );
            }

            // Collapse
            let old = self.possibilities[cell];
            let mut only = BitSet128::new();
            only.set(state);
            self.possibilities[cell] = only;
            self.entropy_cache[cell] = 1;
            self.collapsed_count += 1;
            self.iterations += 1;
            observer.on_collapse(cell, state);

            // Propagate
            let removed = old.and_not(&only);
            let removals: Vec<(usize, usize)> = removed.iter_ones().map(|s| (cell, s)).collect();

            let prop_result = self.propagate_from(&removals);
            let constraint_result = if prop_result.is_ok() {
                observer.on_propagation_complete();
                self.enforce_constraints(constraints)
            } else {
                prop_result
            };

            if let Err(c) = constraint_result {
                observer.on_contradiction(c.cell);

                // Try to backtrack
                if !self.try_backtrack(rng, observer, constraints)? {
                    return Err(WfcError::Contradiction {
                        contradiction: c,
                        partial_state: self.possibilities.clone(),
                        backtrack_depth: max_depth,
                    });
                }
            }
        }
    }

    /// Attempts to backtrack to a previous state and try a different choice.
    ///
    /// Returns `Ok(true)` if backtracking succeeded and solving can continue.
    /// Returns `Ok(false)` if no more backtrack options are available.
    fn try_backtrack<O: Observer<T>, G: GlobalConstraint<T>>(
        &mut self,
        rng: &mut impl RngCore,
        observer: &mut O,
        constraints: &[&G],
    ) -> Result<bool, WfcError> {
        while let Some(snapshot) = self.snapshots.pop() {
            observer.on_backtrack(self.snapshots.depth());

            // Restore state
            self.possibilities.copy_from_slice(&snapshot.possibilities);
            self.entropy_cache.copy_from_slice(&snapshot.entropy_cache);
            self.collapsed_count = snapshot.collapsed_count;

            // Reinitialize propagator for new state
            self.propagator
                .reinitialize(&self.topo, &self.compat, &self.possibilities);

            let cell = snapshot.collapsed_cell;
            let mut tried = snapshot.tried_states;

            // Find an untried state for this cell
            let available = self.possibilities[cell].and_not(&tried);
            if available.is_empty() {
                // No more states to try at this level, backtrack further
                continue;
            }

            // Pick a new state from the untried ones
            let state = self.collapse_from_set(cell, &available, rng);
            tried.set(state);

            // Re-push snapshot with updated tried states (if we have depth budget)
            let max_depth = match self.config.backtrack {
                BacktrackStrategy::Chronological { max_depth } => max_depth,
                _ => 0,
            };
            if self.snapshots.depth() < max_depth {
                self.snapshots.push(
                    &self.possibilities,
                    &self.entropy_cache,
                    cell,
                    tried,
                    self.collapsed_count,
                );
            }

            // Collapse to the new state
            let old = self.possibilities[cell];
            let mut only = BitSet128::new();
            only.set(state);
            self.possibilities[cell] = only;
            self.entropy_cache[cell] = 1;
            self.collapsed_count += 1;
            self.iterations += 1;
            observer.on_collapse(cell, state);

            // Propagate
            let removed = old.and_not(&only);
            let removals: Vec<(usize, usize)> = removed.iter_ones().map(|s| (cell, s)).collect();

            let prop_result = self.propagate_from(&removals);
            let constraint_result = if prop_result.is_ok() {
                observer.on_propagation_complete();
                self.enforce_constraints(constraints)
            } else {
                prop_result
            };

            if constraint_result.is_ok() {
                return Ok(true);
            }

            // This choice also failed, continue backtracking
            // (the snapshot we just pushed will be popped on the next iteration)
        }

        Ok(false)
    }

    /// Collapses a cell choosing from a specific set of states (for backtracking).
    fn collapse_from_set(&self, _cell: usize, available: &BitSet128, rng: &mut impl RngCore) -> usize {
        let weights = self.compat.weights();
        let mut total_weight = 0.0;
        for s in available.iter_ones() {
            total_weight += weights[s];
        }

        if total_weight <= 0.0 {
            return available.first_one().unwrap_or(0);
        }

        let r = random_f64(rng) * total_weight;
        let mut cumulative = 0.0;
        let mut last_state = 0;
        for s in available.iter_ones() {
            cumulative += weights[s];
            last_state = s;
            if cumulative > r {
                return s;
            }
        }

        last_state
    }

    /// Enforces all global constraints, re-triggering propagation when constraints
    /// remove states, as specified in PRD Section 4.4 step 6.
    ///
    /// Loops until convergence: after each constraint pass, if any possibilities
    /// were reduced, the removed states are propagated and constraints are re-checked.
    fn enforce_constraints<G: GlobalConstraint<T>>(
        &mut self,
        constraints: &[&G],
    ) -> Result<(), Contradiction> {
        if constraints.is_empty() {
            return Ok(());
        }

        loop {
            // Snapshot the current possibilities before enforcing constraints
            let snapshot: Vec<BitSet128> = self.possibilities.to_vec();

            let num_states = self.compat.num_states();
            {
                let mut state = SolverState::new(
                    &mut self.possibilities,
                    &mut self.entropy_cache,
                    num_states,
                    &self.topo,
                );

                for constraint in constraints {
                    constraint.enforce(&mut state)?;
                }
            }

            // Diff: find all removed states and build a propagation stack
            let mut removals: Vec<(usize, usize)> = Vec::new();
            for (cell, old_poss) in snapshot.iter().enumerate() {
                let removed = old_poss.and_not(&self.possibilities[cell]);
                if !removed.is_empty() {
                    // Update entropy cache for this cell
                    self.entropy_cache[cell] = self.possibilities[cell].count_ones();
                    if self.entropy_cache[cell] == 0 {
                        return Err(Contradiction { cell });
                    }
                    for s in removed.iter_ones() {
                        removals.push((cell, s));
                    }
                }
            }

            if removals.is_empty() {
                // Convergence: no constraint made any changes
                break;
            }

            // Propagate the removals caused by constraints
            self.propagate_from(&removals)?;

            // Loop back to re-check constraints after propagation
        }

        // Update collapsed count after constraints may have changed things
        self.collapsed_count = 0;
        for e in self.entropy_cache.iter() {
            if *e <= 1 {
                self.collapsed_count += 1;
            }
        }

        Ok(())
    }

    /// Builds the final result from fully collapsed state.
    fn build_result(&self) -> SolveResult {
        let states: Vec<usize> = self
            .possibilities
            .iter()
            .map(|p| p.first_one().unwrap_or(0))
            .collect();

        SolveResult {
            states,
            iterations: self.iterations,
        }
    }
}

/// A dummy global constraint that does nothing (used for type inference).
struct NoConstraint<T: Topology>(core::marker::PhantomData<T>);

impl<T: Topology> GlobalConstraint<T> for NoConstraint<T> {
    fn enforce(&self, _state: &mut SolverState<'_, T>) -> Result<(), Contradiction> {
        Ok(())
    }
}

/// Generates a random f64 in [0, 1) from an RngCore.
fn random_f64(rng: &mut impl RngCore) -> f64 {
    // Use the upper 53 bits of a u64 for a uniform f64 in [0, 1)
    let val = rng.next_u64();
    (val >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
}
