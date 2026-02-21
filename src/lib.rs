//! A generic, dimension-agnostic Wave Function Collapse library.
//!
//! `wavfc` provides a flexible WFC solver that works with any topology
//! (2D grids, 3D voxels, hex grids, arbitrary graphs) via the [`Topology`] trait.
//!
//! # Features
//!
//! - **Dimension-agnostic**: Works with any graph structure via the [`Topology`] trait
//! - **Dual propagation engines**: Choose between AC-3 (low memory) and AC-4 (fast) via [`PropagatorKind`]
//! - **Multiple heuristics**: Min-count or Shannon entropy cell selection via [`Heuristic`]
//! - **Backtracking**: None, restart, or chronological backtracking via [`BacktrackStrategy`]
//! - **Global constraints**: Inject custom constraint logic via [`GlobalConstraint`]
//! - **Observable**: Monitor solve progress via the [`Observer`] trait
//! - **`no_std` compatible**: Works in embedded and WASM environments
//!
//! # Quick Start
//!
//! 1. Implement [`Topology`] for your world structure
//! 2. Implement [`AdjacencyRules`] for your tile adjacency constraints
//! 3. Create a [`SolverConfig`] and [`WfcSolver`]
//! 4. Call [`WfcSolver::solve`] with an RNG

#![no_std]

extern crate alloc;

pub mod bitset;
pub mod topology;
pub mod rules;
pub mod error;
pub mod observer;
pub mod constraint;
pub mod entropy;
pub mod propagator;
pub mod backtrack;
pub mod config;
pub mod path_constraint;
pub mod frequency_constraint;
pub mod max_run_constraint;
pub mod solver;
pub mod socket;
pub mod rotate;

// Re-export primary API
pub use bitset::{BitSet, BitSet128, BitSet256};
pub use topology::Topology;
pub use rules::AdjacencyRules;
pub use error::{WfcError, Contradiction};
pub use observer::{Observer, NoOpObserver};
pub use constraint::{GlobalConstraint, SolverState};
pub use entropy::Heuristic;
pub use propagator::PropagatorKind;
pub use backtrack::BacktrackStrategy;
pub use config::SolverConfig;
pub use path_constraint::PathConstraint;
pub use frequency_constraint::FrequencyConstraint;
pub use max_run_constraint::MaxRunConstraint;
pub use solver::{WfcSolver, SolveResult, StepResult};
pub use socket::{Socket, SocketRuleBuilder, SocketRules, sockets_match};
pub use rotate::{Dir2D, Rotation, Symmetry, TileMapping, RotationExpander};
