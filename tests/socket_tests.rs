use wavfc::*;
use rand::SeedableRng;
use rand::rngs::StdRng;

// ---- 2D Grid Topology (reused for socket tests) ----

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Dir2D {
    North,
    South,
    East,
    West,
}

const DIRS_2D: [Dir2D; 4] = [Dir2D::North, Dir2D::South, Dir2D::East, Dir2D::West];

struct Grid2D {
    width: usize,
    height: usize,
}

impl Topology for Grid2D {
    type Direction = Dir2D;

    fn num_cells(&self) -> usize {
        self.width * self.height
    }

    fn directions(&self) -> &[Dir2D] {
        &DIRS_2D
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
        match dir {
            Dir2D::North => Dir2D::South,
            Dir2D::South => Dir2D::North,
            Dir2D::East => Dir2D::West,
            Dir2D::West => Dir2D::East,
        }
    }

    fn direction_index(&self, dir: Dir2D) -> usize {
        match dir {
            Dir2D::North => 0,
            Dir2D::South => 1,
            Dir2D::East => 2,
            Dir2D::West => 3,
        }
    }
}

// ---- Socket matching tests ----

#[test]
fn symmetric_match() {
    // Socket(1) matches Socket(1)
    assert!(sockets_match(Socket(1), Socket(1)));
    assert!(sockets_match(Socket(0), Socket(0)));
    assert!(sockets_match(Socket(100), Socket(100)));
}

#[test]
fn symmetric_non_match() {
    // Socket(1) does NOT match Socket(2)
    assert!(!sockets_match(Socket(1), Socket(2)));
    assert!(!sockets_match(Socket(0), Socket(1)));
}

#[test]
fn asymmetric_match() {
    // Socket(1).flipped() matches Socket(1)
    assert!(sockets_match(Socket(1).flipped(), Socket(1)));
    // And in reverse
    assert!(sockets_match(Socket(1), Socket(1).flipped()));
}

#[test]
fn flipped_flipped_non_match() {
    // Socket(1).flipped() does NOT match Socket(1).flipped()
    assert!(!sockets_match(Socket(1).flipped(), Socket(1).flipped()));
}

#[test]
fn flipped_different_base_non_match() {
    // Socket(1).flipped() does NOT match Socket(2)
    assert!(!sockets_match(Socket(1).flipped(), Socket(2)));
    // Socket(1).flipped() does NOT match Socket(2).flipped()
    assert!(!sockets_match(Socket(1).flipped(), Socket(2).flipped()));
}

#[test]
fn socket_flipped_sets_high_bit() {
    let s = Socket(5);
    let f = s.flipped();
    assert_eq!(f.0, 5 | 0x8000);
    assert!(f.is_flipped());
    assert!(!s.is_flipped());
}

#[test]
fn socket_base_clears_high_bit() {
    let f = Socket(5).flipped();
    let b = f.base();
    assert_eq!(b.0, 5);
    assert!(!b.is_flipped());
}

#[test]
fn socket_double_flip_is_idempotent() {
    // flipped() ORs the high bit, so flipping twice is the same as once
    let s = Socket(7);
    assert_eq!(s.flipped().flipped().0, s.flipped().0);
}

// ---- SocketRuleBuilder + SocketRules tests ----

/// Builds a simple tileset with 3 tiles for a 2D grid:
///
/// Tile 0 ("Grass"): symmetric socket 1 on all 4 sides
/// Tile 1 ("Path"):  symmetric socket 2 on N/S, symmetric socket 1 on E/W
/// Tile 2 ("Water"): symmetric socket 3 on all 4 sides
///
/// Expected compatibility:
///   Grass(N=1) <-> Grass(S=1): match (1==1)
///   Grass(N=1) <-> Path(S=2):  no match (1!=2)
///   Grass(E=1) <-> Path(W=1):  match (1==1)
///   Path(N=2)  <-> Path(S=2):  match (2==2)
///   Water(N=3) <-> Water(S=3): match (3==3)
///   Water(N=3) <-> Grass(S=1): no match (3!=1)
fn build_simple_tileset() -> (Grid2D, SocketRules<Dir2D>) {
    let topo = Grid2D {
        width: 4,
        height: 4,
    };

    let mut builder = SocketRuleBuilder::new();

    // Tile 0: Grass (socket 1 on all sides, weight 2.0)
    builder.add_tile(
        vec![
            (Dir2D::North, Socket(1)),
            (Dir2D::South, Socket(1)),
            (Dir2D::East, Socket(1)),
            (Dir2D::West, Socket(1)),
        ],
        2.0,
    );

    // Tile 1: Path (socket 2 on N/S, socket 1 on E/W, weight 1.0)
    builder.add_tile(
        vec![
            (Dir2D::North, Socket(2)),
            (Dir2D::South, Socket(2)),
            (Dir2D::East, Socket(1)),
            (Dir2D::West, Socket(1)),
        ],
        1.0,
    );

    // Tile 2: Water (socket 3 on all sides, weight 1.5)
    builder.add_tile(
        vec![
            (Dir2D::North, Socket(3)),
            (Dir2D::South, Socket(3)),
            (Dir2D::East, Socket(3)),
            (Dir2D::West, Socket(3)),
        ],
        1.5,
    );

    let rules = builder.build(&topo);
    (topo, rules)
}

