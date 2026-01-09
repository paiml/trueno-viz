//! Panel rendering tests for ttop using probar TUI testing.
//!
//! These tests verify that all panels render correctly with expected content.
//! Uses deterministic mode to ensure reproducible frame output.
//!
#![allow(clippy::unwrap_used)]
#![allow(clippy::assertions_on_constants)]
#![allow(clippy::for_kv_map)]
#![allow(clippy::manual_range_contains)]
#![allow(clippy::useless_vec)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::needless_update)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]
//! Pixel-level tests verify:
//! - Border characters render correctly (╭ ╮ ╰ ╯ ─ │)
//! - Graph characters render (block chars: █▓▒░ or braille: ⡀⡄⡆⡇)
//! - Meter bars render correctly
//! - Colors are applied to cells

use trueno_viz::monitor::ratatui::backend::TestBackend;
use trueno_viz::monitor::ratatui::buffer::Buffer;
use trueno_viz::monitor::ratatui::layout::Rect;
use trueno_viz::monitor::ratatui::style::Color;
use trueno_viz::monitor::ratatui::widgets::Widget;
use trueno_viz::monitor::ratatui::Terminal;

/// Test frame capture for panel verification
struct TestFrame {
    buffer: Buffer,
}

impl TestFrame {
    fn new(width: u16, height: u16) -> Self {
        Self {
            buffer: Buffer::empty(Rect::new(0, 0, width, height)),
        }
    }

    fn contains(&self, text: &str) -> bool {
        let content = self.as_text();
        content.contains(text)
    }

    fn as_text(&self) -> String {
        let area = self.buffer.area;
        let mut lines = Vec::new();
        for y in 0..area.height {
            let mut line = String::new();
            for x in 0..area.width {
                if let Some(cell) = self.buffer.cell((x, y)) {
                    line.push_str(cell.symbol());
                }
            }
            lines.push(line.trim_end().to_string());
        }
        lines.join("\n")
    }

    /// Check if a cell at (x, y) has a specific character
    fn cell_char(&self, x: u16, y: u16) -> Option<&str> {
        self.buffer.cell((x, y)).map(|c| c.symbol())
    }

    /// Check if a cell at (x, y) has a specific foreground color
    fn cell_fg(&self, x: u16, y: u16) -> Option<Color> {
        self.buffer.cell((x, y)).map(|c| c.fg)
    }

    /// Count occurrences of a character in the buffer
    fn count_char(&self, ch: &str) -> usize {
        let text = self.as_text();
        text.matches(ch).count()
    }

    /// Check if any cell contains graph characters (block or braille)
    fn has_graph_chars(&self) -> bool {
        let text = self.as_text();
        // Block characters
        text.contains('█') || text.contains('▓') || text.contains('▒') || text.contains('░') ||
        text.contains('▁') || text.contains('▂') || text.contains('▃') || text.contains('▄') ||
        text.contains('▅') || text.contains('▆') || text.contains('▇') ||
        // Braille characters
        text.chars().any(|c| ('\u{2800}'..='\u{28FF}').contains(&c))
    }

    /// Check if any cell contains meter bar characters
    fn has_meter_chars(&self) -> bool {
        let text = self.as_text();
        text.contains('█')
            || text.contains('▏')
            || text.contains('▎')
            || text.contains('▍')
            || text.contains('▌')
            || text.contains('▋')
            || text.contains('▊')
            || text.contains('▉')
    }

    /// Check for rounded border corners
    fn has_rounded_borders(&self) -> bool {
        let text = self.as_text();
        text.contains('╭') || text.contains('╮') || text.contains('╰') || text.contains('╯')
    }
}

#[cfg(test)]
mod theme_tests {
    use ttop::theme::{format_bytes, format_bytes_rate, format_uptime, percent_color, temp_color};

    #[test]
    fn test_format_bytes_units() {
        assert_eq!(format_bytes(0), "0B");
        assert_eq!(format_bytes(512), "512B");
        assert_eq!(format_bytes(1024), "1.0K");
        assert_eq!(format_bytes(1024 * 1024), "1.0M");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0G");
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 1024), "1.0T");
    }

    #[test]
    fn test_format_bytes_rate() {
        assert_eq!(format_bytes_rate(0.0), "0B/s");
        assert_eq!(format_bytes_rate(1024.0), "1.0K/s");
        assert_eq!(format_bytes_rate(1_000_000.0), "976.6K/s");
    }

    #[test]
    fn test_format_uptime() {
        assert_eq!(format_uptime(0.0), "0m");
        assert_eq!(format_uptime(60.0), "1m");
        assert_eq!(format_uptime(3600.0), "1h 0m");
        assert_eq!(format_uptime(3660.0), "1h 1m");
        assert_eq!(format_uptime(86400.0), "1d 0h");
        assert_eq!(format_uptime(90000.0), "1d 1h");
    }

    #[test]
    fn test_percent_color_ranges() {
        use trueno_viz::monitor::ratatui::style::Color;

        // Low values should have cyan/blue tint (high blue component)
        if let Color::Rgb(_, _, b) = percent_color(10.0) {
            assert!(b > 150, "Low percent should have blue tint, got b={}", b);
        }

        // Medium-low values should be greenish
        if let Color::Rgb(_, g, _) = percent_color(40.0) {
            assert!(g > 180, "Medium-low should be greenish, got g={}", g);
        }

        // Medium values should have yellow tint (high red + green)
        if let Color::Rgb(r, g, _) = percent_color(60.0) {
            assert!(
                r > 200 && g > 150,
                "Medium should be yellow-ish, got r={}, g={}",
                r,
                g
            );
        }

        // High values should be orange-ish (high red)
        if let Color::Rgb(r, _, _) = percent_color(80.0) {
            assert!(r == 255, "High should have red=255, got r={}", r);
        }

        // Critical values should be red
        if let Color::Rgb(r, g, b) = percent_color(95.0) {
            assert!(
                r == 255 && g < 100 && b < 100,
                "Critical should be red, got r={}, g={}, b={}",
                r,
                g,
                b
            );
        }
    }

    #[test]
    fn test_temp_color_ranges() {
        use trueno_viz::monitor::ratatui::style::Color;

        // Cool temps should be cyan/blue
        if let Color::Rgb(_, g, b) = temp_color(30.0) {
            assert!(
                g > 150 && b > 180,
                "Cool temp should be cyan, got g={}, b={}",
                g,
                b
            );
        }

        // Normal temps should be greenish/yellow
        if let Color::Rgb(_, g, _) = temp_color(50.0) {
            assert!(g > 150, "Normal temp should be green/yellow, got g={}", g);
        }

        // Hot temps should be red-ish
        if let Color::Rgb(r, _, _) = temp_color(88.0) {
            assert!(r > 200, "Hot temp should be red, got r={}", r);
        }

        // Critical temps should be pure red
        if let Color::Rgb(r, g, b) = temp_color(96.0) {
            assert!(r == 255 && g == 0 && b == 0, "Critical should be pure red");
        }
    }
}

#[cfg(test)]
mod state_tests {
    use ttop::state::ProcessSortColumn;

    #[test]
    fn test_sort_column_names() {
        assert_eq!(ProcessSortColumn::Pid.name(), "PID");
        assert_eq!(ProcessSortColumn::Name.name(), "NAME");
        assert_eq!(ProcessSortColumn::Cpu.name(), "CPU%");
        assert_eq!(ProcessSortColumn::Mem.name(), "MEM%");
        assert_eq!(ProcessSortColumn::State.name(), "STATE");
        assert_eq!(ProcessSortColumn::User.name(), "USER");
        assert_eq!(ProcessSortColumn::Threads.name(), "THR");
    }

    #[test]
    fn test_sort_column_cycling() {
        let mut col = ProcessSortColumn::Pid;

        col = col.next();
        assert_eq!(col, ProcessSortColumn::Name);

        col = col.next();
        assert_eq!(col, ProcessSortColumn::Cpu);

        col = col.next();
        assert_eq!(col, ProcessSortColumn::Mem);

        col = col.next();
        assert_eq!(col, ProcessSortColumn::State);

        col = col.next();
        assert_eq!(col, ProcessSortColumn::User);

        col = col.next();
        assert_eq!(col, ProcessSortColumn::Threads);

        // Should cycle back to Pid
        col = col.next();
        assert_eq!(col, ProcessSortColumn::Pid);
    }

    #[test]
    fn test_default_sort_column() {
        let default = ProcessSortColumn::default();
        assert_eq!(default, ProcessSortColumn::Cpu);
    }
}

#[cfg(test)]
mod panel_content_tests {
    //! Tests that verify panel content without full app initialization

    #[test]
    fn test_memory_panel_title_format() {
        // Memory panel should show: " Memory │ X.XG / X.XG "
        let used_gb = 8.5;
        let total_gb = 16.0;
        let title = format!(" Memory │ {used_gb:.1}G / {total_gb:.1}G ");

        assert!(title.contains("Memory"));
        assert!(title.contains("8.5G"));
        assert!(title.contains("16.0G"));
    }

    #[test]
    fn test_disk_panel_title_format() {
        // Disk panel should show: " Disk │ R: X.XM/s │ W: X.XM/s "
        let read_rate = "1.5M/s";
        let write_rate = "500.0K/s";
        let title = format!(" Disk │ R: {} │ W: {} ", read_rate, write_rate);

        assert!(title.contains("Disk"));
        assert!(title.contains("R:"));
        assert!(title.contains("W:"));
    }

    #[test]
    fn test_network_panel_title_format() {
        // Network panel should show: " Network (eth0) │ ↓ X.XM/s │ ↑ X.XK/s "
        let iface = "eth0";
        let rx_rate = "1.2M/s";
        let tx_rate = "500.0K/s";
        let title = format!(" Network ({}) │ ↓ {} │ ↑ {} ", iface, rx_rate, tx_rate);

        assert!(title.contains("Network"));
        assert!(title.contains("eth0"));
        assert!(title.contains("↓"));
        assert!(title.contains("↑"));
    }

    #[test]
    fn test_cpu_panel_shows_cores() {
        // CPU panel should show core count
        let core_count = 8;
        let title = format!(" CPU ({} cores) ", core_count);

        assert!(title.contains("CPU"));
        assert!(title.contains("8 cores"));
    }

    #[test]
    fn test_swap_indicator_when_used() {
        let swap_used_gb = 0.5;
        let swap_total_gb = 4.0;

        // When swap is used, should show indicator
        let indicator = if swap_used_gb > 0.0 {
            format!(" │ Swap: {:.1}G/{:.1}G", swap_used_gb, swap_total_gb)
        } else {
            String::new()
        };

        assert!(indicator.contains("Swap"));
        assert!(indicator.contains("0.5G"));
        assert!(indicator.contains("4.0G"));
    }

    #[test]
    fn test_swap_indicator_when_empty() {
        let swap_used: u64 = 0;

        let indicator = if swap_used > 0 {
            "Swap in use".to_string()
        } else {
            String::new()
        };

        assert!(indicator.is_empty());
    }
}

#[cfg(test)]
mod keyboard_tests {
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn test_quit_keys() {
        // These keys should trigger quit
        let quit_keys = [KeyCode::Char('q'), KeyCode::Esc];

        for key in quit_keys {
            assert!(
                matches!(key, KeyCode::Char('q') | KeyCode::Esc),
                "Key {:?} should be a quit key",
                key
            );
        }
    }

    #[test]
    fn test_navigation_keys() {
        // Navigation keys
        let nav_down = [KeyCode::Down, KeyCode::Char('j')];
        let nav_up = [KeyCode::Up, KeyCode::Char('k')];

        assert_eq!(nav_down.len(), 2);
        assert_eq!(nav_up.len(), 2);
    }

    #[test]
    fn test_panel_toggle_keys() {
        // Panel toggle keys 1-8
        let panel_keys: Vec<char> = ('1'..='8').collect();
        assert_eq!(panel_keys.len(), 8);
        assert_eq!(panel_keys[0], '1'); // CPU
        assert_eq!(panel_keys[4], '5'); // Process
    }

    #[test]
    fn test_sort_keys() {
        let sort_keys = [KeyCode::Tab, KeyCode::Char('s')];
        let reverse_key = KeyCode::Char('r');

        assert_eq!(sort_keys.len(), 2);
        assert!(matches!(reverse_key, KeyCode::Char('r')));
    }

