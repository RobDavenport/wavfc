//! Max-run constraint for WFC.
//!
//! Prevents consecutive runs of the same tile state exceeding a configurable
//! length along any axis direction. Stateless: recomputes from [`SolverState`]
//! each call, so it works correctly with all backtracking strategies.

use alloc::vec::Vec;

use crate::constraint::{GlobalConstraint, SolverState};
use crate::error::Contradiction;
use crate::topology::Topology;

/// A per-state run-length limit.
#[derive(Debug, Clone, Copy)]
struct RunLimit {
    /// The tile state index this limit applies to.
    state: usize,
    /// Maximum consecutive cells of this state allowed along any single axis.
    max_run: usize,
}

/// A global constraint that prevents consecutive runs of the same tile state
/// from exceeding a configurable maximum length along any axis direction.
///
/// Created via [`MaxRunConstraint::builder`].
///
/// # Algorithm
///
/// For each direction pair `(D, opposite(D))` where
/// `direction_index(D) < direction_index(opposite(D))`, and for each
/// uncollapsed cell C, for each state S still possible in C:
///
/// - Walk from C in direction D counting consecutive collapsed-to-S cells
/// - Walk from C in the opposite direction counting the same
/// - If `run_fwd + run_bwd + 1 > max_run[S]`, remove S from C
///
/// Only half the directions are processed (using direction index comparison)
/// to avoid double-counting axis pairs.
pub struct MaxRunConstraint<const W: usize = 2> {
    /// Per-state run limits, indexed by state. `usize::MAX` means unlimited.
    limits: Vec<usize>,
}

impl<const W: usize> MaxRunConstraint<W> {
    /// Returns a new builder for constructing a `MaxRunConstraint`.
    pub fn builder() -> MaxRunConstraintBuilder<W> {
        MaxRunConstraintBuilder {
            per_state: Vec::new(),
            default_max_run: None,
        }
    }
}

/// Walk from `start` in direction `dir`, counting consecutive collapsed-to-`target_state`
/// cells. Starts from the immediate neighbor of `start` in `dir` and continues
/// walking in the same direction while cells are collapsed to `target_state`.
///
/// Returns the count of consecutive matching collapsed cells found.
fn walk_run<T: Topology, const W: usize>(
    state: &SolverState<'_, T, W>,
    target_state: usize,
    start: usize,
    dir: T::Direction,
) -> usize {
    let topo = state.topology();
    let mut count = 0;
    let mut current = start;

    loop {
        // Find the neighbor of `current` in direction `dir`
        let next = topo
            .neighbors(current)
            .find(|&(_, d)| d == dir)
            .map(|(cell, _)| cell);

        match next {
            Some(neighbor) => {
                let poss = state.possibilities(neighbor);
                // Check if the neighbor is collapsed to `target_state`
                if poss.is_singleton() && poss.test(target_state) {
                    count += 1;
                    current = neighbor;
                } else {
                    break;
                }
            }
            None => break,
        }
    }

    count
}

impl<T: Topology, const W: usize> GlobalConstraint<T, W> for MaxRunConstraint<W> {
    fn enforce(&self, state: &mut SolverState<'_, T, W>) -> Result<(), Contradiction> {
        let num_cells = state.num_cells();
        let topo = state.topology();

        // Collect the direction pairs: only process D where dir_index(D) < dir_index(opposite(D))
        let directions: Vec<T::Direction> = topo.directions().to_vec();
        let mut forward_dirs: Vec<(T::Direction, T::Direction)> = Vec::new();

        for &d in &directions {
            let opp = topo.opposite(d);
            if topo.direction_index(d) < topo.direction_index(opp) {
                forward_dirs.push((d, opp));
            }
        }

        for cell in 0..num_cells {
            let poss = *state.possibilities(cell);

            // Skip already-collapsed cells
            if poss.is_singleton() {
                continue;
            }

            // For each possible state in this cell
            for s in poss.iter_ones() {
                // Skip if no limit for this state
                if s >= self.limits.len() || self.limits[s] == usize::MAX {
                    continue;
                }

                let max_run = self.limits[s];

                // Check each axis direction pair
                let mut should_remove = false;
                for &(fwd, bwd) in &forward_dirs {
                    let run_fwd = walk_run(state, s, cell, fwd);
                    let run_bwd = walk_run(state, s, cell, bwd);

                    if run_fwd + run_bwd + 1 > max_run {
                        should_remove = true;
                        break;
                    }
                }

                if should_remove {
                    state.possibilities_mut(cell).clear(s);
                    let new_count = state.possibilities(cell).count_ones();
                    state.set_entropy(cell, new_count);
                    if new_count == 0 {
                        return Err(Contradiction { cell });
                    }
                }
            }
        }

        Ok(())
    }
}