#[test]
fn socket_rules_num_states() {
    let (_topo, rules) = build_simple_tileset();
    assert_eq!(rules.num_states(), 3);
}

#[test]
fn socket_rules_weights_preserved() {
    let (_topo, rules) = build_simple_tileset();
    assert!((rules.weight(0) - 2.0).abs() < f64::EPSILON);
    assert!((rules.weight(1) - 1.0).abs() < f64::EPSILON);
    assert!((rules.weight(2) - 1.5).abs() < f64::EPSILON);
}

#[test]
fn socket_rules_compatible_grass_grass() {
    let (_topo, rules) = build_simple_tileset();
    // Grass(N=1) looking North -> neighbor's South=1: match
    assert!(rules.compatible(0, 0, Dir2D::North));
    assert!(rules.compatible(0, 0, Dir2D::South));
    assert!(rules.compatible(0, 0, Dir2D::East));
    assert!(rules.compatible(0, 0, Dir2D::West));
}

#[test]
fn socket_rules_compatible_grass_path() {
    let (_topo, rules) = build_simple_tileset();
    // Grass looking North: Grass(N)=1, Path(S)=2 -> no match
    assert!(!rules.compatible(0, 1, Dir2D::North));
    // Grass looking East: Grass(E)=1, Path(W)=1 -> match
    assert!(rules.compatible(0, 1, Dir2D::East));
    // Grass looking West: Grass(W)=1, Path(E)=1 -> match
    assert!(rules.compatible(0, 1, Dir2D::West));
}

#[test]
fn socket_rules_compatible_path_path() {
    let (_topo, rules) = build_simple_tileset();
    // Path(N)=2 looking North -> neighbor's Path(S)=2: match
    assert!(rules.compatible(1, 1, Dir2D::North));
    assert!(rules.compatible(1, 1, Dir2D::South));
    // Path(E)=1 looking East -> neighbor's Path(W)=1: match
    assert!(rules.compatible(1, 1, Dir2D::East));
    assert!(rules.compatible(1, 1, Dir2D::West));
}

#[test]
fn socket_rules_compatible_water_water() {
    let (_topo, rules) = build_simple_tileset();
    assert!(rules.compatible(2, 2, Dir2D::North));
    assert!(rules.compatible(2, 2, Dir2D::South));
    assert!(rules.compatible(2, 2, Dir2D::East));
    assert!(rules.compatible(2, 2, Dir2D::West));
}

#[test]
fn socket_rules_incompatible_grass_water() {
    let (_topo, rules) = build_simple_tileset();
    // Grass(N=1) vs Water(S=3): no match
    assert!(!rules.compatible(0, 2, Dir2D::North));
    assert!(!rules.compatible(0, 2, Dir2D::South));
    assert!(!rules.compatible(0, 2, Dir2D::East));
    assert!(!rules.compatible(0, 2, Dir2D::West));
    // Symmetric
    assert!(!rules.compatible(2, 0, Dir2D::North));
}

#[test]
fn socket_rules_incompatible_path_water() {
    let (_topo, rules) = build_simple_tileset();
    // Path(N=2) vs Water(S=3): no match
    assert!(!rules.compatible(1, 2, Dir2D::North));
    // Path(E=1) vs Water(W=3): no match
    assert!(!rules.compatible(1, 2, Dir2D::East));
}

// ---- Asymmetric socket integration test ----

