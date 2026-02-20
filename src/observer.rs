//! Observer trait for monitoring solve progress.
//!
//! Implement the [`Observer`] trait to receive callbacks during the solve process.
//! Use [`NoOpObserver`] when no monitoring is needed.

use crate::topology::Topology;

/// Callback interface for monitoring solver progress.
///
/// All methods have default no-op implementations, so users only need to
/// override the events they care about.
pub trait Observer<T: Topology> {
    /// Called after a cell is collapsed to a single state.
    fn on_collapse(&mut self, _cell: usize, _state: usize) {}

    /// Called after propagation completes for a step.
    fn on_propagation_complete(&mut self) {}

    /// Called when a contradiction is detected.
    fn on_contradiction(&mut self, _cell: usize) {}

    /// Called when backtracking occurs.
    fn on_backtrack(&mut self, _depth: usize) {}
}

/// A no-op observer that does nothing.
///
/// This is the default observer used when no monitoring is needed.
pub struct NoOpObserver;

impl<T: Topology> Observer<T> for NoOpObserver {}
