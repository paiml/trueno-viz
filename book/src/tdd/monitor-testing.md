# System Monitor TDD

This chapter covers Test-Driven Development for the trueno-viz monitoring module and ttop.

## Testing Philosophy for System Monitors

System monitors present unique testing challenges:
- Hardware-dependent metrics
- Platform-specific code paths
- Real-time data that changes constantly
- External command dependencies

## Collector Testing Pattern

### Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collector_new() {
        let collector = CpuCollector::new();
        // Verify initialization doesn't panic
        assert!(collector.core_count() > 0);
    }

    #[test]
    fn test_collect_returns_metrics() {
        let mut collector = CpuCollector::new();
        let metrics = collector.collect().expect("should collect metrics");
        assert!(!metrics.is_empty());
    }

    #[test]
    fn test_metrics_within_bounds() {
        let mut collector = CpuCollector::new();
        let metrics = collector.collect().expect("should collect");

        if let Some(usage) = metrics.get_gauge("cpu.total") {
            assert!(usage >= 0.0 && usage <= 100.0);
        }
    }
}
```

## Mock-Based Testing

For deterministic tests, mock external data sources:

```rust
pub struct MockCpuCollector {
    mock_usage: f64,
    mock_cores: usize,
}

impl MockCpuCollector {
    pub fn with_usage(usage: f64) -> Self {
        Self {
            mock_usage: usage,
            mock_cores: 8,
        }
    }
}

impl Collector for MockCpuCollector {
    fn collect(&mut self) -> Result<MetricSet> {
        let mut metrics = MetricSet::new();
        metrics.set_gauge("cpu.total", self.mock_usage);
        Ok(metrics)
    }
}

#[test]
fn test_ui_with_high_cpu() {
    let collector = MockCpuCollector::with_usage(95.0);
    let app = App::with_collector(collector);
    // Verify high CPU is displayed correctly
    assert!(app.cpu_display().contains("95"));
}
```

## History Buffer Testing

ttop maintains rolling history buffers for sparklines:

```rust
#[test]
fn test_history_buffer_capacity() {
    let mut buffer = HistoryBuffer::new(60);

    for i in 0..100 {
        buffer.push(i as f64);
    }

    // Should maintain fixed capacity
    assert_eq!(buffer.len(), 60);
    // Should contain most recent values
    assert_eq!(buffer.get(59), Some(99.0));
}

#[test]
fn test_history_buffer_normalization() {
    let mut buffer = HistoryBuffer::new(10);
    buffer.push(0.0);
    buffer.push(50.0);
    buffer.push(100.0);

    let normalized = buffer.normalized();
    assert_eq!(normalized[0], 0.0);
    assert_eq!(normalized[1], 0.5);
    assert_eq!(normalized[2], 1.0);
}
```

## Platform-Specific Testing

Use conditional compilation for platform tests:

```rust
#[cfg(target_os = "macos")]
#[test]
fn test_macos_gpu_detection() {
    let collector = GpuCollector::new();
    // Should detect at least one GPU on macOS
    assert!(collector.gpu_count() >= 1);
}

#[cfg(target_os = "linux")]
#[test]
fn test_linux_proc_parsing() {
    let collector = CpuCollector::new();
    // Should parse /proc/stat
    let metrics = collector.collect().expect("should parse /proc/stat");
    assert!(metrics.get_gauge("cpu.total").is_some());
}

#[cfg(all(target_os = "macos", feature = "apple-hardware"))]
#[test]
fn test_apple_accelerators_available() {
    let collector = AppleAcceleratorsCollector::new();
    // At minimum, Metal GPU should be available
    assert!(collector.metal.available);
}
```

## Deterministic Mode Testing

ttop supports deterministic mode for reproducible tests:

```rust
#[test]
fn test_deterministic_rendering() {
    let app = App::new_deterministic();

    // First render
    let frame1 = app.render_to_string();

    // Reset and render again
    app.reset();
    let frame2 = app.render_to_string();

    // Should be identical
    assert_eq!(frame1, frame2);
}
```

## TUI Testing with jugar-probar

Use pixel-perfect TUI testing:

```rust
use jugar_probar::tui::{Terminal, TestBackend};

#[test]
fn test_cpu_panel_renders() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let app = App::new_deterministic();

    terminal.draw(|f| {
        draw_cpu_panel(f, &app, f.area());
    }).unwrap();

    let buffer = terminal.backend().buffer();

    // Verify panel title
    assert!(buffer.content().contains("CPU"));

    // Verify sparkline characters present
    assert!(buffer.content().contains("▁") ||
            buffer.content().contains("▄") ||
            buffer.content().contains("█"));
}

