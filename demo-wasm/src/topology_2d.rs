use wavfc::Topology;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Dir2D {
    North,
    South,
    East,
    West,
}

static DIRECTIONS: [Dir2D; 4] = [Dir2D::North, Dir2D::South, Dir2D::East, Dir2D::West];

pub struct Grid2D {
    pub width: usize,
    pub height: usize,
    pub wrapping: bool,
}

impl Topology for Grid2D {
    type Direction = Dir2D;

    fn num_cells(&self) -> usize {
        self.width * self.height
    }

    fn directions(&self) -> &[Dir2D] {
        &DIRECTIONS
    }

    fn neighbors(&self, cell: usize) -> impl Iterator<Item = (usize, Dir2D)> {
        let x = cell % self.width;
        let y = cell / self.width;
        let w = self.width;
        let h = self.height;
        let wrapping = self.wrapping;
        let mut result = Vec::new();

        // North
        if y > 0 {
            result.push(((y - 1) * w + x, Dir2D::North));
        } else if wrapping {
            result.push(((h - 1) * w + x, Dir2D::North));
        }
        // South
        if y + 1 < h {
            result.push(((y + 1) * w + x, Dir2D::South));
        } else if wrapping {
            result.push((x, Dir2D::South));
        }
        // West
        if x > 0 {
            result.push((y * w + x - 1, Dir2D::West));
        } else if wrapping {
            result.push((y * w + w - 1, Dir2D::West));
        }
        // East
        if x + 1 < w {
            result.push((y * w + x + 1, Dir2D::East));
        } else if wrapping {
            result.push((y * w, Dir2D::East));
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
