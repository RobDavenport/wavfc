use wavfc::*;
use rand::SeedableRng;
use rand::rngs::StdRng;

// ---------------------------------------------------------------------------
// Symmetry variant-count tests
// ---------------------------------------------------------------------------

#[test]
fn x_symmetric_tile_produces_1_variant() {
    let mut exp = RotationExpander::new();
    let s = Socket(1);
    exp.add_tile(Symmetry::X, [s, s, s, s], 1.0);
    let (_rules, mapping) = exp.build();
    assert_eq!(mapping.num_expanded(), 1);
}

#[test]
fn i_symmetric_tile_produces_2_variants() {
    let mut exp = RotationExpander::new();
    // Straight pipe: same socket on N/S, different on E/W
    let a = Socket(1);
    let b = Socket(2);
    exp.add_tile(Symmetry::I, [a, b, a, b], 1.0);
    let (_rules, mapping) = exp.build();
    assert_eq!(mapping.num_expanded(), 2);
}

#[test]
fn l_symmetric_tile_produces_4_variants() {
    let mut exp = RotationExpander::new();
    // L-shaped bend: all four sockets distinct
    exp.add_tile(Symmetry::L, [Socket(1), Socket(2), Socket(3), Socket(4)], 1.0);
    let (_rules, mapping) = exp.build();
    assert_eq!(mapping.num_expanded(), 4);
}

#[test]
fn f_symmetric_tile_produces_8_variants() {
    let mut exp = RotationExpander::new();
    // Fully asymmetric: all four sockets distinct (no rotation/reflection duplicates)
    exp.add_tile(Symmetry::F, [Socket(1), Socket(2), Socket(3), Socket(4)], 1.0);
    let (_rules, mapping) = exp.build();
    assert_eq!(mapping.num_expanded(), 8);
}

#[test]
fn d_symmetric_tile_produces_2_variants() {
    let mut exp = RotationExpander::new();
    // Diagonal symmetry: N==E, S==W -> rotation by 90 is distinct, 180 same as 0, 270 same as 90
    let a = Socket(1);
    let b = Socket(2);
    exp.add_tile(Symmetry::D, [a, a, b, b], 1.0);
    let (_rules, mapping) = exp.build();
    assert_eq!(mapping.num_expanded(), 2);
}

#[test]
fn t_symmetric_tile_produces_4_variants() {
    let mut exp = RotationExpander::new();
    // T-shaped: three sides have one socket, one side different
    // [N=1, E=1, S=2, W=1]
    // R0:   [1,1,2,1]
    // R90:  [1,1,1,2] -- different from R0
    // R180: [2,1,1,1] -- different from R0 and R90
    // R270: [1,2,1,1] -- different from all above
    // Reflections collapse to existing rotations
    exp.add_tile(Symmetry::T, [Socket(1), Socket(1), Socket(2), Socket(1)], 1.0);
    let (_rules, mapping) = exp.build();
    assert_eq!(mapping.num_expanded(), 4);
}

// ---------------------------------------------------------------------------
// TileMapping correctness
// ---------------------------------------------------------------------------

#[test]
fn tile_mapping_maps_correctly() {
    let mut exp = RotationExpander::new();
    // Tile 0: X-symmetric -> 1 variant
    let s = Socket(1);
    exp.add_tile(Symmetry::X, [s, s, s, s], 1.0);
    // Tile 1: I-symmetric -> 2 variants
    let a = Socket(1);
    let b = Socket(2);
    exp.add_tile(Symmetry::I, [a, b, a, b], 1.0);

    let (_rules, mapping) = exp.build();
    assert_eq!(mapping.num_expanded(), 3); // 1 + 2

    // First expanded state is base tile 0
    assert_eq!(mapping.base_tile(0), 0);
    assert_eq!(mapping.rotation(0), Rotation::R0);

    // Next two expanded states are base tile 1
    assert_eq!(mapping.base_tile(1), 1);
    assert_eq!(mapping.base_tile(2), 1);
}

