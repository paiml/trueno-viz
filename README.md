# trueno-viz

SIMD/GPU/WASM-accelerated visualization for Data Science, Physics, and ML.

[![CI](https://github.com/paiml/trueno-viz/actions/workflows/ci.yml/badge.svg)](https://github.com/paiml/trueno-viz/actions)
[![Crates.io](https://img.shields.io/crates/v/trueno-viz.svg)](https://crates.io/crates/trueno-viz)

**Pure Rust** - zero JavaScript, zero browser dependencies.

## Installation

```toml
[dependencies]
trueno-viz = "0.1"

# Optional: GPU acceleration
trueno-viz = { version = "0.1", features = ["gpu"] }

# Optional: System monitoring with SIMD collectors
trueno-viz = { version = "0.1", features = ["monitor"] }
```

## Quick Start

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

### Core Visualization
- **SIMD Framebuffer**: 64-byte aligned for AVX-512
- **GPU Compute**: CUDA/Vulkan/Metal and WebGPU
- **Plot Types**: Scatter, Line, Heatmap, Histogram, Box, Violin, Confusion Matrix, ROC/PR
- **Output**: PNG, SVG, Terminal (ASCII/Unicode/ANSI 24-bit)
- **ML Integration**: Loss curves, metrics visualization

### SIMD Collectors (v0.1.14)
Real platform intrinsics for system monitoring:
- **SSE2/AVX2** (x86_64): `std::arch::x86_64` intrinsics
- **NEON** (aarch64): `std::arch::aarch64` intrinsics
- **5.6x measured speedup** for byte scanning operations
- **Three-tier storage**: Hot (SimdRingBuffer) → Warm (Compressed) → Cold (Disk)

## Examples

```bash
# Visualization examples
cargo run --example scatter_basic
cargo run --example heatmap_correlation
cargo run --example loss_training
cargo run --example confusion_matrix_ml
cargo run --example roc_pr_curves
cargo run --example grammar_of_graphics
cargo run --example terminal_output
cargo run --example svg_output

# All examples
cargo run --example readme_demo
cargo run --example box_violin
cargo run --example force_graph
cargo run --example dashboard_widgets
cargo run --example text_prompt
```

## ttop - System Monitor

A pure Rust system monitor built on trueno-viz SIMD collectors:

```bash
# Install from crates.io
cargo install ttop

# Or build from source
cd crates/ttop && cargo build --release
```

## Architecture

```
trueno-viz/
├── src/
│   ├── framebuffer.rs      # SIMD-aligned pixel buffer
│   ├── plots/              # Scatter, Line, Heatmap, etc.
│   ├── grammar/            # Grammar of Graphics (ggplot-style)
│   ├── output/             # PNG, SVG, Terminal encoders
│   └── monitor/            # System monitoring (feature: monitor)
│       └── simd/           # Real SIMD kernels (SSE2/AVX2/NEON)
│           ├── kernels.rs      # Platform intrinsics
│           ├── ring_buffer.rs  # O(1) statistics
│           ├── timeseries.rs   # Three-tier storage
│           └── correlation.rs  # Pearson correlation
└── crates/
    └── ttop/               # System monitor binary
```

## Performance

| Operation | Speedup | Implementation |
|-----------|---------|----------------|
| Byte scanning | 5.6x | SSE2 `_mm_cmpeq_epi8` |
| Delta calculation | 2-3x | AVX2 `_mm256_sub_epi64` |
| Statistics | 2-4x | AVX2 reductions |
| Ring buffer stats | O(1) | Running aggregates |

## License

MIT OR Apache-2.0
