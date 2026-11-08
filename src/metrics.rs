use hilbert_curve_locality::{Grid, Order};
use std::{hint::black_box, ops::Range, time::Instant};

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

pub fn queries(grid: Grid) -> Vec<(&'static str, Rect)> {
    let side = grid.side();
    let quarter = side / 4;
    let mut queries = vec![
        ("Top row", Rect::new(0, 0, side, 1)),
        ("Left column", Rect::new(0, 0, 1, side)),
        (
            "Aligned square",
            Rect::new(quarter, quarter, quarter, quarter),
        ),
        (
            "Shifted square",
            Rect::new(quarter - 1, quarter - 1, quarter, quarter),
        ),
        (
            "Tall rectangle",
            Rect::new(3 * side / 8, side / 8, 3 * side / 16, side / 2 + 1),
        ),
    ];

    let mut seed = 0x4849_4c42_4552_5421_u64;
    let mut draw = |limit: u32| {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 32) as u32 % limit
    };
    for _ in 0..32 {
        let width = 1 + draw(side);
        let height = 1 + draw(side);
        let x = draw(side - width + 1);
        let y = draw(side - height + 1);
        queries.push(("Seeded", Rect::new(x, y, width, height)));
    }
    queries
}

pub fn intervals(grid: Grid, order: Order, rect: Rect) -> Vec<Range<u32>> {
    let mut indices = Vec::with_capacity((rect.width * rect.height) as usize);
    for y in rect.y..rect.y + rect.height {
        for x in rect.x..rect.x + rect.width {
            indices.push(grid.encode(order, x, y).unwrap());
        }
    }
    indices.sort_unstable();

    let mut runs: Vec<Range<u32>> = Vec::new();
    for index in indices {
        match runs.last_mut() {
            Some(run) if run.end == index => run.end += 1,
            _ => runs.push(index..index + 1),
        }
    }
    runs
}

pub struct Locality {
    pub mean_jump: f64,
    pub max_jump: u32,
    pub median_gap: f64,
    pub p95_gap: u32,
    pub max_gap: u32,
    pub cell_gaps: Vec<u32>,
}

pub fn locality(grid: Grid, order: Order) -> Locality {
    let side = grid.side();
    let path: Vec<_> = (0..grid.cells())
        .map(|index| grid.decode(order, index).unwrap())
        .collect();
    let (mut sum_jump, mut max_jump) = (0, 0);
    for pair in path.windows(2) {
        let [(x1, y1), (x2, y2)] = [pair[0], pair[1]];
        let distance = x1.abs_diff(x2) + y1.abs_diff(y2);
        sum_jump += distance;
        max_jump = max_jump.max(distance);
    }

    let mut cell_gaps = vec![0; grid.cells() as usize];
    let mut gaps = Vec::new();
    for y in 0..side {
        for x in 0..side {
            let index = grid.encode(order, x, y).unwrap();
            for (nx, ny) in [(x + 1, y), (x, y + 1)] {
                if let Some(neighbor) = grid.encode(order, nx, ny) {
                    let gap = index.abs_diff(neighbor);
                    gaps.push(gap);
                    for offset in [y * side + x, ny * side + nx] {
                        let cell_gap = &mut cell_gaps[offset as usize];
                        *cell_gap = (*cell_gap).max(gap);
                    }
                }
            }
        }
    }
    gaps.sort_unstable();
    let len = gaps.len();
    Locality {
        mean_jump: f64::from(sum_jump) / f64::from(grid.cells() - 1),
        max_jump,
        median_gap: f64::from(gaps[(len - 1) / 2] + gaps[len / 2]) / 2.0,
        p95_gap: gaps[(len * 95).div_ceil(100) - 1],
        max_gap: gaps[len - 1],
        cell_gaps,
    }
}

pub fn mapping_times(grid: Grid, order: Order) -> (f64, f64) {
    const CALLS: u32 = 1 << 20;
    // An odd multiplier permutes the indices of a power-of-two grid.
    let inputs: Vec<_> = (0..grid.cells())
        .map(|i| (i * 40503) & (grid.cells() - 1))
        .collect();
    let points: Vec<_> = inputs
        .iter()
        .map(|&i| grid.decode(Order::RowMajor, i).unwrap())
        .collect();
    let rounds = CALLS / grid.cells();

    let start = Instant::now();
    for _ in 0..rounds {
        for &(x, y) in &points {
            let (x, y) = black_box((x, y));
            black_box(grid.encode(order, x, y));
        }
    }
    let encode = start.elapsed().as_secs_f64();
    let start = Instant::now();
    for _ in 0..rounds {
        for &index in &inputs {
            black_box(grid.decode(order, black_box(index)));
        }
    }
    let decode = start.elapsed().as_secs_f64();
    let ns_per_call = 1e9 / f64::from(CALLS);
    (encode * ns_per_call, decode * ns_per_call)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shifted_square_matches_reference_counts() {
        let grid = Grid::new(32).unwrap();
        let rect = Rect::new(7, 7, 8, 8);
        for (order, expected) in Order::ALL.into_iter().zip([8, 22, 12]) {
            assert_eq!(intervals(grid, order, rect).len(), expected);
        }
    }

    #[test]
    fn intervals_cover_exactly_the_query_cells() {
        for side in [8, 16, 32, 64, 128] {
            let grid = Grid::new(side).unwrap();
            for (_, rect) in queries(grid) {
                for order in Order::ALL {
                    let runs = intervals(grid, order, rect);
                    assert!(runs.windows(2).all(|pair| pair[0].end < pair[1].start));
                    assert_eq!(
                        runs.iter().map(Range::len).sum::<usize>(),
                        (rect.width * rect.height) as usize
                    );
                    for index in runs.into_iter().flatten() {
                        let (x, y) = grid.decode(order, index).unwrap();
                        assert!((rect.x..rect.x + rect.width).contains(&x));
                        assert!((rect.y..rect.y + rect.height).contains(&y));
                    }
                }
            }
        }
    }
}
