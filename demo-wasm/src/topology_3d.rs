use wavfc::Topology;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Dir3D {
    North,
    South,
    East,
    West,
    Up,
    Down,
}

static DIRECTIONS: [Dir3D; 6] = [
    Dir3D::North,
    Dir3D::South,
    Dir3D::East,
    Dir3D::West,
    Dir3D::Up,
    Dir3D::Down,
];

pub struct Grid3D {
    pub width: usize,
    pub height: usize,
    pub depth: usize,
    pub wrapping: bool,
}

impl Grid3D {
    #[inline]
    fn index(&self, x: usize, y: usize, z: usize) -> usize {
        z * (self.width * self.height) + y * self.width + x
    }
}

impl Topology for Grid3D {
    type Direction = Dir3D;

    fn num_cells(&self) -> usize {
        self.width * self.height * self.depth
    }

    fn directions(&self) -> &[Dir3D] {
        &DIRECTIONS
    }

    fn neighbors(&self, cell: usize) -> impl Iterator<Item = (usize, Dir3D)> {
        let layer_size = self.width * self.height;
        let z = cell / layer_size;
        let rem = cell % layer_size;
        let y = rem / self.width;
        let x = rem % self.width;
        let w = self.width;
        let h = self.height;
        let d = self.depth;
        let wrapping = self.wrapping;
        let mut result = Vec::new();

        // North (y-1)
        if y > 0 {
            result.push((self.index(x, y - 1, z), Dir3D::North));
        } else if wrapping {
            result.push((self.index(x, h - 1, z), Dir3D::North));
        }
        // South (y+1)
        if y + 1 < h {
            result.push((self.index(x, y + 1, z), Dir3D::South));
        } else if wrapping {
            result.push((self.index(x, 0, z), Dir3D::South));
        }
        // West (x-1)
        if x > 0 {
            result.push((self.index(x - 1, y, z), Dir3D::West));
        } else if wrapping {
            result.push((self.index(w - 1, y, z), Dir3D::West));
        }
        // East (x+1)
        if x + 1 < w {
            result.push((self.index(x + 1, y, z), Dir3D::East));
        } else if wrapping {
            result.push((self.index(0, y, z), Dir3D::East));
        }
        // Down (z-1) -- never wraps
        if z > 0 {
            result.push((self.index(x, y, z - 1), Dir3D::Down));
        }
        // Up (z+1) -- never wraps
        if z + 1 < d {
            result.push((self.index(x, y, z + 1), Dir3D::Up));
        }

        result.into_iter()
    }

    fn opposite(&self, dir: Dir3D) -> Dir3D {
        match dir {
            Dir3D::North => Dir3D::South,
            Dir3D::South => Dir3D::North,
            Dir3D::East => Dir3D::West,
            Dir3D::West => Dir3D::East,
            Dir3D::Up => Dir3D::Down,
            Dir3D::Down => Dir3D::Up,
        }
    }

    fn direction_index(&self, dir: Dir3D) -> usize {
        match dir {
            Dir3D::North => 0,
            Dir3D::South => 1,
            Dir3D::East => 2,
            Dir3D::West => 3,
            Dir3D::Up => 4,
            Dir3D::Down => 5,
        }
    }
}
