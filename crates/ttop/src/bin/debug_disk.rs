//! Debug disk data

use ttop::app::App;
use ttop::theme;

fn main() {
    let mut app = App::new(false, false);

    // Collect a couple times to get rates
    app.collect_metrics();
    std::thread::sleep(std::time::Duration::from_secs(1));
    app.collect_metrics();

    println!("=== PANEL LABELS (what should appear) ===");
    let rates = app.disk.rates();
    for mount in app.disk.mounts() {
        if mount.total_bytes == 0 {
            continue;
        }

        let total_gb = mount.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let used_gb = mount.used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

        // Label (same as panel code)
        let label: String = if mount.mount_point == "/" {
            "/".to_string()
        } else {
            mount.mount_point.rsplit('/').next().unwrap_or(&mount.mount_point).chars().take(8).collect()
        };

        // Find I/O rate
        let device_name = mount.device.rsplit('/').next().unwrap_or("");
        let io_info = rates.get(device_name).or_else(|| {
            let base: String = device_name.chars().take_while(|c| !c.is_ascii_digit()).collect();
            rates.get(&base)
        });

        let size_str = if total_gb >= 1000.0 {
            format!("{:.1}T/{:.1}T", used_gb / 1024.0, total_gb / 1024.0)
        } else {
            format!("{:.0}G/{:.0}G", used_gb, total_gb)
        };

        let io_str = if let Some(io) = io_info {
            format!(" R:{} W:{}",
                theme::format_bytes_rate(io.read_bytes_per_sec),
                theme::format_bytes_rate(io.write_bytes_per_sec))
        } else {
            " (no io)".to_string()
        };

        let full_label = format!("{} {}{}", label, size_str, io_str);
        println!("  [{}]", full_label);
    }

    println!("\n=== MEMORY PANEL TITLE ===");
    let thrashing = app.thrashing_severity();
    let zram_info = if app.has_zram() {
        format!(" │ ZRAM:{:.1}x", app.zram_ratio())
    } else {
        String::new()
    };
    println!("  PSI: {:?}{}", thrashing, zram_info);
}
