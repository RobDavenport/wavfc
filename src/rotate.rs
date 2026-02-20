//! 2D rotation/reflection expansion for tile variants.
//!
//! Given a base tile with a [`Symmetry`] class and sockets on each cardinal edge,
//! [`RotationExpander`] automatically generates all unique rotated and reflected
//! variants, deduplicating by socket arrays. The expanded tileset is returned as
//! a [`SocketRules<Dir2D>`] together with a [`TileMapping`] that maps each
//! expanded state back to its base tile and [`Rotation`].
//!
//! # Symmetry classes
//!
//! | Class | Unique variants | Description                          |
//! |-------|-----------------|--------------------------------------|
//! | `X`   | 1               | 4-fold rotational + reflective       |
//! | `I`   | 2               | 2-fold rotational (0, 90)            |
//! | `T`   | 4               | No rotational symmetry (4 rotations) |
//! | `L`   | 4               | No rotational symmetry (4 rotations) |
//! | `D`   | 2               | Diagonal symmetry (0, 90)            |
//! | `F`   | 8               | No symmetry (4 rotations x 2 flips)  |

use alloc::vec;
use alloc::vec::Vec;

use crate::socket::{Socket, SocketRuleBuilder, SocketRules};
use crate::topology::Topology;

// ---------------------------------------------------------------------------
// Dir2D
// ---------------------------------------------------------------------------

/// Cardinal directions for a 2D grid.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Dir2D {
    North,
    East,
    South,
    West,
}

impl Dir2D {
    /// Returns the opposite direction.
    #[inline]
    pub fn opposite(self) -> Dir2D {
        match self {
            Dir2D::North => Dir2D::South,
            Dir2D::South => Dir2D::North,
            Dir2D::East => Dir2D::West,
            Dir2D::West => Dir2D::East,
        }
    }
}

/// All four cardinal directions in order: N, E, S, W.
const DIRS_2D: [Dir2D; 4] = [Dir2D::North, Dir2D::East, Dir2D::South, Dir2D::West];

// ---------------------------------------------------------------------------
// Dir2DTopo -- minimal Topology impl for SocketRuleBuilder::build()
// ---------------------------------------------------------------------------

/// Minimal topology that only provides direction metadata for [`Dir2D`].
///
/// This is used internally by [`RotationExpander::build`] to satisfy the
/// [`Topology`] requirement of [`SocketRuleBuilder::build`]. It has zero cells
/// and is not meant for solving.
struct Dir2DTopo;

impl Topology for Dir2DTopo {
    type Direction = Dir2D;

    fn num_cells(&self) -> usize {
        0
    }

    fn directions(&self) -> &[Dir2D] {
        &DIRS_2D
    }

    fn neighbors(&self, _cell: usize) -> impl Iterator<Item = (usize, Dir2D)> {
        core::iter::empty()
    }

    fn opposite(&self, dir: Dir2D) -> Dir2D {
        dir.opposite()
    }

    fn direction_index(&self, dir: Dir2D) -> usize {
        match dir {
            Dir2D::North => 0,
            Dir2D::East => 1,
            Dir2D::South => 2,
            Dir2D::West => 3,
        }
    }
}

// ---------------------------------------------------------------------------
// Rotation
// ---------------------------------------------------------------------------

/// A 2D rotation/reflection transform.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Rotation {
    /// 0 degrees.
    R0,
    /// 90 degrees clockwise.
    R90,
    /// 180 degrees.
    R180,
    /// 270 degrees clockwise.
    R270,
    /// 0 degrees + horizontal flip.
    R0Flip,
    /// 90 degrees clockwise + horizontal flip.
    R90Flip,
    /// 180 degrees + horizontal flip.
    R180Flip,
    /// 270 degrees clockwise + horizontal flip.
    R270Flip,
}

/// All eight possible transforms.
const ALL_TRANSFORMS: [Rotation; 8] = [
    Rotation::R0,
    Rotation::R90,
    Rotation::R180,
    Rotation::R270,
    Rotation::R0Flip,
    Rotation::R90Flip,
    Rotation::R180Flip,
    Rotation::R270Flip,
];

// ---------------------------------------------------------------------------
// Symmetry
// ---------------------------------------------------------------------------