#[test]
fn socket_rules_asymmetric_sockets() {
    let topo = Grid2D {
        width: 2,
        height: 1,
    };

    let mut builder = SocketRuleBuilder::new();

    // Tile 0: Socket(10) on East, Socket(10).flipped() on West
    builder.add_tile(
        vec![
            (Dir2D::North, Socket(99)),
            (Dir2D::South, Socket(99)),
            (Dir2D::East, Socket(10)),
            (Dir2D::West, Socket(10).flipped()),
        ],
        1.0,
    );

    // Tile 1: Socket(10).flipped() on East, Socket(10) on West
    builder.add_tile(
        vec![
            (Dir2D::North, Socket(99)),
            (Dir2D::South, Socket(99)),
            (Dir2D::East, Socket(10).flipped()),
            (Dir2D::West, Socket(10)),
        ],
        1.0,
    );

    let rules = builder.build(&topo);

    // Tile 0 East=Socket(10), Tile 1 West=Socket(10): symmetric match
    assert!(rules.compatible(0, 1, Dir2D::East));

    // Tile 1 East=Socket(10).flipped(), Tile 0 West=Socket(10).flipped():
    // flipped-flipped -> no match
    assert!(!rules.compatible(1, 0, Dir2D::East));

    // Tile 0 East=Socket(10), Tile 0 West=Socket(10).flipped():
    // Socket(10) vs Socket(10).flipped() -> asymmetric match
    assert!(rules.compatible(0, 0, Dir2D::East));
}

// ---- Full solve integration test ----

#[test]
fn socket_rules_solve_simple_tileset() {
    let topo = Grid2D {
        width: 4,
        height: 4,
    };

    let mut builder = SocketRuleBuilder::new();

    // Grass: socket 1 everywhere (weight 2.0)
    builder.add_tile(
        vec![
            (Dir2D::North, Socket(1)),
            (Dir2D::South, Socket(1)),
            (Dir2D::East, Socket(1)),
            (Dir2D::West, Socket(1)),
        ],
        2.0,
    );

    // Path: socket 2 N/S, socket 1 E/W (weight 1.0)
    builder.add_tile(
        vec![
            (Dir2D::North, Socket(2)),
            (Dir2D::South, Socket(2)),
            (Dir2D::East, Socket(1)),
            (Dir2D::West, Socket(1)),
        ],
        1.0,
    );

    // Water: socket 3 everywhere (weight 1.5)
    builder.add_tile(
        vec![
            (Dir2D::North, Socket(3)),
            (Dir2D::South, Socket(3)),
            (Dir2D::East, Socket(3)),
            (Dir2D::West, Socket(3)),
        ],
        1.5,
    );

    let rules = builder.build(&topo);

    let config = SolverConfig::new()
        .backtrack(BacktrackStrategy::Restart { max_restarts: 10 });
    let mut solver = WfcSolver::new(topo, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(42);
    let result = solver.solve(&mut rng).unwrap();

    assert_eq!(result.states().len(), 16);

    // Verify every adjacency satisfies socket constraints
    let verify_topo = Grid2D {
        width: 4,
        height: 4,
    };
    for cell in 0..verify_topo.num_cells() {
        let state = result.state(cell);
        assert!(state < rules.num_states(), "state {} out of range at cell {}", state, cell);

        for (neighbor, dir) in verify_topo.neighbors(cell) {
            let nb_state = result.state(neighbor);
            assert!(
                rules.compatible(state, nb_state, dir),
                "Adjacency violation: cell {} (tile {}) and cell {} (tile {}) in direction {:?}",
                cell, state, neighbor, nb_state, dir
            );
        }
    }
}

#[test]
fn socket_rules_solve_uniform_tileset() {
    // Single tile with the same socket on all sides -- should always solve trivially
    let topo = Grid2D {
        width: 5,
        height: 5,
    };

    let mut builder = SocketRuleBuilder::new();
    builder.add_tile(
        vec![
            (Dir2D::North, Socket(1)),
            (Dir2D::South, Socket(1)),
            (Dir2D::East, Socket(1)),
            (Dir2D::West, Socket(1)),
        ],
        1.0,
    );

    let rules = builder.build(&topo);
    let config = SolverConfig::new();
    let mut solver = WfcSolver::new(topo, &rules, config).unwrap();
    let mut rng = StdRng::seed_from_u64(0);
    let result = solver.solve(&mut rng).unwrap();

    // All cells must be tile 0
    for cell in 0..25 {
        assert_eq!(result.state(cell), 0);
    }
}

#[test]
fn socket_rules_builder_returns_correct_indices() {
    let mut builder: SocketRuleBuilder<Dir2D> = SocketRuleBuilder::new();
    let i0 = builder.add_tile(vec![(Dir2D::North, Socket(1))], 1.0);
    let i1 = builder.add_tile(vec![(Dir2D::North, Socket(2))], 1.0);
    let i2 = builder.add_tile(vec![(Dir2D::North, Socket(3))], 1.0);
    assert_eq!(i0, 0);
    assert_eq!(i1, 1);
    assert_eq!(i2, 2);
}
