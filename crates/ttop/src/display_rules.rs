//! Display Rules System (SPEC-024 Section 28)
//!
//! Provides consistent formatting for bytes, percentages, durations, and columns.
//! All formatters guarantee NO COLUMN BLEED - output is always bounded to specified width.
//!
//! ## Key Features
//! - SI byte formatting (1.5K, 2.3M, 4.7G)
//! - IEC byte formatting (1.5KiB, 2.3MiB)
//! - Column-aligned formatting with truncation strategies
//! - Percentage formatting with clamping
//! - Duration formatting (human-readable)

use std::fmt::Write;

/// Truncation strategy for text that exceeds column width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TruncateStrategy {
    /// Truncate from the end with ellipsis: "very long te…"
    #[default]
    End,
    /// Truncate from the start with ellipsis: "…ong text here"
    Start,
    /// Truncate in the middle: "/home/…/file.txt"
    Middle,
    /// Smart path truncation: keeps filename, truncates directories
    Path,
}

/// Column alignment for formatted output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColumnAlign {
    /// Left-align text.
    #[default]
    Left,
    /// Right-align text (common for numbers).
    Right,
    /// Center-align text.
    Center,
}

// =============================================================================
// BYTE FORMATTING
// =============================================================================

/// Format bytes using SI units (powers of 1000).
///
/// # Examples
/// ```
/// use ttop::display_rules::format_bytes_si;
/// assert_eq!(format_bytes_si(0), "0B");
/// assert_eq!(format_bytes_si(500), "500B");
/// assert_eq!(format_bytes_si(1500), "1.50K");
/// assert_eq!(format_bytes_si(1_500_000), "1.50M");
/// assert_eq!(format_bytes_si(1_500_000_000), "1.50G");
/// ```
#[must_use]
pub fn format_bytes_si(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "K", "M", "G", "T", "P", "E"];
    const THRESHOLD: f64 = 1000.0;

    if bytes == 0 {
        return "0B".to_string();
    }

    let mut value = bytes as f64;
    let mut unit_idx = 0;

    while value >= THRESHOLD && unit_idx < UNITS.len() - 1 {
        value /= THRESHOLD;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{}B", bytes)
    } else if value >= 100.0 {
        format!("{:.0}{}", value, UNITS[unit_idx])
    } else if value >= 10.0 {
        format!("{:.1}{}", value, UNITS[unit_idx])
    } else {
        format!("{:.2}{}", value, UNITS[unit_idx])
    }
}

/// Format bytes using IEC units (powers of 1024).
///
/// # Examples
/// ```
/// use ttop::display_rules::format_bytes_iec;
/// assert_eq!(format_bytes_iec(0), "0B");
/// assert_eq!(format_bytes_iec(1024), "1.00KiB");
/// assert_eq!(format_bytes_iec(1536), "1.50KiB");
/// ```
#[must_use]
pub fn format_bytes_iec(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB"];
    const THRESHOLD: f64 = 1024.0;

    if bytes == 0 {
        return "0B".to_string();
    }

    let mut value = bytes as f64;
    let mut unit_idx = 0;

    while value >= THRESHOLD && unit_idx < UNITS.len() - 1 {
        value /= THRESHOLD;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{}B", bytes)
    } else {
        format!("{:.2}{}", value, UNITS[unit_idx])
    }
}

/// Format bytes into a fixed-width column.
///
/// # Examples
/// ```
/// use ttop::display_rules::format_bytes_column;
/// assert_eq!(format_bytes_column(1500, 6), " 1.50K");
/// assert_eq!(format_bytes_column(1_500_000_000, 6), " 1.50G");
/// ```
#[must_use]
pub fn format_bytes_column(bytes: u64, width: usize) -> String {
    let formatted = format_bytes_si(bytes);
    format_column(&formatted, width, ColumnAlign::Right, TruncateStrategy::End)
}

// =============================================================================
// PERCENTAGE FORMATTING
// =============================================================================

/// Format a percentage value (0.0 to 100.0).
///
/// # Examples
/// ```
/// use ttop::display_rules::format_percent;
/// assert_eq!(format_percent(45.3), "45.3%");
/// assert_eq!(format_percent(100.0), "100.0%");
/// assert_eq!(format_percent(0.0), "0.0%");
/// ```
#[must_use]
pub fn format_percent(value: f64) -> String {
    format!("{:.1}%", value)
}

