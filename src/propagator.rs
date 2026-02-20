//! Propagation engines for constraint propagation.
//!
//! Two engines are provided:
//! - **AC-3**: Low memory usage, O(remaining_states) per removal. Best for constrained targets.
//! - **AC-4**: High memory usage, O(1) per removal. Best for large grids.
//!
//! Select the engine via [`PropagatorKind`].

use alloc::vec;
use alloc::vec::Vec;

use crate::bitset::BitSet;
use crate::error::Contradiction;
use crate::rules::CompatibilityTable;
use crate::topology::Topology;

/// Which propagation engine to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PropagatorKind {
    /// Low memory, O(remaining_states) per removal. Best for constrained targets.
    #[default]
    Ac3,
    /// High memory, O(1) per removal. Best for large grids.
    Ac4,
}

/// AC-3 propagation engine.
///
/// Uses a simple worklist of (cell, removed_state) pairs. For each removal,
/// recomputes the full support set for each neighbor.
pub(crate) struct Ac3Propagator {
    /// Worklist of (cell_index, removed_state_index) pairs to process.
    stack: Vec<(usize, usize)>,
}

impl Ac3Propagator {
    /// Creates a new AC-3 propagator.
    fn new() -> Self {
        Self { stack: Vec::new() }
    }

    /// Runs propagation from the given initial removals.
    fn propagate<const W: usize>(
        &mut self,
        possibilities: &mut [BitSet<W>],
        entropy_cache: &mut [u32],
        topo: &impl Topology,
        compat: &CompatibilityTable<W>,
        initial_stack: &[(usize, usize)],
    ) -> Result<(), Contradiction> {
        self.stack.clear();
        self.stack.extend_from_slice(initial_stack);

        while let Some((cell, _removed_state)) = self.stack.pop() {
            // For each neighbor of the changed cell
            for (neighbor, dir) in topo.neighbors(cell) {
                let dir_idx = topo.direction_index(dir);

                // Compute the support set: what states are compatible with
                // the current possibilities of `cell` when viewed from `neighbor`?
                let mut support = BitSet::<W>::new();
                for s in possibilities[cell].iter_ones() {
                    // s is a state still possible in `cell`.
                    // For each such state, what states does it allow in `neighbor`
                    // in direction `dir` from `cell`?
                    // compatible(s, t, dir) means "s can have t in direction dir",
                    // which is equivalent to "t can have s in opposite(dir)".
                    support |= *compat.compatible_states(s, dir_idx);
                }

                // Restrict neighbor's possibilities to the support set
                let old = possibilities[neighbor];
                let new = {
                    let mut tmp = old;
                    tmp &= support;
                    tmp
                };

                if new != old {
                    // Some states were removed from the neighbor
                    let removed = old.and_not(&new);
                    possibilities[neighbor] = new;
                    let new_count = new.count_ones();
                    entropy_cache[neighbor] = new_count;

                    if new_count == 0 {
                        return Err(Contradiction { cell: neighbor });
                    }

                    // Push all removed states for further propagation
                    for s in removed.iter_ones() {
                        self.stack.push((neighbor, s));
                    }
                }
            }
        }

        Ok(())
    }
}

/// AC-4 propagation engine.
///
/// Maintains per-cell, per-state, per-direction support counters for O(1)
/// removal processing. Uses more memory but is faster for large grids.
pub(crate) struct Ac4Propagator {
    /// Support counters: `support_count[cell * num_states * num_dirs + state * num_dirs + dir]`
    /// counts how many states in the neighbor along `dir` support `state` in `cell`.
    support_count: Vec<u16>,
    /// Number of states.
    num_states: usize,
    /// Number of directions.
    num_dirs: usize,
    /// Worklist of (cell_index, removed_state_index) pairs to process.
    stack: Vec<(usize, usize)>,
}

impl Ac4Propagator {
    /// Creates a new AC-4 propagator and initializes support counters.
    fn new<const W: usize>(
        topo: &impl Topology,
        compat: &CompatibilityTable<W>,
        possibilities: &[BitSet<W>],
    ) -> Self {
        let num_cells = topo.num_cells();
        let num_states = compat.num_states();
        let num_dirs = compat.num_directions();
        let total = num_cells * num_states * num_dirs;
        let mut support_count = vec![0u16; total];

        // Initialize support counters
        for cell in 0..num_cells {
            for (neighbor, dir) in topo.neighbors(cell) {
                let dir_idx = topo.direction_index(dir);
                // For each state s in cell, count how many states in neighbor support it
                for s in 0..num_states {
                    if !possibilities[cell].test(s) {
                        continue;
                    }
                    let mut count = 0u16;
                    let compat_set = compat.compatible_states(s, dir_idx);
                    for nb_state in possibilities[neighbor].iter_ones() {
                        if compat_set.test(nb_state) {
                            count += 1;
                        }
                    }
                    support_count[cell * num_states * num_dirs + s * num_dirs + dir_idx] = count;
                }
            }
        }

        Self {
            support_count,
            num_states,
            num_dirs,
            stack: Vec::new(),
        }
    }

