# ttop - Terminal Top

**10X Better Than btop** - A pure Rust system monitor with GPU support, ML stack integration, and deterministic rendering.

## Installation

```bash
cargo install ttop
```

## Features

- **Pure Rust**: Zero C dependencies, cross-platform (Linux + macOS)
- **8ms Frame Time**: 2X faster than btop's 16ms target
- **GPU Monitoring**: NVIDIA (via NVML), AMD (via ROCm SMI), Apple Silicon
- **macOS Native**: Full support for Apple Silicon and Intel Macs
- **Deterministic Mode**: Reproducible rendering for testing
- **CIELAB Colors**: Perceptually uniform gradients
- **Sovereign AI Stack**: Integration with trueno, aprender, realizar

## Panels

| Panel | Key | Description |
|-------|-----|-------------|
| CPU | 1 | Per-core utilization with sparklines |
| Memory | 2 | RAM/Swap with usage graphs |
| Disk | 3 | Mount points and I/O rates |
| Network | 4 | RX/TX throughput per interface |
| Process | 5 | Sortable process table with tree view |
| GPU | 6 | NVIDIA/AMD utilization and memory |
| Battery | 7 | Charge level and time remaining |
| Sensors | 8 | Temperature readings |

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

### General
- `q`, `Esc` - Quit
- `?`, `F1` - Toggle help
- `0` - Reset all panels

## Command Line Options

```
ttop [OPTIONS]

Options:
  -r, --refresh <MS>     Refresh rate in milliseconds [default: 1000]
      --deterministic    Enable deterministic mode for testing
  -c, --config <PATH>    Config file path
      --show-fps         Show frame timing statistics
  -h, --help             Print help
  -V, --version          Print version
```

## Building from Source

```bash
# Clone the repository
git clone https://github.com/paiml/trueno-viz
cd trueno-viz

# Build ttop
cargo build -p ttop --release

# Run
./target/release/ttop
```

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `nvidia` | Yes | NVIDIA GPU monitoring via NVML |
| `tracing` | No | Syscall tracing via renacer |
| `full` | No | All features enabled |

## License

MIT OR Apache-2.0