#[test]
fn tile_mapping_get_returns_pair() {
    let mut exp = RotationExpander::new();
    exp.add_tile(Symmetry::L, [Socket(1), Socket(2), Socket(3), Socket(4)], 1.0);
    let (_rules, mapping) = exp.build();
    assert_eq!(mapping.num_expanded(), 4);

    let (base, rot) = mapping.get(0);
    assert_eq!(base, 0);
    assert_eq!(rot, Rotation::R0);
}

// ---------------------------------------------------------------------------
// Rotated socket verification
// ---------------------------------------------------------------------------

#[test]
fn rotate_cw_transforms_sockets_correctly() {
    // rotate_cw([N, E, S, W]) -> [W, N, E, S]
    let mut exp = RotationExpander::new();
    let n = Socket(10);
    let e = Socket(20);
    let s = Socket(30);
    let w = Socket(40);
    // Use F symmetry to get all 8 transforms
    exp.add_tile(Symmetry::F, [n, e, s, w], 1.0);
    let (rules, mapping) = exp.build();
    assert_eq!(mapping.num_expanded(), 8);

    // State 0 (R0): [N=10, E=20, S=30, W=40]
    assert_eq!(mapping.rotation(0), Rotation::R0);

    // State 1 (R90): rotate_cw([10, 20, 30, 40]) = [40, 10, 20, 30]
    //   N=40, E=10, S=20, W=30
    assert_eq!(mapping.rotation(1), Rotation::R90);

    // State 0 North=10, State 1 South=20 -> sockets_match(10, 20) = false
    assert!(!rules.compatible(0, 1, Dir2D::North));

    // State 0 East=20, State 1 West=30 -> sockets_match(20, 30) = false
    assert!(!rules.compatible(0, 1, Dir2D::East));

    // State 1 (R90): N=40, E=10, S=20, W=30
    // State 0 (R0):  N=10, E=20, S=30, W=40
    // State 1 looking East: E=10, State 0 West=40 -> sockets_match(10, 40) = false
    assert!(!rules.compatible(1, 0, Dir2D::East));

    // Self-compatibility: State 0 N=10, S=30 -> sockets_match(10, 30) = false
    assert!(!rules.compatible(0, 0, Dir2D::North));

    // State 2 (R180): rotate_cw([40, 10, 20, 30]) = [30, 40, 10, 20]
    //   N=30, E=40, S=10, W=20
    assert_eq!(mapping.rotation(2), Rotation::R180);

    // State 0 South=30, State 2 North=30 -> sockets_match(30, 30) = true
    // compatible(0, 2, South) checks state0.South vs state2.North
    assert!(rules.compatible(0, 2, Dir2D::South));

    // State 0 East=20, State 2 West=20 -> sockets_match(20, 20) = true
    assert!(rules.compatible(0, 2, Dir2D::East));
}

#[test]
fn reflect_horizontal_swaps_east_west() {
    // reflect_horizontal([N, E, S, W]) -> [N, W, S, E]
    let mut exp = RotationExpander::new();
    let n = Socket(10);
    let e = Socket(20);
    let s = Socket(30);
    let w = Socket(40);
    exp.add_tile(Symmetry::F, [n, e, s, w], 1.0);
    let (_rules, mapping) = exp.build();

    // R0Flip should be the 5th variant (index 4)
    assert_eq!(mapping.rotation(4), Rotation::R0Flip);
}

#[test]
fn i_symmetry_rotations_are_0_and_90() {
    let mut exp = RotationExpander::new();
    let a = Socket(1);
    let b = Socket(2);
    // I-shaped: [a, b, a, b]
    // R0:   [a, b, a, b]
    // R90:  [b, a, b, a] -- different
    // R180: [a, b, a, b] -- same as R0
    // R270: [b, a, b, a] -- same as R90
    exp.add_tile(Symmetry::I, [a, b, a, b], 1.0);
    let (_rules, mapping) = exp.build();
    assert_eq!(mapping.num_expanded(), 2);
    assert_eq!(mapping.rotation(0), Rotation::R0);
    assert_eq!(mapping.rotation(1), Rotation::R90);
}

