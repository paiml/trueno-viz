//! Theme and color system for ttop.
//!
//! btop-style dark theme with vibrant gradients.
//! Uses perceptually uniform color transitions.

use trueno_viz::monitor::ratatui::style::Color;

/// btop-style color gradient for percentage values (0-100)
/// Uses smooth transition: cyan → green → yellow → orange → red
pub fn percent_color(percent: f64) -> Color {
    // Clamp to valid range
    let p = percent.clamp(0.0, 100.0);

    // btop-style 5-stop gradient
    if p >= 90.0 {
        // Critical: bright red (255, 64, 64)
        Color::Rgb(255, 64, 64)
    } else if p >= 75.0 {
        // High: orange-red gradient
        let t = (p - 75.0) / 15.0;
        let r = 255;
        let g = (180.0 - t * 116.0) as u8;
        let b = (64.0 + t * 0.0) as u8;
        Color::Rgb(r, g, b)
    } else if p >= 50.0 {
        // Medium-high: yellow to orange
        let t = (p - 50.0) / 25.0;
        let r = 255;
        let g = (220.0 - t * 40.0) as u8;
        let b = (64.0 + t * 0.0) as u8;
        Color::Rgb(r, g, b)
    } else if p >= 25.0 {
        // Medium-low: green to yellow
        let t = (p - 25.0) / 25.0;
        let r = (100.0 + t * 155.0) as u8;
        let g = (220.0 + t * 0.0) as u8;
        let b = (100.0 - t * 36.0) as u8;
        Color::Rgb(r, g, b)
    } else {
        // Low: cyan to green
        let t = p / 25.0;
        let r = (64.0 + t * 36.0) as u8;
        let g = (180.0 + t * 40.0) as u8;
        let b = (220.0 - t * 120.0) as u8;
        Color::Rgb(r, g, b)
    }
}

/// Temperature color gradient (Celsius)
/// Cool colors for low temps, warm for high
pub fn temp_color(temp: f64) -> Color {
    if temp > 95.0 {
        Color::Rgb(255, 0, 0) // Critical: pure red
    } else if temp > 85.0 {
        Color::Rgb(255, 50, 50) // Very hot
    } else if temp > 75.0 {
        Color::Rgb(255, 100, 50) // Hot: orange-red
    } else if temp > 65.0 {
        Color::Rgb(255, 180, 50) // Warm: orange
    } else if temp > 50.0 {
        Color::Rgb(220, 220, 80) // Normal-warm: yellow
    } else if temp > 35.0 {
        Color::Rgb(100, 220, 100) // Normal: green
    } else {
        Color::Rgb(80, 180, 220) // Cool: cyan
    }
}

/// Panel border colors - btop-style vibrant distinct colors
pub mod borders {
    use trueno_viz::monitor::ratatui::style::Color;
    use trueno_viz::monitor::ratatui::widgets::BorderType;

    // btop uses vibrant, saturated colors for borders
    pub const CPU: Color = Color::Rgb(100, 200, 255); // Bright cyan
    pub const MEMORY: Color = Color::Rgb(180, 120, 255); // Purple
    pub const DISK: Color = Color::Rgb(100, 180, 255); // Blue
    pub const NETWORK: Color = Color::Rgb(255, 150, 100); // Orange
    pub const PROCESS: Color = Color::Rgb(220, 180, 100); // Gold
    pub const GPU: Color = Color::Rgb(100, 255, 150); // Bright green
    pub const BATTERY: Color = Color::Rgb(255, 220, 100); // Yellow
    pub const SENSORS: Color = Color::Rgb(255, 100, 150); // Pink
    pub const FILES: Color = Color::Rgb(180, 140, 100); // Warm brown/amber

    /// Rounded border style for btop-like appearance
    pub const STYLE: BorderType = BorderType::Rounded;
}

/// Graph colors - high contrast for visibility
pub mod graph {
    use trueno_viz::monitor::ratatui::style::Color;

