//! Error types for the WFC solver.

use core::fmt;

/// Index of a cell that caused a contradiction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Contradiction {
    /// The cell index where no possible states remained.
    pub cell: usize,
}

impl fmt::Display for Contradiction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "contradiction at cell {}", self.cell)
    }
}

/// Errors returned by the WFC solver.
#[derive(Debug, Clone)]
pub enum WfcError {
    /// A contradiction was reached and could not be resolved.
    Contradiction {
        /// Details about where the contradiction occurred.
        contradiction: Contradiction,
        /// The partial solver state at the time of contradiction (possibilities per cell).
        partial_state: alloc::vec::Vec<crate::bitset::BitSet128>,
        /// Backtrack depth reached before giving up (0 if backtracking disabled).
        backtrack_depth: usize,
    },
    /// The number of states exceeds BitSet128 capacity (128).
    TooManyStates {
        /// The number of states that was requested.
        requested: usize,
        /// The maximum supported.
        max: usize,
    },
    /// A pinned cell/state combination is invalid.
    InvalidPin {
        /// The cell index.
        cell: usize,
        /// The state that was requested.
        state: usize,
    },
}

impl fmt::Display for WfcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Contradiction {
                contradiction,
                backtrack_depth,
                ..
            } => {
                write!(f, "{contradiction} (backtrack depth: {backtrack_depth})")
            }
            Self::TooManyStates { requested, max } => {
                write!(f, "too many states: {requested} (max {max})")
            }
            Self::InvalidPin { cell, state } => {
                write!(f, "invalid pin: cell {cell}, state {state}")
            }
        }
    }
}
