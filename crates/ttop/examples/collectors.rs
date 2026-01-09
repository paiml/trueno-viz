//! Example: Using individual collectors via App
//!
//! Shows how to access ttop's system monitoring data.
//!
//! ```bash
//! cargo run --example collectors
//! ```

use ttop::app::App;

fn main() -> anyhow::Result<()> {
    println!("=== ttop Collectors Demo ===\n");

    // Create app and collect initial data
    let mut app = App::new(false, false);

    // CPU info
    println!("CPU:");
    println!("  Cores: {}", app.cpu.core_count());
    println!("  Uptime: {:.0}s", app.cpu.uptime_secs());
    let load = app.cpu.load_average();
    println!("  Load: {:.2} / {:.2} / {:.2}", load.one, load.five, load.fifteen);

    // Get second sample for CPU percentage
    std::thread::sleep(std::time::Duration::from_millis(100));
    app.collect_metrics();

    if let Some(cpu_pct) = app.cpu_history.last() {
        println!("  Usage: {:.1}%", cpu_pct * 100.0);
    }

    // Per-core usage
    if !app.per_core_percent.is_empty() {
        let max_core = app.per_core_percent
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap());
        if let Some((idx, pct)) = max_core {
            println!("  Hottest core: {} at {:.1}%", idx, pct);
        }
    }
    println!();

    // Memory info
    println!("Memory:");
    println!("  Total: {:.1} GB", app.mem_total as f64 / 1e9);
    println!("  Used: {:.1} GB", app.mem_used as f64 / 1e9);
    println!("  Available: {:.1} GB", app.mem_available as f64 / 1e9);
    println!("  Cached: {:.1} GB", app.mem_cached as f64 / 1e9);
    if app.swap_total > 0 {
        println!("  Swap: {:.1}/{:.1} GB",
                 app.swap_used as f64 / 1e9,
                 app.swap_total as f64 / 1e9);
    }
    println!();

    // Disk info
    println!("Disks:");
    for mount in app.disk.mounts().iter().take(5) {
        if mount.total_bytes > 0 {
            let total_gb = mount.total_bytes as f64 / 1e9;
            let usage = mount.usage_percent();
            println!("  {}: {:.0} GB ({:.0}% used)", mount.mount_point, total_gb, usage);
        }
    }
    println!();

    // Network info
    println!("Network:");
    for (iface, rate) in app.network.all_rates() {
        if rate.rx_bytes_per_sec > 100.0 || rate.tx_bytes_per_sec > 100.0 {
            println!("  {}: RX {:.1} KB/s, TX {:.1} KB/s",
                     iface,
                     rate.rx_bytes_per_sec / 1024.0,
                     rate.tx_bytes_per_sec / 1024.0);
        }
    }

    // Process summary
    println!("\nProcesses: {} total", app.process.processes().len());

    println!("\nRun `ttop` for the full TUI!");
    Ok(())
}