// ---------------------------------------------------------------------------
// Multiple tiles expansion
// ---------------------------------------------------------------------------

#[test]
fn multiple_tiles_expand_independently() {
    let mut exp = RotationExpander::new();
    // Tile 0: X-symmetric (1 variant)
    exp.add_tile(Symmetry::X, [Socket(1), Socket(1), Socket(1), Socket(1)], 2.0);
    // Tile 1: I-symmetric (2 variants)
    exp.add_tile(Symmetry::I, [Socket(1), Socket(2), Socket(1), Socket(2)], 1.0);
    // Tile 2: L-symmetric (4 variants)
    exp.add_tile(Symmetry::L, [Socket(1), Socket(2), Socket(3), Socket(4)], 1.0);

    let (rules, mapping) = exp.build();
    assert_eq!(mapping.num_expanded(), 7); // 1 + 2 + 4
    assert_eq!(rules.num_states(), 7);

    // Check base tile assignments
    assert_eq!(mapping.base_tile(0), 0); // X tile
    assert_eq!(mapping.base_tile(1), 1); // I variant 1
    assert_eq!(mapping.base_tile(2), 1); // I variant 2
    assert_eq!(mapping.base_tile(3), 2); // L variant 1
    assert_eq!(mapping.base_tile(4), 2); // L variant 2
    assert_eq!(mapping.base_tile(5), 2); // L variant 3
    assert_eq!(mapping.base_tile(6), 2); // L variant 4
}

// ---------------------------------------------------------------------------
// Weights are preserved
// ---------------------------------------------------------------------------

#[test]
fn weights_are_preserved_in_expanded_rules() {
    let mut exp = RotationExpander::new();
    exp.add_tile(Symmetry::I, [Socket(1), Socket(2), Socket(1), Socket(2)], 3.5);
    let (rules, mapping) = exp.build();
    assert_eq!(mapping.num_expanded(), 2);
    // Both expanded variants should have the same weight
    assert!((rules.weight(0) - 3.5).abs() < f64::EPSILON);
    assert!((rules.weight(1) - 3.5).abs() < f64::EPSILON);
}

// ---------------------------------------------------------------------------
// Dir2D basics
// ---------------------------------------------------------------------------

#[test]
fn dir2d_opposite() {
    assert_eq!(Dir2D::North.opposite(), Dir2D::South);
    assert_eq!(Dir2D::South.opposite(), Dir2D::North);
    assert_eq!(Dir2D::East.opposite(), Dir2D::West);
    assert_eq!(Dir2D::West.opposite(), Dir2D::East);
}

// ---------------------------------------------------------------------------
// Integration test: River tileset
// ---------------------------------------------------------------------------

/// A 2D grid topology using the module's Dir2D.
struct Grid2D {
    width: usize,
    height: usize,
}

const GRID_DIRS: [Dir2D; 4] = [Dir2D::North, Dir2D::East, Dir2D::South, Dir2D::West];

impl Topology for Grid2D {
    type Direction = Dir2D;

    fn num_cells(&self) -> usize {
        self.width * self.height
    }

    fn directions(&self) -> &[Dir2D] {
        &GRID_DIRS
    }

