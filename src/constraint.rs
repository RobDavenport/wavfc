//! Global constraint trait for non-local constraint logic.
//!
//! Global constraints allow injecting additional logic into the solve loop
//! beyond basic adjacency rules. They run after each propagation pass.

use crate::bitset::BitSet;
use crate::error::Contradiction;
use crate::topology::Topology;

/// Mutable view of the solver state for global constraints.
///
/// Provides access to the current possibilities, entropy cache, and topology,
/// allowing global constraints to inspect and modify solver state.
pub struct SolverState<'a, T: Topology, const W: usize = 2> {
    possibilities: &'a mut [BitSet<W>],
    entropy_cache: &'a mut [u32],
    num_states: usize,
    topology: &'a T,
}

impl<'a, T: Topology, const W: usize> SolverState<'a, T, W> {
    /// Creates a new solver state view.
    pub(crate) fn new(
        possibilities: &'a mut [BitSet<W>],
        entropy_cache: &'a mut [u32],
        num_states: usize,
        topology: &'a T,
    ) -> Self {
        Self {
            possibilities,
            entropy_cache,
            num_states,
            topology,
        }
    }

    /// Returns a reference to the topology.
    #[inline]
    pub fn topology(&self) -> &T {
        self.topology
    }

    /// Returns the possibilities bitset for a cell.
    #[inline]
    pub fn possibilities(&self, cell: usize) -> &BitSet<W> {
        &self.possibilities[cell]
    }

    /// Returns a mutable reference to the possibilities bitset for a cell.
    #[inline]
    pub fn possibilities_mut(&mut self, cell: usize) -> &mut BitSet<W> {
        &mut self.possibilities[cell]
    }

    /// Returns the cached entropy (remaining state count) for a cell.
    #[inline]
    pub fn entropy(&self, cell: usize) -> u32 {
        self.entropy_cache[cell]
    }

    /// Sets the cached entropy for a cell.
    #[inline]
    pub fn set_entropy(&mut self, cell: usize, val: u32) {
        self.entropy_cache[cell] = val;
    }

    /// Returns the total number of cells.
    #[inline]
    pub fn num_cells(&self) -> usize {
        self.possibilities.len()
    }

    /// Returns the total number of possible states.
    #[inline]
    pub fn num_states(&self) -> usize {
        self.num_states
    }
}

/// Allows injecting non-local constraint logic into the solve loop.
///
/// Called after each propagation pass. May further restrict cell possibilities.
///
/// # Warning
///
/// This is a power-user API. Incorrect implementations can cause unsolvable
/// states or violate solver invariants.
pub trait GlobalConstraint<T: Topology, const W: usize = 2> {
    /// Enforce this constraint on the current solver state.
    ///
    /// Return `Err(Contradiction)` to trigger backtracking.
    fn enforce(&self, state: &mut SolverState<'_, T, W>) -> Result<(), Contradiction>;
}