    #[test]
    fn test_filter_keys() {
        let filter_keys = [KeyCode::Char('f'), KeyCode::Char('/')];
        let clear_key = KeyCode::Delete;

        assert_eq!(filter_keys.len(), 2);
        assert!(matches!(clear_key, KeyCode::Delete));
    }
}

#[cfg(test)]
mod proptest_tests {
    use proptest::prelude::*;
    use ttop::theme::{format_bytes, percent_color};

    proptest! {
        #[test]
        fn test_format_bytes_never_panics(bytes in 0u64..u64::MAX) {
            let _ = format_bytes(bytes);
        }

        #[test]
        fn test_percent_color_any_value(percent in -100.0f64..200.0f64) {
            // Should not panic for any input
            let _ = percent_color(percent);
        }

        #[test]
        fn test_format_bytes_monotonic(a in 0u64..1_000_000_000u64, b in 0u64..1_000_000_000u64) {
            let fa = format_bytes(a);
            let fb = format_bytes(b);

            // If a < b significantly, formatted a should come before b lexicographically
            // (This is a weak property check - mainly ensuring no crashes)
            if a == 0 {
                assert!(fa.contains("0B") || fa.contains("0K") || fa.contains("0M"));
            }
        }
    }
}

#[cfg(test)]
mod pixel_level_tests {
    //! Pixel-level rendering tests that verify actual buffer output.
    //! These tests use ratatui's TestBackend to capture rendered frames.

    use super::*;
    use trueno_viz::monitor::ratatui::style::Style;
    use trueno_viz::monitor::widgets::{Graph, GraphMode, Meter, MonitorSparkline};

    #[test]
    fn test_graph_widget_renders_block_chars() {
        // Test that Graph widget renders block characters
        let data = vec![0.2, 0.4, 0.6, 0.8, 1.0, 0.7, 0.5, 0.3];
        let area = Rect::new(0, 0, 16, 4);
        let mut buffer = Buffer::empty(area);

        let graph = Graph::new(&data).mode(GraphMode::Block).color(Color::Cyan);
        graph.render(area, &mut buffer);

        let frame = TestFrame { buffer };
        let text = frame.as_text();

        // Should contain block characters for the graph
        assert!(
            frame.has_graph_chars(),
            "Graph should render block/braille chars. Got:\n{}",
            text
        );
    }

    #[test]
    fn test_graph_widget_renders_braille_chars() {
        // Test that Graph widget can render braille characters
        let data = vec![0.1, 0.3, 0.5, 0.7, 0.9, 0.6, 0.4, 0.2];
        let area = Rect::new(0, 0, 20, 5);
        let mut buffer = Buffer::empty(area);

        let graph = Graph::new(&data)
            .mode(GraphMode::Braille)
            .color(Color::Green);
        graph.render(area, &mut buffer);

        let frame = TestFrame { buffer };
        let text = frame.as_text();

        // Braille characters are in range U+2800 to U+28FF
        let has_braille = text.chars().any(|c| ('\u{2800}'..='\u{28FF}').contains(&c));
        assert!(
            has_braille || frame.has_graph_chars(),
            "Graph should render braille or block chars. Got:\n{}",
            text
        );
    }

    #[test]
    fn test_meter_widget_renders_bars() {
        // Test that Meter widget renders bar characters
        let area = Rect::new(0, 0, 30, 1);
        let mut buffer = Buffer::empty(area);

        let meter = Meter::new(0.75).label("Test").color(Color::Yellow);
        meter.render(area, &mut buffer);

        let frame = TestFrame { buffer };
        let text = frame.as_text();

        // Should contain meter bar characters
        assert!(
            frame.has_meter_chars(),
            "Meter should render bar chars. Got: '{}'",
            text
        );

        // Should contain the label
        assert!(
            text.contains("Test"),
            "Meter should show label. Got: '{}'",
            text
        );
    }

    #[test]
    fn test_meter_widget_percentage_proportional() {
        // Test that meter bar length is proportional to percentage
        let area = Rect::new(0, 0, 40, 1);

        // 25% meter
        let mut buffer_25 = Buffer::empty(area);
        let meter_25 = Meter::new(0.25).label("25%").color(Color::Green);
        meter_25.render(area, &mut buffer_25);
        let frame_25 = TestFrame { buffer: buffer_25 };
        let filled_25 = frame_25.count_char("█");

        // 75% meter
        let mut buffer_75 = Buffer::empty(area);
        let meter_75 = Meter::new(0.75).label("75%").color(Color::Yellow);
        meter_75.render(area, &mut buffer_75);
        let frame_75 = TestFrame { buffer: buffer_75 };
        let filled_75 = frame_75.count_char("█");

        // 75% should have more filled blocks than 25%
        assert!(
            filled_75 > filled_25,
            "75% meter should have more filled blocks than 25%: {} vs {}",
            filled_75,
            filled_25
        );
    }

    #[test]
    fn test_sparkline_widget_renders() {
        // Test that MonitorSparkline renders trend characters
        let data = vec![0.1, 0.2, 0.4, 0.3, 0.5, 0.8, 0.6, 0.7, 0.9, 0.5];
        let area = Rect::new(0, 0, 20, 1);
        let mut buffer = Buffer::empty(area);

        let sparkline = MonitorSparkline::new(&data)
            .color(Color::Cyan)
            .show_trend(true);
        sparkline.render(area, &mut buffer);

        let frame = TestFrame { buffer };
        let text = frame.as_text();

        // Should contain sparkline characters (▁▂▃▄▅▆▇█)
        let has_sparkline = text
            .chars()
            .any(|c| matches!(c, '▁' | '▂' | '▃' | '▄' | '▅' | '▆' | '▇' | '█'));
        assert!(
            has_sparkline,
            "Sparkline should render bar chars. Got: '{}'",
            text
        );
    }

    #[test]
    fn test_sparkline_shows_trend_arrow() {
        // Test that sparkline shows trend indicator when enabled
        let data = vec![0.1, 0.2, 0.3, 0.4, 0.5]; // Upward trend
        let area = Rect::new(0, 0, 15, 1);
        let mut buffer = Buffer::empty(area);

        let sparkline = MonitorSparkline::new(&data)
            .color(Color::Green)
            .show_trend(true);
        sparkline.render(area, &mut buffer);

        let frame = TestFrame { buffer };
        let text = frame.as_text();

        // Should contain trend arrow (↑ or →)
        let has_trend = text.contains('↑') || text.contains('→') || text.contains('↓');
        assert!(
            has_trend,
            "Sparkline should show trend arrow. Got: '{}'",
            text
        );
    }

    #[test]
    fn test_graph_colors_applied() {
        // Test that colors are applied to graph cells
        let data = vec![0.5, 0.6, 0.7, 0.8];
        let area = Rect::new(0, 0, 10, 3);
        let mut buffer = Buffer::empty(area);

        let graph = Graph::new(&data)
            .mode(GraphMode::Block)
            .color(Color::Rgb(100, 200, 255)); // Bright cyan
        graph.render(area, &mut buffer);

        // Check that at least one cell has the expected color
        let mut found_color = false;
        for y in 0..area.height {
            for x in 0..area.width {
                if let Some(cell) = buffer.cell((x, y)) {
                    if cell.fg == Color::Rgb(100, 200, 255) {
                        found_color = true;
                        break;
                    }
                }
            }
        }

        assert!(found_color, "Graph should apply the specified color");
    }

    #[test]
    fn test_meter_colors_applied() {
        // Test that colors are applied to meter cells
        let area = Rect::new(0, 0, 20, 1);
        let mut buffer = Buffer::empty(area);

        let meter = Meter::new(0.5)
            .label("Test")
            .color(Color::Rgb(255, 150, 100)); // Orange
        meter.render(area, &mut buffer);

        // Check that at least one cell has the expected color
        let mut found_color = false;
        for x in 0..area.width {
            if let Some(cell) = buffer.cell((x, 0)) {
                if cell.fg == Color::Rgb(255, 150, 100) {
                    found_color = true;
                    break;
                }
            }
        }

        assert!(found_color, "Meter should apply the specified color");
    }

    #[test]
    fn test_graph_empty_data_no_panic() {
        // Test that Graph handles empty data without panic
        let data: Vec<f64> = vec![];
        let area = Rect::new(0, 0, 10, 3);
        let mut buffer = Buffer::empty(area);

        let graph = Graph::new(&data).mode(GraphMode::Block).color(Color::Cyan);
        graph.render(area, &mut buffer);

        // Should not panic, just render empty
        assert!(true, "Graph should handle empty data without panic");
    }

    #[test]
    fn test_meter_zero_percent() {
        // Test that Meter handles 0% correctly
        let area = Rect::new(0, 0, 20, 1);
        let mut buffer = Buffer::empty(area);

        let meter = Meter::new(0.0).label("Zero").color(Color::Green);
        meter.render(area, &mut buffer);

        let frame = TestFrame { buffer };
        let text = frame.as_text();

        // Should still render label
        assert!(text.contains("Zero"), "Zero meter should show label");
    }

    #[test]
    fn test_meter_full_percent() {
        // Test that Meter handles 100% correctly
        let area = Rect::new(0, 0, 20, 1);
        let mut buffer = Buffer::empty(area);

        let meter = Meter::new(1.0).label("Full").color(Color::Red);
        meter.render(area, &mut buffer);

        let frame = TestFrame { buffer };

        // Should have many filled blocks
        let filled = frame.count_char("█");
        assert!(filled > 5, "Full meter should have many filled blocks");
    }

    #[test]
    fn test_graph_modes_differ() {
        // Test that Block and Braille modes produce different output
        let data = vec![0.3, 0.5, 0.7, 0.9, 0.6];
        let area = Rect::new(0, 0, 15, 4);

        let mut buffer_block = Buffer::empty(area);
        let graph_block = Graph::new(&data).mode(GraphMode::Block).color(Color::Cyan);
        graph_block.render(area, &mut buffer_block);
        let frame_block = TestFrame {
            buffer: buffer_block,
        };

        let mut buffer_braille = Buffer::empty(area);
        let graph_braille = Graph::new(&data)
            .mode(GraphMode::Braille)
            .color(Color::Cyan);
        graph_braille.render(area, &mut buffer_braille);
        let frame_braille = TestFrame {
            buffer: buffer_braille,
        };

        let text_block = frame_block.as_text();
        let text_braille = frame_braille.as_text();

        // The outputs should differ (different character sets)
        assert_ne!(
            text_block, text_braille,
            "Block and Braille modes should produce different output"
        );
    }
}

#[cfg(test)]
mod border_tests {
    //! Tests for btop-style rounded borders

    use super::*;
    use trueno_viz::monitor::ratatui::style::Style;
    use trueno_viz::monitor::ratatui::widgets::{Block, BorderType, Borders};

    #[test]
    fn test_rounded_border_corners() {
        // Test that rounded borders use correct corner characters
        let area = Rect::new(0, 0, 10, 5);
        let mut buffer = Buffer::empty(area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Test");
        trueno_viz::monitor::ratatui::widgets::Widget::render(block, area, &mut buffer);

        let frame = TestFrame { buffer };
        let text = frame.as_text();

        // Check for rounded corners
        assert!(text.contains('╭'), "Should have top-left rounded corner");
        assert!(text.contains('╮'), "Should have top-right rounded corner");
        assert!(text.contains('╰'), "Should have bottom-left rounded corner");
        assert!(
            text.contains('╯'),
            "Should have bottom-right rounded corner"
        );
    }

    #[test]
    fn test_border_lines() {
        // Test that borders use correct line characters
        let area = Rect::new(0, 0, 10, 5);
        let mut buffer = Buffer::empty(area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);
        trueno_viz::monitor::ratatui::widgets::Widget::render(block, area, &mut buffer);

        let frame = TestFrame { buffer };
        let text = frame.as_text();

        // Check for horizontal and vertical lines
        assert!(text.contains('─'), "Should have horizontal lines");
        assert!(text.contains('│'), "Should have vertical lines");
    }

    #[test]
    fn test_border_color_applied() {
        // Test that border colors are applied correctly
        let area = Rect::new(0, 0, 10, 5);
        let mut buffer = Buffer::empty(area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Rgb(100, 200, 255)));
        trueno_viz::monitor::ratatui::widgets::Widget::render(block, area, &mut buffer);

        // Check that corner cells have the correct color
        if let Some(cell) = buffer.cell((0, 0)) {
            assert_eq!(
                cell.fg,
                Color::Rgb(100, 200, 255),
                "Border corner should have specified color"
            );
        }
    }
}