/// Symmetry class of a 2D tile.
///
/// Determines how many unique variants are generated when applying rotations
/// and reflections.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Symmetry {
    /// 4-fold rotational + reflective symmetry. 1 unique variant.
    X,
    /// 2-fold rotational symmetry (e.g., a straight pipe). 2 variants.
    I,
    /// T-shaped: 4 rotations, no reflective symmetry collapse. 4 variants.
    T,
    /// L-shaped: 4 rotations, no reflective symmetry collapse. 4 variants.
    L,
    /// Diagonal symmetry. 2 variants.
    D,
    /// Fully asymmetric. 8 variants (4 rotations x 2 reflections).
    F,
}

/// Only the four rotations (no reflections).
const ROTATIONS_ONLY: [Rotation; 4] = [
    Rotation::R0,
    Rotation::R90,
    Rotation::R180,
    Rotation::R270,
];

/// Only identity and 90-degree rotation.
const TWO_ROTATIONS: [Rotation; 2] = [Rotation::R0, Rotation::R90];

/// Only identity.
const IDENTITY_ONLY: [Rotation; 1] = [Rotation::R0];

impl Symmetry {
    /// Returns the candidate transforms for this symmetry class.
    ///
    /// The symmetry class determines the maximum number of unique variants.
    /// Deduplication may further reduce the count if the sockets happen to
    /// coincide, but the candidate list is already limited to avoid generating
    /// spurious variants.
    fn candidate_transforms(self) -> &'static [Rotation] {
        match self {
            // X: 4-fold rotational + reflective -> only 1 unique variant.
            Symmetry::X => &IDENTITY_ONLY,
            // I: 2-fold rotational -> 2 unique variants (0, 90).
            Symmetry::I => &TWO_ROTATIONS,
            // T: 4 rotations, reflections collapse to existing rotations -> 4.
            Symmetry::T => &ROTATIONS_ONLY,
            // L: 4 rotations, reflections collapse to existing rotations -> 4.
            Symmetry::L => &ROTATIONS_ONLY,
            // D: diagonal symmetry -> 2 unique variants (0, 90).
            Symmetry::D => &TWO_ROTATIONS,
            // F: fully asymmetric -> all 8 unique.
            Symmetry::F => &ALL_TRANSFORMS,
        }
    }
}

// ---------------------------------------------------------------------------
// Socket-array transforms
// ---------------------------------------------------------------------------

/// Rotate sockets clockwise: `[N, E, S, W] -> [W, N, E, S]`.
///
/// After a 90-degree clockwise rotation of the tile, the socket that was on the
/// West edge now faces North, North->East, East->South, South->West.
fn rotate_cw(sockets: [Socket; 4]) -> [Socket; 4] {
    // Input:  [N, E, S, W]
    // Output: [W, N, E, S]
    [sockets[3], sockets[0], sockets[1], sockets[2]]
}

/// Reflect horizontally: swap East and West sockets.
///
/// `[N, E, S, W] -> [N, W, S, E]`
fn reflect_horizontal(sockets: [Socket; 4]) -> [Socket; 4] {
    [sockets[0], sockets[3], sockets[2], sockets[1]]
}

/// Apply a [`Rotation`] transform to a socket array `[N, E, S, W]`.
fn apply_transform(sockets: [Socket; 4], rotation: Rotation) -> [Socket; 4] {
    match rotation {
        Rotation::R0 => sockets,
        Rotation::R90 => rotate_cw(sockets),
        Rotation::R180 => rotate_cw(rotate_cw(sockets)),
        Rotation::R270 => rotate_cw(rotate_cw(rotate_cw(sockets))),
        Rotation::R0Flip => reflect_horizontal(sockets),
        Rotation::R90Flip => rotate_cw(reflect_horizontal(sockets)),
        Rotation::R180Flip => rotate_cw(rotate_cw(reflect_horizontal(sockets))),
        Rotation::R270Flip => rotate_cw(rotate_cw(rotate_cw(reflect_horizontal(sockets)))),
    }
}

// ---------------------------------------------------------------------------
// TileMapping
// ---------------------------------------------------------------------------

