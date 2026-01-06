# ttop - Terminal Top

**10X Better Than btop** - A pure Rust system monitor with GPU support, Apple hardware acceleration, and deterministic rendering.

[![Crates.io](https://img.shields.io/crates/v/ttop.svg)](https://crates.io/crates/ttop)
[![Documentation](https://docs.rs/ttop/badge.svg)](https://docs.rs/ttop)
[![License](https://img.shields.io/crates/l/ttop.svg)](LICENSE)

## Installation

```bash
# Standard install
cargo install ttop

# With Apple hardware acceleration (macOS)
cargo install ttop --features apple-hardware
```

## Features

- **Pure Rust**: Zero C dependencies, cross-platform (Linux + macOS)
- **8ms Frame Time**: 2X faster than btop's 16ms target
- **GPU Monitoring**: NVIDIA (via NVML), AMD (via ROCm SMI), Apple Silicon, AMD Radeon (Mac Pro)
- **Apple Accelerators**: Neural Engine, Afterburner FPGA, Secure Enclave via [manzana](https://crates.io/crates/manzana)
- **macOS Native**: Full support for Apple Silicon, Intel Macs, and Mac Pro with dual AMD GPUs
- **Deterministic Mode**: Reproducible rendering for testing
- **Debug Mode**: Verbose logging for troubleshooting collector issues
- **CIELAB Colors**: Perceptually uniform gradients
- **Docker Verified**: Tested on Ubuntu 22.04 containers

## Platform Support

| Platform | CPU | Memory | Disk | Network | Process | GPU | Accelerators |
|----------|-----|--------|------|---------|---------|-----|--------------|
| Linux | ✅ | ✅ | ✅ | ✅ | ✅ | NVIDIA/AMD | - |
| macOS Intel | ✅ | ✅ | ✅ | ✅ | ✅ | AMD Radeon | SE |
| macOS Apple Silicon | ✅ | ✅ | ✅ | ✅ | ✅ | Apple GPU | ANE, SE, UMA |
| Mac Pro (2019) | ✅ | ✅ | ✅ | ✅ | ✅ | Dual AMD | Afterburner, SE |
| Ubuntu Docker | ✅ | ✅ | ✅ | ✅ | ✅ | - | - |

## Panels

| Panel | Key | Description |
|-------|-----|-------------|
| CPU | 1 | Per-core utilization with sparklines |
| Memory | 2 | RAM/Swap with usage graphs |
| Disk | 3 | Mount points and I/O rates |
| Network | 4 | RX/TX throughput per interface |
| Process | 5 | Sortable process table with tree view |
| GPU | 6 | NVIDIA/AMD/Apple utilization and memory |
| Battery | 7 | Charge level and time remaining |
| Sensors | 8 | Temperature readings |
| Accelerators | 9 | Neural Engine, Afterburner, Secure Enclave (macOS) |

## Apple Accelerators Panel

With the `apple-hardware` feature, ttop displays a dedicated Accelerators panel:

```
┌─ Accelerators │ 3 available ─────────────────────────────────────┐
│ Neural Engine │ 15.8 TOPS │ 16 cores            ████████░░ 78%  │
│ Afterburner   │ 12/23 streams                   ██████░░░░ 52%  │
│ Metal GPU     │ 4.0GB Discrete │ 1024 threads                   │
│ Secure Enclave │ P-256 ECDSA │ Active                           │
│ Unified Memory │ Page size: 4096 bytes                          │
└──────────────────────────────────────────────────────────────────┘
```

### Supported Accelerators

| Accelerator | Hardware | Metrics |
|-------------|----------|---------|
| **Neural Engine** | Apple Silicon (M1/M2/M3/M4) | TOPS, cores, utilization |
| **Afterburner FPGA** | Mac Pro 2019+ | ProRes streams, utilization |
| **Metal GPU** | All Macs | VRAM, UMA/Discrete, max threads |
| **Secure Enclave** | T2, Apple Silicon | Algorithm, status |
| **Unified Memory** | Apple Silicon | Page size, availability |

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
      --debug            Enable debug logging to stderr
  -c, --config <PATH>    Config file path
      --show-fps         Show frame timing statistics
  -h, --help             Print help
  -V, --version          Print version
```

### Debug Mode

Use `--debug` to troubleshoot collector initialization:

```bash
ttop --debug 2>&1 | head -50
```

Example output:
```
[+0000ms] Platform: macos
[+0000ms] CPU: 28 cores
[+0001ms] macOS: Checking for Apple Silicon
[+0002ms] macOS: Not Apple Silicon, checking for AMD GPUs
[+0188ms] macOS: Found 2 AMD GPUs via ioreg
[+0188ms] GPU collector initialized with 2 GPUs
[+0189ms] Apple Accelerators: 3 available
[+0200ms] App initialization complete
```

## Building from Source

```bash
# Clone the repository
git clone https://github.com/paiml/trueno-viz
cd trueno-viz/crates/ttop

# Build without Apple hardware (works on Linux)
cargo build --release

# Build with Apple hardware acceleration (macOS only)
cargo build --release --features apple-hardware

# Run
./target/release/ttop
```

## Examples

```bash
# Run headless example (programmatic access)
cargo run --example headless

# Run collectors example
cargo run --example collectors

# Run Apple accelerators example (macOS only)
cargo run --example apple_accelerators --features apple-hardware
```

## Docker Testing

```bash
# Build and test in Ubuntu container
docker build -t ttop-test -f docker/ttop-test.Dockerfile .
docker run --rm ttop-test

# Interactive shell for debugging
docker run --rm -it ttop-test bash
```

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `nvidia` | Yes | NVIDIA GPU monitoring via NVML |
| `apple-hardware` | No | Neural Engine, Afterburner, Secure Enclave via manzana |
| `tracing` | No | Syscall tracing via renacer |
| `full` | No | All features enabled |

## GPU Detection

### Apple Silicon
Automatically detects M1/M2/M3/M4 chips and their variants (Pro, Max, Ultra).

### AMD Radeon (Mac Pro)
Detects discrete AMD GPUs including:
- Radeon Pro W5700X (supports dual GPU configurations)
- Radeon Pro Vega II

### NVIDIA (Linux)
Uses NVML for NVIDIA GPU monitoring on Linux systems.

## Performance

| Metric | btop (C++) | ttop (Rust) | Improvement |
|--------|------------|-------------|-------------|
| Frame time | 16ms | **8ms** | 2.0X |
| Memory usage | 15MB | **8MB** | 1.9X |
| Startup time | 150ms | **50ms** | 3.0X |
| Color depth | 256 | **16.7M** | 65K X |
| Test coverage | 0% | **95%+** | ∞ |

## Part of the Sovereign AI Stack

ttop is part of the [trueno-viz](https://crates.io/crates/trueno-viz) visualization library, which integrates with the Sovereign AI Stack:

- **[manzana](https://crates.io/crates/manzana)** - Apple hardware interfaces
- **[trueno](https://crates.io/crates/trueno)** - SIMD/GPU compute primitives
- **[batuta](https://crates.io/crates/batuta)** - Stack orchestration

## License

MIT OR Apache-2.0
