use wavfc::AdjacencyRules;

use crate::topology_2d::Dir2D;
use crate::topology_3d::Dir3D;

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

// ===========================================================================
// 3D Voxel tilesets
// ===========================================================================

// ---------------------------------------------------------------------------
// Terrain Voxel tileset (6 states)
// ---------------------------------------------------------------------------
//
// 0: Air      (#87CEEB)
// 1: Grass    (#4CAF50)
// 2: Dirt     (#8B6914)
// 3: Stone    (#808080)
// 4: Ore      (#FFD700)
// 5: Bedrock  (#1A1A1A)

pub struct TerrainVoxelRules;

impl TerrainVoxelRules {
    fn is_horizontal(dir: Dir3D) -> bool {
        matches!(dir, Dir3D::North | Dir3D::South | Dir3D::East | Dir3D::West)
    }
}

impl AdjacencyRules for TerrainVoxelRules {
    type Direction = Dir3D;

    fn num_states(&self) -> usize {
        6
    }

    fn compatible(&self, a: usize, b: usize, dir: Dir3D) -> bool {
        if Self::is_horizontal(dir) {
            // Horizontal: symmetric, direction-independent
            let (lo, hi) = (a.min(b), a.max(b));
            matches!(
                (lo, hi),
                // Self-adjacency
                (0, 0) | (1, 1) | (2, 2) | (3, 3) | (4, 4) | (5, 5)
                // Air <-> Grass
                | (0, 1)
                // Grass <-> Dirt
                | (1, 2)
                // Dirt <-> Stone
                | (2, 3)
                // Stone <-> Ore
                | (3, 4)
                // Stone <-> Bedrock
                | (3, 5)
                // Air <-> Dirt (cliffs)
                | (0, 2)
                // Air <-> Stone (cave openings, cliffs)
                | (0, 3)
            )
        } else {
            // Vertical: (lower, upper) pairs
            // dir == Up means b is above a; dir == Down means b is below a
            let (lower, upper) = if dir == Dir3D::Up {
                (a, b)
            } else {
                (b, a)
            };
            matches!(
                (lower, upper),
                (5, 5) | (5, 3)    // Bedrock base
                | (3, 3) | (3, 2) | (3, 4) | (3, 0) // Stone layers
                | (4, 3) | (4, 4)  // Ore within stone
                | (2, 2) | (2, 1) | (2, 0) // Dirt layers
                | (1, 0)           // Grass surface
                | (0, 0)           // Air stacks
            )
        }
    }

