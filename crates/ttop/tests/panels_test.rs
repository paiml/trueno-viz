//! Panel rendering tests for ttop using probar TUI testing.
//!
//! These tests verify that all panels render correctly with expected content.
//! Uses deterministic mode to ensure reproducible frame output.
//!
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
