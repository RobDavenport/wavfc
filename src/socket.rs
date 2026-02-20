//! Socket-based tile adjacency rules.
//!
//! Sockets are labels placed on tile edges. Two tiles can be adjacent when their
//! facing sockets match. This module provides a dimension-agnostic builder
//! ([`SocketRuleBuilder`]) and a final rule set ([`SocketRules`]) that implements
//! [`AdjacencyRules`].
//!
//! # Socket matching
//!
//! - `Socket(x)` matches `Socket(x)` (symmetric).
//! - `Socket(x).flipped()` matches `Socket(x)` (asymmetric pair).
//! - `Socket(x).flipped()` does **not** match `Socket(x).flipped()`.

use alloc::vec::Vec;

use crate::rules::AdjacencyRules;
use crate::topology::Topology;

/// A socket label for a tile edge.
///
/// The high bit (`0x8000`) encodes flip state for asymmetric sockets.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Socket(pub u16);

impl Socket {
    /// Returns the flipped variant of this socket (sets the high bit).
    ///
    /// Used for asymmetric connections: `Socket(x).flipped()` matches `Socket(x)`.
    #[inline]
    pub fn flipped(self) -> Socket {
        Socket(self.0 | 0x8000)
    }

    /// Returns the base socket with the high bit cleared.
    #[inline]
    pub fn base(self) -> Socket {
        Socket(self.0 & 0x7FFF)
    }

    /// Returns `true` if the high bit is set (this is a flipped socket).
    #[inline]
    pub fn is_flipped(self) -> bool {
        self.0 & 0x8000 != 0
    }
}

/// Returns `true` if two sockets are compatible for adjacency.
///
/// Matching rules:
/// - `Socket(x)` matches `Socket(x)` (symmetric).
/// - `Socket(x).flipped()` matches `Socket(x)` (asymmetric pair).
/// - `Socket(x).flipped()` does NOT match `Socket(x).flipped()`.
pub fn sockets_match(a: Socket, b: Socket) -> bool {
    // Both unflipped with same base: symmetric match
    if !a.is_flipped() && !b.is_flipped() {
        return a.0 == b.0;
    }
    // Exactly one is flipped and they share the same base: asymmetric match
    if a.is_flipped() != b.is_flipped() {
        return a.base().0 == b.base().0;
    }
    // Both flipped: no match
    false
}

/// A tile definition accumulated by [`SocketRuleBuilder`].
struct TileDef<D> {
    /// Socket assigned per direction: `(direction, socket)`.
    sockets: Vec<(D, Socket)>,
    /// Weight/frequency of this tile.
    weight: f64,
}

/// Builder for socket-based adjacency rules.
///
/// Accumulate tile definitions with [`add_tile`](Self::add_tile), then call
/// [`build`](Self::build) to produce a [`SocketRules`] that implements
/// [`AdjacencyRules`].
///
/// `D` is the direction type (e.g., an enum with `North`, `South`, `East`, `West`).
pub struct SocketRuleBuilder<D> {
    tiles: Vec<TileDef<D>>,
}

impl<D: Copy + Eq> SocketRuleBuilder<D> {
    /// Creates an empty builder.
    pub fn new() -> Self {
        Self { tiles: Vec::new() }
    }

    /// Adds a tile with the given per-direction sockets and weight.
    ///
    /// Returns the tile index (zero-based, in insertion order).
    pub fn add_tile(&mut self, sockets: Vec<(D, Socket)>, weight: f64) -> usize {
        let idx = self.tiles.len();
        self.tiles.push(TileDef { sockets, weight });
        idx
    }

    /// Builds the final [`SocketRules`] using the given topology for
    /// direction-to-opposite mapping.
    ///
    /// The topology is queried at build time to precompute the opposite of each
    /// direction, so that `SocketRules::compatible()` can operate without
    /// topology access.
    pub fn build<T: Topology<Direction = D>>(self, topo: &T) -> SocketRules<D> {
        let directions = topo.directions().to_vec();
        let num_directions = directions.len();

        // Precompute opposite for each direction.
        let opposites: Vec<D> = directions.iter().map(|&d| topo.opposite(d)).collect();

        // Build per-tile socket lookup: tile_sockets[tile][dir_index] = Socket.
        // If a tile does not specify a socket for a direction, default to Socket(u16::MAX)
        // which will never match anything in practice.
        let num_tiles = self.tiles.len();
        let mut tile_sockets: Vec<Vec<Socket>> =
            Vec::with_capacity(num_tiles);
        let mut weights: Vec<f64> = Vec::with_capacity(num_tiles);

        for tile in &self.tiles {
            let mut sockets_for_dirs = alloc::vec![Socket(u16::MAX); num_directions];
            for &(dir, socket) in &tile.sockets {
                if let Some(idx) = directions.iter().position(|&d| d == dir) {
                    sockets_for_dirs[idx] = socket;
                }
            }
            tile_sockets.push(sockets_for_dirs);
            weights.push(tile.weight);
        }

        SocketRules {
            tile_sockets,
            weights,
            directions,
            opposites,
        }
    }
}

impl<D: Copy + Eq> Default for SocketRuleBuilder<D> {
    fn default() -> Self {
        Self::new()
    }
}

/// Socket-based adjacency rules.
///
/// Implements [`AdjacencyRules`] by looking up each tile's socket for the
/// queried direction and its opposite, then applying [`sockets_match`].
pub struct SocketRules<D> {
    /// `tile_sockets[tile][dir_index]` = socket for that tile in that direction.
    tile_sockets: Vec<Vec<Socket>>,
    /// Per-tile weight.
    weights: Vec<f64>,
    /// Ordered list of directions (matches topology's `directions()`).
    directions: Vec<D>,
    /// `opposites[dir_index]` = opposite direction of `directions[dir_index]`.
    opposites: Vec<D>,
}

impl<D: Copy + Eq + core::fmt::Debug> AdjacencyRules for SocketRules<D> {
    type Direction = D;

    fn num_states(&self) -> usize {
        self.tile_sockets.len()
    }

    fn compatible(&self, state_a: usize, state_b: usize, direction: Self::Direction) -> bool {
        // Find the direction index for `direction`.
        let dir_idx = match self.directions.iter().position(|&d| d == direction) {
            Some(i) => i,
            None => return false,
        };

        // Find the direction index for the opposite direction.
        let opp = self.opposites[dir_idx];
        let opp_idx = match self.directions.iter().position(|&d| d == opp) {
            Some(i) => i,
            None => return false,
        };

        let socket_a = self.tile_sockets[state_a][dir_idx];
        let socket_b = self.tile_sockets[state_b][opp_idx];

        sockets_match(socket_a, socket_b)
    }

    fn weight(&self, state: usize) -> f64 {
        self.weights[state]
    }
}
