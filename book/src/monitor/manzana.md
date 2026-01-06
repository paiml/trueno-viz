# Manzana Integration

[manzana](https://crates.io/crates/manzana) provides safe Rust interfaces to Apple hardware accelerators for the Sovereign AI Stack.

## Overview

trueno-viz integrates manzana to provide Apple-specific hardware monitoring through the `apple-hardware` feature flag.

```toml
[dependencies]
trueno-viz = { version = "0.1", features = ["apple-hardware"] }
```

## Supported Hardware

| Accelerator | Hardware | Use Case |
|-------------|----------|----------|
| **Afterburner FPGA** | Mac Pro 2019+ | ProRes decode (23x 4K streams) |
| **Neural Engine** | Apple Silicon | ML inference (15.8+ TOPS) |
| **Metal GPU** | All Macs | General GPU compute |
| **Secure Enclave** | T2, Apple Silicon | P-256 ECDSA signing |
| **Unified Memory** | Apple Silicon | Zero-copy CPU/GPU buffers |

## AppleAcceleratorsCollector

The `AppleAcceleratorsCollector` wraps manzana to provide unified access to all Apple accelerators:

```rust
#[cfg(all(target_os = "macos", feature = "apple-hardware"))]
{
    use trueno_viz::monitor::collectors::AppleAcceleratorsCollector;
    use trueno_viz::monitor::types::Collector;

    let mut collector = AppleAcceleratorsCollector::new();

    // Check available accelerators
    println!("Available: {} accelerators", collector.available_count());

    // Neural Engine (Apple Silicon)
    if collector.neural_engine.available {
        println!("Neural Engine: {:.1} TOPS, {} cores",
            collector.neural_engine.tops,
            collector.neural_engine.core_count);
    }

    // Afterburner FPGA (Mac Pro)
    if collector.afterburner.available {
        println!("Afterburner: {}/{} streams, {:.1}% utilization",
            collector.afterburner.streams_active,
            collector.afterburner.streams_capacity,
            collector.afterburner.utilization);
    }

    // Metal GPU
    if collector.metal.available {
        println!("Metal GPU: {} ({:.1}GB {})",
            collector.metal.name,
            collector.metal.vram_gb,
            if collector.metal.is_uma { "UMA" } else { "Discrete" });
    }

    // Secure Enclave
    if collector.secure_enclave.available {
        println!("Secure Enclave: {}", collector.secure_enclave.algorithm);
    }

    // Collect metrics
    let metrics = collector.collect()?;
    println!("Collected {} metrics", metrics.len());
}
```

## Metrics Collected

### Afterburner Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `afterburner.streams_active` | Counter | Active ProRes decode streams |
| `afterburner.streams_capacity` | Counter | Maximum stream capacity (23) |
| `afterburner.util` | Gauge | Utilization percentage |

### Neural Engine Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `neural_engine.tops` | Gauge | Performance in TOPS |
| `neural_engine.cores` | Counter | Neural Engine cores |
| `neural_engine.util` | Gauge | Estimated utilization |

### Metal GPU Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `metal.vram_gb` | Gauge | VRAM/UMA allocation (GB) |
| `metal.max_threads` | Counter | Max threads per threadgroup |
| `metal.is_uma` | Counter | 1 if unified memory, 0 if discrete |

### Secure Enclave Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `secure_enclave.available` | Counter | 1 if available, 0 otherwise |

### Unified Memory Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `uma.page_size` | Counter | Memory page size (bytes) |

## History Buffers

The collector maintains rolling history buffers for utilization metrics:

```rust
// Get Afterburner utilization history (normalized 0-1)
let afterburner_history = collector.afterburner_history();
for value in afterburner_history.iter() {
    println!("Afterburner util: {:.1}%", value * 100.0);
}

// Get Neural Engine utilization history
let ne_history = collector.neural_engine_history();
```

## Platform Detection

The collector automatically detects available hardware:

```rust
let collector = AppleAcceleratorsCollector::new();

// Mac Pro 2019+ with Afterburner
if collector.afterburner.available {
    println!("Mac Pro detected with Afterburner FPGA");
}

// Apple Silicon
if collector.neural_engine.available {
    println!("Apple Silicon detected with Neural Engine");
}

// T2 or Apple Silicon
if collector.secure_enclave.available {
    println!("Secure Enclave available for cryptographic operations");
}
```

## Integration with ttop

ttop displays manzana metrics in the Accelerators panel:

```bash
# Build ttop with Apple hardware support
cargo build -p ttop --release --features apple-hardware

# Run
./target/release/ttop
```

The Accelerators panel shows:
- Neural Engine utilization meter
- Afterburner stream usage
- Metal GPU info
- Secure Enclave status
- UMA availability

## Direct manzana Usage

For more advanced use cases, you can use manzana directly:

```rust
use manzana::afterburner::AfterburnerMonitor;
use manzana::neural_engine::NeuralEngineSession;
use manzana::metal::MetalCompute;
use manzana::secure_enclave::{SecureEnclaveSigner, KeyConfig};

// Afterburner monitoring
if AfterburnerMonitor::is_available() {
    let monitor = AfterburnerMonitor::new().unwrap();
    let stats = monitor.stats()?;
    println!("ProRes streams: {}/{}", stats.streams_active, stats.streams_capacity);
}

// Neural Engine capabilities
if NeuralEngineSession::is_available() {
    if let Some(caps) = NeuralEngineSession::capabilities() {
        println!("Neural Engine: {:.1} TOPS", caps.tops);
    }
}

// Metal GPU enumeration
let devices = MetalCompute::devices();
for device in &devices {
    println!("GPU: {} ({:.1}GB)", device.name, device.vram_gb());
}

// Secure Enclave signing
if SecureEnclaveSigner::is_available() {
    let config = KeyConfig::new("com.example.signing");
    let signer = SecureEnclaveSigner::create(config)?;
    let signature = signer.sign(b"message")?;
}
```

## Safety Architecture

manzana follows a strict safety architecture:

```
+-------------------------------------------------------------+
|                  PUBLIC API (100% Safe Rust)                |
|  #![deny(unsafe_code)]                                      |
+-------------------------------------------------------------+
|                  FFI QUARANTINE ZONE                        |
|  Isolated unsafe code for IOKit, Security.framework         |
+-------------------------------------------------------------+
|                  macOS Kernel / Frameworks                  |
+-------------------------------------------------------------+
```

All unsafe code is quarantined in the `src/ffi/` directory and has been audited for memory safety.

## References

- [manzana on crates.io](https://crates.io/crates/manzana)
- [manzana on GitHub](https://github.com/paiml/manzana)
- [Apple Afterburner](https://support.apple.com/en-us/HT210918)
- [Apple Neural Engine](https://machinelearning.apple.com/research/neural-engine-transformers)
