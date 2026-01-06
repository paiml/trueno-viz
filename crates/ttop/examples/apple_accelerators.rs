//! Apple Accelerators Example
//!
//! Demonstrates programmatic access to Apple hardware accelerators via manzana.
//!
//! Run with: cargo run --example apple_accelerators --features apple-hardware

fn main() {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║      TTOP - Apple Accelerators Demo (via manzana)          ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();

    #[cfg(all(target_os = "macos", feature = "apple-hardware"))]
    {
        use trueno_viz::monitor::collectors::AppleAcceleratorsCollector;
        use trueno_viz::monitor::types::Collector;

        let mut collector = AppleAcceleratorsCollector::new();

        println!("Platform: macOS");
        println!("Available accelerators: {}", collector.available_count());
        println!();

        // Afterburner FPGA (Mac Pro 2019+)
        println!("┌─────────────────────────────────────────────────────────────┐");
        println!("│ Afterburner FPGA                                            │");
        println!("├─────────────────────────────────────────────────────────────┤");
        if collector.afterburner.available {
            println!(
                "│ Status: ✓ AVAILABLE                                         │"
            );
            println!(
                "│ Streams: {:>2} / {:>2} active                                    │",
                collector.afterburner.streams_active, collector.afterburner.streams_capacity
            );
            println!(
                "│ Utilization: {:>5.1}%                                         │",
                collector.afterburner.utilization
            );
        } else {
            println!("│ Status: ✗ Not available (requires Mac Pro 2019+)           │");
        }
        println!("└─────────────────────────────────────────────────────────────┘");
        println!();

        // Neural Engine
        println!("┌─────────────────────────────────────────────────────────────┐");
        println!("│ Apple Neural Engine                                         │");
        println!("├─────────────────────────────────────────────────────────────┤");
        if collector.neural_engine.available {
            println!(
                "│ Status: ✓ AVAILABLE                                         │"
            );
            println!(
                "│ Performance: {:>5.1} TOPS                                     │",
                collector.neural_engine.tops
            );
            println!(
                "│ Cores: {:>2}                                                   │",
                collector.neural_engine.core_count
            );
        } else {
            println!("│ Status: ✗ Not available (requires Apple Silicon)           │");
        }
        println!("└─────────────────────────────────────────────────────────────┘");
        println!();

        // Metal GPU
        println!("┌─────────────────────────────────────────────────────────────┐");
        println!("│ Metal GPU                                                   │");
        println!("├─────────────────────────────────────────────────────────────┤");
        if collector.metal.available {
            println!(
                "│ Status: ✓ AVAILABLE                                         │"
            );
            println!("│ Name: {:<53} │", &collector.metal.name);
            println!(
                "│ VRAM: {:>6.1} GB ({})                                     │",
                collector.metal.vram_gb,
                if collector.metal.is_uma { "UMA" } else { "Discrete" }
            );
            println!(
                "│ Max Threads: {:>6}                                          │",
                collector.metal.max_threads
            );
        } else {
            println!("│ Status: ✗ Not available                                     │");
        }
        println!("└─────────────────────────────────────────────────────────────┘");
        println!();

        // Secure Enclave
        println!("┌─────────────────────────────────────────────────────────────┐");
        println!("│ Secure Enclave                                              │");
        println!("├─────────────────────────────────────────────────────────────┤");
        if collector.secure_enclave.available {
            println!(
                "│ Status: ✓ AVAILABLE                                         │"
            );
            println!("│ Algorithm: {:<48} │", collector.secure_enclave.algorithm);
        } else {
            println!("│ Status: ✗ Not available                                     │");
        }
        println!("└─────────────────────────────────────────────────────────────┘");
        println!();

        // Unified Memory
        println!("┌─────────────────────────────────────────────────────────────┐");
        println!("│ Unified Memory Architecture                                 │");
        println!("├─────────────────────────────────────────────────────────────┤");
        if collector.uma.available {
            println!(
                "│ Status: ✓ AVAILABLE                                         │"
            );
            println!(
                "│ Page Size: {} bytes                                        │",
                collector.uma.page_size
            );
        } else {
            println!("│ Status: ✗ Not available (requires Apple Silicon)           │");
        }
        println!("└─────────────────────────────────────────────────────────────┘");
        println!();

        // Collect metrics
        println!("Collecting metrics...");
        if let Ok(metrics) = collector.collect() {
            println!("✓ Collected {} metrics", metrics.len());
        }

        println!();
        println!("╔════════════════════════════════════════════════════════════╗");
        println!("║                    Demo Complete                           ║");
        println!("╚════════════════════════════════════════════════════════════╝");
    }

    #[cfg(not(all(target_os = "macos", feature = "apple-hardware")))]
    {
        println!("This example requires macOS with the 'apple-hardware' feature.");
        println!();
        println!("Build with:");
        println!("  cargo run --example apple_accelerators --features apple-hardware");
    }
}