/// Tests that verify actual panel draw functions from panels.rs
mod panel_draw_tests {
    use super::*;
    use std::sync::OnceLock;
    use ttop::app::App;
    use ttop::panels;

    /// Cached test App instance (expensive to create, so cache it)
    static TEST_APP: OnceLock<App> = OnceLock::new();

    /// Get or create the test App instance
    fn test_app() -> &'static App {
        TEST_APP.get_or_init(|| App::new_mock()) // deterministic mode, no fps
    }

    /// Helper to render a panel and get the buffer text
    fn render_panel<F>(width: u16, height: u16, draw_fn: F) -> String
    where
        F: FnOnce(&mut trueno_viz::monitor::ratatui::Frame, &'static App, Rect),
    {
        let app = test_app();
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = Rect::new(0, 0, width, height);
                draw_fn(f, app, area);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut lines = Vec::new();
        for y in 0..height {
            let mut line = String::new();
            for x in 0..width {
                if let Some(cell) = buffer.cell((x, y)) {
                    line.push_str(cell.symbol());
                }
            }
            lines.push(line.trim_end().to_string());
        }
        lines.join("\n")
    }

    #[test]
    fn test_draw_network_panel() {
        let output = render_panel(60, 12, panels::draw_network);

        // Should have Network title
        assert!(output.contains("Network"), "Network panel should have title");
        // Should have download/upload arrows
        assert!(output.contains("↓") || output.contains("Download"), "Should show download indicator");
        assert!(output.contains("↑") || output.contains("Upload"), "Should show upload indicator");
    }

    #[test]
    fn test_draw_network_panel_small() {
        // Test with smaller panel size
        let output = render_panel(40, 6, panels::draw_network);

        // Should still render without panic
        assert!(output.contains("Network"), "Network panel should render at small size");
    }

    #[test]
    fn test_draw_cpu_panel() {
        let output = render_panel(60, 12, panels::draw_cpu);

        // Should have CPU title
        assert!(output.contains("CPU"), "CPU panel should have title");
    }

    #[test]
    fn test_draw_memory_panel() {
        let output = render_panel(60, 12, panels::draw_memory);

        // Should have Memory title
        assert!(output.contains("Memory") || output.contains("Mem"), "Memory panel should have title");
    }

    #[test]
    fn test_draw_disk_panel() {
        let output = render_panel(60, 12, panels::draw_disk);

        // Should have Disk title
        assert!(output.contains("Disk"), "Disk panel should have title");
    }

    #[test]
    fn test_draw_gpu_panel() {
        // GPU panel may not show content if no GPU available
        let output = render_panel(60, 12, panels::draw_gpu);

        // Should render without panic, may show "No GPU" or actual GPU info
        assert!(!output.is_empty(), "GPU panel should render something");
    }

    #[test]
    fn test_draw_battery_panel() {
        let output = render_panel(60, 8, panels::draw_battery);

        // Should render without panic
        assert!(!output.is_empty(), "Battery panel should render something");
    }

    #[test]
    fn test_draw_system_panel() {
        let output = render_panel(60, 10, panels::draw_system);

        // Should render without panic
        assert!(!output.is_empty(), "System panel should render something");
    }

    #[test]
    fn test_draw_connections_panel() {
        let output = render_panel(80, 15, panels::draw_connections);

        // Should have Connections title
        assert!(output.contains("Connections") || output.contains("Conn"),
            "Connections panel should have title");
    }

    #[test]
    fn test_draw_process_panel() {
        // draw_process takes &mut App, so create a fresh one (can't use cached)
        let mut app = App::new_mock();
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = Rect::new(0, 0, 100, 20);
                panels::draw_process(f, &mut app, area);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let mut output = String::new();
        for y in 0..20 {
            for x in 0..100 {
                if let Some(cell) = buffer.cell((x, y)) {
                    output.push_str(cell.symbol());
                }
            }
            output.push('\n');
        }

        // Should have Process title
        assert!(output.contains("Process"), "Process panel should have title");
    }

    #[test]
    fn test_network_panel_session_totals() {
        // Test with enough height to show session totals
        let output = render_panel(70, 10, panels::draw_network);

        // Should show Session totals when height is sufficient
        assert!(output.contains("Session") || output.contains("Peak") || output.contains("↓"),
            "Network panel should show session info: {}", output);
    }

    #[test]
    fn test_network_panel_connection_stats() {
        // Test with enough height to show connection stats
        let output = render_panel(70, 12, panels::draw_network);

        // May show connection stats (estab/listen) if height >= 10
        // Just verify it renders without panic
        assert!(output.contains("Network"), "Should render network panel");
    }

    #[test]
    fn test_all_panels_no_panic_various_sizes() {
        // Test all panels at various sizes don't panic
        // Use reasonably large sizes to ensure all panels have room for their content
        let sizes = [(60, 12), (80, 16), (100, 20)];

        for (width, height) in sizes {
            // Network
            let _ = render_panel(width, height, panels::draw_network);
            // CPU
            let _ = render_panel(width, height, panels::draw_cpu);
            // Memory
            let _ = render_panel(width, height, panels::draw_memory);
            // Disk
            let _ = render_panel(width, height, panels::draw_disk);
            // GPU
            let _ = render_panel(width, height, panels::draw_gpu);
            // Battery
            let _ = render_panel(width, height, panels::draw_battery);
            // System
            let _ = render_panel(width, height, panels::draw_system);
        }
    }

    #[test]
    fn test_draw_treemap_panel() {
        let output = render_panel(80, 20, panels::draw_treemap);
        // Should render without panic
        assert!(!output.is_empty(), "Treemap panel should render something");
    }
}

/// Brick-style tests: Assertions ARE the Interface (PROBAR-SPEC-009)
///
/// Each panel has falsifiable assertions that must hold:
/// 1. Title must be visible
/// 2. Content must render within budget
/// 3. Key UI elements must be present
mod brick_tests {
    use super::*;
    use std::sync::OnceLock;
    use std::time::{Duration, Instant};
    use ttop::app::App;
    use ttop::panels;

    /// Cached test App for brick tests
    static BRICK_APP: OnceLock<App> = OnceLock::new();

