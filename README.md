# trueno-viz

SIMD/GPU/WASM-accelerated visualization for Data Science, Physics, and ML.

[![CI](https://github.com/paiml/trueno-viz/actions/workflows/ci.yml/badge.svg)](https://github.com/paiml/trueno-viz/actions)

**Pure Rust** - zero JavaScript, zero browser dependencies.

## Installation

```toml
[dependencies]
trueno-viz = "0.1"

# Optional: GPU acceleration
trueno-viz = { version = "0.1", features = ["gpu"] }
```

## Usage

```rust
use trueno_viz::prelude::*;
use trueno_viz::output::{TerminalEncoder, TerminalMode};

let plot = ScatterPlot::new()
    .x(&[1.0, 2.0, 3.0, 4.0])
    .y(&[1.0, 4.0, 2.0, 8.0])
    .build()?;

let fb = plot.to_framebuffer()?;
TerminalEncoder::new()
    .mode(TerminalMode::Ascii)
    .print(&fb);
```

## Features

- **SIMD Framebuffer**: 64-byte aligned for AVX-512
- **GPU Compute**: CUDA/Vulkan/Metal and WebGPU
- **Plot Types**: Scatter, Line, Heatmap, Histogram, Box, Violin, Confusion Matrix, ROC/PR
- **Output**: PNG, Terminal (ASCII/Unicode/ANSI 24-bit)
- **ML Integration**: Loss curves, metrics visualization

## Examples

```bash
cargo run --example scatter_basic
cargo run --example heatmap_correlation
cargo run --example loss_training
cargo run --example confusion_matrix_ml
cargo run --example roc_pr_curves
```

## License

MIT OR Apache-2.0
