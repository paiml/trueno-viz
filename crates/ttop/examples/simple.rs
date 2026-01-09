//! Simple example: Run ttop programmatically
//!
//! ```bash
//! cargo run --example simple
//! ```

use ttop::app::App;

fn main() -> anyhow::Result<()> {
    // Create app with real data collection (not deterministic mode)
    let mut app = App::new(false, false);

    // Print system info
    println!("System Information:");
    println!("  CPU cores: {}", app.cpu.core_count());
    println!("  Memory: {:.1} GB", app.mem_total as f64 / 1e9);
    println!("  Uptime: {:.0} seconds", app.cpu.uptime_secs());

    // Collect metrics (second sample needed for CPU delta)
    std::thread::sleep(std::time::Duration::from_millis(100));
    app.collect_metrics();

    // Print current stats
    if let Some(cpu_pct) = app.cpu_history.last() {
        println!("  CPU usage: {:.1}%", cpu_pct * 100.0);
    }

    let mem_pct = if app.mem_total > 0 {
        (app.mem_used as f64 / app.mem_total as f64) * 100.0
    } else {
        0.0
    };
    println!("  Memory usage: {:.1}%", mem_pct);

    println!("\nRun `ttop` for the full TUI experience!");

    Ok(())
}
