//! Example: Using ttop collectors directly
//!
//! Demonstrates how to use the monitoring collectors without the TUI.
//!
//! Run: cargo run --example collectors

use trueno_viz::monitor::collectors::{CpuCollector, MemoryCollector, ProcessCollector};
use trueno_viz::monitor::types::{Collector, MetricValue};

fn main() {
    println!("ttop Collectors Example");
    println!("========================\n");

    // CPU Collector
    println!("CPU Metrics:");
    let mut cpu = CpuCollector::new();
    match cpu.collect() {
        Ok(metrics) => {
            println!("  Collector: {}", cpu.id());
            // Print all gauge values
            for (key, value) in metrics.iter() {
                if let Some(v) = value.as_gauge() {
                    println!("  {}: {:.2}", key, v);
                }
            }
        }
        Err(e) => println!("  Error: {}", e),
    }
    println!();

    // Memory Collector
    println!("Memory Metrics:");
    let mut memory = MemoryCollector::new();
    match memory.collect() {
        Ok(metrics) => {
            println!("  Collector: {}", memory.id());
            for (key, value) in metrics.iter() {
                match value {
                    MetricValue::Gauge(v) if key.contains("total") || key.contains("used") || key.contains("free") => {
                        println!("  {}: {:.2} GB", key, v / 1024.0 / 1024.0 / 1024.0);
                    }
                    MetricValue::Gauge(v) => {
                        println!("  {}: {:.2}", key, v);
                    }
                    _ => {}
                }
            }
        }
        Err(e) => println!("  Error: {}", e),
    }
    println!();

    // Process Collector
    println!("Process Metrics:");
    let mut procs = ProcessCollector::new();
    match procs.collect() {
        Ok(metrics) => {
            println!("  Collector: {}", procs.id());
            for (key, value) in metrics.iter() {
                if let Some(v) = value.as_gauge() {
                    println!("  {}: {:.0}", key, v);
                }
            }
        }
        Err(e) => println!("  Error: {}", e),
    }
    println!();

    println!("Done! Use 'ttop' for the full TUI experience.");
}
