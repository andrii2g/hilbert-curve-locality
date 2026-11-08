# Hilbert Curve Locality

A small Rust experiment comparing row-major, Morton (Z-order), and Hilbert
indexing on square grids. All three give every cell a unique integer index.
Changing the order changes which cells sit together in a linear array.

## Run

```sh
cargo run --release
cargo run --release -- 128 reports/128
cargo test
```

The optional positional arguments are the grid side (8, 16, 32, 64, or 128)
and the output directory. They default to 32 and `reports`. The project
uses only the Rust standard library.

The CLI prints the metrics and writes two SVGs. Open them in a browser.

- `traversals.svg` shows all three paths and the same shifted square. Colored
  segments show runs of consecutive indices in the query. A dot marks the
  start of each run, including single-cell runs.
- `heatmaps.svg` colors each cell by its largest index gap to an edge-sharing
  neighbor. All panels use the same logarithmic scale.

## How to read the results

- **Jump distance** is the Manhattan distance between consecutive cells in
  index order. Hilbert always takes a single edge step.
- **Neighbor index gap** is the absolute index difference for each horizontal
  and vertical grid edge. The table reports the median, nearest-rank 95th
  percentile, and maximum. A single edge step along the Hilbert path can
  coexist with large index gaps across other grid edges.
- **Query intervals** count maximal contiguous runs containing exactly the
  selected cells. The CLI reports five fixed shapes and the mean across 32
  reproducibly seeded rectangles. Internally, intervals are half-open.
- **Mapping steps** count bit pairs processed by Morton and quadrant levels
  processed by Hilbert. Row-major has no per-level loop.
- **Runtime** is elapsed nanoseconds per conversion over 1,048,576 calls
  per direction and ordering. Each ordering receives the same permuted input
  sequence. Treat the reported times as machine-specific estimates that
  include loop overhead. Use release mode for comparisons.

At the default size, the aligned square takes 8, 1, and 1 intervals in
row-major, Morton, and Hilbert order. Shifting it one cell up and left changes
the counts to 8, 22, and 12.

The library exposes checked coordinate-to-index and index-to-coordinate
mappings. Tests cover both round trips at all five sizes, Hilbert edge
adjacency, and exact query coverage.