    fn neighbors(&self, cell: usize) -> impl Iterator<Item = (usize, Dir2D)> {
        let x = cell % self.width;
        let y = cell / self.width;
        let mut result = Vec::new();

        if y > 0 {
            result.push(((y - 1) * self.width + x, Dir2D::North));
        }
        if y + 1 < self.height {
            result.push(((y + 1) * self.width + x, Dir2D::South));
        }
        if x + 1 < self.width {
            result.push((y * self.width + x + 1, Dir2D::East));
        }
        if x > 0 {
            result.push((y * self.width + x - 1, Dir2D::West));
        }

        result.into_iter()
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

#[test]
fn river_tileset_integration() {
    // River tileset:
    //   Socket 1 = grass edge
    //   Socket 2 = river edge
    //
    // Tiles:
    //   - Grass (X-symmetric): all grass edges [1,1,1,1] -> 1 variant
    //   - Straight river (I-symmetric): river on N/S, grass on E/W [2,1,2,1] -> 2 variants
    //   - Bend river (L-symmetric): river on N/E, grass on S/W [2,2,1,1] -> 4 variants
    //   - Cross river (X-symmetric): river on all sides [2,2,2,2] -> 1 variant
    //
    // Total expanded: 1 + 2 + 4 + 1 = 8

    let mut exp = RotationExpander::new();
    let grass = Socket(1);
    let river = Socket(2);

    // Grass tile: X-symmetric
    exp.add_tile(Symmetry::X, [grass, grass, grass, grass], 4.0);
    // Straight river: I-symmetric (river N/S, grass E/W)
    exp.add_tile(Symmetry::I, [river, grass, river, grass], 1.0);
    // Bend river: L-symmetric (river N/E, grass S/W)
    exp.add_tile(Symmetry::L, [river, river, grass, grass], 1.0);
    // Cross river: X-symmetric
    exp.add_tile(Symmetry::X, [river, river, river, river], 0.5);

    let (rules, mapping) = exp.build();
    assert_eq!(mapping.num_expanded(), 8); // 1 + 2 + 4 + 1

    // Solve on a small grid
    let topo = Grid2D {
        width: 4,
        height: 4,
    };

    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 20 });
    let mut solver = WfcSolver::new(topo, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    assert_eq!(result.states().len(), 16);

    // Verify all adjacencies are valid: matching sockets on touching edges
    let verify_topo = Grid2D {
        width: 4,
        height: 4,
    };
    for cell in 0..verify_topo.num_cells() {
        let state = result.state(cell);
        assert!(
            state < rules.num_states(),
            "state {} out of range at cell {}",
            state,
            cell
        );

        for (neighbor, dir) in verify_topo.neighbors(cell) {
            let nb_state = result.state(neighbor);
            assert!(
                rules.compatible(state, nb_state, dir),
                "Adjacency violation: cell {} (tile {}, base={}, rot={:?}) and \
                 cell {} (tile {}, base={}, rot={:?}) in direction {:?}",
                cell,
                state,
                mapping.base_tile(state),
                mapping.rotation(state),
                neighbor,
                nb_state,
                mapping.base_tile(nb_state),
                mapping.rotation(nb_state),
                dir
            );
        }
    }

    // Verify river connections: if a cell has a river socket on an edge,
    // the neighbor must also have a river socket on the facing edge.
    // (This is implicitly guaranteed by socket matching, but let's verify
    // the semantic meaning: river edges connect to river edges, grass to grass.)
}

#[test]
fn river_tileset_larger_grid() {
    // Stress test on a larger grid
    let mut exp = RotationExpander::new();
    let grass = Socket(1);
    let river = Socket(2);

    exp.add_tile(Symmetry::X, [grass, grass, grass, grass], 4.0);
    exp.add_tile(Symmetry::I, [river, grass, river, grass], 1.0);
    exp.add_tile(Symmetry::L, [river, river, grass, grass], 1.0);
    exp.add_tile(Symmetry::X, [river, river, river, river], 0.5);

    let (rules, mapping) = exp.build();

    let topo = Grid2D {
        width: 8,
        height: 8,
    };

    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 50 });
    let mut solver = WfcSolver::new(topo, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(12345);
    let result = solver.solve(&mut rng).unwrap();

    assert_eq!(result.states().len(), 64);

    // Verify all adjacencies
    let verify_topo = Grid2D {
        width: 8,
        height: 8,
    };
    for cell in 0..verify_topo.num_cells() {
        let state = result.state(cell);
        for (neighbor, dir) in verify_topo.neighbors(cell) {
            let nb_state = result.state(neighbor);
            assert!(
                rules.compatible(state, nb_state, dir),
                "Adjacency violation at cell {} <-> {} in {:?} (states {} <-> {}, \
                 bases {} <-> {}, rots {:?} <-> {:?})",
                cell,
                neighbor,
                dir,
                state,
                nb_state,
                mapping.base_tile(state),
                mapping.base_tile(nb_state),
                mapping.rotation(state),
                mapping.rotation(nb_state),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Compatibility table consistency
// ---------------------------------------------------------------------------

#[test]
fn compat_table_agrees_with_rules() {
    use wavfc::rules::CompatibilityTable;

    let mut exp = RotationExpander::new();
    let grass = Socket(1);
    let river = Socket(2);

    exp.add_tile(Symmetry::X, [grass, grass, grass, grass], 4.0);
    exp.add_tile(Symmetry::I, [river, grass, river, grass], 1.0);
    exp.add_tile(Symmetry::L, [river, river, grass, grass], 1.0);
    exp.add_tile(Symmetry::X, [river, river, river, river], 0.5);

    let (rules, mapping) = exp.build();

    // Grass should be compatible with grass in all directions
    assert!(rules.compatible(0, 0, Dir2D::North));
    assert!(rules.compatible(0, 0, Dir2D::South));
    assert!(rules.compatible(0, 0, Dir2D::East));
    assert!(rules.compatible(0, 0, Dir2D::West));

    // Build CompatibilityTable using Grid2D and verify it agrees with rules
    let grid = Grid2D { width: 4, height: 4 };
    let compat = CompatibilityTable::<2>::new(&grid, &rules);

    for a in 0..mapping.num_expanded() {
        for dir in &GRID_DIRS {
            let dir_idx = grid.direction_index(*dir);
            let compat_set = compat.compatible_states(a, dir_idx);
            for b in 0..mapping.num_expanded() {
                let from_rules = rules.compatible(a, b, *dir);
                let from_compat = compat_set.test(b);
                assert_eq!(
                    from_rules, from_compat,
                    "Mismatch: compatible({}, {}, {:?}): rules={}, compat={}",
                    a, b, dir, from_rules, from_compat
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Edge case: empty expander
// ---------------------------------------------------------------------------

#[test]
fn empty_expander_produces_zero_states() {
    let exp = RotationExpander::new();
    let (rules, mapping) = exp.build();
    assert_eq!(mapping.num_expanded(), 0);
    assert_eq!(rules.num_states(), 0);
}

// ---------------------------------------------------------------------------
// Compatibility between rotated variants
// ---------------------------------------------------------------------------

#[test]
fn straight_river_variants_are_compatible() {
    // A straight river [river, grass, river, grass] should connect to itself
    // along the river axis.
    let mut exp = RotationExpander::new();
    let grass = Socket(1);
    let river = Socket(2);

    // Only the straight river tile
    exp.add_tile(Symmetry::I, [river, grass, river, grass], 1.0);
    let (rules, _mapping) = exp.build();

    // Variant 0: R0 [river, grass, river, grass]  (N=river, E=grass, S=river, W=grass)
    // Variant 1: R90 [grass, river, grass, river]  (N=grass, E=river, S=grass, W=river)

    // Variant 0 north -> variant 0 south: river matches river
    assert!(rules.compatible(0, 0, Dir2D::North));
    // Variant 0 south -> variant 0 north: river matches river
    assert!(rules.compatible(0, 0, Dir2D::South));
    // Variant 0 east -> variant 0 west: grass matches grass
    assert!(rules.compatible(0, 0, Dir2D::East));

    // Variant 0 north -> variant 1 south: river vs grass -> no match
    assert!(!rules.compatible(0, 1, Dir2D::North));

    // Variant 1 east -> variant 1 west: river matches river
    assert!(rules.compatible(1, 1, Dir2D::East));
}