/// Format a percentage with clamping to 0-100 range.
///
/// # Examples
/// ```
/// use ttop::display_rules::format_percent_clamped;
/// assert_eq!(format_percent_clamped(150.0), "100.0%");
/// assert_eq!(format_percent_clamped(-10.0), "0.0%");
/// ```
#[must_use]
pub fn format_percent_clamped(value: f64) -> String {
    format_percent(value.clamp(0.0, 100.0))
}

/// Format a percentage into a fixed-width column.
///
/// # Examples
/// ```
/// use ttop::display_rules::format_percent_column;
/// assert_eq!(format_percent_column(45.3, 7), "  45.3%");
/// ```
#[must_use]
pub fn format_percent_column(value: f64, width: usize) -> String {
    let formatted = format_percent(value);
    format_column(&formatted, width, ColumnAlign::Right, TruncateStrategy::End)
}

/// Format a percentage with fixed decimal places.
///
/// # Examples
/// ```
/// use ttop::display_rules::format_percent_fixed;
/// assert_eq!(format_percent_fixed(45.333, 2), "45.33%");
/// assert_eq!(format_percent_fixed(5.0, 1), "5.0%");
/// ```
#[must_use]
pub fn format_percent_fixed(value: f64, decimals: usize) -> String {
    format!("{:.prec$}%", value, prec = decimals)
}

// =============================================================================
// DURATION FORMATTING
// =============================================================================

/// Format a duration in seconds to human-readable form.
///
/// # Examples
/// ```
/// use ttop::display_rules::format_duration;
/// assert_eq!(format_duration(45), "45s");
/// assert_eq!(format_duration(125), "2m 5s");
/// assert_eq!(format_duration(3725), "1h 2m");
/// assert_eq!(format_duration(90061), "1d 1h");
/// ```
#[must_use]
pub fn format_duration(seconds: u64) -> String {
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;

    if seconds < MINUTE {
        format!("{}s", seconds)
    } else if seconds < HOUR {
        let mins = seconds / MINUTE;
        let secs = seconds % MINUTE;
        if secs == 0 {
            format!("{}m", mins)
        } else {
            format!("{}m {}s", mins, secs)
        }
    } else if seconds < DAY {
        let hours = seconds / HOUR;
        let mins = (seconds % HOUR) / MINUTE;
        if mins == 0 {
            format!("{}h", hours)
        } else {
            format!("{}h {}m", hours, mins)
        }
    } else {
        let days = seconds / DAY;
        let hours = (seconds % DAY) / HOUR;
        if hours == 0 {
            format!("{}d", days)
        } else {
            format!("{}d {}h", days, hours)
        }
    }
}

/// Format a duration compactly (always fixed width).
///
/// # Examples
/// ```
/// use ttop::display_rules::format_duration_compact;
/// assert_eq!(format_duration_compact(45), "   45s");
/// assert_eq!(format_duration_compact(3725), " 1h02m");
/// ```
#[must_use]
pub fn format_duration_compact(seconds: u64) -> String {
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;

    if seconds < MINUTE {
        format!("{:>5}s", seconds)
    } else if seconds < HOUR {
        let mins = seconds / MINUTE;
        let secs = seconds % MINUTE;
        format!("{:>2}m{:02}s", mins, secs)
    } else if seconds < DAY {
        let hours = seconds / HOUR;
        let mins = (seconds % HOUR) / MINUTE;
        format!("{:>2}h{:02}m", hours, mins)
    } else {
        let days = seconds / DAY;
        let hours = (seconds % DAY) / HOUR;
        format!("{:>2}d{:02}h", days, hours)
    }
}

// =============================================================================
// NUMBER FORMATTING
// =============================================================================

/// Format a number with thousands separators.
///
/// # Examples
/// ```
/// use ttop::display_rules::format_number;
/// assert_eq!(format_number(1234567), "1,234,567");
/// assert_eq!(format_number(999), "999");
/// ```
#[must_use]
pub fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    let chars: Vec<char> = s.chars().collect();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }

    result
}

/// Format a number into a fixed-width column.
#[must_use]
pub fn format_number_column(n: u64, width: usize) -> String {
    let formatted = format_number(n);
    format_column(&formatted, width, ColumnAlign::Right, TruncateStrategy::End)
}