    /// Reinitializes support counters (e.g., after backtrack restore).
    fn reinitialize<const W: usize>(
        &mut self,
        topo: &impl Topology,
        compat: &CompatibilityTable<W>,
        possibilities: &[BitSet<W>],
    ) {
        let num_cells = topo.num_cells();
        // Zero out
        for v in self.support_count.iter_mut() {
            *v = 0;
        }

        for cell in 0..num_cells {
            for (neighbor, dir) in topo.neighbors(cell) {
                let dir_idx = topo.direction_index(dir);
                for s in 0..self.num_states {
                    if !possibilities[cell].test(s) {
                        continue;
                    }
                    let mut count = 0u16;
                    let compat_set = compat.compatible_states(s, dir_idx);
                    for nb_state in possibilities[neighbor].iter_ones() {
                        if compat_set.test(nb_state) {
                            count += 1;
                        }
                    }
                    self.support_count[cell * self.num_states * self.num_dirs + s * self.num_dirs + dir_idx] = count;
                }
            }
        }
    }

    /// Runs propagation from the given initial removals.
    fn propagate<const W: usize>(
        &mut self,
        possibilities: &mut [BitSet<W>],
        entropy_cache: &mut [u32],
        topo: &impl Topology,
        compat: &CompatibilityTable<W>,
        initial_stack: &[(usize, usize)],
    ) -> Result<(), Contradiction> {
        self.stack.clear();
        self.stack.extend_from_slice(initial_stack);

        while let Some((cell, removed_state)) = self.stack.pop() {
            // For each neighbor of the cell where a state was removed
            for (neighbor, dir) in topo.neighbors(cell) {
                let opp_dir = topo.opposite(dir);
                let opp_dir_idx = topo.direction_index(opp_dir);

                // For each state s still possible in the neighbor,
                // check if the removed_state was supporting it
                // We need to iterate a copy since we may modify possibilities[neighbor]
                let neighbor_poss = possibilities[neighbor];
                for s in neighbor_poss.iter_ones() {
                    // Was removed_state compatible with s in the opposite direction?
                    let compat_set = compat.compatible_states(s, opp_dir_idx);
                    if !compat_set.test(removed_state) {
                        continue;
                    }

                    // Decrement the support counter
                    let idx = neighbor * self.num_states * self.num_dirs
                        + s * self.num_dirs
                        + opp_dir_idx;
                    self.support_count[idx] = self.support_count[idx].saturating_sub(1);

                    if self.support_count[idx] == 0 {
                        // No more support for state s in neighbor from this direction
                        possibilities[neighbor].clear(s);
                        let new_count = possibilities[neighbor].count_ones();
                        entropy_cache[neighbor] = new_count;

                        if new_count == 0 {
                            return Err(Contradiction { cell: neighbor });
                        }

                        self.stack.push((neighbor, s));
                    }
                }
            }
        }

        Ok(())
    }
}

/// Enum wrapper for propagation engines.
///
/// Dispatches to either [`Ac3Propagator`] or [`Ac4Propagator`] based on
/// the configured [`PropagatorKind`].
pub(crate) enum Propagator {
    /// AC-3 propagation engine.
    Ac3(Ac3Propagator),
    /// AC-4 propagation engine.
    Ac4(Ac4Propagator),
}

impl Propagator {
    /// Creates a new propagator of the specified kind.
    pub fn new<const W: usize>(
        kind: PropagatorKind,
        topo: &impl Topology,
        compat: &CompatibilityTable<W>,
        possibilities: &[BitSet<W>],
    ) -> Self {
        match kind {
            PropagatorKind::Ac3 => Propagator::Ac3(Ac3Propagator::new()),
            PropagatorKind::Ac4 => Propagator::Ac4(Ac4Propagator::new(topo, compat, possibilities)),
        }
    }

    /// Runs constraint propagation from the given initial removals.
    ///
    /// Returns `Err(Contradiction)` if any cell is reduced to zero possibilities.
    pub fn propagate<const W: usize>(
        &mut self,
        possibilities: &mut [BitSet<W>],
        entropy_cache: &mut [u32],
        topo: &impl Topology,
        compat: &CompatibilityTable<W>,
        stack: &[(usize, usize)],
    ) -> Result<(), Contradiction> {
        match self {
            Propagator::Ac3(p) => p.propagate(possibilities, entropy_cache, topo, compat, stack),
            Propagator::Ac4(p) => p.propagate(possibilities, entropy_cache, topo, compat, stack),
        }
    }

    /// Reinitializes the propagator state (needed for AC-4 after backtrack restore).
    pub fn reinitialize<const W: usize>(
        &mut self,
        topo: &impl Topology,
        compat: &CompatibilityTable<W>,
        possibilities: &[BitSet<W>],
    ) {
        match self {
            Propagator::Ac3(_) => {
                // AC-3 is stateless, nothing to reinitialize
            }
            Propagator::Ac4(p) => p.reinitialize(topo, compat, possibilities),
        }
    }
}
