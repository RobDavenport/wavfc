//! Frequency constraint for WFC.
//!
//! Enforces per-state minimum and maximum occurrence counts across the grid.
//! Stateless: recomputes counts from [`SolverState`] each call, so it works
//! correctly with all backtracking strategies.

use alloc::vec::Vec;

use crate::bitset::BitSet;
use crate::constraint::{GlobalConstraint, SolverState};
use crate::error::Contradiction;
use crate::topology::Topology;

/// A single min/max occurrence bound for a tile state.
#[derive(Debug, Clone, Copy)]
pub struct FrequencyBound {
    /// The tile state index this bound applies to.
    pub state: usize,
    /// Minimum number of cells that must collapse to this state.
    pub min: usize,
    /// Maximum number of cells that may collapse to this state.
    pub max: usize,
}

/// A global constraint that enforces per-state minimum and maximum occurrence
/// counts across the entire grid.
///
/// Created via [`FrequencyConstraint::builder`].
///
/// # Algorithm
///
/// For each bound, a single pass over all cells counts:
/// - **committed**: cells already collapsed to the bound's state
/// - **possible**: cells that can still become the bound's state (including committed)
///
/// Then:
/// - **Max enforcement**: when `committed >= max`, the state is removed from all
///   uncollapsed cells. If any cell hits 0 possibilities, a [`Contradiction`] is returned.
/// - **Min enforcement**: when `possible < min`, a [`Contradiction`] is returned.
///   When `possible == min && committed < min`, all remaining possible cells are
///   forced to the state.
pub struct FrequencyConstraint<const W: usize = 2> {
    bounds: Vec<FrequencyBound>,
}

impl<const W: usize> FrequencyConstraint<W> {
    /// Returns a new builder for constructing a `FrequencyConstraint`.
    pub fn builder() -> FrequencyConstraintBuilder<W> {
        FrequencyConstraintBuilder {
            bounds: Vec::new(),
        }
    }
}

impl<T: Topology, const W: usize> GlobalConstraint<T, W> for FrequencyConstraint<W> {
    fn enforce(&self, state: &mut SolverState<'_, T, W>) -> Result<(), Contradiction> {
        let num_cells = state.num_cells();

        for bound in &self.bounds {
            let target = bound.state;
            let mut committed = 0usize;
            let mut possible = 0usize;

            // Single pass: count committed and possible
            for cell in 0..num_cells {
                let poss = state.possibilities(cell);
                if poss.test(target) {
                    possible += 1;
                    if poss.is_singleton() {
                        committed += 1;
                    }
                }
            }

            // --- Max enforcement ---
            if committed >= bound.max {
                // Remove the state from all uncollapsed cells
                for cell in 0..num_cells {
                    let poss = *state.possibilities(cell);
                    if poss.test(target) && !poss.is_singleton() {
                        state.possibilities_mut(cell).clear(target);
                        let new_count = state.possibilities(cell).count_ones();
                        state.set_entropy(cell, new_count);
                        if new_count == 0 {
                            return Err(Contradiction { cell });
                        }
                    }
                }
            }

            // --- Min enforcement ---
            if possible < bound.min {
                // Not enough cells can even possibly hold this state
                // Find a cell to blame
                for cell in 0..num_cells {
                    if !state.possibilities(cell).test(target)
                        && state.possibilities(cell).count_ones() > 0
                    {
                        return Err(Contradiction { cell });
                    }
                }
                return Err(Contradiction { cell: 0 });
            }

            if possible == bound.min && committed < bound.min {
                // Force all remaining possible cells to this state
                for cell in 0..num_cells {
                    let poss = *state.possibilities(cell);
                    if poss.test(target) && !poss.is_singleton() {
                        let mut only = BitSet::<W>::new();
                        only.set(target);
                        *state.possibilities_mut(cell) = only;
                        state.set_entropy(cell, 1);
                    }
                }
            }
        }

        Ok(())
    }
}

/// Builder for [`FrequencyConstraint`].
///
/// Accumulates [`FrequencyBound`]s and produces a finished constraint via
/// [`build`](FrequencyConstraintBuilder::build).
pub struct FrequencyConstraintBuilder<const W: usize = 2> {
    bounds: Vec<FrequencyBound>,
}

impl<const W: usize> FrequencyConstraintBuilder<W> {
    /// Sets a maximum occurrence count for `state`. The minimum defaults to 0.
    pub fn max(mut self, state: usize, count: usize) -> Self {
        self.bounds.push(FrequencyBound {
            state,
            min: 0,
            max: count,
        });
        self
    }

    /// Sets a minimum occurrence count for `state`. The maximum defaults to `usize::MAX`.
    pub fn min(mut self, state: usize, count: usize) -> Self {
        self.bounds.push(FrequencyBound {
            state,
            min: count,
            max: usize::MAX,
        });
        self
    }

    /// Sets both minimum and maximum occurrence counts for `state`.
    ///
    /// # Panics
    ///
    /// Panics if `min > max`.
    pub fn bound(mut self, state: usize, min: usize, max: usize) -> Self {
        assert!(min <= max, "FrequencyBound: min ({min}) must be <= max ({max})");
        self.bounds.push(FrequencyBound { state, min, max });
        self
    }

    /// Convenience: sets a maximum as a fraction of total cells.
    ///
    /// Computes `floor(fraction * total_cells)`, but clamps to at least 1
    /// when `fraction > 0.0`.
    pub fn max_fraction(self, state: usize, fraction: f64, total_cells: usize) -> Self {
        let count = fraction_to_max_count(fraction, total_cells);
        self.max(state, count)
    }

    /// Convenience: sets a minimum as a fraction of total cells.
    ///
    /// Computes `ceil(fraction * total_cells)`.
    pub fn min_fraction(self, state: usize, fraction: f64, total_cells: usize) -> Self {
        let count = fraction_to_min_count(fraction, total_cells);
        self.min(state, count)
    }

    /// Consumes the builder and produces a [`FrequencyConstraint`].
    pub fn build(self) -> FrequencyConstraint<W> {
        FrequencyConstraint {
            bounds: self.bounds,
        }
    }
}

/// Converts a fraction to a max count: `floor(fraction * total)`, clamped to >= 1 if fraction > 0.
///
/// Uses truncation (`as usize`) which is equivalent to floor for non-negative values.
fn fraction_to_max_count(fraction: f64, total: usize) -> usize {
    let raw = (fraction * total as f64) as usize;
    if fraction > 0.0 && raw == 0 {
        1
    } else {
        raw
    }
}

/// Converts a fraction to a min count: `ceil(fraction * total)`.
///
/// Implemented without `f64::ceil` for `no_std` compatibility.
fn fraction_to_min_count(fraction: f64, total: usize) -> usize {
    let product = fraction * total as f64;
    let truncated = product as usize;
    // If there was a fractional part, round up
    if (truncated as f64) < product {
        truncated + 1
    } else {
        truncated
    }
}
