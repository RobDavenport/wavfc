use wavfc::AdjacencyRules;

use crate::topology_2d::Dir2D;

// ---------------------------------------------------------------------------
// Terrain tileset (7 states)
// ---------------------------------------------------------------------------
//
// 0: Deep Water  (#1a5276)
// 1: Water       (#2e86c1)
// 2: Sand        (#f9e79f)
// 3: Grass       (#27ae60)
// 4: Forest      (#1e8449)
// 5: Hills       (#af7ac5)
// 6: Mountain    (#7f8c8d)
//
// Adjacency: each state can be adjacent to itself and its immediate neighbors
// in the gradient. No skipping levels.

pub struct TerrainRules;

impl AdjacencyRules for TerrainRules {
    type Direction = Dir2D;

    fn num_states(&self) -> usize {
        7
    }

    fn compatible(&self, a: usize, b: usize, _dir: Dir2D) -> bool {
        let diff = if a > b { a - b } else { b - a };
        diff <= 1
    }

    fn weight(&self, state: usize) -> f64 {
        match state {
            1 => 2.0, // Water
            3 => 3.0, // Grass
            4 => 2.0, // Forest
            _ => 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Dungeon tileset (5 states)
// ---------------------------------------------------------------------------
//
// 0: Wall        (#2c3e50)
// 1: Floor       (#d4a574)
// 2: Corridor    (#c4a064)
// 3: Door        (#8b4513)
// 4: Pillar      (#95a5a6)
//
// Adjacency:
//   Wall-Wall, Wall-Floor, Wall-Door
//   Floor-Floor, Floor-Corridor, Floor-Door, Floor-Pillar
//   Corridor-Corridor, Corridor-Door
//   Pillar-Pillar

pub struct DungeonRules;

impl AdjacencyRules for DungeonRules {
    type Direction = Dir2D;

    fn num_states(&self) -> usize {
        5
    }

    fn compatible(&self, a: usize, b: usize, _dir: Dir2D) -> bool {
        matches!(
            (a.min(b), a.max(b)),
            (0, 0)
                | (0, 1)
                | (0, 3)
                | (1, 1)
                | (1, 2)
                | (1, 3)
                | (1, 4)
                | (2, 2)
                | (2, 3)
                | (4, 4)
        )
    }

    fn weight(&self, state: usize) -> f64 {
        match state {
            0 => 3.0, // Wall (most common)
            1 => 2.0, // Floor
            2 => 1.5, // Corridor
            3 => 0.3, // Door (rare)
            4 => 0.2, // Pillar (very rare)
            _ => 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Color and name helpers
// ---------------------------------------------------------------------------

pub fn terrain_color(state: usize) -> &'static str {
    match state {
        0 => "#1a5276", // Deep Water
        1 => "#2e86c1", // Water
        2 => "#f9e79f", // Sand
        3 => "#27ae60", // Grass
        4 => "#1e8449", // Forest
        5 => "#af7ac5", // Hills
        6 => "#7f8c8d", // Mountain
        _ => "#000000",
    }
}

pub fn dungeon_color(state: usize) -> &'static str {
    match state {
        0 => "#2c3e50", // Wall
        1 => "#d4a574", // Floor
        2 => "#c4a064", // Corridor
        3 => "#8b4513", // Door
        4 => "#95a5a6", // Pillar
        _ => "#000000",
    }
}

pub fn terrain_name(state: usize) -> &'static str {
    match state {
        0 => "Deep Water",
        1 => "Water",
        2 => "Sand",
        3 => "Grass",
        4 => "Forest",
        5 => "Hills",
        6 => "Mountain",
        _ => "Unknown",
    }
}

pub fn dungeon_name(state: usize) -> &'static str {
    match state {
        0 => "Wall",
        1 => "Floor",
        2 => "Corridor",
        3 => "Door",
        4 => "Pillar",
        _ => "Unknown",
    }
}