/// Format frequency in MHz.
///
/// # Examples
/// ```
/// use ttop::display_rules::format_freq_mhz;
/// assert_eq!(format_freq_mhz(3200), "3.2GHz");
/// assert_eq!(format_freq_mhz(800), "800MHz");
/// ```
#[must_use]
pub fn format_freq_mhz(mhz: u32) -> String {
    if mhz >= 1000 {
        format!("{:.1}GHz", mhz as f64 / 1000.0)
    } else {
        format!("{}MHz", mhz)
    }
}

// =============================================================================
// COLUMN FORMATTING AND TRUNCATION
// =============================================================================

/// Truncate a string to fit within a maximum width.
///
/// # Examples
/// ```
/// use ttop::display_rules::{truncate, TruncateStrategy};
/// assert_eq!(truncate("hello world", 8, TruncateStrategy::End), "hello w…");
/// assert_eq!(truncate("hello world", 8, TruncateStrategy::Start), "…o world");
/// assert_eq!(truncate("hello world", 8, TruncateStrategy::Middle), "hel…orld");
/// assert_eq!(truncate("short", 10, TruncateStrategy::End), "short");
/// ```
#[must_use]
pub fn truncate(s: &str, max_width: usize, strategy: TruncateStrategy) -> String {
    if max_width == 0 {
        return String::new();
    }

    let char_count = s.chars().count();

    if char_count <= max_width {
        return s.to_string();
    }

    if max_width == 1 {
        return "…".to_string();
    }

    match strategy {
        TruncateStrategy::End => {
            let chars: String = s.chars().take(max_width - 1).collect();
            format!("{}…", chars)
        }
        TruncateStrategy::Start => {
            let chars: String = s.chars().skip(char_count - max_width + 1).collect();
            format!("…{}", chars)
        }
        TruncateStrategy::Middle => {
            let left_len = (max_width - 1) / 2;
            let right_len = max_width - 1 - left_len;
            let left: String = s.chars().take(left_len).collect();
            let right: String = s.chars().skip(char_count - right_len).collect();
            format!("{}…{}", left, right)
        }
        TruncateStrategy::Path => truncate_path(s, max_width),
    }
}

/// Smart path truncation that preserves the filename.
///
/// **GUARANTEE**: Output length will NEVER exceed `max_width` characters.
///
/// # Examples
/// ```
/// use ttop::display_rules::truncate_path;
/// assert_eq!(truncate_path("/home/user/documents/file.txt", 20), "/home/user…/file.txt");
/// assert_eq!(truncate_path("/a/b/c.txt", 20), "/a/b/c.txt");
/// ```
#[must_use]
pub fn truncate_path(path: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }

    let char_count = path.chars().count();

    if char_count <= max_width {
        return path.to_string();
    }

    // Find the last path separator
    if let Some(last_sep) = path.rfind('/') {
        let filename = &path[last_sep..]; // includes the leading /
        let filename_len = filename.chars().count();

        // If filename alone (with /) is too long or equals max_width, just truncate from end
        if filename_len >= max_width {
            return truncate(path, max_width, TruncateStrategy::End);
        }

        // Calculate space for directory part: max_width - filename_len - 1 (for ellipsis)
        let dir_space = max_width.saturating_sub(filename_len).saturating_sub(1);

        if dir_space == 0 {
            // Not enough room for any directory chars, just use ellipsis + filename
            // But we need to ensure we don't exceed max_width
            let result = format!("…{}", filename);
            if result.chars().count() <= max_width {
                return result;
            } else {
                return truncate(path, max_width, TruncateStrategy::End);
            }
        }

        // Get the directory part
        let dir = &path[..last_sep];
        let dir_chars: Vec<char> = dir.chars().collect();

        if dir_chars.len() <= dir_space {
            // Directory fits, but we're here because total was too long - shouldn't happen
            return path.to_string();
        }

        // Truncate directory, keeping the start
        let truncated_dir: String = dir_chars.iter().take(dir_space).collect();
        let result = format!("{}…{}", truncated_dir, filename);

        // Final safety check - ensure we never exceed max_width
        if result.chars().count() <= max_width {
            result
        } else {
            truncate(path, max_width, TruncateStrategy::End)
        }
    } else {
        truncate(path, max_width, TruncateStrategy::End)
    }
}

