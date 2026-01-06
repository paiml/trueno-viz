# trueno-viz

<p align="center">
  <img src="docs/hero.svg" alt="trueno-viz" width="800">
</p>

SIMD/GPU/WASM-accelerated visualization for Data Science, Physics, and ML.

[![CI](https://github.com/paiml/trueno-viz/actions/workflows/ci.yml/badge.svg)](https://github.com/paiml/trueno-viz/actions)
[![Crates.io](https://img.shields.io/crates/v/trueno-viz.svg)](https://crates.io/crates/trueno-viz)

**Pure Rust** - zero JavaScript, zero browser dependencies.

## ttop - Terminal System Monitor

**10X Better Than btop** - Install the standalone system monitor:

```bash
# Standard install
cargo install ttop

# With Apple hardware acceleration (macOS)
cargo install ttop --features apple-hardware
```

Features:
- **GPU Monitoring**: NVIDIA/AMD/Apple Silicon
- **Apple Accelerators**: Neural Engine, Afterburner FPGA, Secure Enclave via [manzana](https://crates.io/crates/manzana)
- **8ms Frame Time**: 2X faster than btop
- **Cross-Platform**: Linux + macOS (Intel & Apple Silicon)

See [crates/ttop](crates/ttop) for full documentation.

## Installation

```toml
[dependencies]
trueno-viz = "0.1"

# Optional: GPU acceleration
trueno-viz = { version = "0.1", features = ["gpu"] }

# Optional: Apple hardware (Neural Engine, Afterburner, Secure Enclave)
trueno-viz = { version = "0.1", features = ["apple-hardware"] }
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