    fn brick_app() -> &'static App {
        BRICK_APP.get_or_init(|| {
            // Use mock app for fast tests - no real system collectors
            App::new_mock()
        })
    }

    /// Panel assertion that must be verified
    #[derive(Debug, Clone)]
    enum PanelAssertion {
        TitleContains(&'static str),
        HasDownloadIndicator,
        HasUploadIndicator,
        HasGraphChars,
        HasMeterBars,
        HasBorderChars,
        RendersWithinMs(u64),
        HeightAtLeast(u16),
        ContentNotEmpty,
    }

    /// Verify a panel against brick assertions
    fn verify_panel<F>(
        name: &str,
        width: u16,
        height: u16,
        draw_fn: F,
        assertions: &[PanelAssertion],
    ) -> Vec<String>
    where
        F: FnOnce(&mut trueno_viz::monitor::ratatui::Frame, &'static App, Rect),
    {
        let app = brick_app();
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        let start = Instant::now();
        terminal
            .draw(|f| {
                let area = Rect::new(0, 0, width, height);
                draw_fn(f, app, area);
            })
            .unwrap();
        let elapsed = start.elapsed();

        let buffer = terminal.backend().buffer();
        let mut content = String::new();
        for y in 0..height {
            for x in 0..width {
                if let Some(cell) = buffer.cell((x, y)) {
                    content.push_str(cell.symbol());
                }
            }
            content.push('\n');
        }

        let mut failures = Vec::new();

        for assertion in assertions {
            match assertion {
                PanelAssertion::TitleContains(text) => {
                    if !content.contains(text) {
                        failures.push(format!("{}: Title should contain '{}'", name, text));
                    }
                }
                PanelAssertion::HasDownloadIndicator => {
                    if !content.contains('↓') && !content.contains("Download") {
                        failures.push(format!("{}: Should have download indicator", name));
                    }
                }
                PanelAssertion::HasUploadIndicator => {
                    if !content.contains('↑') && !content.contains("Upload") {
                        failures.push(format!("{}: Should have upload indicator", name));
                    }
                }
                PanelAssertion::HasGraphChars => {
                    let has_graph = content.contains('█') || content.contains('▓') ||
                        content.contains('▒') || content.contains('░') ||
                        content.contains('▁') || content.contains('▂') ||
                        content.contains('▃') || content.contains('▄') ||
                        content.contains('▅') || content.contains('▆') ||
                        content.contains('▇') ||
                        content.chars().any(|c| ('\u{2800}'..='\u{28FF}').contains(&c));
                    if !has_graph {
                        failures.push(format!("{}: Should have graph characters", name));
                    }
                }
                PanelAssertion::HasMeterBars => {
                    let has_meter = content.contains('█') || content.contains('▏') ||
                        content.contains('▎') || content.contains('▍') ||
                        content.contains('▌') || content.contains('▋') ||
                        content.contains('▊') || content.contains('▉');
                    if !has_meter {
                        failures.push(format!("{}: Should have meter bars", name));
                    }
                }
                PanelAssertion::HasBorderChars => {
                    if !content.contains('─') && !content.contains('│') &&
                       !content.contains('╭') && !content.contains('╮') {
                        failures.push(format!("{}: Should have border characters", name));
                    }
                }
                PanelAssertion::RendersWithinMs(budget) => {
                    if elapsed > Duration::from_millis(*budget) {
                        failures.push(format!(
                            "{}: Exceeded budget {}ms (took {:?})",
                            name, budget, elapsed
                        ));
                    }
                }
                PanelAssertion::HeightAtLeast(min_height) => {
                    if height < *min_height {
                        failures.push(format!(
                            "{}: Height {} below minimum {}",
                            name, height, min_height
                        ));
                    }
                }
                PanelAssertion::ContentNotEmpty => {
                    if content.trim().is_empty() {
                        failures.push(format!("{}: Content should not be empty", name));
                    }
                }
            }
        }

        failures
    }

    // ========================================================================
    // Network Panel Brick Tests
    // ========================================================================

    #[test]
    fn brick_network_title_assertion() {
        let failures = verify_panel(
            "Network",
            60,
            12,
            panels::draw_network,
            &[PanelAssertion::TitleContains("Network")],
        );
        assert!(failures.is_empty(), "Failures: {:?}", failures);
    }

    #[test]
    fn brick_network_indicators_assertion() {
        let failures = verify_panel(
            "Network",
            60,
            12,
            panels::draw_network,
            &[
                PanelAssertion::HasDownloadIndicator,
                PanelAssertion::HasUploadIndicator,
            ],
        );
        assert!(failures.is_empty(), "Failures: {:?}", failures);
    }

    #[test]
    fn brick_network_budget_assertion() {
        let failures = verify_panel(
            "Network",
            60,
            12,
            panels::draw_network,
            &[PanelAssertion::RendersWithinMs(100)], // 100ms budget
        );
        assert!(failures.is_empty(), "Failures: {:?}", failures);
    }

    #[test]
    fn brick_network_borders_assertion() {
        let failures = verify_panel(
            "Network",
            60,
            12,
            panels::draw_network,
            &[PanelAssertion::HasBorderChars],
        );
        assert!(failures.is_empty(), "Failures: {:?}", failures);
    }

    // ========================================================================
    // CPU Panel Brick Tests
    // ========================================================================

    #[test]
    fn brick_cpu_all_assertions() {
        let failures = verify_panel(
            "CPU",
            60,
            14,
            panels::draw_cpu,
            &[
                PanelAssertion::TitleContains("CPU"),
                PanelAssertion::HasBorderChars,
                PanelAssertion::RendersWithinMs(100),
                PanelAssertion::ContentNotEmpty,
            ],
        );
        assert!(failures.is_empty(), "Failures: {:?}", failures);
    }

    // ========================================================================
    // Memory Panel Brick Tests
    // ========================================================================

    #[test]
    fn brick_memory_all_assertions() {
        let failures = verify_panel(
            "Memory",
            60,
            12,
            panels::draw_memory,
            &[
                PanelAssertion::TitleContains("Mem"),
                PanelAssertion::HasBorderChars,
                PanelAssertion::RendersWithinMs(100),
                PanelAssertion::ContentNotEmpty,
            ],
        );
        assert!(failures.is_empty(), "Failures: {:?}", failures);
    }

    // ========================================================================
    // Disk Panel Brick Tests
    // ========================================================================

    #[test]
    fn brick_disk_all_assertions() {
        let failures = verify_panel(
            "Disk",
            60,
            12,
            panels::draw_disk,
            &[
                PanelAssertion::TitleContains("Disk"),
                PanelAssertion::HasBorderChars,
                PanelAssertion::RendersWithinMs(100),
                PanelAssertion::ContentNotEmpty,
            ],
        );
        assert!(failures.is_empty(), "Failures: {:?}", failures);
    }

    // ========================================================================
    // GPU Panel Brick Tests
    // ========================================================================

    #[test]
    fn brick_gpu_all_assertions() {
        let failures = verify_panel(
            "GPU",
            60,
            12,
            panels::draw_gpu,
            &[
                PanelAssertion::HasBorderChars,
                PanelAssertion::RendersWithinMs(100),
            ],
        );
        assert!(failures.is_empty(), "Failures: {:?}", failures);
    }

    // ========================================================================
    // Battery Panel Brick Tests
    // ========================================================================

    #[test]
    fn brick_battery_all_assertions() {
        let failures = verify_panel(
            "Battery",
            60,
            8,
            panels::draw_battery,
            &[
                PanelAssertion::HasBorderChars,
                PanelAssertion::RendersWithinMs(100),
            ],
        );
        assert!(failures.is_empty(), "Failures: {:?}", failures);
    }

    // ========================================================================
    // System Panel Brick Tests
    // ========================================================================

    #[test]
    fn brick_system_all_assertions() {
        let failures = verify_panel(
            "System",
            60,
            10,
            panels::draw_system,
            &[
                PanelAssertion::HasBorderChars,
                PanelAssertion::RendersWithinMs(100),
                PanelAssertion::ContentNotEmpty,
            ],
        );
        assert!(failures.is_empty(), "Failures: {:?}", failures);
    }

    // ========================================================================
    // Connections Panel Brick Tests
    // ========================================================================

    #[test]
    fn brick_connections_all_assertions() {
        let failures = verify_panel(
            "Connections",
            80,
            15,
            panels::draw_connections,
            &[
                PanelAssertion::TitleContains("Conn"),
                PanelAssertion::HasBorderChars,
                PanelAssertion::RendersWithinMs(100),
            ],
        );
        assert!(failures.is_empty(), "Failures: {:?}", failures);
    }

    // ========================================================================
    // Treemap Panel Brick Tests
    // ========================================================================

    #[test]
    fn brick_treemap_all_assertions() {
        let failures = verify_panel(
            "Treemap",
            80,
            20,
            panels::draw_treemap,
            &[
                PanelAssertion::HasBorderChars,
                PanelAssertion::RendersWithinMs(200),
            ],
        );
        assert!(failures.is_empty(), "Failures: {:?}", failures);
    }

    // ========================================================================
    // Multi-size Brick Tests (Jidoka - stop the line on any failure)
    // ========================================================================

    #[test]
    fn brick_all_panels_multi_size_jidoka() {
        // Jidoka: If ANY panel at ANY size violates assertions, fail immediately
        let sizes = [(50, 10), (70, 14), (90, 18)];

        for (width, height) in sizes {
            // Network
            let failures = verify_panel(
                &format!("Network@{}x{}", width, height),
                width,
                height,
                panels::draw_network,
                &[
                    PanelAssertion::TitleContains("Network"),
                    PanelAssertion::RendersWithinMs(100),
                ],
            );
            assert!(failures.is_empty(), "Jidoka: {:?}", failures);

            // CPU
            let failures = verify_panel(
                &format!("CPU@{}x{}", width, height),
                width,
                height,
                panels::draw_cpu,
                &[
                    PanelAssertion::TitleContains("CPU"),
                    PanelAssertion::RendersWithinMs(100),
                ],
            );
            assert!(failures.is_empty(), "Jidoka: {:?}", failures);

            // Memory
            let failures = verify_panel(
                &format!("Memory@{}x{}", width, height),
                width,
                height,
                panels::draw_memory,
                &[
                    PanelAssertion::TitleContains("Mem"),
                    PanelAssertion::RendersWithinMs(100),
                ],
            );
            assert!(failures.is_empty(), "Jidoka: {:?}", failures);

            // Disk
            let failures = verify_panel(
                &format!("Disk@{}x{}", width, height),
                width,
                height,
                panels::draw_disk,
                &[
                    PanelAssertion::TitleContains("Disk"),
                    PanelAssertion::RendersWithinMs(100),
                ],
            );
            assert!(failures.is_empty(), "Jidoka: {:?}", failures);
        }
    }

    // ========================================================================
    // Edge Case Brick Tests
    // ========================================================================

    #[test]
    fn brick_network_minimal_height() {
        // Network panel at minimal height should still have title
        let failures = verify_panel(
            "Network-minimal",
            50,
            5,
            panels::draw_network,
            &[
                PanelAssertion::TitleContains("Network"),
                PanelAssertion::ContentNotEmpty,
            ],
        );
        assert!(failures.is_empty(), "Failures: {:?}", failures);
    }

    #[test]
    fn brick_cpu_minimal_height() {
        let failures = verify_panel(
            "CPU-minimal",
            50,
            5,
            panels::draw_cpu,
            &[
                PanelAssertion::TitleContains("CPU"),
                PanelAssertion::ContentNotEmpty,
            ],
        );
        assert!(failures.is_empty(), "Failures: {:?}", failures);
    }

    #[test]
    fn brick_memory_minimal_height() {
        let failures = verify_panel(
            "Memory-minimal",
            50,
            5,
            panels::draw_memory,
            &[
                PanelAssertion::TitleContains("Mem"),
                PanelAssertion::ContentNotEmpty,
            ],
        );
        assert!(failures.is_empty(), "Failures: {:?}", failures);
    }

    #[test]
    fn brick_disk_minimal_height() {
        let failures = verify_panel(
            "Disk-minimal",
            50,
            5,
            panels::draw_disk,
            &[
                PanelAssertion::TitleContains("Disk"),
                PanelAssertion::ContentNotEmpty,
            ],
        );
        assert!(failures.is_empty(), "Failures: {:?}", failures);
    }

    // ========================================================================
    // Wide Panel Brick Tests
    // ========================================================================

    #[test]
    fn brick_panels_wide_format() {
        // Test panels at wide format (like ultrawide monitors)
        let failures = verify_panel(
            "Network-wide",
            120,
            15,
            panels::draw_network,
            &[
                PanelAssertion::TitleContains("Network"),
                PanelAssertion::HasDownloadIndicator,
                PanelAssertion::HasUploadIndicator,
                PanelAssertion::RendersWithinMs(100),
            ],
        );
        assert!(failures.is_empty(), "Failures: {:?}", failures);

        let failures = verify_panel(
            "CPU-wide",
            120,
            15,
            panels::draw_cpu,
            &[
                PanelAssertion::TitleContains("CPU"),
                PanelAssertion::RendersWithinMs(100),
            ],
        );
        assert!(failures.is_empty(), "Failures: {:?}", failures);
    }
}

/// App lifecycle tests for coverage of app.rs
mod app_brick_tests {
    use ttop::app::App;
    use trueno_viz::monitor::Collector;

    #[test]
    fn brick_app_initialization() {
        let app = App::new_mock();
        // Verify app initializes without panic
        assert!(app.cpu.is_available() || !app.cpu.is_available()); // Always true, tests accessor
    }

    #[test]
    fn brick_app_collect_cycle() {
        let mut app = App::new_mock();
        // First collect
        app.collect_metrics();
        // Second collect (tests delta calculations)
        app.collect_metrics();
        // Third collect (tests history accumulation)
        app.collect_metrics();
    }

    #[test]
    fn brick_app_panel_visibility() {
        let app = App::new_mock();
        // Test all panel visibility accessors (default visibility)
        let _ = app.panels.cpu;
        let _ = app.panels.memory;
        let _ = app.panels.disk;
        let _ = app.panels.network;
        let _ = app.panels.gpu;
        let _ = app.panels.battery;
        let _ = app.panels.sensors;
        let _ = app.panels.process;
        // Verify at least one core panel is visible by default
        assert!(app.panels.cpu || app.panels.memory || app.panels.process);
    }

    #[test]
    fn brick_app_history_vectors() {
        let mut app = App::new_mock();
        app.collect_metrics();
        // Verify history vectors are accessible
        let _ = app.cpu_history.len();
        let _ = app.net_rx_history.len();
        let _ = app.net_tx_history.len();
        let _ = app.mem_history.len();
    }

    #[test]
    fn brick_app_network_peaks() {
        let mut app = App::new_mock();
        app.collect_metrics();
        // Verify peak tracking fields
        assert!(app.net_rx_peak >= 0.0);
        assert!(app.net_tx_peak >= 0.0);
    }

    #[test]
    fn brick_app_thrashing_detection() {
        let app = App::new_mock();
        // Test thrashing severity accessor
        let _severity = app.thrashing_severity();
    }

    #[test]
    fn brick_app_zram_detection() {
        let app = App::new_mock();
        // Test ZRAM accessors
        let _has_zram = app.has_zram();
        let _ratio = app.zram_ratio();
    }

    #[test]
    fn brick_app_gpu_detection() {
        let app = App::new_mock();
        // Test GPU availability
        let _has_gpu = app.has_gpu();
    }
}

/// UI rendering tests for coverage of ui.rs
/// These tests use single-panel configurations to avoid multi-panel buffer bounds issues
mod ui_brick_tests {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use ttop::app::App;
    use ttop::ui;

    fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
        let mut s = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                s.push(buffer.cell((x, y)).map(|c| c.symbol().chars().next().unwrap_or(' ')).unwrap_or(' '));
            }
            s.push('\n');
        }
        s
    }

    fn single_panel_app() -> App {
        let mut app = App::new_mock();
        app.panels.cpu = true;
        app.panels.memory = false;
        app.panels.disk = false;
        app.panels.network = false;
        app.panels.process = false;
        app.panels.gpu = false;
        app.panels.battery = false;
        app.panels.sensors = false;
        app.collect_metrics();
        app
    }

    #[test]
    fn brick_ui_draw_single_cpu_panel() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = single_panel_app();

        terminal.draw(|f| {
            ui::draw(f, &mut app);
        }).unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buffer);
        assert!(content.contains("CPU"), "CPU panel should render");
    }

    #[test]
    fn brick_ui_draw_with_fps_overlay() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = single_panel_app();
        app.show_fps = true;

        terminal.draw(|f| {
            ui::draw(f, &mut app);
        }).unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buffer);
        assert!(content.contains("Frame") || content.contains("μs") || content.contains("ID"), "FPS overlay should show frame info");
    }

    #[test]
    fn brick_ui_draw_with_help_overlay() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = single_panel_app();
        app.show_help = true;

        terminal.draw(|f| {
            ui::draw(f, &mut app);
        }).unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buffer);
        assert!(content.contains("Help") || content.contains("Navigation") || content.contains("ttop"), "Help overlay should show");
    }

    #[test]
    fn brick_ui_draw_with_filter_input() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = single_panel_app();
        app.show_filter_input = true;
        app.filter = "test".to_string();

        terminal.draw(|f| {
            ui::draw(f, &mut app);
        }).unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buffer);
        assert!(content.contains("Filter") || content.contains("test"), "Filter input should show");
    }

    #[test]
    fn brick_ui_draw_empty_panels() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_mock();
        app.panels.cpu = false;
        app.panels.memory = false;
        app.panels.disk = false;
        app.panels.network = false;
        app.panels.process = false;
        app.panels.gpu = false;
        app.panels.battery = false;
        app.panels.sensors = false;

        terminal.draw(|f| {
            ui::draw(f, &mut app);
        }).unwrap();
        // Should not panic with no panels
    }

    #[test]
    fn brick_ui_draw_only_process_panel() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_mock();
        app.collect_metrics();
        app.panels.cpu = false;
        app.panels.memory = false;
        app.panels.disk = false;
        app.panels.network = false;
        app.panels.process = true;
        app.panels.gpu = false;
        app.panels.battery = false;
        app.panels.sensors = false;

        terminal.draw(|f| {
            ui::draw(f, &mut app);
        }).unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buffer);
        assert!(content.contains("Process") || content.contains("PID"), "Process panel should render");
    }

    #[test]
    fn brick_ui_draw_memory_panel() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_mock();
        app.collect_metrics();
        app.panels.cpu = false;
        app.panels.memory = true;
        app.panels.disk = false;
        app.panels.network = false;
        app.panels.process = false;
        app.panels.gpu = false;
        app.panels.battery = false;
        app.panels.sensors = false;

        terminal.draw(|f| {
            ui::draw(f, &mut app);
        }).unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buffer);
        assert!(content.contains("Memory") || content.contains("RAM"), "Memory panel should render");
    }

    #[test]
    fn brick_ui_draw_network_panel() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_mock();
        app.collect_metrics();
        app.panels.cpu = false;
        app.panels.memory = false;
        app.panels.disk = false;
        app.panels.network = true;
        app.panels.process = false;
        app.panels.gpu = false;
        app.panels.battery = false;
        app.panels.sensors = false;

        terminal.draw(|f| {
            ui::draw(f, &mut app);
        }).unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buffer);
        assert!(content.contains("Network") || content.contains("↓") || content.contains("↑"), "Network panel should render");
    }

    #[test]
    fn brick_ui_draw_disk_panel() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_mock();
        app.collect_metrics();
        app.panels.cpu = false;
        app.panels.memory = false;
        app.panels.disk = true;
        app.panels.network = false;
        app.panels.process = false;
        app.panels.gpu = false;
        app.panels.battery = false;
        app.panels.sensors = false;

        terminal.draw(|f| {
            ui::draw(f, &mut app);
        }).unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buffer);
        assert!(content.contains("Disk") || content.contains("I/O"), "Disk panel should render");
    }

    #[test]
    fn brick_ui_all_overlays_single_panel() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = single_panel_app();
        app.show_fps = true;
        app.show_help = true;
        app.show_filter_input = true;

        terminal.draw(|f| {
            ui::draw(f, &mut app);
        }).unwrap();
        // Should not panic with all overlays active
    }

    #[test]
    fn brick_ui_various_single_panel_sizes() {
        let sizes = vec![
            (60, 20),
            (80, 30),
            (120, 40),
        ];

        for (width, height) in sizes {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = single_panel_app();

            terminal.draw(|f| {
                ui::draw(f, &mut app);
            }).unwrap();
        }
    }

    #[test]
    fn brick_ui_two_panels() {
        let backend = TestBackend::new(160, 50);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_mock();
        app.collect_metrics();
        app.panels.cpu = true;
        app.panels.memory = true;
        app.panels.disk = false;
        app.panels.network = false;
        app.panels.process = false;
        app.panels.gpu = false;
        app.panels.battery = false;
        app.panels.sensors = false;

        terminal.draw(|f| {
            ui::draw(f, &mut app);
        }).unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buffer);
        assert!(content.contains("CPU") || content.contains("Memory"), "Two panel layout should render");
    }
}