/// Maps expanded tile indices to their base tile and rotation.
///
/// After [`RotationExpander::build`] expands tiles into rotated/reflected
/// variants, each expanded state index corresponds to a `(base_tile_id, Rotation)`
/// pair.
#[derive(Clone, Debug)]
pub struct TileMapping {
    /// `entries[expanded_index] = (base_tile_id, rotation)`
    entries: Vec<(usize, Rotation)>,
}

impl TileMapping {
    /// Returns `(base_tile_id, rotation)` for an expanded state index.
    #[inline]
    pub fn get(&self, expanded_state: usize) -> (usize, Rotation) {
        self.entries[expanded_state]
    }

    /// Total number of expanded states.
    #[inline]
    pub fn num_expanded(&self) -> usize {
        self.entries.len()
    }

    /// Returns the base tile id for an expanded state.
    #[inline]
    pub fn base_tile(&self, expanded_state: usize) -> usize {
        self.entries[expanded_state].0
    }

    /// Returns the rotation for an expanded state.
    #[inline]
    pub fn rotation(&self, expanded_state: usize) -> Rotation {
        self.entries[expanded_state].1
    }
}

// ---------------------------------------------------------------------------
// RotationExpander
// ---------------------------------------------------------------------------

/// Accumulates base tile definitions and produces an expanded tileset with all
/// rotated/reflected variants.
///
/// # Example
///
/// ```
/// use wavfc::rotate::*;
/// use wavfc::Socket;
///
/// let mut expander = RotationExpander::new();
///
/// // An X-symmetric tile (same socket on all sides): 1 variant.
/// let s = Socket(1);
/// expander.add_tile(Symmetry::X, [s, s, s, s], 1.0);
///
/// let (rules, mapping) = expander.build();
/// assert_eq!(mapping.num_expanded(), 1);
/// ```
pub struct RotationExpander {
    tiles: Vec<BaseTile>,
}

struct BaseTile {
    symmetry: Symmetry,
    /// Sockets ordered `[N, E, S, W]`.
    sockets: [Socket; 4],
    weight: f64,
}

impl RotationExpander {
    /// Creates a new, empty expander.
    pub fn new() -> Self {
        Self { tiles: Vec::new() }
    }

    /// Adds a base tile with the given symmetry, sockets, and weight.
    ///
    /// Sockets are ordered `[North, East, South, West]`.
    ///
    /// Returns the base tile index (zero-based, in insertion order).
    pub fn add_tile(&mut self, symmetry: Symmetry, sockets: [Socket; 4], weight: f64) -> usize {
        let idx = self.tiles.len();
        self.tiles.push(BaseTile {
            symmetry,
            sockets,
            weight,
        });
        idx
    }

    /// Expands all base tiles into their rotated/reflected variants and builds
    /// [`SocketRules<Dir2D>`] together with a [`TileMapping`].
    ///
    /// Deduplication is performed per base tile: two transforms that produce
    /// identical socket arrays are collapsed into a single expanded state (the
    /// first transform wins).
    pub fn build(self) -> (SocketRules<Dir2D>, TileMapping) {
        let mut builder: SocketRuleBuilder<Dir2D> = SocketRuleBuilder::new();
        let mut entries: Vec<(usize, Rotation)> = Vec::new();

        for (base_id, tile) in self.tiles.iter().enumerate() {
            let candidates = tile.symmetry.candidate_transforms();
            let mut seen: Vec<[Socket; 4]> = Vec::new();

            for &rot in candidates {
                let transformed = apply_transform(tile.sockets, rot);

                // Dedup: skip if we already have an identical socket array for
                // this base tile.
                let duplicate = seen.iter().any(|s| *s == transformed);
                if duplicate {
                    continue;
                }
                seen.push(transformed);

                // Register expanded tile in the builder.
                let sock_vec = vec![
                    (Dir2D::North, transformed[0]),
                    (Dir2D::East, transformed[1]),
                    (Dir2D::South, transformed[2]),
                    (Dir2D::West, transformed[3]),
                ];
                builder.add_tile(sock_vec, tile.weight);
                entries.push((base_id, rot));
            }
        }

        let topo = Dir2DTopo;
        let rules = builder.build(&topo);
        let mapping = TileMapping { entries };

        (rules, mapping)
    }
}

impl Default for RotationExpander {
    fn default() -> Self {
        Self::new()
    }
}