/// Format text into a fixed-width column with alignment and truncation.
///
/// **GUARANTEE**: Output length will NEVER exceed `width` characters.
///
/// # Examples
/// ```
/// use ttop::display_rules::{format_column, ColumnAlign, TruncateStrategy};
/// assert_eq!(format_column("test", 8, ColumnAlign::Left, TruncateStrategy::End), "test    ");
/// assert_eq!(format_column("test", 8, ColumnAlign::Right, TruncateStrategy::End), "    test");
/// assert_eq!(format_column("test", 8, ColumnAlign::Center, TruncateStrategy::End), "  test  ");
/// assert_eq!(format_column("very long text", 8, ColumnAlign::Left, TruncateStrategy::End), "very lo…");
/// ```
#[must_use]
pub fn format_column(
    text: &str,
    width: usize,
    align: ColumnAlign,
    truncate_strategy: TruncateStrategy,
) -> String {
    let char_count = text.chars().count();

    // Truncate if necessary
    let truncated = if char_count > width {
        truncate(text, width, truncate_strategy)
    } else {
        text.to_string()
    };

    let truncated_len = truncated.chars().count();
    let padding = width.saturating_sub(truncated_len);

    match align {
        ColumnAlign::Left => {
            let mut result = truncated;
            for _ in 0..padding {
                result.push(' ');
            }
            result
        }
        ColumnAlign::Right => {
            let mut result = String::with_capacity(width);
            for _ in 0..padding {
                result.push(' ');
            }
            result.push_str(&truncated);
            result
        }
        ColumnAlign::Center => {
            let left_pad = padding / 2;
            let right_pad = padding - left_pad;
            let mut result = String::with_capacity(width);
            for _ in 0..left_pad {
                result.push(' ');
            }
            result.push_str(&truncated);
            for _ in 0..right_pad {
                result.push(' ');
            }
            result
        }
    }
}

