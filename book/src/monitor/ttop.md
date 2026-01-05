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

## Platform Support

| Platform | CPU | Memory | Disk | Network | Process | GPU |
|----------|-----|--------|------|---------|---------|-----|
| Linux | ✅ | ✅ | ✅ | ✅ | ✅ | NVIDIA/AMD |
| macOS Intel | ✅ | ✅ | ✅ | ✅ | ✅ | AMD Radeon |
| macOS Apple Silicon | ✅ | ✅ | ✅ | ✅ | ✅ | Apple GPU |

## Keyboard Shortcuts

### Panel Toggles

| Key | Panel |
|-----|-------|
| `1` | CPU |
| `2` | Memory |
| `3` | Disk |
| `4` | Network |
| `5` | GPU |
| `6` | Process |

### Navigation

| Key | Action |
|-----|--------|
| `j/k`, `↑/↓` | Move up/down |
| `PgUp/PgDn` | Page up/down |
| `g/G` | Go to top/bottom |

### Sorting & Filtering

| Key | Action |
|-----|--------|
| `s`, `Tab` | Cycle sort column |
| `r` | Reverse sort order |
| `f`, `/` | Filter processes |
| `Del` | Clear filter |
| `t` | Toggle tree view |

### General

| Key | Action |
|-----|--------|
| `q`, `Esc` | Quit |
| `?`, `F1` | Toggle help |
| `0` | Reset all panels |

## Command Line Options

```bash
ttop [OPTIONS]

Options:
  -r, --refresh <MS>     Refresh rate in milliseconds [default: 1000]
      --deterministic    Enable deterministic mode for testing
  -c, --config <PATH>    Config file path
      --show-fps         Show frame timing statistics
  -h, --help             Print help
  -V, --version          Print version
```

## macOS Collectors

ttop uses native macOS commands for metrics collection:

| Collector | Source |
|-----------|--------|
| CPU | `sysctl`, `top` |
| Memory | `vm_stat`, `sysctl` |
| Network | `netstat -ib` |
| Disk | `df`, `iostat` |
| Process | `ps -axo` |
| GPU | `ioreg`, `sysctl` |

## GPU Detection

### Apple Silicon

Automatically detects M1/M2/M3/M4 chips and their variants (Pro, Max, Ultra).

### AMD Radeon (Mac Pro)

Detects discrete AMD GPUs including:
- Radeon Pro W5700X (dual GPU configurations)
- Radeon Pro Vega II

### NVIDIA (Linux)

Uses NVML for NVIDIA GPU monitoring on Linux systems.

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `nvidia` | Yes | NVIDIA GPU monitoring via NVML |
| `tracing` | No | Syscall tracing via renacer |
| `full` | No | All features enabled |

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

## Integration with trueno-viz

ttop is built on the trueno-viz monitoring module, which provides:

- **Collectors**: CPU, Memory, Disk, Network, Process, GPU, Battery, Sensors
- **Widgets**: Graph, Meter, Sparkline with multiple render modes
- **Theme System**: CIELAB perceptual gradients
- **Ring Buffers**: Efficient metric history storage

```rust
use trueno_viz::monitor::collectors::{CpuCollector, MemoryCollector};
use trueno_viz::monitor::types::Collector;

let mut cpu = CpuCollector::new();
let metrics = cpu.collect()?;
println!("CPU usage: {:?}", metrics.get_gauge("cpu.total"));
```
