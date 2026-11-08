use crate::metrics::{Locality, Rect, intervals};
use hilbert_curve_locality::{Grid, Order};
use std::{fmt::Write as _, fs, io, ops::Range, path::Path};

const PANEL: u32 = 256;
const LEFT: u32 = 24;
const TOP: u32 = 84;
const STRIDE: u32 = 280;
const PALETTE: [&str; 6] = [
    "#b91c1c", "#1d4ed8", "#047857", "#7e22ce", "#92400e", "#0e7490",
];

pub fn write_reports(
    directory: &Path,
    grid: Grid,
    query: Rect,
    stats: &[Locality; 3],
) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    fs::write(directory.join("traversals.svg"), traversals(grid, query))?;
    fs::write(directory.join("heatmaps.svg"), heatmaps(grid, stats))
}

fn header(title: &str, subtitle: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 864 402" width="864" height="402" role="img" aria-labelledby="title desc">
<title id="title">{title}</title>
<desc id="desc">{subtitle}</desc>
<rect width="864" height="402" fill="#fff"/>
<g font-family="system-ui, sans-serif" fill="#172033">
<text x="24" y="32" font-size="22" font-weight="bold">{title}</text>
<text x="24" y="54" font-size="13">{subtitle}</text>
</g>
"##
    )
}

fn label(svg: &mut String, x: u32, y: u32, value: &str) {
    writeln!(svg, r##"<text x="{x}" y="{y}" font-family="system-ui, sans-serif" font-size="13" fill="#172033">{value}</text>"##).unwrap();
}

fn traversals(grid: Grid, query: Rect) -> String {
    let cell = PANEL / grid.side();
    let subtitle = format!(
        "Shifted {} × {} query at ({}, {}) on a {} × {} grid",
        query.width,
        query.height,
        query.x,
        query.y,
        grid.side(),
        grid.side()
    );
    let mut svg = header("Traversal and query fragmentation", &subtitle);

    for (panel, order) in Order::ALL.into_iter().enumerate() {
        let left = LEFT + STRIDE * panel as u32;
        let runs = intervals(grid, order, query);
        label(&mut svg, left, TOP - 9, order.name());
        let (qx, qy) = (left + query.x * cell, TOP + query.y * cell);
        let (qw, qh) = (query.width * cell, query.height * cell);
        writeln!(svg, r##"<rect x="{left}" y="{TOP}" width="{PANEL}" height="{PANEL}" fill="#f8fafc" stroke="#cbd5e1"/>
<rect x="{qx}" y="{qy}" width="{qw}" height="{qh}" fill="#fef3c7" stroke="#a16207"/>"##).unwrap();

        let center = |index| {
            let (x, y) = grid.decode(order, index).unwrap();
            (left + x * cell + cell / 2, TOP + y * cell + cell / 2)
        };
        polyline(&mut svg, 0..grid.cells(), center, "#94a3b8", 0.8);
        for (number, run) in runs.iter().enumerate() {
            let color = PALETTE[number % PALETTE.len()];
            let (x, y) = center(run.start);
            polyline(&mut svg, run.clone(), center, color, 1.7);
            writeln!(svg, r#"<circle cx="{x}" cy="{y}" r="1.5" fill="{color}"/>"#).unwrap();
        }
        label(
            &mut svg,
            left,
            TOP + PANEL + 20,
            &format!("{} exact intervals", runs.len()),
        );
    }
    label(
        &mut svg,
        LEFT,
        384,
        "Colored segments show the selected runs. Dots mark the first cell of each run.",
    );
    svg.push_str("</svg>\n");
    svg
}

fn polyline(
    svg: &mut String,
    run: Range<u32>,
    center: impl Fn(u32) -> (u32, u32),
    color: &str,
    width: f64,
) {
    if run.len() < 2 {
        return;
    }
    write!(svg, r#"<polyline fill="none" stroke="{color}" stroke-width="{width}" stroke-linejoin="round" points=""#).unwrap();
    for index in run {
        let (x, y) = center(index);
        write!(svg, "{x},{y} ").unwrap();
    }
    svg.push_str("\"/>\n");
}

fn heatmaps(grid: Grid, stats: &[Locality; 3]) -> String {
    let cell = PANEL / grid.side();
    let max_gap = stats.iter().map(|stats| stats.max_gap).max().unwrap();
    let subtitle = format!(
        "Largest index gap to an edge-sharing neighbor on a {} × {} grid",
        grid.side(),
        grid.side()
    );
    let mut svg = header("Spatial neighbor locality", &subtitle);

    for (panel, (order, stats)) in Order::ALL.into_iter().zip(stats).enumerate() {
        let left = LEFT + STRIDE * panel as u32;
        label(&mut svg, left, TOP - 9, order.name());
        for (index, &gap) in (0..grid.cells()).zip(&stats.cell_gaps) {
            let x = left + index % grid.side() * cell;
            let y = TOP + index / grid.side() * cell;
            let t = f64::from(gap).ln() / f64::from(max_gap).ln();
            let color = gap_color(t);
            writeln!(svg, r#"<rect x="{x}" y="{y}" width="{cell}" height="{cell}" fill="{color}" shape-rendering="crispEdges"/>"#).unwrap();
        }
        label(
            &mut svg,
            left,
            TOP + PANEL + 20,
            &format!("Max gap {}", stats.max_gap),
        );
    }

    for step in 0..32 {
        let x = 44 + 5 * step;
        let color = gap_color(f64::from(step) / 31.0);
        writeln!(
            svg,
            r#"<rect x="{x}" y="372" width="5" height="12" fill="{color}"/>"#
        )
        .unwrap();
    }
    label(&mut svg, LEFT, 383, "1");
    label(
        &mut svg,
        214,
        383,
        &format!("{max_gap}  (shared logarithmic scale)"),
    );
    svg.push_str("</svg>\n");
    svg
}

fn gap_color(t: f64) -> String {
    let channel = |light: f64, dark: f64| (light + (dark - light) * t).round() as u8;
    let (r, g, b) = (
        channel(239.0, 49.0),
        channel(246.0, 46.0),
        channel(255.0, 129.0),
    );
    format!("#{r:02x}{g:02x}{b:02x}")
}
