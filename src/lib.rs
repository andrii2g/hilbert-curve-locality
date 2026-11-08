#[derive(Clone, Copy, Debug)]
pub enum Order {
    RowMajor,
    Morton,
    Hilbert,
}

impl Order {
    pub const ALL: [Self; 3] = [Self::RowMajor, Self::Morton, Self::Hilbert];

    pub fn name(self) -> &'static str {
        match self {
            Self::RowMajor => "Row-major",
            Self::Morton => "Morton",
            Self::Hilbert => "Hilbert",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Grid {
    side: u32,
}

impl Grid {
    pub fn new(side: u32) -> Option<Self> {
        (side.is_power_of_two() && (8..=128).contains(&side)).then_some(Self { side })
    }

    pub fn side(self) -> u32 {
        self.side
    }

    pub fn cells(self) -> u32 {
        self.side * self.side
    }

    pub fn scale_steps(self, order: Order) -> u32 {
        match order {
            Order::RowMajor => 0,
            Order::Morton | Order::Hilbert => self.side.ilog2(),
        }
    }

    pub fn encode(self, order: Order, mut x: u32, mut y: u32) -> Option<u32> {
        if x >= self.side || y >= self.side {
            return None;
        }
        let mut index = 0;
        match order {
            Order::RowMajor => index = y * self.side + x,
            Order::Morton => {
                for bit in 0..self.scale_steps(order) {
                    index |= ((x >> bit) & 1) << (2 * bit);
                    index |= ((y >> bit) & 1) << (2 * bit + 1);
                }
            }
            Order::Hilbert => {
                let mut half = self.side / 2;
                while half != 0 {
                    let rx = x / half;
                    let ry = y / half;
                    index = 4 * index + ((3 * rx) ^ ry);
                    (x, y) = orient(half, x % half, y % half, rx, ry);
                    half /= 2;
                }
            }
        }
        Some(index)
    }

    pub fn decode(self, order: Order, mut index: u32) -> Option<(u32, u32)> {
        if index >= self.cells() {
            return None;
        }
        let (mut x, mut y) = (0, 0);
        match order {
            Order::RowMajor => (x, y) = (index % self.side, index / self.side),
            Order::Morton => {
                for bit in 0..self.scale_steps(order) {
                    x |= ((index >> (2 * bit)) & 1) << bit;
                    y |= ((index >> (2 * bit + 1)) & 1) << bit;
                }
            }
            Order::Hilbert => {
                let mut half = 1;
                while half < self.side {
                    let rx = (index >> 1) & 1;
                    let ry = (index ^ rx) & 1;
                    (x, y) = orient(half, x, y, rx, ry);
                    x += half * rx;
                    y += half * ry;
                    index >>= 2;
                    half *= 2;
                }
            }
        }
        Some((x, y))
    }
}

// Work within the quadrant so reflections stay within unsigned bounds.
fn orient(side: u32, x: u32, y: u32, rx: u32, ry: u32) -> (u32, u32) {
    match (rx, ry) {
        (_, 1) => (x, y),
        (0, _) => (y, x),
        _ => (side - 1 - y, side - 1 - x),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mappings_round_trip_at_every_supported_size() {
        for side in [8, 16, 32, 64, 128] {
            let grid = Grid::new(side).unwrap();
            for order in Order::ALL {
                for index in 0..grid.cells() {
                    let (x, y) = grid.decode(order, index).unwrap();
                    assert_eq!(grid.encode(order, x, y), Some(index));

                    let (x, y) = (index % side, index / side);
                    let encoded = grid.encode(order, x, y).unwrap();
                    assert_eq!(grid.decode(order, encoded), Some((x, y)));
                }
                assert_eq!(grid.decode(order, grid.cells()), None);
                assert_eq!(grid.encode(order, side, 0), None);
                assert_eq!(grid.encode(order, 0, side), None);
            }

            for index in 1..grid.cells() {
                let (x, y) = grid.decode(Order::Hilbert, index - 1).unwrap();
                let (next_x, next_y) = grid.decode(Order::Hilbert, index).unwrap();
                assert_eq!(x.abs_diff(next_x) + y.abs_diff(next_y), 1);
            }
        }
    }

    #[test]
    fn grid_validation_and_orientation() {
        for side in [0, 4, 12, 256, u32::MAX] {
            assert!(Grid::new(side).is_none());
        }
        let grid = Grid::new(8).unwrap();
        for (order, expected) in [
            (Order::RowMajor, [(0, 0), (1, 0), (2, 0), (3, 0)]),
            (Order::Morton, [(0, 0), (1, 0), (0, 1), (1, 1)]),
            (Order::Hilbert, [(0, 0), (0, 1), (1, 1), (1, 0)]),
        ] {
            for (index, point) in (0..4).zip(expected) {
                assert_eq!(grid.decode(order, index), Some(point));
            }
        }
    }
}