    fn weight(&self, state: usize) -> f64 {
        match state {
            0 => 2.0,  // Air
            1 => 1.0,  // Grass
            2 => 1.5,  // Dirt
            3 => 3.0,  // Stone
            4 => 0.3,  // Ore
            5 => 1.0,  // Bedrock
            _ => 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Forest Voxel tileset (8 states) -- extends Terrain with trees
// ---------------------------------------------------------------------------
//
// 0-5: same as Terrain
// 6: Trunk   (#6D4C41)
// 7: Leaves  (#2E7D32)

pub struct ForestVoxelRules;

impl ForestVoxelRules {
    fn is_horizontal(dir: Dir3D) -> bool {
        matches!(dir, Dir3D::North | Dir3D::South | Dir3D::East | Dir3D::West)
    }
}

impl AdjacencyRules for ForestVoxelRules {
    type Direction = Dir3D;

    fn num_states(&self) -> usize {
        8
    }

    fn compatible(&self, a: usize, b: usize, dir: Dir3D) -> bool {
        if Self::is_horizontal(dir) {
            let (lo, hi) = (a.min(b), a.max(b));
            matches!(
                (lo, hi),
                // Terrain base horizontal rules
                (0, 0) | (1, 1) | (2, 2) | (3, 3) | (4, 4) | (5, 5)
                | (0, 1) | (1, 2) | (2, 3) | (3, 4) | (3, 5)
                | (0, 2) | (0, 3)
                // Forest additions
                | (0, 6)  // Air <-> Trunk
                | (0, 7)  // Air <-> Leaves
                | (7, 7)  // Leaves <-> Leaves
                | (6, 7)  // Trunk <-> Leaves
                | (1, 6)  // Grass <-> Trunk
                | (6, 6)  // Trunk self
            )
        } else {
            let (lower, upper) = if dir == Dir3D::Up {
                (a, b)
            } else {
                (b, a)
            };
            matches!(
                (lower, upper),
                // Terrain base vertical rules
                (5, 5) | (5, 3)
                | (3, 3) | (3, 2) | (3, 4) | (3, 0)
                | (4, 3) | (4, 4)
                | (2, 2) | (2, 1) | (2, 0)
                | (1, 0)
                | (0, 0)
                // Forest additions
                | (1, 6)  // Grass -> Trunk (trees grow from grass)
                | (6, 6)  // Trunk -> Trunk (trunk stacks)
                | (6, 7)  // Trunk -> Leaves (canopy on trunk)
                | (7, 7)  // Leaves -> Leaves (leaves stack)
                | (7, 0)  // Leaves -> Air (top of tree)
            )
        }
    }

    fn weight(&self, state: usize) -> f64 {
        match state {
            0 => 2.0,   // Air
            1 => 1.0,   // Grass
            2 => 1.5,   // Dirt
            3 => 3.0,   // Stone
            4 => 0.3,   // Ore
            5 => 1.0,   // Bedrock
            6 => 0.15,  // Trunk
            7 => 0.4,   // Leaves
            _ => 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Village Voxel tileset (11 states) -- extends Forest with buildings
// ---------------------------------------------------------------------------
//
// 0-7: same as Forest
// 8:  Wall   (#795548)
// 9:  Floor  (#D4A574)
// 10: Roof   (#B71C1C)

pub struct VillageVoxelRules;

impl VillageVoxelRules {
    fn is_horizontal(dir: Dir3D) -> bool {
        matches!(dir, Dir3D::North | Dir3D::South | Dir3D::East | Dir3D::West)
    }
}

impl AdjacencyRules for VillageVoxelRules {
    type Direction = Dir3D;

    fn num_states(&self) -> usize {
        11
    }

    fn compatible(&self, a: usize, b: usize, dir: Dir3D) -> bool {
        if Self::is_horizontal(dir) {
            let (lo, hi) = (a.min(b), a.max(b));
            matches!(
                (lo, hi),
                // Terrain base
                (0, 0) | (1, 1) | (2, 2) | (3, 3) | (4, 4) | (5, 5)
                | (0, 1) | (1, 2) | (2, 3) | (3, 4) | (3, 5)
                | (0, 2) | (0, 3)
                // Forest additions
                | (0, 6) | (0, 7) | (7, 7) | (6, 7) | (1, 6) | (6, 6)
                // Village additions
                | (8, 8)   // Wall <-> Wall
                | (8, 9)   // Wall <-> Floor
                | (0, 8)   // Wall <-> Air (exterior)
                | (1, 8)   // Wall <-> Grass (base)
                | (9, 9)   // Floor <-> Floor
                | (10, 10) // Roof <-> Roof
                | (0, 10)  // Roof <-> Air (edge)
            )
        } else {
            let (lower, upper) = if dir == Dir3D::Up {
                (a, b)
            } else {
                (b, a)
            };
            matches!(
                (lower, upper),
                // Terrain base vertical
                (5, 5) | (5, 3)
                | (3, 3) | (3, 2) | (3, 4) | (3, 0)
                | (4, 3) | (4, 4)
                | (2, 2) | (2, 1) | (2, 0)
                | (1, 0)
                | (0, 0)
                // Forest vertical
                | (1, 6) | (6, 6) | (6, 7) | (7, 7) | (7, 0)
                // Village vertical
                | (1, 8)   // Grass -> Wall (building on grass)
                | (1, 9)   // Grass -> Floor (ground floor)
                | (9, 8)   // Floor -> Wall (walls on floor)
                | (9, 0)   // Floor -> Air (doorways)
                | (8, 8)   // Wall -> Wall (stacking)
                | (8, 10)  // Wall -> Roof
                | (8, 0)   // Wall -> Air (window)
                | (10, 0)  // Roof -> Air (roof top)
                | (9, 9)   // Floor -> Floor (multi-story)
            )
        }
    }

    fn weight(&self, state: usize) -> f64 {
        match state {
            0 => 2.0,   // Air
            1 => 1.0,   // Grass
            2 => 1.5,   // Dirt
            3 => 3.0,   // Stone
            4 => 0.3,   // Ore
            5 => 1.0,   // Bedrock
            6 => 0.15,  // Trunk
            7 => 0.4,   // Leaves
            8 => 0.1,   // Wall
            9 => 0.08,  // Floor
            10 => 0.06, // Roof
            _ => 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// 3D Color and name helpers
// ---------------------------------------------------------------------------

pub fn voxel_terrain_color(state: usize) -> &'static str {
    match state {
        0 => "#87CEEB", // Air
        1 => "#4CAF50", // Grass
        2 => "#8B6914", // Dirt
        3 => "#808080", // Stone
        4 => "#FFD700", // Ore
        5 => "#1A1A1A", // Bedrock
        _ => "#000000",
    }
}

pub fn voxel_forest_color(state: usize) -> &'static str {
    match state {
        0 => "#87CEEB", // Air
        1 => "#4CAF50", // Grass
        2 => "#8B6914", // Dirt
        3 => "#808080", // Stone
        4 => "#FFD700", // Ore
        5 => "#1A1A1A", // Bedrock
        6 => "#6D4C41", // Trunk
        7 => "#2E7D32", // Leaves
        _ => "#000000",
    }
}

pub fn voxel_village_color(state: usize) -> &'static str {
    match state {
        0 => "#87CEEB", // Air
        1 => "#4CAF50", // Grass
        2 => "#8B6914", // Dirt
        3 => "#808080", // Stone
        4 => "#FFD700", // Ore
        5 => "#1A1A1A", // Bedrock
        6 => "#6D4C41", // Trunk
        7 => "#2E7D32", // Leaves
        8 => "#795548", // Wall
        9 => "#D4A574", // Floor
        10 => "#B71C1C", // Roof
        _ => "#000000",
    }
}

pub fn voxel_terrain_name(state: usize) -> &'static str {
    match state {
        0 => "Air",
        1 => "Grass",
        2 => "Dirt",
        3 => "Stone",
        4 => "Ore",
        5 => "Bedrock",
        _ => "Unknown",
    }
}

pub fn voxel_forest_name(state: usize) -> &'static str {
    match state {
        0 => "Air",
        1 => "Grass",
        2 => "Dirt",
        3 => "Stone",
        4 => "Ore",
        5 => "Bedrock",
        6 => "Trunk",
        7 => "Leaves",
        _ => "Unknown",
    }
}

pub fn voxel_village_name(state: usize) -> &'static str {
    match state {
        0 => "Air",
        1 => "Grass",
        2 => "Dirt",
        3 => "Stone",
        4 => "Ore",
        5 => "Bedrock",
        6 => "Trunk",
        7 => "Leaves",
        8 => "Wall",
        9 => "Floor",
        10 => "Roof",
        _ => "Unknown",
    }
}
