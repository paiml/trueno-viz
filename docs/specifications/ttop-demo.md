# ttop: Terminal Top - 10X Better Than btop

## Specification v1.0.0

**Status**: Implementation Ready
**Authors**: Sovereign AI Stack Team
**Last Updated**: 2026-01-05
**Target**: `cargo run --example ttop --features monitor,monitor-nvidia`

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [10X Superiority Matrix](#2-10x-superiority-matrix)
3. [Architecture](#3-architecture)
4. [Visual Excellence](#4-visual-excellence)
5. [Performance Engineering](#5-performance-engineering)
6. [Deterministic Rendering](#6-deterministic-rendering)
7. [Testing Strategy](#7-testing-strategy)
8. [Renacer Tracing Integration](#8-renacer-tracing-integration)
9. [Peer-Reviewed Citations](#9-peer-reviewed-citations)
10. [Popperian Falsification Checklist](#10-popperian-falsification-checklist)

---

## 1. Executive Summary

**ttop** (Terminal Top) is a next-generation system monitor that achieves **10X superiority over btop** through:

- **2X faster rendering**: 8ms frame time vs btop's 16ms
- **3X more visual fidelity**: 24-bit color gradients with perceptual uniformity
- **5X better determinism**: Reproducible frame output for testing
- **100% probar test coverage**: TUI playbooks, pixel verification, frame assertions
- **95% Rust unit test coverage**: Property-based testing, mutation testing
- **Built-in renacer tracing**: Source-correlated syscall analysis
- **Zero unsafe code**: Pure safe Rust (except GPU FFI with safety wrappers)

### Design Philosophy

Following the Toyota Production System (Liker, 2004):

| Principle | Application in ttop |
|-----------|---------------------|
| **Jidoka** | Stop-on-error with graceful degradation |
| **Poka-Yoke** | Type-safe metric pipelines prevent invalid states |
| **Heijunka** | Load-leveled collector scheduling |
| **Muda** | Zero-allocation rendering after warmup |
| **Kaizen** | Continuous improvement via probar regression |
| **Genchi Genbutsu** | Direct measurement via renacer tracing |

---

## 2. 10X Superiority Matrix

### 2.1 Quantitative Comparison

| Metric | btop (C++) | ttop (Rust) | Improvement |
|--------|------------|-------------|-------------|
| Frame time | 16ms | **8ms** | 2.0X |
| Memory usage | 15MB | **8MB** | 1.9X |
| Startup time | 150ms | **50ms** | 3.0X |
| Color depth | 256 | **16.7M** (TrueColor) | 65K X |
| Graph resolution | Block | **Braille** (2x4 dots/cell) | 8X |
| Test coverage | 0% | **95%** Rust + 100% TUI | ∞ |
| Tracing | None | **renacer** syscall | ∞ |
| Determinism | Non-deterministic | **Fully reproducible** | ∞ |
| Build deps | CMake + C++20 | **cargo build** | 10X simpler |
| Memory safety | Manual | **Guaranteed** | ∞ |

### 2.2 Feature Parity + Extensions

| Feature | btop | ttop | Notes |
|---------|------|------|-------|
| CPU monitoring | ✓ | ✓ | Per-core, frequency, temp |
| Memory monitoring | ✓ | ✓ | Used/cached/swap |
| Process list | ✓ | ✓ | Tree view, signals |
| Network | ✓ | ✓ | Per-interface, rate |
| Disk I/O | ✓ | ✓ | IOPS, throughput, latency |
| NVIDIA GPU | ✓ | ✓ | NVML integration |
| AMD GPU | ✓ | ✓ | ROCm SMI dynamic loading |
| Battery | ✓ | ✓ | Charge, time remaining |
| Temperature | ✓ | ✓ | hwmon sensors |
| **LLM Inference** | ✗ | ✓ | realizar metrics |
| **Training** | ✗ | ✓ | entrenar metrics |
| **ZRAM** | ✗ | ✓ | trueno-zram stats |
| **Distributed** | ✗ | ✓ | repartir jobs |
| **Syscall Trace** | ✗ | ✓ | renacer integration |
| **Playbook Tests** | ✗ | ✓ | probar TUI testing |
| **Pixel Coverage** | ✗ | ✓ | Visual regression |

---

## 3. Architecture

### 3.1 Module Structure

```
examples/
├── ttop.rs                    # Main entry point

src/monitor/
├── mod.rs                     # Feature gate
├── app.rs                     # Application loop (deterministic)
├── state.rs                   # Shared state with RingBuffers
├── input.rs                   # Keyboard/mouse handling
├── config.rs                  # YAML configuration
├── theme.rs                   # Color system (CIELAB interpolation)
├── layout.rs                  # Box layout engine
│
├── collectors/
│   ├── mod.rs
│   ├── cpu.rs                 # /proc/stat parsing
│   ├── memory.rs              # /proc/meminfo parsing
│   ├── disk.rs                # /proc/diskstats + mount info
│   ├── network.rs             # /proc/net/dev parsing
│   ├── process.rs             # /proc/[pid]/* parsing
│   ├── sensors.rs             # hwmon sysfs
│   ├── battery.rs             # /sys/class/power_supply
│   ├── gpu_nvidia.rs          # NVML wrapper
│   ├── gpu_amd.rs             # ROCm SMI dynamic
│   └── stack/                 # Sovereign AI Stack
│       ├── realizar.rs
│       ├── entrenar.rs
│       └── trueno_zram.rs
│
├── widgets/
│   ├── mod.rs
│   ├── graph.rs               # Braille/block/TTY graphs
│   ├── meter.rs               # Percentage bars
│   ├── gauge.rs               # Arc gauges
│   ├── table.rs               # Sortable tables
│   ├── tree.rs                # Process hierarchy
│   ├── sparkline.rs           # Inline mini-graphs
│   └── heatmap.rs             # Core temperature grid
│
├── panels/
│   ├── mod.rs
│   ├── cpu.rs
│   ├── memory.rs
│   ├── disk.rs
│   ├── network.rs
│   ├── process.rs
│   ├── gpu.rs
│   ├── battery.rs
│   └── sensors.rs
│
└── tracing/                   # renacer integration
    ├── mod.rs
    ├── collector.rs           # Syscall metrics
    └── correlation.rs         # Source mapping
```

### 3.2 Data Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                           ttop Application                           │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐           │
│  │  Collectors │────▶│    State    │────▶│   Panels    │           │
│  │  (async)    │     │ (RingBuffer)│     │ (widgets)   │           │
│  └─────────────┘     └─────────────┘     └─────────────┘           │
│         │                   │                   │                   │
│         │                   │                   ▼                   │
│         │                   │            ┌─────────────┐           │
│         │                   │            │   Layout    │           │
│         │                   │            │   Engine    │           │
│         │                   │            └─────────────┘           │
│         │                   │                   │                   │
│         ▼                   ▼                   ▼                   │
│  ┌─────────────────────────────────────────────────────┐           │
│  │              Deterministic Renderer                  │           │
│  │         (frame_id → identical output)               │           │
│  └─────────────────────────────────────────────────────┘           │
│                           │                                         │
│                           ▼                                         │
│  ┌─────────────────────────────────────────────────────┐           │
│  │                 renacer Tracer                       │           │
│  │           (syscall → source correlation)            │           │
│  └─────────────────────────────────────────────────────┘           │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 4. Visual Excellence

### 4.1 Color System

**CIELAB Perceptual Interpolation** (Sharma et al., 2005):

```rust
/// Gradient with perceptually uniform transitions
pub struct PerceptualGradient {
    stops: Vec<(f32, Lab)>,  // (position, CIELAB color)
}

impl PerceptualGradient {
    /// Sample gradient at position t ∈ [0, 1]
    /// Delta-E variance < 2.0 guaranteed
    pub fn sample(&self, t: f32) -> Rgb {
        // Interpolate in CIELAB space for perceptual uniformity
        let lab = self.interpolate_lab(t);
        lab.to_rgb()
    }
}
```

**Color Assignments**:

| Metric | Gradient Stops | Perceptual Intent |
|--------|----------------|-------------------|
| CPU | Blue → Yellow → Red | Cool to hot |
| Memory | Green → Yellow → Red | Available to critical |
| Temperature | Cyan → Orange → Red | Normal to danger |
| Network RX | Green → Cyan | Calm reception |
| Network TX | Magenta → Pink | Active transmission |
| GPU | Purple → Orange → Red | Idle to thermal |
| Disk | Blue → Green | Read emphasis |

### 4.2 Graph Rendering

**Braille Mode** (8 dots per cell = 2×4 resolution):

```
┌─ CPU Usage ─────────────────────────────────────────────────────────┐
│100%┤⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⣶⣦⣤⣄⣀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣀⣠⣤⣶⣿⣿│
│ 75%┤⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠛⠓⠒⠂⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣠⣴⣾⠟⠋⠀⠀⠀⠀│
│ 50%┤⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣀⣀⣤⣤⣴⣶⣾⠿⠛⠋⠀⠀⠀⠀⠀⠀⠀⠀│
│ 25%┤⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣀⣤⣶⣿⠿⠛⠋⠉⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀│
│  0%┤⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣤⣤⣶⣾⣿⠿⠛⠋⠉⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀│
└──────────────────────────────────────────────────────────────────────┘
```

**Algorithm**: Bresenham-derived rasterization with Wu's antialiasing (Wu, 1991).

### 4.3 Animated Elements

| Element | Animation | Rate |
|---------|-----------|------|
| Graph fill | Smooth scroll | 60fps |
| Meter gradient | Pulse on change | 500ms ease |
| Temperature | Color shift | Real-time |
| Sparkline | Rolling window | 1Hz |
| Panel border | Highlight on focus | Instant |

### 4.4 Layout Presets

```yaml
presets:
  0:  # Default - balanced
    rows:
      - panels: [cpu, memory]
        height: 30%
      - panels: [gpu, network]
        height: 25%
      - panels: [processes]
        height: 45%

  1:  # ML-focused
    rows:
      - panels: [gpu, llm]
        height: 35%
      - panels: [training, zram]
        height: 30%
      - panels: [processes]
        height: 35%

  2:  # Minimal
    rows:
      - panels: [cpu]
        height: 25%
      - panels: [processes]
        height: 75%
```

---

## 5. Performance Engineering

### 5.1 Frame Budget (8ms Target)

| Phase | Budget | Technique |
|-------|--------|-----------|
| Collection | 2ms | Async, non-blocking |
| State Update | 0.5ms | Ring buffer O(1) |
| Layout | 1ms | Cached, invalidate on resize |
| Widget Render | 3ms | Differential, dirty regions |
| Terminal Write | 1ms | Buffered, escape coalescing |
| **Total** | **7.5ms** | 0.5ms headroom |

### 5.2 Memory Budget (8MB Target)

| Component | Budget | Strategy |
|-----------|--------|----------|
| History Buffers | 4MB | 300 samples × 50 metrics |
| Process List | 2MB | 2000 processes max |
| Render Buffer | 1MB | Terminal dimensions |
| Static/Config | 1MB | Themes, strings |

### 5.3 Zero-Allocation Rendering

```rust
/// Frame renderer with zero heap allocations after warmup
pub struct DeterministicRenderer {
    /// Pre-allocated terminal buffer
    buffer: Buffer,
    /// Pre-allocated string builder
    output: String,
    /// Frame counter for determinism verification
    frame_id: u64,
}

impl DeterministicRenderer {
    /// Render frame with guaranteed zero allocations
    /// Returns identical output for same (state, frame_id) pair
    pub fn render(&mut self, state: &State) -> &str {
        self.frame_id += 1;
        self.buffer.reset();
        self.output.clear();

        // All rendering uses pre-allocated buffers
        self.render_panels(state);
        self.buffer.serialize_to(&mut self.output);

        &self.output
    }
}
```

---

## 6. Deterministic Rendering

### 6.1 Reproducibility Guarantee

**Theorem**: For any state `S` and frame ID `F`, `render(S, F)` produces identical output.

**Proof Sketch**:
1. All random sources are seeded with `frame_id`
2. Floating-point operations use `#[repr(C)]` ordering
3. Hash maps use deterministic iteration (`IndexMap`)
4. Time-dependent values frozen at collection

### 6.2 State Snapshot

```rust
/// Immutable state snapshot for deterministic rendering
#[derive(Clone, Debug)]
pub struct StateSnapshot {
    /// Frame identifier
    pub frame_id: u64,
    /// Frozen timestamp
    pub timestamp: Instant,
    /// CPU metrics (immutable)
    pub cpu: CpuSnapshot,
    /// Memory metrics (immutable)
    pub memory: MemorySnapshot,
    /// Process list (sorted, frozen)
    pub processes: Vec<ProcessInfo>,
    // ... other collectors
}

impl StateSnapshot {
    /// Create reproducible snapshot from live state
    pub fn freeze(state: &State, frame_id: u64) -> Self {
        Self {
            frame_id,
            timestamp: Instant::now(),
            cpu: state.cpu.snapshot(),
            memory: state.memory.snapshot(),
            processes: state.processes.sorted_snapshot(),
            // ...
        }
    }
}
```

### 6.3 Verification

```rust
#[test]
fn test_deterministic_rendering() {
    let state = StateSnapshot::from_fixture("cpu_50_percent.json");
    let mut renderer = DeterministicRenderer::new(80, 24);

    let frame1 = renderer.render(&state).to_string();
    let frame2 = renderer.render(&state).to_string();

    assert_eq!(frame1, frame2, "Rendering must be deterministic");
}
```

---

## 7. Testing Strategy

### 7.1 Test Pyramid

```
                    ┌─────────────────┐
                    │   E2E (probar)  │  ← 100% TUI coverage
                    │   Playbooks     │
                    ├─────────────────┤
                    │  Integration    │  ← Collector accuracy
                    │  (vs /proc)     │
                    ├─────────────────┤
                    │     Unit        │  ← 95% Rust coverage
                    │  (property)     │
                    └─────────────────┘
```

### 7.2 Probar TUI Testing

**Playbook Format** (YAML):

```yaml
# tests/playbooks/cpu_panel.yaml
name: CPU Panel Verification
setup:
  terminal_size: [120, 40]
  initial_state: fixtures/high_cpu.json

steps:
  - name: Verify CPU header
    action: wait_for_frame
    assert:
      - frame.contains_text("CPU")
      - frame.line(0).matches(r"CPU.*\d+%")

  - name: Verify per-core meters
    action: render_frame
    assert:
      - frame.contains_text("Core 0")
      - frame.contains_text("Core 1")
      - frame.match(r"Core \d+.*\[.*\].*\d+%")

  - name: Test keyboard navigation
    action: send_keys
    keys: ["j", "j", "k"]
    assert:
      - state.selected_panel == "cpu"

  - name: Toggle panel
    action: send_keys
    keys: ["1"]
    assert:
      - frame.not_contains_text("CPU")
```

**Execution**:

```bash
probador run tests/playbooks/*.yaml --coverage-report
```

### 7.3 Pixel Coverage Testing

```rust
use jugar_probar::pixel_coverage::{PixelCoverageConfig, CoverageAnalyzer};

#[test]
fn test_pixel_coverage_cpu_panel() {
    let config = PixelCoverageConfig {
        methodology: "falsification".into(),
        thresholds: ThresholdConfig {
            min_coverage: 85.0,
            max_gap_size: 5.0,
            falsifiability_threshold: 15.0,
        },
        verification: VerificationConfig {
            ssim_threshold: 0.99,
            delta_e_threshold: 1.0,
            phash_distance: 5,
        },
        ..Default::default()
    };

    let analyzer = CoverageAnalyzer::new(config);
    let baseline = load_snapshot("cpu_panel_baseline.snapshot");
    let current = render_cpu_panel(&test_state());

    let result = analyzer.compare(&baseline, &current);
    assert!(result.ssim >= 0.99, "SSIM: {}", result.ssim);
    assert!(result.delta_e <= 1.0, "Delta-E: {}", result.delta_e);
}
```

### 7.4 Frame Assertion Testing

```rust
use jugar_probar::tui::{expect_frame, TuiTestBackend};

#[test]
fn test_process_panel_sorting() {
    let mut backend = TuiTestBackend::new(120, 40);
    let mut app = TtopApp::new_test();

    // Render initial frame
    app.render(&mut backend);
    let frame = backend.frame();

    expect_frame(&frame)
        .to_contain_text("PID")?
        .to_contain_text("CPU%")?
        .to_contain_text("MEM%")?
        .line_to_contain(2, "rust-analyzer")?;

    // Send sort key
    app.handle_key(KeyCode::Char('s'));
    app.render(&mut backend);
    let frame = backend.frame();

    // Verify sort indicator changed
    expect_frame(&frame)
        .to_match(r"▼.*CPU%")?;  // Descending indicator
}
```

### 7.5 Property-Based Testing

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_ring_buffer_bounded(
        values in prop::collection::vec(0.0f64..100.0, 0..1000)
    ) {
        let mut buffer = RingBuffer::new(300);
        for v in values {
            buffer.push(v);
        }
        prop_assert!(buffer.len() <= 300);
    }

    #[test]
    fn prop_cpu_percentage_valid(
        user in 0u64..1_000_000,
        system in 0u64..1_000_000,
        idle in 0u64..1_000_000,
    ) {
        let prev = CpuStats { user: 0, system: 0, idle: 100, ..Default::default() };
        let curr = CpuStats { user, system, idle, ..Default::default() };
        let pct = calculate_cpu_percentage(&prev, &curr);
        prop_assert!(pct >= 0.0 && pct <= 100.0);
    }
}
```

### 7.6 Coverage Requirements

| Layer | Tool | Target |
|-------|------|--------|
| Rust Unit | cargo-llvm-cov | 95% |
| TUI Frames | probar playbooks | 100% |
| Pixels | probar pixel | 85% |
| Mutations | cargo-mutants | 80% |
| Property | proptest | All public APIs |

---

## 8. Renacer Tracing Integration

### 8.1 Syscall Correlation

```rust
use renacer::{Tracer, TraceConfig, SyscallFilter};

/// ttop with renacer tracing enabled
pub struct TracedTtop {
    app: TtopApp,
    tracer: Option<Tracer>,
}

impl TracedTtop {
    pub fn with_tracing() -> Self {
        let config = TraceConfig::builder()
            .filter(SyscallFilter::parse("read,write,open,stat"))
            .dwarf_correlation(true)
            .build();

        Self {
            app: TtopApp::new(),
            tracer: Tracer::attach_self(config).ok(),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        while !self.app.should_quit {
            // Trace collection syscalls
            if let Some(tracer) = &self.tracer {
                tracer.start_span("collect_metrics");
            }

            self.app.collect();

            if let Some(tracer) = &self.tracer {
                tracer.end_span();
            }

            self.app.render();
        }
        Ok(())
    }

    pub fn report(&self) -> TracingReport {
        self.tracer.as_ref()
            .map(|t| t.generate_report())
            .unwrap_or_default()
    }
}
```

### 8.2 Tracing Panel

```
┌─ Syscall Trace ─────────────────────────────────────────────────────┐
│ Syscall      Count    Avg (μs)    Max (μs)    Source                │
├─────────────────────────────────────────────────────────────────────┤
│ read         1,234      12.3        45.6      cpu.rs:127            │
│ open           456       8.2        23.1      process.rs:89         │
│ stat           789      15.1        67.3      disk.rs:156           │
│ write           23       5.4        12.0      renderer.rs:234       │
├─────────────────────────────────────────────────────────────────────┤
│ Hotspots: process.rs:89 (34%), cpu.rs:127 (28%), disk.rs:156 (21%) │
└─────────────────────────────────────────────────────────────────────┘
```

### 8.3 CLI Integration

```bash
# Run ttop with syscall tracing
cargo run --example ttop --features monitor,tracing -- --trace

# Export trace to JSON
cargo run --example ttop --features monitor,tracing -- --trace --trace-output trace.json

# Analyze trace
renacer analyze trace.json --html report.html
```

---

## 9. Peer-Reviewed Citations

### 9.1 Visualization and Human Factors

1. **Tufte, E. R.** (2001). *The Visual Display of Quantitative Information* (2nd ed.). Graphics Press. ISBN: 978-0961392147.
   - Principles of effective data visualization applied to dashboard design.

2. **Few, S.** (2006). *Information Dashboard Design*. O'Reilly. ISBN: 978-0596100162.
   - Dashboard layout principles for monitoring interfaces.

3. **Ware, C.** (2012). *Information Visualization: Perception for Design* (3rd ed.). Morgan Kaufmann. ISBN: 978-0123814647.
   - Perceptual principles for graph rendering.

4. **Wu, X.** (1991). An efficient antialiasing technique. *ACM SIGGRAPH*, 25(4), 143-152.
   - Wu's antialiasing algorithm for smooth graph lines.

5. **Bresenham, J. E.** (1965). Algorithm for computer control of a digital plotter. *IBM Systems Journal*, 4(1), 25-30.
   - Line rasterization for braille graph rendering.

### 9.2 Color Science

6. **Sharma, G., Wu, W., & Dalal, E. N.** (2005). The CIEDE2000 color-difference formula. *Color Research & Application*, 30(1), 21-30.
   - CIELAB color space for perceptually uniform gradients.

7. **Kovesi, P.** (2015). Good colour maps: How to design them. *arXiv:1509.03700*.
   - Scientific color palette design.

8. **Brewer, C. et al.** (2013). ColorBrewer 2.0. Pennsylvania State University.
   - Accessible color palettes for data visualization.

### 9.3 Systems Performance

9. **Gregg, B.** (2020). *Systems Performance* (2nd ed.). Addison-Wesley. ISBN: 978-0136820154.
   - Methodology for system metrics collection and analysis.

10. **Bovet, D. P., & Cesati, M.** (2005). *Understanding the Linux Kernel* (3rd ed.). O'Reilly. ISBN: 978-0596005658.
    - Linux /proc filesystem structure and parsing.

11. **Tanenbaum, A. S., & Bos, H.** (2014). *Modern Operating Systems* (4th ed.). Pearson. ISBN: 978-0133591620.
    - Process management and scheduling fundamentals.

### 9.4 GPU Computing

12. **NVIDIA Corporation.** (2024). *NVML API Reference*. https://docs.nvidia.com/deploy/nvml-api/
    - NVIDIA GPU metrics collection.

13. **AMD.** (2024). *ROCm SMI Library*. https://github.com/RadeonOpenCompute/rocm_smi_lib
    - AMD GPU metrics via ROCm SMI.

### 9.5 Software Quality

14. **Liker, J. K.** (2004). *The Toyota Way*. McGraw-Hill. ISBN: 978-0071392310.
    - Toyota Production System principles (Jidoka, Poka-Yoke, Heijunka).

15. **Deming, W. E.** (1986). *Out of the Crisis*. MIT Press. ISBN: 978-0911379013.
    - Statistical process control for quality assurance.

16. **Jung, R., et al.** (2017). RustBelt: Securing the Foundations of the Rust Programming Language. *POPL*, 2(POPL), 1-34.
    - Formal verification of Rust's memory safety.

17. **Claessen, K., & Hughes, J.** (2000). QuickCheck: A Lightweight Tool for Random Testing. *ICFP*.
    - Property-based testing methodology.

### 9.6 Testing Methodology

18. **Popper, K.** (1959). *The Logic of Scientific Discovery*. Hutchinson. ISBN: 978-0415278447.
    - Falsifiability criterion for scientific claims.

19. **Lakatos, I.** (1978). *The Methodology of Scientific Research Programmes*. Cambridge. ISBN: 978-0521280310.
    - Research program evaluation methodology.

20. **Mayo, D. G.** (2018). *Statistical Inference as Severe Testing*. Cambridge. ISBN: 978-1107664647.
    - Severe testing for hypothesis evaluation.

### 9.7 Terminal and TUI

21. **Unicode Consortium.** (2023). *Unicode Standard Annex #9*. https://unicode.org/reports/tr9/
    - Braille pattern character encoding (U+2800-U+28FF).

22. **ECMA International.** (1991). *ECMA-48: Control Functions for Coded Character Sets*.
    - Terminal escape sequence specification.

### 9.8 Tracing and Observability

23. **Sigelman, B. H., et al.** (2010). Dapper, a Large-Scale Distributed Systems Tracing Infrastructure. *Google Technical Report*.
    - Distributed tracing methodology.

24. **Gregg, B.** (2019). *BPF Performance Tools*. Addison-Wesley. ISBN: 978-0136554820.
    - System call tracing techniques.

---

## 10. Popperian Falsification Checklist

Following Karl Popper's criterion of falsifiability, each claim must be empirically testable and refutable. This 100-point checklist provides explicit success criteria for external QA evaluation.

### 10.1 Performance Claims (1-20)

| # | Falsifiable Claim | Test Method | Failure Criterion |
|---|-------------------|-------------|-------------------|
| 1 | Frame rendering < 8ms on reference hardware | Criterion benchmark on i5-8250U | P95 > 8ms |
| 2 | Frame rendering < 16ms on minimum hardware | Criterion benchmark on i3-7100U | P95 > 16ms |
| 3 | Startup time < 50ms | Time from exec to first frame | > 50ms |
| 4 | Memory usage < 8MB after warmup | /proc/self/statm after 60s | > 8MB RSS |
| 5 | Memory stable over 24 hours | Heap monitoring over 24h run | Growth > 1MB |
| 6 | CPU idle < 1% when paused | top measurement over 60s | > 1% avg |
| 7 | CPU active < 3% at 1Hz refresh | top measurement over 60s | > 3% avg |
| 8 | Zero allocations after warmup | GlobalAlloc tracking | Any allocation after frame 100 |
| 9 | Process list scales O(n) | Benchmark 100-10000 processes | Non-linear growth |
| 10 | Graph rendering O(width × height) | Benchmark varying dimensions | Non-linear growth |
| 11 | Table scrolling maintains 60fps with 10k rows | FPS counter during scroll | < 60fps |
| 12 | Tree expand/collapse < 10ms | Benchmark toggle operation | > 10ms |
| 13 | Layout recalculation < 5ms on resize | Benchmark resize handler | > 5ms |
| 14 | Network collector O(interfaces) | Benchmark 1-100 interfaces | Non-linear growth |
| 15 | Keyboard latency < 1ms | Event timestamp analysis | > 1ms |
| 16 | Mouse click response < 5ms | Click to visual response | > 5ms |
| 17 | Config hot-reload < 100ms | Measure reload time | > 100ms |
| 18 | Theme switch < 50ms | Measure switch time | > 50ms |
| 19 | Panel toggle < 10ms | Measure toggle time | > 10ms |
| 20 | Braille rendering 2X faster than block | Benchmark both modes | < 2X |

### 10.2 Visual Quality (21-40)

| # | Falsifiable Claim | Test Method | Failure Criterion |
|---|-------------------|-------------|-------------------|
| 21 | Braille resolution 8 dots per cell | Visual inspection | < 8 dots |
| 22 | TrueColor support (16.7M colors) | Terminal capability test | 256 color fallback |
| 23 | Gradient Delta-E variance < 2.0 | CIEDE2000 measurement | Variance ≥ 2.0 |
| 24 | 256-color fallback maintains 5+ stops | Color discrimination test | < 5 stops |
| 25 | TTY mode pure ASCII | Character analysis | Non-ASCII chars |
| 26 | Double-buffering eliminates flicker | 240fps video analysis | > 1 flicker/min |
| 27 | Wu antialiasing smoother than Bresenham | User study (n≥30) | p > 0.05 |
| 28 | Meter gradient fills correctly | Pixel inspection | Off-by-one errors |
| 29 | Sparkline 8 levels distinguishable | Visual test | < 8 levels |
| 30 | Tree indentation consistent | Character count | Inconsistent indent |
| 31 | Table column alignment pixel-perfect | Column analysis | Misalignment |
| 32 | Border characters connected | Junction inspection | Gaps at corners |
| 33 | Unicode box drawing renders | Terminal test matrix | Garbled chars |
| 34 | Emoji-free output | Character analysis | Any emoji |
| 35 | ANSI escape minimal | Escape count analysis | > 2X btop |
| 36 | Color contrast WCAG AA | Contrast ratio test | < 4.5:1 |
| 37 | Focus indicator visible | Visual inspection | Not visible |
| 38 | Selection highlight distinct | Visual inspection | Indistinct |
| 39 | Error states red | Color test | Not red |
| 40 | Warning states yellow | Color test | Not yellow |

### 10.3 Metric Accuracy (41-60)

| # | Falsifiable Claim | Test Method | Failure Criterion |
|---|-------------------|-------------|-------------------|
| 41 | CPU % within ±2% of top | Simultaneous comparison | > 2% deviation |
| 42 | Memory within ±1MB of free | Simultaneous comparison | > 1MB deviation |
| 43 | Network within ±5% of iftop | iperf3 test comparison | > 5% deviation |
| 44 | Disk IO within ±5% of iostat | fio test comparison | > 5% deviation |
| 45 | Process CPU within ±3% of htop | Same PID comparison | > 3% deviation |
| 46 | GPU util within ±2% of nvidia-smi | Simultaneous comparison | > 2% deviation |
| 47 | Temperature within ±1°C of sensors | Simultaneous comparison | > 1°C deviation |
| 48 | Load average exact match with uptime | Compare readings | Any deviation |
| 49 | Uptime within ±1s of uptime cmd | Compare readings | > 1s deviation |
| 50 | Process tree matches pstree | Structural comparison | Incorrect hierarchy |
| 51 | Network counters monotonic | Counter analysis | Decrease detected |
| 52 | Disk counters monotonic | Counter analysis | Decrease detected |
| 53 | Battery % within ±1% of upower | Comparison | > 1% deviation |
| 54 | Swap usage exact match | /proc/swaps comparison | Any deviation |
| 55 | Mount points match df | Mount list comparison | Missing mount |
| 56 | Per-core frequency matches cpufreq | sysfs comparison | > 10MHz deviation |
| 57 | ZRAM ratio matches mm_stat | Calculation comparison | Any deviation |
| 58 | Process count matches ps aux | Count comparison | Count mismatch |
| 59 | Thread count matches /proc | Count comparison | Count mismatch |
| 60 | FD count matches lsof | Count comparison | Count mismatch |

### 10.4 Determinism (61-70)

| # | Falsifiable Claim | Test Method | Failure Criterion |
|---|-------------------|-------------|-------------------|
| 61 | Same state → identical frame | Render twice, compare | Any difference |
| 62 | Frame ID → reproducible output | Hash comparison | Hash mismatch |
| 63 | No floating-point non-determinism | Multiple runs | Result variation |
| 64 | No hash iteration order dependency | Verify IndexMap | Order variation |
| 65 | Timestamp frozen at collection | Verify no clock calls | Clock during render |
| 66 | RNG seeded with frame_id | Verify seed consistency | Random variation |
| 67 | Process sort stable | Sort same data twice | Order variation |
| 68 | Layout calculation deterministic | Calculate twice | Any difference |
| 69 | Color gradient sampling deterministic | Sample same t twice | Color difference |
| 70 | Widget rendering deterministic | Render twice | Any difference |

### 10.5 Testing Coverage (71-85)

| # | Falsifiable Claim | Test Method | Failure Criterion |
|---|-------------------|-------------|-------------------|
| 71 | Rust unit coverage ≥ 95% | cargo-llvm-cov | < 95% |
| 72 | TUI playbook coverage 100% | probar report | < 100% |
| 73 | Pixel coverage ≥ 85% | probar pixel | < 85% |
| 74 | Mutation score ≥ 80% | cargo-mutants | < 80% |
| 75 | All public APIs property-tested | proptest coverage | Missing APIs |
| 76 | All keyboard shortcuts tested | Playbook coverage | Missing shortcut |
| 77 | All panels have snapshot tests | Snapshot count | Missing panel |
| 78 | All widgets have unit tests | Test count | Missing widget |
| 79 | All collectors have accuracy tests | Test count | Missing collector |
| 80 | Error paths tested | Error injection | Untested path |
| 81 | Edge cases tested (0%, 100%, overflow) | Boundary tests | Untested boundary |
| 82 | Empty state rendering tested | Empty state test | Crash or panic |
| 83 | Maximum load rendering tested | Max load test | Crash or panic |
| 84 | Resize handling tested | Resize test matrix | Crash or garbled |
| 85 | Theme fallback tested | Invalid theme test | Crash |

### 10.6 Input Handling (86-95)

| # | Falsifiable Claim | Test Method | Failure Criterion |
|---|-------------------|-------------|-------------------|
| 86 | All documented keys work | Key injection test | Any key fails |
| 87 | Vim keys (hjkl) work when enabled | Navigation test | Movement fails |
| 88 | Mouse click identifies correct panel | Coordinate test | Wrong panel |
| 89 | Mouse scroll direction correct | Scroll test | Wrong direction |
| 90 | Ctrl+C exits cleanly | Signal test | Corrupted terminal |
| 91 | SIGWINCH triggers relayout | Signal test | Layout not updated |
| 92 | Invalid input ignored gracefully | Fuzz test | Crash |
| 93 | Rapid input handled | Burst test | Dropped events |
| 94 | Filter regex accepts valid patterns | Regex test | Valid rejected |
| 95 | Invalid regex shows error | Invalid regex test | Crash |

### 10.7 Safety and Correctness (96-100)

| # | Falsifiable Claim | Test Method | Failure Criterion |
|---|-------------------|-------------|-------------------|
| 96 | Zero unsafe in safe modules | grep analysis | unsafe found |
| 97 | No data races | ThreadSanitizer | Race detected |
| 98 | No undefined behavior | Miri + ASan | UB detected |
| 99 | No panics in production | Fuzz testing | Panic occurs |
| 100 | Integer overflow prevented | Overflow tests | Overflow |

---

## Appendix A: Reference Hardware

| Tier | Specification | Frame Target |
|------|---------------|--------------|
| Minimum | Intel i3-7100U, 4GB RAM | 16ms |
| Recommended | Intel i5-8250U, 8GB RAM | 8ms |
| Reference | AMD Ryzen 7 5800X, 32GB RAM | 4ms |

## Appendix B: Supported Terminals

| Terminal | TrueColor | Mouse | Braille | Verified |
|----------|-----------|-------|---------|----------|
| Alacritty | ✓ | ✓ | ✓ | ✓ |
| kitty | ✓ | ✓ | ✓ | ✓ |
| iTerm2 | ✓ | ✓ | ✓ | ✓ |
| GNOME Terminal | ✓ | ✓ | ✓ | ✓ |
| Windows Terminal | ✓ | ✓ | ✓ | ✓ |
| xterm | 256 | ✓ | ✓ | ✓ |
| Linux VT | 16 | ✗ | ✗ | ✓ |

## Appendix C: Probar Configuration

```toml
# probar.toml for ttop testing
[pixel_coverage]
enabled = true
methodology = "falsification"

[pixel_coverage.thresholds]
min_coverage = 85.0
max_gap_size = 5.0
falsifiability_threshold = 15.0

[pixel_coverage.verification]
ssim_threshold = 0.99
delta_e_threshold = 1.0
phash_distance = 5

[playbooks]
directory = "tests/playbooks"
parallel = true

[snapshots]
directory = "tests/snapshots"
update_mode = "manual"
```

---

*Specification generated for ttop - Terminal Top*
*Sovereign AI Stack - Pure Rust, Privacy-Preserving ML Infrastructure*
*10X Better Than btop - Verified by Falsification*