// =============================================================================
// TESTS - EXTREME TDD
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // BYTE FORMATTING TESTS
    // =========================================================================

    mod bytes_si_tests {
        use super::*;

        #[test]
        fn test_zero_bytes() {
            assert_eq!(format_bytes_si(0), "0B");
        }

        #[test]
        fn test_bytes_under_1k() {
            assert_eq!(format_bytes_si(1), "1B");
            assert_eq!(format_bytes_si(999), "999B");
            assert_eq!(format_bytes_si(500), "500B");
        }

        #[test]
        fn test_kilobytes() {
            assert_eq!(format_bytes_si(1000), "1.00K");
            assert_eq!(format_bytes_si(1500), "1.50K");
            assert_eq!(format_bytes_si(10_000), "10.0K");
            assert_eq!(format_bytes_si(100_000), "100K");
            assert_eq!(format_bytes_si(999_000), "999K");
        }

        #[test]
        fn test_megabytes() {
            assert_eq!(format_bytes_si(1_000_000), "1.00M");
            assert_eq!(format_bytes_si(1_500_000), "1.50M");
            assert_eq!(format_bytes_si(10_000_000), "10.0M");
            assert_eq!(format_bytes_si(100_000_000), "100M");
        }

        #[test]
        fn test_gigabytes() {
            assert_eq!(format_bytes_si(1_000_000_000), "1.00G");
            assert_eq!(format_bytes_si(1_500_000_000), "1.50G");
            assert_eq!(format_bytes_si(10_000_000_000), "10.0G");
        }

        #[test]
        fn test_terabytes() {
            assert_eq!(format_bytes_si(1_000_000_000_000), "1.00T");
            assert_eq!(format_bytes_si(2_500_000_000_000), "2.50T");
        }

        #[test]
        fn test_petabytes() {
            assert_eq!(format_bytes_si(1_000_000_000_000_000), "1.00P");
        }

        #[test]
        fn test_exabytes() {
            assert_eq!(format_bytes_si(1_000_000_000_000_000_000), "1.00E");
        }
    }

    mod bytes_iec_tests {
        use super::*;

        #[test]
        fn test_zero_bytes_iec() {
            assert_eq!(format_bytes_iec(0), "0B");
        }

        #[test]
        fn test_bytes_under_1kib() {
            assert_eq!(format_bytes_iec(1), "1B");
            assert_eq!(format_bytes_iec(1023), "1023B");
        }

        #[test]
        fn test_kibibytes() {
            assert_eq!(format_bytes_iec(1024), "1.00KiB");
            assert_eq!(format_bytes_iec(1536), "1.50KiB");
            assert_eq!(format_bytes_iec(10240), "10.00KiB");
        }

        #[test]
        fn test_mebibytes() {
            assert_eq!(format_bytes_iec(1024 * 1024), "1.00MiB");
            assert_eq!(format_bytes_iec(1024 * 1024 + 512 * 1024), "1.50MiB");
        }

        #[test]
        fn test_gibibytes() {
            assert_eq!(format_bytes_iec(1024 * 1024 * 1024), "1.00GiB");
        }

        #[test]
        fn test_tebibytes() {
            assert_eq!(format_bytes_iec(1024_u64 * 1024 * 1024 * 1024), "1.00TiB");
        }
    }

    mod bytes_column_tests {
        use super::*;

        #[test]
        fn test_bytes_column_right_aligned() {
            let result = format_bytes_column(1500, 6);
            assert_eq!(result.len(), 6);
            assert_eq!(result, " 1.50K");
        }

        #[test]
        fn test_bytes_column_small_value() {
            let result = format_bytes_column(500, 6);
            assert_eq!(result.len(), 6);
            assert_eq!(result, "  500B");
        }

        #[test]
        fn test_bytes_column_large_value() {
            let result = format_bytes_column(1_500_000_000, 6);
            assert_eq!(result.len(), 6);
            assert_eq!(result, " 1.50G");
        }
    }

    // =========================================================================
    // PERCENTAGE FORMATTING TESTS
    // =========================================================================

    mod percent_tests {
        use super::*;

        #[test]
        fn test_format_percent_basic() {
            assert_eq!(format_percent(0.0), "0.0%");
            assert_eq!(format_percent(50.0), "50.0%");
            assert_eq!(format_percent(100.0), "100.0%");
        }

        #[test]
        fn test_format_percent_decimal() {
            assert_eq!(format_percent(45.3), "45.3%");
            assert_eq!(format_percent(99.9), "99.9%");
        }

        #[test]
        fn test_format_percent_clamped() {
            assert_eq!(format_percent_clamped(150.0), "100.0%");
            assert_eq!(format_percent_clamped(-10.0), "0.0%");
            assert_eq!(format_percent_clamped(50.0), "50.0%");
        }

        #[test]
        fn test_format_percent_column() {
            let result = format_percent_column(45.3, 7);
            assert_eq!(result.len(), 7);
            assert_eq!(result, "  45.3%");
        }

        #[test]
        fn test_format_percent_fixed() {
            assert_eq!(format_percent_fixed(45.333, 2), "45.33%");
            assert_eq!(format_percent_fixed(5.0, 1), "5.0%");
            assert_eq!(format_percent_fixed(99.999, 0), "100%");
        }
    }

    // =========================================================================
    // DURATION FORMATTING TESTS
    // =========================================================================

    mod duration_tests {
        use super::*;

        #[test]
        fn test_format_duration_seconds() {
            assert_eq!(format_duration(0), "0s");
            assert_eq!(format_duration(1), "1s");
            assert_eq!(format_duration(45), "45s");
            assert_eq!(format_duration(59), "59s");
        }

        #[test]
        fn test_format_duration_minutes() {
            assert_eq!(format_duration(60), "1m");
            assert_eq!(format_duration(61), "1m 1s");
            assert_eq!(format_duration(125), "2m 5s");
            assert_eq!(format_duration(3599), "59m 59s");
        }

        #[test]
        fn test_format_duration_hours() {
            assert_eq!(format_duration(3600), "1h");
            assert_eq!(format_duration(3660), "1h 1m");
            assert_eq!(format_duration(3725), "1h 2m");
            assert_eq!(format_duration(7200), "2h");
        }

        #[test]
        fn test_format_duration_days() {
            assert_eq!(format_duration(86400), "1d");
            assert_eq!(format_duration(90000), "1d 1h");
            assert_eq!(format_duration(90061), "1d 1h");
            assert_eq!(format_duration(172800), "2d");
        }

        #[test]
        fn test_format_duration_compact() {
            assert_eq!(format_duration_compact(45), "   45s");
            assert_eq!(format_duration_compact(125), " 2m05s");
            assert_eq!(format_duration_compact(3725), " 1h02m");
            assert_eq!(format_duration_compact(90061), " 1d01h");
        }

        #[test]
        fn test_format_duration_compact_length() {
            // All compact durations should be 6 characters
            assert_eq!(format_duration_compact(0).len(), 6);
            assert_eq!(format_duration_compact(59).len(), 6);
            assert_eq!(format_duration_compact(3599).len(), 6);
            assert_eq!(format_duration_compact(86399).len(), 6);
            assert_eq!(format_duration_compact(86400).len(), 6);
        }
    }

    // =========================================================================
    // NUMBER FORMATTING TESTS
    // =========================================================================

    mod number_tests {
        use super::*;

        #[test]
        fn test_format_number_small() {
            assert_eq!(format_number(0), "0");
            assert_eq!(format_number(1), "1");
            assert_eq!(format_number(999), "999");
        }

        #[test]
        fn test_format_number_thousands() {
            assert_eq!(format_number(1000), "1,000");
            assert_eq!(format_number(1234), "1,234");
            assert_eq!(format_number(999999), "999,999");
        }

        #[test]
        fn test_format_number_millions() {
            assert_eq!(format_number(1000000), "1,000,000");
            assert_eq!(format_number(1234567), "1,234,567");
        }

        #[test]
        fn test_format_number_column() {
            let result = format_number_column(1234567, 12);
            assert_eq!(result.len(), 12);
            assert_eq!(result, "   1,234,567");
        }

        #[test]
        fn test_format_freq_mhz() {
            assert_eq!(format_freq_mhz(800), "800MHz");
            assert_eq!(format_freq_mhz(999), "999MHz");
            assert_eq!(format_freq_mhz(1000), "1.0GHz");
            assert_eq!(format_freq_mhz(3200), "3.2GHz");
            assert_eq!(format_freq_mhz(4500), "4.5GHz");
        }
    }

    // =========================================================================
    // TRUNCATION TESTS
    // =========================================================================

    mod truncation_tests {
        use super::*;

        #[test]
        fn test_truncate_no_truncation_needed() {
            assert_eq!(truncate("short", 10, TruncateStrategy::End), "short");
            assert_eq!(truncate("exact", 5, TruncateStrategy::End), "exact");
        }

        #[test]
        fn test_truncate_end() {
            assert_eq!(truncate("hello world", 8, TruncateStrategy::End), "hello w…");
            assert_eq!(
                truncate("very long text here", 10, TruncateStrategy::End),
                "very long…"
            );
        }

        #[test]
        fn test_truncate_start() {
            assert_eq!(
                truncate("hello world", 8, TruncateStrategy::Start),
                "…o world"
            );
            assert_eq!(
                truncate("very long text here", 10, TruncateStrategy::Start),
                "…text here"
            );
        }

        #[test]
        fn test_truncate_middle() {
            assert_eq!(
                truncate("hello world", 8, TruncateStrategy::Middle),
                "hel…orld"
            );
            assert_eq!(
                truncate("abcdefghij", 7, TruncateStrategy::Middle),
                "abc…hij"
            );
        }

        #[test]
        fn test_truncate_very_short_width() {
            assert_eq!(truncate("hello", 1, TruncateStrategy::End), "…");
            assert_eq!(truncate("hello", 2, TruncateStrategy::End), "h…");
        }

        #[test]
        fn test_truncate_unicode() {
            assert_eq!(truncate("héllo wörld", 8, TruncateStrategy::End), "héllo w…");
        }
    }

    mod path_truncation_tests {
        use super::*;

        #[test]
        fn test_truncate_path_no_truncation() {
            assert_eq!(truncate_path("/a/b/c.txt", 20), "/a/b/c.txt");
        }

        #[test]
        fn test_truncate_path_long_dir() {
            // "/home/user/documents/file.txt" = 29 chars, max = 20
            // filename = "/file.txt" = 9 chars
            // dir_space = 20 - 9 - 1 = 10 chars for directory
            let result = truncate_path("/home/user/documents/file.txt", 20);
            assert_eq!(result.chars().count(), 20);
            assert!(result.contains("…"));
            assert!(result.ends_with("/file.txt"));
        }

        #[test]
        fn test_truncate_path_very_long() {
            let result = truncate_path("/very/long/path/to/some/deeply/nested/file.txt", 25);
            assert!(
                result.chars().count() <= 25,
                "Result '{}' has {} chars, expected <= 25",
                result,
                result.chars().count()
            );
            assert!(result.contains("…") || result.chars().count() <= 25);
        }

        #[test]
        fn test_truncate_path_no_separator() {
            assert_eq!(truncate_path("verylongfilename.txt", 10), "verylongf…");
        }

        #[test]
        fn test_truncate_path_filename_too_long() {
            let result = truncate_path("/dir/verylongfilename.txt", 15);
            assert!(
                result.chars().count() <= 15,
                "Result '{}' has {} chars, expected <= 15",
                result,
                result.chars().count()
            );
        }

        #[test]
        fn test_truncate_path_guarantees_width() {
            // Test various paths with various widths to ensure NO BLEED
            let paths = [
                "/home/user/documents/file.txt",
                "/a/b/c/d/e/f/g.txt",
                "/very/deeply/nested/path/structure/here/file.txt",
                "no_separator.txt",
            ];

            for path in &paths {
                for width in 5..30 {
                    let result = truncate_path(path, width);
                    assert!(
                        result.chars().count() <= width,
                        "Path '{}' with width {} produced '{}' ({} chars)",
                        path,
                        width,
                        result,
                        result.chars().count()
                    );
                }
            }
        }
    }

    // =========================================================================
    // COLUMN FORMATTING TESTS
    // =========================================================================

    mod column_tests {
        use super::*;

        #[test]
        fn test_format_column_left() {
            assert_eq!(
                format_column("test", 8, ColumnAlign::Left, TruncateStrategy::End),
                "test    "
            );
        }

        #[test]
        fn test_format_column_right() {
            assert_eq!(
                format_column("test", 8, ColumnAlign::Right, TruncateStrategy::End),
                "    test"
            );
        }

        #[test]
        fn test_format_column_center() {
            assert_eq!(
                format_column("test", 8, ColumnAlign::Center, TruncateStrategy::End),
                "  test  "
            );
            // Odd padding goes to right
            assert_eq!(
                format_column("ab", 5, ColumnAlign::Center, TruncateStrategy::End),
                " ab  "
            );
        }

        #[test]
        fn test_format_column_truncates() {
            assert_eq!(
                format_column("very long text", 8, ColumnAlign::Left, TruncateStrategy::End),
                "very lo…"
            );
        }

        #[test]
        fn test_format_column_exact_width() {
            assert_eq!(
                format_column("exact", 5, ColumnAlign::Left, TruncateStrategy::End),
                "exact"
            );
        }

        #[test]
        fn test_format_column_guarantees_width() {
            // Test that output NEVER exceeds width
            let inputs = ["short", "exactly_ten", "this is a very long string that should be truncated"];
            let widths = [5, 10, 15, 20];

            for input in &inputs {
                for &width in &widths {
                    for align in [ColumnAlign::Left, ColumnAlign::Right, ColumnAlign::Center] {
                        let result =
                            format_column(input, width, align, TruncateStrategy::End);
                        assert!(
                            result.chars().count() <= width,
                            "Output '{}' exceeds width {} for input '{}'",
                            result,
                            width,
                            input
                        );
                    }
                }
            }
        }
    }

    // =========================================================================
    // EDGE CASE TESTS
    // =========================================================================

    mod edge_cases {
        use super::*;

        #[test]
        fn test_empty_string_truncate() {
            assert_eq!(truncate("", 5, TruncateStrategy::End), "");
        }

        #[test]
        fn test_empty_string_column() {
            assert_eq!(
                format_column("", 5, ColumnAlign::Left, TruncateStrategy::End),
                "     "
            );
        }

        #[test]
        fn test_zero_width_column() {
            assert_eq!(
                format_column("test", 0, ColumnAlign::Left, TruncateStrategy::End),
                ""
            );
        }

        #[test]
        fn test_large_numbers() {
            assert_eq!(format_bytes_si(u64::MAX), "18.4E");
            let _ = format_number(u64::MAX); // Just ensure no panic
        }

        #[test]
        fn test_negative_percent_clamping() {
            assert_eq!(format_percent_clamped(f64::NEG_INFINITY), "0.0%");
        }

        #[test]
        fn test_infinity_percent_clamping() {
            assert_eq!(format_percent_clamped(f64::INFINITY), "100.0%");
        }

        #[test]
        fn test_nan_percent() {
            let result = format_percent(f64::NAN);
            assert!(result.contains("NaN"));
        }
    }
}
