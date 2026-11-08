mod metrics;
mod svg;

use hilbert_curve_locality::{Grid, Order};
use metrics::{intervals, locality, mapping_times, queries};
use std::{env, path::Path, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args == ["--help"] || args == ["-h"] {
        println!("Usage: hilbert-curve-locality [SIDE] [OUTPUT_DIR]");
        println!("SIDE: 8, 16, 32 (default), 64, or 128");
        println!("OUTPUT_DIR: reports (default)");
        return Ok(());
    }
    if args.len() > 2 {
        return Err("Expected a grid size and optional output directory. See --help.".into());
    }
    let grid = args
        .first()
        .map_or(Grid::new(32), |value| {
            value.parse().ok().and_then(Grid::new)
        })
        .ok_or("Grid size must be 8, 16, 32, 64, or 128.")?;

    let queries = queries(grid);
    println!("{}×{} grid", grid.side(), grid.side());
    println!(
        "{:<10} {:>9} {:>8} {:>10} {:>7} {:>7} {:>7} {:>10} {:>10}",
        "Order",
        "Mean jump",
        "Max jump",
        "Median gap",
        "P95 gap",
        "Max gap",
        "Steps",
        "Enc ns/op",
        "Dec ns/op"
    );
    let stats = Order::ALL.map(|order| locality(grid, order));
    for (order, stats) in Order::ALL.into_iter().zip(&stats) {
        let (encode, decode) = mapping_times(grid, order);
        println!(
            "{:<10} {:>9.3} {:>8} {:>10.1} {:>7} {:>7} {:>7} {:>10.2} {:>10.2}",
            order.name(),
            stats.mean_jump,
            stats.max_jump,
            stats.median_gap,
            stats.p95_gap,
            stats.max_gap,
            grid.scale_steps(order),
            encode,
            decode
        );
    }

    println!("\nExact intervals per query");
    println!(
        "{:<16} {:>12} {:>8} {:>8}",
        "Query", "Row-major", "Morton", "Hilbert"
    );
    for (name, rect) in queries.iter().filter(|(name, _)| *name != "Seeded") {
        let [row, morton, hilbert] = Order::ALL.map(|order| intervals(grid, order, *rect).len());
        println!("{name:<16} {row:>12} {morton:>8} {hilbert:>8}");
    }
    let seeded: Vec<_> = queries
        .iter()
        .filter(|(name, _)| *name == "Seeded")
        .collect();
    let [row, morton, hilbert] = Order::ALL.map(|order| {
        let total: usize = seeded
            .iter()
            .map(|(_, rect)| intervals(grid, order, *rect).len())
            .sum();
        total as f64 / seeded.len() as f64
    });
    println!(
        "{:<16} {row:>12.2} {morton:>8.2} {hilbert:>8.2}",
        "Seeded mean"
    );
    let output = args
        .get(1)
        .map_or(Path::new("reports"), |path| Path::new(path));
    let query = queries
        .iter()
        .find(|(name, _)| *name == "Shifted square")
        .unwrap()
        .1;
    svg::write_reports(output, grid, query, &stats)
        .map_err(|error| format!("Could not write reports to {}: {error}", output.display()))?;
    println!(
        "\nWrote traversals.svg and heatmaps.svg to {}",
        output.display()
    );
    Ok(())
}
