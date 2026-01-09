# ttop - Terminal Top

**10X Better Than btop** - A pure Rust system monitor with GPU support, file analytics, and deterministic rendering.

[![Crates.io](https://img.shields.io/crates/v/ttop.svg)](https://crates.io/crates/ttop)
[![License](https://img.shields.io/crates/l/ttop.svg)](LICENSE)

## Installation

```bash
cargo install ttop
```

## Features

- **Pure Rust**: Zero C dependencies, cross-platform (Linux + macOS)
- **8ms Frame Time**: 2X faster than btop's 16ms target
- **GPU Monitoring**: NVIDIA (via NVML), AMD (via ROCm SMI), Apple Silicon
- **macOS Native**: Full support for Apple Silicon and Intel Macs
- **File Analytics**: Large file detection, duplicates, entropy analysis
- **Deterministic Mode**: Reproducible rendering for testing
- **CIELAB Colors**: Perceptually uniform gradients

## Panels

| Panel | Key | Description |
|-------|-----|-------------|
| CPU | 1 | Per-core utilization with sparklines |
| Memory | 2 | RAM/Swap with usage graphs |
| Disk | 3 | Mount points, I/O rates, entropy |
| Network | 4 | RX/TX throughput per interface |
| Process | 5 | Sortable process table with tree view |
| GPU | 6 | NVIDIA/AMD/Apple utilization and memory |
| Battery | 7 | Charge level and time remaining |
| Sensors | 8 | Temperature readings with health status |
| Files | 9 | Large files, duplicates, I/O activity |

## Keyboard Shortcuts

### Navigation
- `j/k`, `↑/↓` - Move up/down
- `PgUp/PgDn` - Page up/down
- `g/G` - Go to top/bottom

### Sorting & Filtering
- `s`, `Tab` - Cycle sort column
- `r` - Reverse sort order
- `f`, `/` - Filter processes
- `Del` - Clear filter
- `t` - Toggle tree view

### Panels
- `1-9` - Toggle individual panels
- `0` - Reset all panels
- `Space` - Expand/collapse panel

### General
- `q`, `Esc` - Quit
- `?`, `F1` - Toggle help

## Command Line Options

```
ttop [OPTIONS]

Options:
  -r, --refresh <MS>     Refresh rate in milliseconds [default: 1000]
      --deterministic    Enable deterministic mode for testing
  -c, --config <PATH>    Config file path
      --show-fps         Show frame timing statistics
      --debug            Enable debug logging
  -h, --help             Print help
  -V, --version          Print version
```

## Examples

```bash
# Run with default settings
ttop

# Fast refresh (500ms)
ttop -r 500

# Show frame timing
ttop --show-fps

# Debug mode (logs to stderr)
ttop --debug 2>ttop.log
```

### Programmatic Usage

```rust
use ttop::app::App;

fn main() {
    let mut app = App::new(false, false);
    app.collect_metrics();

    println!("CPU: {} cores", app.cpu.core_count());
    println!("Memory: {:.1} GB", app.mem_total as f64 / 1e9);
}
```

See `examples/` for more:
```bash
cargo run --example simple
cargo run --example collectors
```

## Building from Source

```bash
git clone https://github.com/paiml/trueno-viz
cd trueno-viz/crates/ttop
cargo build --release
./target/release/ttop
```

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `nvidia` | Yes | NVIDIA GPU monitoring via NVML |
| `apple-hardware` | No | Apple Neural Engine, Metal stats |
| `tracing` | No | Syscall tracing via renacer |
| `full` | No | All features enabled |

## License

MIT OR Apache-2.0
