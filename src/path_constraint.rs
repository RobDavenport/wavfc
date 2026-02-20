//! Path connectivity constraint for WFC.
//!
//! Ensures that designated "required" cells are all connected via path tiles
//! in the final solution. Uses a Union-Find data structure to efficiently
//! check connectivity between collapsed path cells.

use alloc::vec;
use alloc::vec::Vec;

use crate::bitset::BitSet;
use crate::constraint::{GlobalConstraint, SolverState};
use crate::error::Contradiction;
use crate::topology::Topology;

/// Disjoint-set (Union-Find) data structure with path compression and union by rank.
///
/// Used internally by [`PathConstraint`] to track connected components of path tiles.
pub struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    /// Creates a new Union-Find with `n` disjoint singleton sets.
    pub fn new(n: usize) -> Self {
        let mut parent = Vec::with_capacity(n);
        for i in 0..n {
            parent.push(i);
        }
        Self {
            parent,
            rank: vec![0; n],
        }
    }

    /// Finds the representative of the set containing `x`, with path compression.
    pub fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    /// Merges the sets containing `x` and `y` using union by rank.
    pub fn union(&mut self, x: usize, y: usize) {
        let rx = self.find(x);
        let ry = self.find(y);
        if rx == ry {
            return;
        }
        match self.rank[rx].cmp(&self.rank[ry]) {
            core::cmp::Ordering::Less => {
                self.parent[rx] = ry;
            }
            core::cmp::Ordering::Greater => {
                self.parent[ry] = rx;
            }
            core::cmp::Ordering::Equal => {
                self.parent[ry] = rx;
                self.rank[rx] += 1;
            }
        }
    }

    /// Returns `true` if `x` and `y` are in the same set.
    pub fn connected(&mut self, x: usize, y: usize) -> bool {
        self.find(x) == self.find(y)
    }
}

/// A global constraint that ensures required cells are connected via path tiles.
///
/// Path tiles are identified by a [`BitSet`] of state indices. Required cells
/// must all collapse to path states and be reachable from each other through
/// adjacent path cells.
///
/// # Algorithm
///
/// During enforcement:
/// 1. A Union-Find is built over all cells.
/// 2. Collapsed path cells are unioned with their collapsed path neighbors.
/// 3. If any required cell is collapsed to a non-path state, a contradiction is returned.
/// 4. All collapsed-to-path required cells must be in the same connected component.
/// 5. Uncollapsed required cells are skipped (they may still become path tiles).
/// 6. No proactive pruning is performed.
pub struct PathConstraint<const W: usize = 2> {
    /// Which tile states count as "path" tiles.
    path_states: BitSet<W>,
    /// Cell indices that must be connected via path tiles.
    required_cells: Vec<usize>,
}

impl<const W: usize> PathConstraint<W> {
    /// Creates a new path constraint.
    ///
    /// # Arguments
    ///
    /// * `path_states` - A bitset indicating which states are path tiles.
    /// * `required_cells` - Cell indices that must all be connected via path tiles.
    pub fn new(path_states: BitSet<W>, required_cells: Vec<usize>) -> Self {
        Self {
            path_states,
            required_cells,
        }
    }

    /// Returns `true` if the given cell's possibilities indicate it is collapsed
    /// to exactly one state, and that state is a path state.
    fn is_collapsed_path(&self, poss: &BitSet<W>) -> bool {
        if !poss.is_singleton() {
            return false;
        }
        // The cell is collapsed to a single state; check if it's a path state.
        if let Some(state) = poss.first_one() {
            self.path_states.test(state)
        } else {
            false
        }
    }

    /// Returns `true` if the given cell's possibilities indicate it is collapsed
    /// to exactly one state (regardless of whether it's a path state).
    fn is_collapsed(poss: &BitSet<W>) -> bool {
        poss.is_singleton()
    }
}

impl<T: Topology, const W: usize> GlobalConstraint<T, W> for PathConstraint<W> {
    fn enforce(&self, state: &mut SolverState<'_, T, W>) -> Result<(), Contradiction> {
        // If there are no required cells, the constraint is trivially satisfied.
        if self.required_cells.is_empty() {
            return Ok(());
        }

        let num_cells = state.num_cells();

        // Step 1: Build UnionFind over all cells.
        let mut uf = UnionFind::new(num_cells);

        // Step 2: For each collapsed path cell, union it with collapsed path neighbors.
        for cell in 0..num_cells {
            let poss = *state.possibilities(cell);
            if !self.is_collapsed_path(&poss) {
                continue;
            }
            // Union with each neighbor that is also a collapsed path cell.
            for (neighbor, _dir) in state.topology().neighbors(cell) {
                let nb_poss = *state.possibilities(neighbor);
                if self.is_collapsed_path(&nb_poss) {
                    uf.union(cell, neighbor);
                }
            }
        }

        // Step 3: Check required cells collapsed to non-path states.
        // Step 4: Check connectivity of collapsed path required cells.
        let mut first_collapsed_path: Option<usize> = None;

        for &req_cell in &self.required_cells {
            let poss = *state.possibilities(req_cell);

            if Self::is_collapsed(&poss) {
                // The cell is collapsed.
                if !self.is_collapsed_path(&poss) {
                    // Step 3: Required cell collapsed to a non-path state.
                    return Err(Contradiction { cell: req_cell });
                }

                // It's a collapsed path cell. Check connectivity.
                match first_collapsed_path {
                    None => {
                        first_collapsed_path = Some(req_cell);
                    }
                    Some(first) => {
                        // Step 4: All collapsed path required cells must share a component.
                        if !uf.connected(first, req_cell) {
                            return Err(Contradiction { cell: req_cell });
                        }
                    }
                }
            }
            // Step 5: Uncollapsed required cells are skipped.
        }

        Ok(())
    }
}