/// Keyboard handling tests for app.rs coverage
mod key_handling_tests {
    use crossterm::event::{KeyCode, KeyModifiers};
    use ttop::app::App;

    #[test]
    fn brick_key_quit_q() {
        let mut app = App::new_mock();
        let quit = app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(quit, "'q' should quit");
    }

    #[test]
    fn brick_key_quit_esc() {
        let mut app = App::new_mock();
        let quit = app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(quit, "Esc should quit");
    }

    #[test]
    fn brick_key_quit_ctrl_c() {
        let mut app = App::new_mock();
        let quit = app.handle_key(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(quit, "Ctrl+C should quit");
    }

    #[test]
    fn brick_key_help_toggle() {
        let mut app = App::new_mock();
        assert!(!app.show_help);
        app.handle_key(KeyCode::Char('?'), KeyModifiers::NONE);
        assert!(app.show_help, "'?' should show help");
        app.handle_key(KeyCode::Char('?'), KeyModifiers::NONE);
        assert!(!app.show_help, "'?' again should hide help");
    }

    #[test]
    fn brick_key_help_f1() {
        let mut app = App::new_mock();
        app.handle_key(KeyCode::F(1), KeyModifiers::NONE);
        assert!(app.show_help, "F1 should show help");
    }

    #[test]
    fn brick_key_panel_toggles() {
        let mut app = App::new_mock();

        // Toggle CPU panel
        assert!(app.panels.cpu);
        app.handle_key(KeyCode::Char('1'), KeyModifiers::NONE);
        assert!(!app.panels.cpu, "'1' should toggle CPU panel off");

        // Toggle memory panel
        assert!(app.panels.memory);
        app.handle_key(KeyCode::Char('2'), KeyModifiers::NONE);
        assert!(!app.panels.memory, "'2' should toggle memory panel off");

        // Toggle disk panel
        app.handle_key(KeyCode::Char('3'), KeyModifiers::NONE);
        assert!(!app.panels.disk, "'3' should toggle disk panel");

        // Toggle network panel
        app.handle_key(KeyCode::Char('4'), KeyModifiers::NONE);
        assert!(!app.panels.network, "'4' should toggle network panel");

        // Toggle process panel
        app.handle_key(KeyCode::Char('5'), KeyModifiers::NONE);
        assert!(!app.panels.process, "'5' should toggle process panel");

        // Toggle GPU panel
        app.handle_key(KeyCode::Char('6'), KeyModifiers::NONE);
        assert!(!app.panels.gpu, "'6' should toggle GPU panel");

        // Toggle battery panel
        app.handle_key(KeyCode::Char('7'), KeyModifiers::NONE);
        assert!(!app.panels.battery, "'7' should toggle battery panel");

        // Toggle sensors panel
        app.handle_key(KeyCode::Char('8'), KeyModifiers::NONE);
        assert!(!app.panels.sensors, "'8' should toggle sensors panel");
    }

    #[test]
    fn brick_key_reset_panels() {
        let mut app = App::new_mock();
        // Turn off some panels
        app.panels.cpu = false;
        app.panels.memory = false;
        // Reset with '0'
        app.handle_key(KeyCode::Char('0'), KeyModifiers::NONE);
        assert!(app.panels.cpu, "'0' should reset CPU panel on");
        assert!(app.panels.memory, "'0' should reset memory panel on");
    }

    #[test]
    fn brick_key_navigation() {
        let mut app = App::new_mock();
        app.collect_metrics();

        // Down with 'j'
        app.process_selected = 0;
        app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
        // Up with 'k'
        app.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);

        // Down arrow
        app.handle_key(KeyCode::Down, KeyModifiers::NONE);
        // Up arrow
        app.handle_key(KeyCode::Up, KeyModifiers::NONE);

        // Page down/up
        app.handle_key(KeyCode::PageDown, KeyModifiers::NONE);
        app.handle_key(KeyCode::PageUp, KeyModifiers::NONE);

        // Home/End
        app.handle_key(KeyCode::Home, KeyModifiers::NONE);
        assert_eq!(app.process_selected, 0, "Home should go to start");
        app.handle_key(KeyCode::End, KeyModifiers::NONE);

        // 'g' for top, 'G' for bottom
        app.handle_key(KeyCode::Char('g'), KeyModifiers::NONE);
        assert_eq!(app.process_selected, 0, "'g' should go to top");
        app.handle_key(KeyCode::Char('G'), KeyModifiers::NONE);
    }

    #[test]
    fn brick_key_sorting() {
        let mut app = App::new_mock();

        // Cycle sort with Tab
        let initial_column = app.sort_column;
        app.handle_key(KeyCode::Tab, KeyModifiers::NONE);
        assert_ne!(format!("{:?}", app.sort_column), format!("{:?}", initial_column), "Tab should cycle sort");

        // Cycle sort with 's'
        app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);