    // btop-style graph colors: bright and distinct
    pub const CPU: Color = Color::Rgb(100, 200, 255); // Bright cyan
    pub const MEMORY: Color = Color::Rgb(180, 120, 255); // Purple
    pub const SWAP: Color = Color::Rgb(255, 180, 100); // Orange
    pub const NETWORK_RX: Color = Color::Rgb(100, 200, 255); // Cyan (download)
    pub const NETWORK_TX: Color = Color::Rgb(255, 100, 100); // Red (upload)
    pub const GPU: Color = Color::Rgb(100, 255, 150); // Bright green
    pub const DISK_READ: Color = Color::Rgb(100, 180, 255); // Blue
    pub const DISK_WRITE: Color = Color::Rgb(255, 150, 100); // Orange
}

/// Process state colors
pub mod process_state {
    use trueno_viz::monitor::ratatui::style::Color;

    pub const RUNNING: Color = Color::Rgb(100, 255, 100); // Bright green
    pub const SLEEPING: Color = Color::Rgb(120, 120, 140); // Gray
    pub const DISK_WAIT: Color = Color::Rgb(255, 200, 100); // Yellow-orange
    pub const ZOMBIE: Color = Color::Rgb(255, 80, 80); // Red
    pub const STOPPED: Color = Color::Rgb(255, 150, 100); // Orange
    pub const UNKNOWN: Color = Color::Rgb(180, 180, 180); // Light gray
}

/// Format bytes to human-readable compact string (SI units).
///
/// Delegates to [`batuta_common::fmt::format_bytes_si`].
pub fn format_bytes(bytes: u64) -> String {
    batuta_common::fmt::format_bytes_si(bytes)
}

/// Format bytes per second.
///
/// Delegates to [`batuta_common::fmt::format_bytes_rate`].
pub fn format_bytes_rate(bytes_per_sec: f64) -> String {
    batuta_common::fmt::format_bytes_rate(bytes_per_sec)
}

/// Format uptime seconds to human-readable string.
///
/// Delegates to [`batuta_common::fmt::format_duration`].
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn format_uptime(secs: f64) -> String {
    batuta_common::fmt::format_duration(secs as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500B");
        assert_eq!(format_bytes(1000), "1.00K");
        assert_eq!(format_bytes(1_000_000), "1.00M");
        assert_eq!(format_bytes(1_000_000_000), "1.00G");
    }

    #[test]
    fn test_format_uptime() {
        assert_eq!(format_uptime(300.0), "5m");
        assert_eq!(format_uptime(3700.0), "1h 1m");
        assert_eq!(format_uptime(90000.0), "1d 1h");
    }

    #[test]
    fn test_percent_color_ranges() {
        // Should not panic for any valid percentage
        for p in 0..=100 {
            let _ = percent_color(p as f64);
        }
    }

    #[test]
    fn test_percent_color_gradient() {
        // Low values should have high blue component
        if let Color::Rgb(_, _, b) = percent_color(10.0) {
            assert!(b > 150, "Low percent should have blue tint");
        }

        // High values should have high red component
        if let Color::Rgb(r, _, _) = percent_color(95.0) {
            assert_eq!(r, 255, "High percent should be red");
        }
    }

    #[test]
    fn test_temp_color_gradient() {
        // Cool temps should be cyan/blue
        if let Color::Rgb(_, g, b) = temp_color(30.0) {
            assert!(g > 150 && b > 150, "Cool temp should be cyan");
        }

        // Hot temps should be red
        if let Color::Rgb(r, g, _) = temp_color(90.0) {
            assert!(r > 200 && g < 100, "Hot temp should be red");
        }
    }

    #[test]
    fn test_percent_color_handles_edge_cases() {
        // Should not panic for edge cases
        let _ = percent_color(-10.0);
        let _ = percent_color(150.0);
        let _ = percent_color(0.0);
        let _ = percent_color(100.0);
        let _ = percent_color(f64::NAN);
        let _ = percent_color(f64::INFINITY);
    }
}
