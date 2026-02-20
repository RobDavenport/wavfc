//! Backtracking strategies and snapshot management.
//!
//! Provides different strategies for handling contradictions during solving,
//! and a snapshot stack for chronological backtracking.

use alloc::vec::Vec;

use crate::bitset::BitSet128;

/// Backtracking strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BacktrackStrategy {
    /// Fail immediately on contradiction.
    #[default]
    None,
    /// Re-initialize and retry with new RNG state, up to N times.
    Restart {
        /// Maximum number of restarts before giving up.
        max_restarts: usize,
    },
    /// Save state before each collapse, restore on contradiction.
    Chronological {
        /// Maximum backtrack depth before giving up.
        max_depth: usize,
    },
}

/// A saved snapshot of solver state for backtracking.
#[derive(Clone)]
pub(crate) struct Snapshot {
    /// The possibilities at the time of the snapshot.
    pub possibilities: Vec<BitSet128>,
    /// The entropy cache at the time of the snapshot.
    pub entropy_cache: Vec<u32>,
    /// The cell that was being collapsed.
    pub collapsed_cell: usize,
    /// Which states have already been tried for this cell.
    pub tried_states: BitSet128,
    /// The collapsed count at the time of the snapshot.
    pub collapsed_count: usize,
}

/// Stack of snapshots for chronological backtracking.
pub(crate) struct SnapshotStack {
    snapshots: Vec<Snapshot>,
}

impl SnapshotStack {
    /// Creates a new empty snapshot stack.
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
        }
    }

    /// Saves the current state before a collapse.
    ///
    /// Records the possibilities, entropy cache, the cell being collapsed,
    /// and which states have been tried so far.
    pub fn push(
        &mut self,
        possibilities: &[BitSet128],
        entropy_cache: &[u32],
        collapsed_cell: usize,
        tried_states: BitSet128,
        collapsed_count: usize,
    ) {
        self.snapshots.push(Snapshot {
            possibilities: possibilities.to_vec(),
            entropy_cache: entropy_cache.to_vec(),
            collapsed_cell,
            tried_states,
            collapsed_count,
        });
    }

    /// Restores the previous state.
    ///
    /// Returns the snapshot so the solver can try the next untried state
    /// for the collapsed cell.
    pub fn pop(&mut self) -> Option<Snapshot> {
        self.snapshots.pop()
    }

    /// Returns the current backtrack depth.
    pub fn depth(&self) -> usize {
        self.snapshots.len()
    }

    /// Clears all snapshots.
    pub fn clear(&mut self) {
        self.snapshots.clear();
    }
}
