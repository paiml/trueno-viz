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
- **GPU Processes**: Live GPU process monitoring with nvidia-smi pmon
- **macOS Native**: Full support for Apple Silicon and Intel Macs
- **Deterministic Mode**: Reproducible rendering for testing
- **CIELAB Colors**: Perceptually uniform gradients

### Advanced Analyzers

- **PSI Pressure Monitoring**: Detect resource contention before OOM (Linux 4.20+)
- **Container/Docker Dashboard**: Live container CPU/memory stats
- **Network Connections**: Little Snitch-style connection tracking
- **Treemap Visualization**: Grand Perspective-style large file display
- **Disk I/O Analysis**: Per-disk sparklines with workload classification
- **Swap Thrashing Detection**: ZRAM stats and thrashing severity

## Platform Support

| Platform | CPU | Memory | Disk | Network | Process | GPU | PSI | Docker |
|----------|-----|--------|------|---------|---------|-----|-----|--------|
| Linux | ✅ | ✅ | ✅ | ✅ | ✅ | NVIDIA/AMD | ✅ | ✅ |
| macOS Intel | ✅ | ✅ | ✅ | ✅ | ✅ | AMD Radeon | ❌ | ✅ |
| macOS Apple Silicon | ✅ | ✅ | ✅ | ✅ | ✅ | Apple GPU | ❌ | ✅ |

## Layout

```
┌─────────────────────────────────────────────────────────────────────┐
│  CPU (per-core)  │  Memory (used/cached/free)  │  GPU + Processes  │
├──────────────────┼─────────────────────────────┼───────────────────┤
│  Disk I/O        │  Network (RX/TX)            │  System Health    │
│  (per-disk)      │                             │  • Sensors        │
│                  │                             │  • PSI Pressure   │
│                  │                             │  • Containers     │
├──────────────────┴─────────────────────────────┴───────────────────┤
│  Processes (40%)  │  Connections (30%)  │  File Treemap (30%)      │
└─────────────────────────────────────────────────────────────────────┘
```

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

## Advanced Panels

### PSI Pressure Monitoring

Shows Linux Pressure Stall Information for CPU, memory, and I/O:

```
CPU ○ 2.1%  MEM ○ 0.0%  I/O ◔ 1.5%
Full stall: CPU 0.0%  MEM 0.0%  I/O 1.2%
```

Pressure levels: ○ none → ◔ low → ◑ medium → ◕ high → ● critical

### Container Dashboard

Live Docker container stats:

```
▶  0.1%    2M duende-test
▶  5.2%  512M web-app
```

### Network Connections

Little Snitch-style connection tracking:

```
ESTAB TCP  nginx     → 192.168.1.100:443
ESTAB TCP  chrome    → 142.250.80.46:443
```

### File Treemap

Pareto-style visualization of large files (>50MB):
- Top 20%: Warm amber (vital few)
- Middle 30%: Muted gold
- Bottom 50%: Cool slate

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
GPU processes shown via `nvidia-smi pmon`:

```
───────────────────────
G  11%   3% Xorg
G   8%   2% gnome-shell
```

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

## Analyzers API

```rust
use ttop::analyzers::{PsiAnalyzer, ContainerAnalyzer, GpuProcessAnalyzer};

// PSI Pressure
let mut psi = PsiAnalyzer::new();
psi.collect();
println!("CPU pressure: {:.1}%", psi.cpu.some_avg10);

// Container stats
let mut containers = ContainerAnalyzer::new();
containers.collect();
for c in containers.top_containers(5) {
    println!("{}: {:.1}% CPU", c.name, c.cpu_pct);
}

// GPU processes
let mut gpu_procs = GpuProcessAnalyzer::new();
gpu_procs.collect();
for p in gpu_procs.top_processes(3) {
    println!("{}: {}% SM", p.command, p.sm_util);
}
```