#[test]
fn test_accelerators_panel_renders() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new_deterministic();
    app.panels.accelerators = true;

    terminal.draw(|f| {
        draw_accelerators(f, &app, f.area());
    }).unwrap();

    let buffer = terminal.backend().buffer();

    #[cfg(all(target_os = "macos", feature = "apple-hardware"))]
    {
        // Should show accelerator names
        assert!(buffer.content().contains("Metal") ||
                buffer.content().contains("Neural") ||
                buffer.content().contains("Afterburner"));
    }
}
```

## Apple Hardware TDD

### Testing with manzana

```rust
#[cfg(all(target_os = "macos", feature = "apple-hardware"))]
mod apple_tests {
    use trueno_viz::monitor::collectors::AppleAcceleratorsCollector;
    use trueno_viz::monitor::types::Collector;

    #[test]
    fn test_metal_detection() {
        let collector = AppleAcceleratorsCollector::new();
        // All Macs have Metal GPU
        assert!(collector.metal.available);
        assert!(!collector.metal.name.is_empty());
    }

    #[test]
    fn test_secure_enclave_on_t2_or_silicon() {
        let collector = AppleAcceleratorsCollector::new();
        // T2 Macs and Apple Silicon have Secure Enclave
        // Note: This test only passes on compatible hardware
        if collector.secure_enclave.available {
            assert_eq!(collector.secure_enclave.algorithm, "P-256 ECDSA");
        }
    }

    #[test]
    fn test_afterburner_on_mac_pro() {
        let collector = AppleAcceleratorsCollector::new();
        // Afterburner only available on Mac Pro 2019+
        if collector.afterburner.available {
            assert!(collector.afterburner.streams_capacity >= 23);
        }
    }

    #[test]
    fn test_neural_engine_on_apple_silicon() {
        let collector = AppleAcceleratorsCollector::new();
        // Neural Engine only on Apple Silicon
        if collector.neural_engine.available {
            assert!(collector.neural_engine.tops > 0.0);
            assert!(collector.neural_engine.core_count > 0);
        }
    }

    #[test]
    fn test_collect_metrics() {
        let mut collector = AppleAcceleratorsCollector::new();
        let metrics = collector.collect().expect("should collect");

        // Should have at least Metal metrics
        assert!(metrics.get_gauge("metal.vram_gb").is_some() ||
                metrics.get_counter("metal.max_threads").is_some());
    }

    #[test]
    fn test_history_buffers() {
        let mut collector = AppleAcceleratorsCollector::new();

        // Collect multiple times
        for _ in 0..5 {
            let _ = collector.collect();
        }

        // History should accumulate
        let ab_history = collector.afterburner_history();
        let ne_history = collector.neural_engine_history();

        // Histories should have data
        assert!(!ab_history.is_empty() || !ne_history.is_empty() ||
                !collector.afterburner.available && !collector.neural_engine.available);
    }
}
```

## Integration Tests

```rust
// tests/monitor_integration.rs
use trueno_viz::monitor::collectors::*;
use trueno_viz::monitor::types::Collector;

#[test]
fn test_all_collectors_initialize() {
    let _ = CpuCollector::new();
    let _ = MemoryCollector::new();
    let _ = DiskCollector::new();
    let _ = NetworkCollector::new();
    let _ = ProcessCollector::new();

    #[cfg(all(target_os = "macos", feature = "apple-hardware"))]
    let _ = AppleAcceleratorsCollector::new();
}

#[test]
fn test_all_collectors_collect() {
    let mut cpu = CpuCollector::new();
    let mut mem = MemoryCollector::new();

    assert!(cpu.collect().is_ok());
    assert!(mem.collect().is_ok());
}
```

## Running Monitor Tests

```bash
# All monitor tests
cargo test -p ttop

# With apple-hardware
cargo test -p ttop --features apple-hardware

# Deterministic tests only
cargo test -p ttop deterministic

# Platform tests
cargo test -p ttop platform

# With debug output
cargo test -p ttop -- --nocapture
```

## Test Coverage for ttop

```bash
# Generate coverage report
cd crates/ttop
cargo llvm-cov --html --features apple-hardware

# Verify 95%+ coverage
cargo llvm-cov --fail-under-lines 95
```

## Best Practices

1. **Test initialization** - Verify collectors don't panic on creation
2. **Test bounds** - Ensure metrics stay within valid ranges (0-100% for utilization)
3. **Test platform code** - Use cfg attributes for platform-specific tests
4. **Use mocks** - For deterministic UI testing
5. **Test history** - Verify rolling buffers maintain correct size
6. **Test gracefully** - Handle missing hardware (e.g., no Afterburner on non-Mac Pro)

## Next Chapter

Continue to [Property-Based Testing](./property-testing.md) for advanced testing techniques.