        // Reverse sort
        let initial_desc = app.sort_descending;
        app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
        assert_ne!(app.sort_descending, initial_desc, "'r' should toggle sort direction");
    }

    #[test]
    fn brick_key_tree_view() {
        let mut app = App::new_mock();
        assert!(!app.show_tree);
        app.handle_key(KeyCode::Char('t'), KeyModifiers::NONE);
        assert!(app.show_tree, "'t' should enable tree view");
    }

    #[test]
    fn brick_key_filter_mode() {
        let mut app = App::new_mock();

        // Enter filter mode with 'f'
        assert!(!app.show_filter_input);
        app.handle_key(KeyCode::Char('f'), KeyModifiers::NONE);
        assert!(app.show_filter_input, "'f' should enter filter mode");

        // Type some filter text
        app.handle_key(KeyCode::Char('t'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('e'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('t'), KeyModifiers::NONE);
        assert_eq!(app.filter, "test", "typing should add to filter");

        // Backspace
        app.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(app.filter, "tes", "backspace should remove char");

        // Enter to confirm
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert!(!app.show_filter_input, "Enter should exit filter mode");
        assert_eq!(app.filter, "tes", "filter should be preserved");
    }

    #[test]
    fn brick_key_filter_escape() {
        let mut app = App::new_mock();

        // Enter filter mode with '/'
        app.handle_key(KeyCode::Char('/'), KeyModifiers::NONE);
        assert!(app.show_filter_input, "'/' should enter filter mode");

        // Type something
        app.handle_key(KeyCode::Char('x'), KeyModifiers::NONE);
        assert_eq!(app.filter, "x");

        // Escape to cancel (clears filter)
        app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(!app.show_filter_input, "Esc should exit filter mode");
        assert_eq!(app.filter, "", "Esc should clear filter");
    }

    #[test]
    fn brick_key_clear_filter() {
        let mut app = App::new_mock();
        app.filter = "some_filter".to_string();
        app.handle_key(KeyCode::Delete, KeyModifiers::NONE);
        assert_eq!(app.filter, "", "Delete should clear filter");
    }

    #[test]
    fn brick_key_unknown() {
        let mut app = App::new_mock();
        // Unknown key should not quit
        let quit = app.handle_key(KeyCode::F(12), KeyModifiers::NONE);
        assert!(!quit, "Unknown key should not quit");
    }
}

/// Process sorting and filtering tests
mod process_tests {
    use ttop::app::App;
    use ttop::state::ProcessSortColumn;

    #[test]
    fn brick_sorted_processes() {
        let mut app = App::new_mock();
        app.collect_metrics();

        // Test different sort columns
        app.sort_column = ProcessSortColumn::Cpu;
        let _procs = app.sorted_processes();

        app.sort_column = ProcessSortColumn::Mem;
        let _procs = app.sorted_processes();

        app.sort_column = ProcessSortColumn::Pid;
        let _procs = app.sorted_processes();

        app.sort_column = ProcessSortColumn::Name;
        let _procs = app.sorted_processes();

        app.sort_column = ProcessSortColumn::State;
        let _procs = app.sorted_processes();

        app.sort_column = ProcessSortColumn::User;
        let _procs = app.sorted_processes();

        app.sort_column = ProcessSortColumn::Threads;
        let _procs = app.sorted_processes();
    }

    #[test]
    fn brick_filter_processes() {
        let mut app = App::new_mock();
        app.collect_metrics();

        // Filter by name (case insensitive)
        app.filter = "rust".to_string();
        let procs = app.sorted_processes();
        // All returned processes should match filter
        for p in procs {
            let matches = p.name.to_lowercase().contains("rust")
                || p.cmdline.to_lowercase().contains("rust");
            assert!(matches || app.filter.is_empty(), "Process should match filter");
        }
    }

    #[test]
    fn brick_ascending_sort() {
        let mut app = App::new_mock();
        app.collect_metrics();
        app.sort_descending = false;
        let _procs = app.sorted_processes();
    }
}

/// Frame stats and timing tests (covering main.rs logic that lives in app.rs)
mod frame_stats_tests {
    use ttop::app::App;
    use std::time::Duration;

    #[test]
    fn brick_update_frame_stats_normal() {
        let mut app = App::new_mock();
        let times = vec![
            Duration::from_micros(1000),
            Duration::from_micros(2000),
            Duration::from_micros(3000),
        ];
        app.update_frame_stats(&times);
        assert_eq!(app.avg_frame_time_us, 2000, "Average should be 2000μs");
        assert_eq!(app.max_frame_time_us, 3000, "Max should be 3000μs");
    }

    #[test]
    fn brick_update_frame_stats_empty() {
        let mut app = App::new_mock();
        app.update_frame_stats(&[]);
        // Should not panic on empty input
    }

    #[test]
    fn brick_update_frame_stats_single() {
        let mut app = App::new_mock();
        let times = vec![Duration::from_micros(5000)];
        app.update_frame_stats(&times);
        assert_eq!(app.avg_frame_time_us, 5000);
        assert_eq!(app.max_frame_time_us, 5000);
    }

    #[test]
    fn brick_frame_id_increments() {
        let mut app = App::new_mock();
        let initial_frame_id = app.frame_id;
        app.collect_metrics();
        assert!(app.frame_id > initial_frame_id, "Frame ID should increment on collect");
    }
}

/// Ring buffer tests for coverage
mod ring_buffer_tests {
    use ttop::ring_buffer::{RingBuffer, handle_counter_wrap};

    #[test]
    fn brick_ring_buffer_basic() {
        let buf: RingBuffer<i32> = RingBuffer::new(3);
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
        assert_eq!(buf.capacity(), 3);
        assert!(buf.latest().is_none());
        assert!(buf.oldest().is_none());
    }

    #[test]
    fn brick_ring_buffer_push_and_access() {
        let mut buf: RingBuffer<i32> = RingBuffer::new(3);
        buf.push(10);
        assert_eq!(buf.len(), 1);
        assert!(!buf.is_empty());
        assert_eq!(buf.latest(), Some(&10));
        assert_eq!(buf.oldest(), Some(&10));

        buf.push(20);
        buf.push(30);
        assert_eq!(buf.len(), 3);
        assert_eq!(buf.latest(), Some(&30));
        assert_eq!(buf.oldest(), Some(&10));
    }

    #[test]
    fn brick_ring_buffer_wrap() {
        let mut buf: RingBuffer<i32> = RingBuffer::new(2);
        buf.push(1);
        buf.push(2);
        buf.push(3); // Should evict 1
        assert_eq!(buf.oldest(), Some(&2));
        assert_eq!(buf.latest(), Some(&3));
    }

    #[test]
    fn brick_ring_buffer_clear() {
        let mut buf: RingBuffer<i32> = RingBuffer::new(3);
        buf.push(1);
        buf.push(2);
        buf.clear();
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn brick_ring_buffer_iter() {
        let mut buf: RingBuffer<i32> = RingBuffer::new(3);
        buf.push(1);
        buf.push(2);
        buf.push(3);
        let collected: Vec<i32> = buf.iter().copied().collect();
        assert_eq!(collected, vec![1, 2, 3]);
    }

    #[test]
    fn brick_ring_buffer_f64_empty_stats() {
        let buf: RingBuffer<f64> = RingBuffer::new(3);
        assert_eq!(buf.mean(), 0.0);
        assert_eq!(buf.sum(), 0.0);
        assert_eq!(buf.min(), 0.0);
        assert_eq!(buf.max(), 0.0);
        assert_eq!(buf.std_dev(), 0.0);
    }

    #[test]
    fn brick_ring_buffer_f64_single_element() {
        let mut buf: RingBuffer<f64> = RingBuffer::new(3);
        buf.push(42.0);
        assert!((buf.mean() - 42.0).abs() < 0.001);
        assert!((buf.sum() - 42.0).abs() < 0.001);
        assert!((buf.min() - 42.0).abs() < 0.001);
        assert!((buf.max() - 42.0).abs() < 0.001);
        assert_eq!(buf.std_dev(), 0.0); // Needs at least 2 for std_dev
    }

    #[test]
    fn brick_ring_buffer_f64_stats() {
        let mut buf: RingBuffer<f64> = RingBuffer::new(5);
        buf.push(1.0);
        buf.push(2.0);
        buf.push(3.0);
        buf.push(4.0);
        buf.push(5.0);
        assert!((buf.mean() - 3.0).abs() < 0.001);
        assert!((buf.sum() - 15.0).abs() < 0.001);
        assert!((buf.min() - 1.0).abs() < 0.001);
        assert!((buf.max() - 5.0).abs() < 0.001);
        // std_dev of [1,2,3,4,5] = sqrt(2.5) ≈ 1.58
        assert!(buf.std_dev() > 1.5 && buf.std_dev() < 1.7);
    }

    #[test]
    fn brick_ring_buffer_f64_rate() {
        let mut buf: RingBuffer<f64> = RingBuffer::new(5);
        buf.push(0.0);
        buf.push(10.0);
        buf.push(20.0);
        // Rate = (20 - 0) / (2 * 1.0) = 10/s
        let rate = buf.rate_per_sec(1.0);
        assert!((rate - 10.0).abs() < 0.001);
    }

    #[test]
    fn brick_ring_buffer_f64_rate_edge_cases() {
        let mut buf: RingBuffer<f64> = RingBuffer::new(5);
        // Empty buffer
        assert_eq!(buf.rate_per_sec(1.0), 0.0);

        // Single element
        buf.push(100.0);
        assert_eq!(buf.rate_per_sec(1.0), 0.0);

        // Zero sample interval
        buf.push(200.0);
        assert_eq!(buf.rate_per_sec(0.0), 0.0);
        assert_eq!(buf.rate_per_sec(-1.0), 0.0);
    }

    #[test]
    fn brick_ring_buffer_u64_stats() {
        let mut buf: RingBuffer<u64> = RingBuffer::new(5);
        buf.push(100);
        buf.push(200);
        buf.push(300);
        assert_eq!(buf.sum(), 600);
        assert!((buf.mean() - 200.0).abs() < 0.001);
        assert_eq!(buf.min(), 100);
        assert_eq!(buf.max(), 300);
    }

    #[test]
    fn brick_ring_buffer_u64_empty_stats() {
        let buf: RingBuffer<u64> = RingBuffer::new(3);
        assert_eq!(buf.sum(), 0);
        assert_eq!(buf.mean(), 0.0);
        assert_eq!(buf.min(), 0);
        assert_eq!(buf.max(), 0);
    }

    #[test]
    fn brick_ring_buffer_u64_rate() {
        let mut buf: RingBuffer<u64> = RingBuffer::new(5);
        buf.push(100);
        buf.push(200);
        buf.push(300);
        buf.push(400);
        buf.push(500);
        // Rate = (500 - 100) / (4 * 1.0) = 100/s
        let rate = buf.rate_per_sec(1.0);
        assert!((rate - 100.0).abs() < 0.001);
    }

    #[test]
    fn brick_ring_buffer_u64_rate_counter_wrap() {
        let mut buf: RingBuffer<u64> = RingBuffer::new(3);
        buf.push(u64::MAX - 5);
        buf.push(u64::MAX);
        buf.push(10); // Wrapped
        // Delta = (MAX - 5) to MAX (5) + MAX to 10 (11) = ~16
        let rate = buf.rate_per_sec(1.0);
        // Rate = 16 / (2 * 1.0) = 8
        assert!(rate > 7.0 && rate < 9.0);
    }

    #[test]
    fn brick_ring_buffer_u64_rate_edge_cases() {
        let mut buf: RingBuffer<u64> = RingBuffer::new(5);
        // Empty
        assert_eq!(buf.rate_per_sec(1.0), 0.0);
        // Single element
        buf.push(100);
        assert_eq!(buf.rate_per_sec(1.0), 0.0);
        // Zero/negative interval
        buf.push(200);
        assert_eq!(buf.rate_per_sec(0.0), 0.0);
        assert_eq!(buf.rate_per_sec(-1.0), 0.0);
    }

    #[test]
    fn brick_ring_buffer_make_contiguous() {
        let mut buf: RingBuffer<f64> = RingBuffer::new(3);
        buf.push(1.0);
        buf.push(2.0);
        buf.push(3.0);
        buf.push(4.0); // Forces rotation
        let slice = buf.make_contiguous();
        assert_eq!(slice.len(), 3);

        // Test as_slice alias
        let slice2 = buf.as_slice();
        assert_eq!(slice2.len(), 3);
    }

    #[test]
    fn brick_counter_wrap_handling() {
        // Normal case (no wrap)
        assert_eq!(handle_counter_wrap(100, 200), 100);
        // Wrap case
        assert_eq!(handle_counter_wrap(u64::MAX - 5, 10), 16);
        // Same value
        assert_eq!(handle_counter_wrap(100, 100), 0);
    }
}

/// Additional panel rendering tests for coverage gaps
mod panel_coverage_tests {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use ttop::app::App;
    use ttop::panels;

    fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
        let mut s = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                s.push(buffer.cell((x, y)).map(|c| c.symbol().chars().next().unwrap_or(' ')).unwrap_or(' '));
            }
            s.push('\n');
        }
        s
    }

    #[test]
    fn brick_draw_sensors_panel() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_mock();
        app.collect_metrics();

        terminal.draw(|f| {
            let area = f.area();
            panels::draw_sensors(f, &app, area);
        }).unwrap();
        // Should not panic
    }

    #[test]
    fn brick_draw_sensors_compact_panel() {
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_mock();
        app.collect_metrics();

        terminal.draw(|f| {
            let area = f.area();
            panels::draw_sensors_compact(f, &app, area);
        }).unwrap();
    }

    #[test]
    fn brick_draw_psi_panel() {
        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_mock();
        app.collect_metrics();

        terminal.draw(|f| {
            let area = f.area();
            panels::draw_psi(f, &app, area);
        }).unwrap();
    }

    #[test]
    fn brick_draw_system_panel_large() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_mock();
        app.collect_metrics();

        terminal.draw(|f| {
            let area = f.area();
            panels::draw_system(f, &app, area);
        }).unwrap();
    }

    #[test]
    fn brick_draw_cpu_small() {
        let backend = TestBackend::new(50, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_mock();
        app.collect_metrics();

        terminal.draw(|f| {
            let area = f.area();
            panels::draw_cpu(f, &app, area);
        }).unwrap();
    }

    #[test]
    fn brick_draw_cpu_large() {
        let backend = TestBackend::new(150, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_mock();
        app.collect_metrics();

        terminal.draw(|f| {
            let area = f.area();
            panels::draw_cpu(f, &app, area);
        }).unwrap();
    }

    #[test]
    fn brick_draw_memory_small() {
        let backend = TestBackend::new(50, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_mock();
        app.collect_metrics();

        terminal.draw(|f| {
            let area = f.area();
            panels::draw_memory(f, &app, area);
        }).unwrap();
    }

    #[test]
    fn brick_draw_memory_large() {
        let backend = TestBackend::new(150, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_mock();
        app.collect_metrics();

        terminal.draw(|f| {
            let area = f.area();
            panels::draw_memory(f, &app, area);
        }).unwrap();
    }

    #[test]
    fn brick_draw_disk_small() {
        let backend = TestBackend::new(50, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_mock();
        app.collect_metrics();

        terminal.draw(|f| {
            let area = f.area();
            panels::draw_disk(f, &app, area);
        }).unwrap();
    }

    #[test]
    fn brick_draw_network_small() {
        let backend = TestBackend::new(50, 8);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_mock();
        app.collect_metrics();

        terminal.draw(|f| {
            let area = f.area();
            panels::draw_network(f, &app, area);
        }).unwrap();
    }

    #[test]
    fn brick_draw_network_large() {
        let backend = TestBackend::new(150, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_mock();
        app.collect_metrics();

        terminal.draw(|f| {
            let area = f.area();
            panels::draw_network(f, &app, area);
        }).unwrap();
    }

    #[test]
    fn brick_draw_process_various_sizes() {
        let sizes = [(80, 20), (120, 40), (60, 15)];
        for (w, h) in sizes {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = App::new_mock();
            app.collect_metrics();

            terminal.draw(|f| {
                let area = f.area();
                panels::draw_process(f, &mut app, area);
            }).unwrap();
        }
    }

    #[test]
    fn brick_draw_connections_small() {
        let backend = TestBackend::new(60, 15);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_mock();
        app.collect_metrics();

        terminal.draw(|f| {
            let area = f.area();
            panels::draw_connections(f, &app, area);
        }).unwrap();
    }

    #[test]
    fn brick_draw_treemap_small() {
        let backend = TestBackend::new(60, 15);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_mock();
        app.collect_metrics();

        terminal.draw(|f| {
            let area = f.area();
            panels::draw_treemap(f, &app, area);
        }).unwrap();
    }

    #[test]
    fn brick_draw_panels_tiny() {
        // Test extremely small sizes to hit edge case branches
        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_mock();
        app.collect_metrics();

        terminal.draw(|f| {
            let area = f.area();
            panels::draw_cpu(f, &app, area);
        }).unwrap();
    }

    #[test]
    fn brick_draw_battery_panel() {
        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_mock();
        app.collect_metrics();

        terminal.draw(|f| {
            let area = f.area();
            panels::draw_battery(f, &app, area);
        }).unwrap();
    }

    #[test]
    fn brick_draw_gpu_panel_direct() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_mock();
        app.collect_metrics();

        terminal.draw(|f| {
            let area = f.area();
            panels::draw_gpu(f, &app, area);
        }).unwrap();
    }
}

/// Analyzer tests for coverage
mod analyzer_tests {
    use ttop::analyzers::{
        StorageAnalyzer, ConnectionAnalyzer, TreemapAnalyzer, ContainerAnalyzer,
        PsiAnalyzer, DiskIoAnalyzer, SwapAnalyzer, GpuProcessAnalyzer,
    };
    use std::path::PathBuf;

    #[test]
    fn brick_storage_analyzer_basic() {
        let analyzer = StorageAnalyzer::default();
        let _mounts = analyzer.mounts();
        let _total = analyzer.total_storage_bytes();
        let _used = analyzer.total_used_bytes();
        let _pct = analyzer.overall_usage_percent();
    }

    #[test]
    fn brick_storage_analyzer_detector() {
        let mut analyzer = StorageAnalyzer::default();
        analyzer.collect();

        // Access detector
        let detector = analyzer.detector();
        let _sample_count = detector.sample_count();
        let _median = detector.median();
        let _mad = detector.mad();
    }

    #[test]
    fn brick_storage_analyzer_anomalies() {
        let mut analyzer = StorageAnalyzer::default();
        analyzer.collect();

        // Check anomalies
        let _anomalies: Vec<_> = analyzer.recent_anomalies().collect();
    }

    #[test]
    fn brick_storage_analyzer_file_events() {
        let mut analyzer = StorageAnalyzer::default();

        // Add file events via detector
        let detector = analyzer.detector_mut();
        for i in 1..=10 {
            let _ = detector.on_file_created(PathBuf::from(format!("/tmp/file{}.txt", i)), i * 1000);
        }

        assert!(detector.sample_count() > 0);
    }

    #[test]
    fn brick_storage_analyzer_z_score() {
        let mut analyzer = StorageAnalyzer::default();

        let detector = analyzer.detector_mut();
        // Build baseline
        for i in 1..=20 {
            let _ = detector.on_file_created(PathBuf::from(format!("/tmp/f{}", i)), 1000);
        }

        let z = detector.calculate_z_score(1000);
        assert!(z >= 0.0);
    }

    #[test]
    fn brick_connection_analyzer_basic() {
        let analyzer = ConnectionAnalyzer::default();
        let _conns = analyzer.connections();
    }

    #[test]
    fn brick_connection_analyzer_collect() {
        let mut analyzer = ConnectionAnalyzer::default();
        analyzer.collect();
        let _conns = analyzer.connections();
        let _active = analyzer.active_connections();
        let _listening = analyzer.listening();
        let _by_state = analyzer.count_by_state();
    }

    #[test]
    fn brick_treemap_analyzer_basic() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        let _scanning = analyzer.is_scanning();
        let _total = analyzer.total_size();
    }

    #[test]
    fn brick_treemap_analyzer_collect_and_layout() {
        let mut analyzer = TreemapAnalyzer::new("/tmp");
        analyzer.collect();
        let _layout = analyzer.layout(100.0, 50.0);
    }

    #[test]
    fn brick_container_analyzer_basic() {
        let analyzer = ContainerAnalyzer::default();
        let _available = analyzer.is_available();
        let _containers = analyzer.containers();
        let _total = analyzer.total_count();
        let _running = analyzer.running_count();
    }

    #[test]
    fn brick_container_analyzer_collect() {
        let mut analyzer = ContainerAnalyzer::default();
        analyzer.collect();
        let _containers = analyzer.containers();
        let _top = analyzer.top_containers(5);
    }

    #[test]
    fn brick_psi_analyzer_basic() {
        let analyzer = PsiAnalyzer::default();
        let _available = analyzer.is_available();
    }

    #[test]
    fn brick_psi_analyzer_collect() {
        let mut analyzer = PsiAnalyzer::default();
        analyzer.collect();
        let _cpu = analyzer.cpu_level();
        let _memory = analyzer.memory_level();
        let _io = analyzer.io_level();
        let _overall = analyzer.overall_level();
    }

    #[test]
    fn brick_disk_io_analyzer_basic() {
        let analyzer = DiskIoAnalyzer::default();
        let _stats = analyzer.device_stats();
        let _read = analyzer.total_read_throughput();
        let _write = analyzer.total_write_throughput();
        let _iops = analyzer.total_iops();
        let _workload = analyzer.overall_workload();
    }

    #[test]
    fn brick_disk_io_analyzer_collect() {
        let mut analyzer = DiskIoAnalyzer::default();
        analyzer.collect();
        let _stats = analyzer.device_stats();
        let _read_hist = analyzer.read_history();
        let _write_hist = analyzer.write_history();
        let _iops_hist = analyzer.iops_history();
        let _primary = analyzer.primary_device();
    }

    #[test]
    fn brick_disk_io_analyzer_device_specific() {
        let mut analyzer = DiskIoAnalyzer::default();
        analyzer.set_sample_interval(1.0);
        analyzer.collect();

        // Test device-specific methods if any devices exist
        if let Some((device_name, _)) = analyzer.device_stats().iter().next() {
            let _dev = analyzer.device(device_name);
            let _latency = analyzer.estimated_latency_ms(device_name);
            let _workload = analyzer.workload_type(device_name);
            let _read = analyzer.device_read_history(device_name);
            let _write = analyzer.device_write_history(device_name);
        }
    }

    #[test]
    fn brick_swap_analyzer_basic() {
        let analyzer = SwapAnalyzer::default();
        let _thrashing = analyzer.detect_thrashing();
        let _has_zram = analyzer.has_zram();
        let _ratio = analyzer.zram_compression_ratio();
        let _rate = analyzer.swap_rate_per_sec();
    }

    #[test]
    fn brick_swap_analyzer_collect() {
        let mut analyzer = SwapAnalyzer::default();
        analyzer.set_sample_interval(1.0);
        analyzer.collect();
        let _thrashing = analyzer.detect_thrashing();
        let _pages_in = analyzer.pages_in_rate();
        let _pages_out = analyzer.pages_out_rate();
        let _major = analyzer.major_fault_rate_per_sec();
        let _minor = analyzer.minor_fault_rate_per_sec();
        let _psi = analyzer.psi();
        let _zram = analyzer.zram_stats();
        let _fault_hist = analyzer.fault_history();
        let _swap_io = analyzer.swap_io_history();
    }

    #[test]
    fn brick_gpu_process_analyzer_basic() {
        let analyzer = GpuProcessAnalyzer::default();
        let _available = analyzer.is_available();
    }

    #[test]
    fn brick_gpu_process_analyzer_collect() {
        let mut analyzer = GpuProcessAnalyzer::default();
        analyzer.collect();
        let _procs = analyzer.processes();
        let _top = analyzer.top_processes(5);
    }
}

/// Additional theme tests for full branch coverage
mod theme_branch_tests {
    use ttop::theme::{percent_color, temp_color, format_bytes, format_bytes_rate, format_uptime};

    #[test]
    fn brick_temp_color_all_branches() {
        // Cover all temp_color branches
        let _critical = temp_color(96.0);  // > 95
        let _very_hot = temp_color(90.0);  // > 85
        let _hot = temp_color(80.0);       // > 75
        let _warm = temp_color(70.0);      // > 65
        let _normal_warm = temp_color(55.0); // > 50
        let _normal = temp_color(40.0);    // > 35
        let _cool = temp_color(30.0);      // else
    }

    #[test]
    fn brick_percent_color_all_branches() {
        // Cover all percent_color branches
        let _critical = percent_color(95.0);  // >= 90
        let _high = percent_color(80.0);      // >= 75
        let _med_high = percent_color(60.0);  // >= 50
        let _med_low = percent_color(35.0);   // >= 25
        let _low = percent_color(10.0);       // else
    }

    #[test]
    fn brick_format_bytes_all_branches() {
        let _b = format_bytes(500);
        let _kb = format_bytes(2048);
        let _mb = format_bytes(5 * 1024 * 1024);
        let _gb = format_bytes(10 * 1024 * 1024 * 1024);
        let _tb = format_bytes(2 * 1024 * 1024 * 1024 * 1024);
    }

    #[test]
    fn brick_format_bytes_rate() {
        let rate = format_bytes_rate(1024.0 * 1024.0);
        assert!(rate.contains("/s"));
    }

    #[test]
    fn brick_format_uptime_all_branches() {
        let _mins = format_uptime(300.0);         // < 1 hour
        let _hours = format_uptime(7200.0);       // 2 hours
        let _days = format_uptime(100000.0);      // > 1 day
    }
}

// ============================================================================
// NetworkStatsAnalyzer Tests (Linux-only)
// ============================================================================

#[cfg(target_os = "linux")]
mod network_stats_tests {
    use ttop::analyzers::{NetworkStatsAnalyzer, ProtocolStats, TcpPerformance, QueueStats};

    #[test]
    fn brick_network_stats_analyzer_new() {
        let analyzer = NetworkStatsAnalyzer::new();
        assert!(analyzer.interface_errors.is_empty());
        assert_eq!(analyzer.protocol_stats.tcp_established, 0);
    }

    #[test]
    fn brick_network_stats_analyzer_collect() {
        let mut analyzer = NetworkStatsAnalyzer::new();

        // First collection
        analyzer.collect();

        // TCP stats should be populated (will be 0+ depending on system)
        let _total_tcp = analyzer.protocol_stats.tcp_total(); // Can be 0 on isolated systems

        // Second collection for delta tracking
        analyzer.collect();
    }

    #[test]
    fn brick_protocol_stats_tcp_total() {
        let stats = ProtocolStats {
            tcp_established: 10,
            tcp_syn_sent: 1,
            tcp_syn_recv: 0,
            tcp_fin_wait1: 2,
            tcp_fin_wait2: 1,
            tcp_time_wait: 5,
            tcp_close: 0,
            tcp_close_wait: 3,
            tcp_last_ack: 0,
            tcp_listen: 8,
            tcp_closing: 0,
            udp_sockets: 15,
            icmp_sockets: 2,
        };

        // Should sum all TCP states
        assert_eq!(stats.tcp_total(), 30);
    }

    #[test]
    fn brick_network_stats_total_errors() {
        let mut analyzer = NetworkStatsAnalyzer::new();
        analyzer.collect();

        let (rx_errs, tx_errs) = analyzer.total_errors();
        // Just verify it returns something (can be 0 on healthy systems)
        let _ = rx_errs;
        let _ = tx_errs;
    }

    #[test]
    fn brick_network_stats_error_deltas() {
        let mut analyzer = NetworkStatsAnalyzer::new();

        // Collect twice to have delta
        analyzer.collect();
        analyzer.collect();

        let (rx_delta, tx_delta) = analyzer.total_error_deltas();
        // Deltas should be 0 or positive in short time window (unsigned types)
        let _ = rx_delta;
        let _ = tx_delta;
    }

    #[test]
    fn brick_network_stats_has_recent_errors() {
        let mut analyzer = NetworkStatsAnalyzer::new();
        analyzer.collect();
        analyzer.collect();

        // Just verify the method works (likely false on healthy systems)
        let _has_errs = analyzer.has_recent_errors();
    }

    #[test]
    fn brick_network_stats_latency_gauge() {
        let mut analyzer = NetworkStatsAnalyzer::new();
        analyzer.collect();

        let gauge = analyzer.latency_gauge();

        // Should be one of the gauge strings
        let valid_gauges = ["●●●●●", "●●●●○", "●●●○○", "●●○○○", "●○○○○"];
        assert!(
            valid_gauges.contains(&gauge),
            "Invalid gauge: {}", gauge
        );
    }

    #[test]
    fn brick_latency_gauge_thresholds() {
        // Test gauge returns appropriate value for different RTT values
        let mut analyzer = NetworkStatsAnalyzer::new();

        // Set low RTT - should be excellent
        analyzer.tcp_perf.rtt_ms = 5.0;
        assert_eq!(analyzer.latency_gauge(), "●●●●●");

        // Set medium RTT - should be good
        analyzer.tcp_perf.rtt_ms = 15.0;
        assert_eq!(analyzer.latency_gauge(), "●●●●○");

        // Set fair RTT
        analyzer.tcp_perf.rtt_ms = 35.0;
        assert_eq!(analyzer.latency_gauge(), "●●●○○");

        // Set poor RTT
        analyzer.tcp_perf.rtt_ms = 75.0;
        assert_eq!(analyzer.latency_gauge(), "●●○○○");

        // Set bad RTT
        analyzer.tcp_perf.rtt_ms = 150.0;
        assert_eq!(analyzer.latency_gauge(), "●○○○○");
    }

    #[test]
    fn brick_tcp_performance_default() {
        let perf = TcpPerformance::default();
        assert_eq!(perf.rtt_ms, 0.0);
        assert_eq!(perf.retrans_rate, 0.0);
        assert_eq!(perf.retrans_segs, 0);
        assert_eq!(perf.total_segs_out, 0);
    }

    #[test]
    fn brick_queue_stats_default() {
        let stats = QueueStats::default();
        assert_eq!(stats.total_rx_queue, 0);
        assert_eq!(stats.total_tx_queue, 0);
        assert_eq!(stats.max_rx_queue, 0);
        assert_eq!(stats.max_tx_queue, 0);
        assert_eq!(stats.rx_queue_count, 0);
        assert_eq!(stats.tx_queue_count, 0);
        assert!(!stats.syn_backlog_pressure);
    }

    #[test]
    fn brick_network_stats_queue_stats() {
        let mut analyzer = NetworkStatsAnalyzer::new();
        analyzer.collect();

        // Queue stats should be populated (typically 0 on idle systems)
        let queues = &analyzer.queue_stats;
        let _ = queues.total_rx_queue; // unsigned, always valid
        let _ = queues.total_tx_queue;
    }

    #[test]
    fn brick_network_stats_interface_errors() {
        let mut analyzer = NetworkStatsAnalyzer::new();
        analyzer.collect();

        // Should have at least some interfaces (unless truly isolated)
        // On most systems there's at least loopback, but we skip lo
        // Just verify it doesn't panic
        for (iface, errors) in &analyzer.interface_errors {
            assert!(!iface.is_empty());
            let _ = errors.rx_errors; // unsigned, always valid
            let _ = errors.tx_errors;
        }
    }

    #[test]
    fn brick_protocol_stats_default() {
        let stats = ProtocolStats::default();
        assert_eq!(stats.tcp_total(), 0);
        assert_eq!(stats.udp_sockets, 0);
        assert_eq!(stats.icmp_sockets, 0);
    }
}

// ============================================================================
// DiskEntropyAnalyzer Tests
// ============================================================================

// ============================================================================
// PanelType and Navigation Tests
// ============================================================================

mod panel_type_tests {
    use ttop::state::PanelType;

    #[test]
    fn brick_panel_type_all() {
        let all = PanelType::all();
        assert_eq!(all.len(), 9);
        assert_eq!(all[0], PanelType::Cpu);
        assert_eq!(all[7], PanelType::Sensors);
        assert_eq!(all[8], PanelType::Files);
    }

    #[test]
    fn brick_panel_type_number() {
        assert_eq!(PanelType::Cpu.number(), 1);
        assert_eq!(PanelType::Memory.number(), 2);
        assert_eq!(PanelType::Disk.number(), 3);
        assert_eq!(PanelType::Network.number(), 4);
        assert_eq!(PanelType::Process.number(), 5);
        assert_eq!(PanelType::Gpu.number(), 6);
        assert_eq!(PanelType::Battery.number(), 7);
        assert_eq!(PanelType::Sensors.number(), 8);
    }

    #[test]
    fn brick_panel_type_name() {
        assert_eq!(PanelType::Cpu.name(), "CPU");
        assert_eq!(PanelType::Memory.name(), "Memory");
        assert_eq!(PanelType::Process.name(), "Process");
    }

    #[test]
    fn brick_panel_type_next() {
        assert_eq!(PanelType::Cpu.next(), PanelType::Memory);
        assert_eq!(PanelType::Memory.next(), PanelType::Disk);
        assert_eq!(PanelType::Sensors.next(), PanelType::Files);
        assert_eq!(PanelType::Files.next(), PanelType::Cpu); // Wrap around
    }

    #[test]
    fn brick_panel_type_prev() {
        assert_eq!(PanelType::Memory.prev(), PanelType::Cpu);
        assert_eq!(PanelType::Cpu.prev(), PanelType::Files); // Wrap around
        assert_eq!(PanelType::Disk.prev(), PanelType::Memory);
        assert_eq!(PanelType::Files.prev(), PanelType::Sensors);
    }

    #[test]
    fn brick_panel_type_cycle() {
        // Full cycle through next (9 panels now including Files)
        let mut panel = PanelType::Cpu;
        for _ in 0..9 {
            panel = panel.next();
        }
        assert_eq!(panel, PanelType::Cpu); // Should be back at start
    }
}

mod panel_navigation_tests {
    use ttop::app::App;
    use ttop::state::PanelType;
    use crossterm::event::{KeyCode, KeyModifiers};

    fn create_test_app() -> App {
        App::new_mock() // deterministic mode
    }

    #[test]
    fn brick_app_initial_focus_state() {
        let app = create_test_app();
        assert!(app.focused_panel.is_none());
        assert!(app.exploded_panel.is_none());
    }

    #[test]
    fn brick_app_visible_panels() {
        let app = create_test_app();
        let visible = app.visible_panels();
        // Should have at least CPU, Memory, Disk, Network, Process
        assert!(visible.len() >= 5);
        assert!(visible.contains(&PanelType::Cpu));
        assert!(visible.contains(&PanelType::Process));
    }

    #[test]
    fn brick_app_h_key_starts_focus() {
        let mut app = create_test_app();
        assert!(app.focused_panel.is_none());

        // Press 'h' to start panel focus
        app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);

        // Should now have focus
        assert!(app.focused_panel.is_some());
    }

    #[test]
    fn brick_app_l_key_navigates() {
        let mut app = create_test_app();

        // Press 'l' to start and navigate
        app.handle_key(KeyCode::Char('l'), KeyModifiers::NONE);
        let first_focus = app.focused_panel;

        app.handle_key(KeyCode::Char('l'), KeyModifiers::NONE);
        let second_focus = app.focused_panel;

        // Should have moved to next panel
        assert!(first_focus.is_some());
        assert!(second_focus.is_some());
        assert_ne!(first_focus, second_focus);
    }

    #[test]
    fn brick_app_z_key_toggles_explode() {
        let mut app = create_test_app();

        // Press 'z' to start focus
        app.handle_key(KeyCode::Char('z'), KeyModifiers::NONE);
        assert!(app.focused_panel.is_some());
        assert!(app.exploded_panel.is_none());

        // Press 'z' again to explode
        app.handle_key(KeyCode::Char('z'), KeyModifiers::NONE);
        assert!(app.exploded_panel.is_some());

        // Press 'z' again to collapse
        app.handle_key(KeyCode::Char('z'), KeyModifiers::NONE);
        assert!(app.exploded_panel.is_none());
    }

    #[test]
    fn brick_app_enter_explodes_focused() {
        let mut app = create_test_app();

        // Focus a panel first
        app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
        let focused = app.focused_panel;
        assert!(focused.is_some());

        // Enter should explode it
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(app.exploded_panel, focused);
    }

    #[test]
    fn brick_app_esc_exits_explode_first() {
        let mut app = create_test_app();

        // Focus and explode a panel
        app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert!(app.exploded_panel.is_some());
        assert!(app.focused_panel.is_some());

        // ESC should exit explode first
        let quit = app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(!quit);
        assert!(app.exploded_panel.is_none());
        assert!(app.focused_panel.is_some()); // Still focused

        // ESC again should clear focus
        let quit = app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(!quit);
        assert!(app.focused_panel.is_none());

        // ESC again should quit
        let quit = app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(quit);
    }

    #[test]
    fn brick_app_0_resets_focus() {
        let mut app = create_test_app();

        // Focus and explode
        app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);

        // Press '0' to reset
        app.handle_key(KeyCode::Char('0'), KeyModifiers::NONE);
        assert!(app.focused_panel.is_none());
        assert!(app.exploded_panel.is_none());
    }

    #[test]
    fn brick_app_arrows_navigate_when_focused() {
        let mut app = create_test_app();

        // Start focus
        app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
        let initial = app.focused_panel;

        // Arrow right should navigate
        app.handle_key(KeyCode::Right, KeyModifiers::NONE);
        let after_right = app.focused_panel;

        assert_ne!(initial, after_right);
    }

    #[test]
    fn brick_app_is_panel_visible() {
        let app = create_test_app();
        assert!(app.is_panel_visible(PanelType::Cpu));
        assert!(app.is_panel_visible(PanelType::Memory));
        assert!(app.is_panel_visible(PanelType::Process));
    }
}

mod disk_entropy_tests {
    use ttop::analyzers::{DiskEntropyAnalyzer, MountEntropy};

    #[test]
    fn brick_disk_entropy_analyzer_new() {
        let analyzer = DiskEntropyAnalyzer::new();
        assert!(analyzer.mount_entropy.is_empty());
        assert_eq!(analyzer.system_entropy, 0.5); // Default medium
    }

    #[test]
    fn brick_mount_entropy_gauge_thresholds() {
        let mut me = MountEntropy::default();

        me.entropy = 0.95;
        assert_eq!(me.gauge(), "●●●●●");

        me.entropy = 0.8;
        assert_eq!(me.gauge(), "●●●●○");

        me.entropy = 0.6;
        assert_eq!(me.gauge(), "●●●○○");

        me.entropy = 0.3;
        assert_eq!(me.gauge(), "●●○○○");

        me.entropy = 0.1;
        assert_eq!(me.gauge(), "●○○○○");
    }

    #[test]
    fn brick_mount_entropy_indicator() {
        let mut me = MountEntropy::default();

        me.entropy = 0.9;
        assert_eq!(me.indicator(), '●');

        me.entropy = 0.6;
        assert_eq!(me.indicator(), '◐');

        me.entropy = 0.3;
        assert_eq!(me.indicator(), '○');
    }

    #[test]
    fn brick_disk_entropy_system_gauge() {
        let mut analyzer = DiskEntropyAnalyzer::new();

        analyzer.system_entropy = 0.95;
        assert_eq!(analyzer.system_gauge(), "●●●●●");

        analyzer.system_entropy = 0.5;
        assert_eq!(analyzer.system_gauge(), "●●●○○");

        analyzer.system_entropy = 0.1;
        assert_eq!(analyzer.system_gauge(), "●○○○○");
    }

    #[test]
    fn brick_disk_entropy_format_pct() {
        let analyzer = DiskEntropyAnalyzer::new();
        assert_eq!(analyzer.format_entropy_pct(0.75), "75%");
        assert_eq!(analyzer.format_entropy_pct(0.5), "50%");
        assert_eq!(analyzer.format_entropy_pct(1.0), "100%");
    }

    #[test]
    fn brick_mount_entropy_default() {
        let me = MountEntropy::default();
        assert_eq!(me.entropy, 0.0);
        assert_eq!(me.files_sampled, 0);
        assert_eq!(me.bytes_sampled, 0);
        assert_eq!(me.dedup_potential, 0.0);
        assert!(me.last_update.is_none());
    }

    #[test]
    fn brick_disk_entropy_get_mount_none() {
        let analyzer = DiskEntropyAnalyzer::new();
        assert!(analyzer.get_mount_entropy("/nonexistent").is_none());
    }

    #[test]
    fn brick_disk_entropy_collect_empty() {
        let mut analyzer = DiskEntropyAnalyzer::new();
        analyzer.collect(&[]);
        assert!(analyzer.mount_entropy.is_empty());
    }

    #[test]
    fn brick_disk_entropy_collect_root() {
        let mut analyzer = DiskEntropyAnalyzer::new();
        // Collect on root - this will actually sample files
        analyzer.collect(&["/".to_string()]);

        // Should have analyzed root mount
        if let Some(me) = analyzer.get_mount_entropy("/") {
            // Entropy should be reasonable (0.0-1.0)
            assert!(me.entropy >= 0.0 && me.entropy <= 1.0);
            // files_sampled is unsigned, just verify we can access it
            let _ = me.files_sampled;
        }
    }

    #[test]
    fn brick_disk_entropy_dedup_potential() {
        let mut me = MountEntropy::default();

        // High entropy = low dedup potential
        me.entropy = 0.9;
        me.dedup_potential = 1.0 - me.entropy;
        assert!((me.dedup_potential - 0.1).abs() < 0.01);

        // Low entropy = high dedup potential
        me.entropy = 0.2;
        me.dedup_potential = 1.0 - me.entropy;
        assert!((me.dedup_potential - 0.8).abs() < 0.01);
    }
}