/// Builder for [`MaxRunConstraint`].
///
/// Accumulates per-state run limits and produces a finished constraint via
/// [`build`](MaxRunConstraintBuilder::build).
pub struct MaxRunConstraintBuilder<const W: usize = 2> {
    per_state: Vec<RunLimit>,
    default_max_run: Option<usize>,
}

impl<const W: usize> MaxRunConstraintBuilder<W> {
    /// Sets the maximum consecutive run length for `state` along any axis.
    pub fn max_run(mut self, state: usize, length: usize) -> Self {
        self.per_state.push(RunLimit {
            state,
            max_run: length,
        });
        self
    }

    /// Sets a default maximum run length that applies to all states not
    /// explicitly configured via [`max_run`](Self::max_run).
    pub fn default_max_run(mut self, length: usize) -> Self {
        self.default_max_run = Some(length);
        self
    }

    /// Consumes the builder and produces a [`MaxRunConstraint`].
    ///
    /// `num_states` is the total number of tile states in the system.
    pub fn build(self, num_states: usize) -> MaxRunConstraint<W> {
        let default = self.default_max_run.unwrap_or(usize::MAX);
        let mut limits = alloc::vec![default; num_states];

        for rl in &self.per_state {
            if rl.state < num_states {
                limits[rl.state] = rl.max_run;
            }
        }

        MaxRunConstraint { limits }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitset::BitSet;
    use crate::constraint::SolverState;

    // Minimal 1D chain topology for unit testing walk_run
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    enum Dir1D {
        Left,
        Right,
    }

    const DIRS_1D: [Dir1D; 2] = [Dir1D::Left, Dir1D::Right];

    struct Chain1D {
        len: usize,
    }

    impl Topology for Chain1D {
        type Direction = Dir1D;

        fn num_cells(&self) -> usize {
            self.len
        }

        fn directions(&self) -> &[Dir1D] {
            &DIRS_1D
        }

        fn neighbors(&self, cell: usize) -> impl Iterator<Item = (usize, Dir1D)> {
            let mut result = alloc::vec::Vec::new();
            if cell > 0 {
                result.push((cell - 1, Dir1D::Left));
            }
            if cell + 1 < self.len {
                result.push((cell + 1, Dir1D::Right));
            }
            result.into_iter()
        }

        fn opposite(&self, dir: Dir1D) -> Dir1D {
            match dir {
                Dir1D::Left => Dir1D::Right,
                Dir1D::Right => Dir1D::Left,
            }
        }

        fn direction_index(&self, dir: Dir1D) -> usize {
            match dir {
                Dir1D::Left => 0,
                Dir1D::Right => 1,
            }
        }
    }

    #[test]
    fn walk_run_counts_correctly() {
        // Build a pre-collapsed 1D grid: [0, 0, 0, 1, 0]
        let chain = Chain1D { len: 5 };
        let num_states = 2;

        let mut possibilities = alloc::vec![BitSet::<2>::new(); 5];
        let mut entropy_cache = alloc::vec![1u32; 5];

        possibilities[0].set(0);
        possibilities[1].set(0);
        possibilities[2].set(0);
        possibilities[3].set(1);
        possibilities[4].set(0);

        let state = SolverState::new(
            &mut possibilities,
            &mut entropy_cache,
            num_states,
            &chain,
        );

        // From cell 0, walking Right: cells 1,2 are state 0 → count 2
        assert_eq!(walk_run(&state, 0, 0, Dir1D::Right), 2);
        // From cell 0, walking Left: no neighbor → 0
        assert_eq!(walk_run(&state, 0, 0, Dir1D::Left), 0);
        // From cell 2, walking Left: cells 1,0 are state 0 → count 2
        assert_eq!(walk_run(&state, 0, 2, Dir1D::Left), 2);
        // From cell 2, walking Right: cell 3 is state 1 → 0
        assert_eq!(walk_run(&state, 0, 2, Dir1D::Right), 0);
        // From cell 3, walking Right: cell 4 is state 0, not 1 → 0
        assert_eq!(walk_run(&state, 1, 3, Dir1D::Right), 0);
        // From cell 3, walking Left: cell 2 is state 0, not 1 → 0
        assert_eq!(walk_run(&state, 1, 3, Dir1D::Left), 0);
        // From cell 4, walking Left: cell 3 is state 1, not 0 → 0
        assert_eq!(walk_run(&state, 0, 4, Dir1D::Left), 0);
        // From cell 4, walking Right: no neighbor → 0
        assert_eq!(walk_run(&state, 0, 4, Dir1D::Right), 0);
    }
}
