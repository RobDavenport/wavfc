//! Adjacency rules trait and precomputed compatibility tables.
//!
//! The [`AdjacencyRules`] trait defines which tile states can appear next to each other
//! in each direction. [`CompatibilityTable`] precomputes bitmask lookups for fast
//! constraint propagation.

use alloc::vec;
use alloc::vec::Vec;

use crate::bitset::BitSet128;
use crate::topology::Topology;

/// Defines which states can be adjacent in which directions.
///
/// Implement this trait for your tile/state system to specify adjacency constraints.
pub trait AdjacencyRules {
    /// The direction type, must match the topology's direction type.
    type Direction: Copy + Eq;

    /// Number of distinct states/tile types.
    fn num_states(&self) -> usize;

    /// Can `state_a` exist when the neighbor in `direction` has `state_b`?
    fn compatible(&self, state_a: usize, state_b: usize, direction: Self::Direction) -> bool;

    /// Weight/frequency of a state. Higher = more likely to be chosen during collapse.
    ///
    /// Defaults to 1.0 for all states.
    fn weight(&self, state: usize) -> f64 {
        let _ = state;
        1.0
    }
}

/// Precomputed bitmask compatibility table.
///
/// Indexed as `table[state * num_directions + dir_index]`.
/// Value: bitset of all states compatible with `state` when looking in direction `dir`.
///
/// This precomputation converts O(n) per-state compatibility checks into O(1) bitmask lookups
/// during propagation.
pub struct CompatibilityTable {
    table: Vec<BitSet128>,
    num_states: usize,
    num_directions: usize,
    weights: Vec<f64>,
}

impl CompatibilityTable {
    /// Precomputes the compatibility table from a topology and adjacency rules.
    ///
    /// For every `(state, direction)` pair, builds a bitmask of all states that
    /// are compatible as neighbors in that direction.
    pub fn new<T: Topology, R: AdjacencyRules<Direction = T::Direction>>(
        topo: &T,
        rules: &R,
    ) -> Self {
        let num_states = rules.num_states();
        let num_directions = topo.directions().len();
        let mut table = vec![BitSet128::new(); num_states * num_directions];
        let mut weights = Vec::with_capacity(num_states);

        for state_a in 0..num_states {
            weights.push(rules.weight(state_a));
            for (dir_idx, &dir) in topo.directions().iter().enumerate() {
                let entry = &mut table[state_a * num_directions + dir_idx];
                for state_b in 0..num_states {
                    if rules.compatible(state_a, state_b, dir) {
                        entry.set(state_b);
                    }
                }
            }
        }

        Self {
            table,
            num_states,
            num_directions,
            weights,
        }
    }

    /// Returns the bitmask of states compatible with `state` when looking in
    /// direction `dir_index`.
    #[inline]
    pub fn compatible_states(&self, state: usize, dir_index: usize) -> &BitSet128 {
        &self.table[state * self.num_directions + dir_index]
    }

    /// Returns the number of states.
    #[inline]
    pub fn num_states(&self) -> usize {
        self.num_states
    }

    /// Returns the number of directions.
    #[inline]
    pub fn num_directions(&self) -> usize {
        self.num_directions
    }

    /// Returns the weight of a state.
    #[inline]
    pub fn weight(&self, state: usize) -> f64 {
        self.weights[state]
    }

    /// Returns the weights slice.
    #[inline]
    pub fn weights(&self) -> &[f64] {
        &self.weights
    }
}
