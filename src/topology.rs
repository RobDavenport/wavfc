//! Topology trait for defining world structure.
//!
//! The topology defines how cells are connected in the world graph.
//! Cells are identified by `usize` indices and connected via typed directions.

/// Defines the structure of the world as an abstract graph.
///
/// Cells are identified by `usize` indices in `0..num_cells()`.
/// Each cell has neighbors reachable via named directions.
///
/// # Examples
///
/// A 2D grid topology would have `Direction` as an enum `{North, South, East, West}`,
/// `num_cells()` returning `width * height`, and `neighbors()` yielding the 2-4
/// adjacent cells for each position.
pub trait Topology {
    /// The direction type for edges in this topology.
    type Direction: Copy + Eq + core::fmt::Debug;

    /// Total number of cells in the world.
    fn num_cells(&self) -> usize;

    /// All valid direction values.
    fn directions(&self) -> &[Self::Direction];

    /// Returns an iterator of `(neighbor_cell_index, direction)` for a given cell.
    fn neighbors(&self, cell: usize) -> impl Iterator<Item = (usize, Self::Direction)>;

    /// The reverse direction. If A is North of B, then B is South of A.
    fn opposite(&self, dir: Self::Direction) -> Self::Direction;

    /// Map a direction to a dense index in `0..directions().len()`.
    fn direction_index(&self, dir: Self::Direction) -> usize;
}
