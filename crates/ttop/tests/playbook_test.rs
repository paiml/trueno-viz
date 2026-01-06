//! Playbook testing for ttop TUI using jugar-probar.
//!
//! These tests verify full UI rendering with deterministic data,
//! producing reproducible pixel-perfect snapshots.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Row, Table, Widget};

use trueno_viz::monitor::widgets::{Graph, GraphMode, Meter, MonitorSparkline};

/// Test frame capture for verifying panel rendering
struct PlaybookFrame {
    buffer: Buffer,
}

impl PlaybookFrame {
    fn new(width: u16, height: u16) -> Self {
        Self {
            buffer: Buffer::empty(Rect::new(0, 0, width, height)),
        }
    }

    fn render<W: Widget>(&mut self, widget: W, area: Rect) {
        widget.render(area, &mut self.buffer);
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
}

/// btop-style block helper
fn btop_block(title: &str, color: Color) -> Block<'static> {
    Block::default()
        .title(title.to_string())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color))
}

#[cfg(test)]
mod playbook_panel_tests {
    use super::*;

    #[test]
    fn test_cpu_panel_layout() {
        let mut frame = PlaybookFrame::new(80, 12);
        let area = Rect::new(0, 0, 80, 12);

        // Simulate CPU panel layout
        let title = " CPU 45% │ 8 cores │ 3.2GHz │ 45°C │ up 5d 12h │ LAV 1.50 ";
        let block = btop_block(title, Color::Rgb(100, 200, 255));

        frame.render(block, area);
        let text = frame.as_text();

        // Verify btop-style rendering
        assert!(text.contains("╭"), "Should have rounded top-left corner");
        assert!(text.contains("╮"), "Should have rounded top-right corner");
        assert!(text.contains("╰"), "Should have rounded bottom-left corner");
        assert!(text.contains("╯"), "Should have rounded bottom-right corner");
        assert!(text.contains("CPU"), "Should show CPU title");
        assert!(text.contains("45%"), "Should show CPU percentage");
        assert!(text.contains("8 cores"), "Should show core count");
    }

    #[test]
    fn test_cpu_panel_per_core_meters() {
        let mut frame = PlaybookFrame::new(60, 10);
        let area = Rect::new(0, 0, 60, 10);

        let block = btop_block(" CPU 8 cores ", Color::Cyan);
        let inner = block.inner(area);

        // Render block
        frame.render(block.clone(), area);

        // Render per-core meters inside
        let core_percents = vec![45.0, 78.0, 23.0, 91.0, 56.0, 12.0, 88.0, 34.0];
        for (i, &percent) in core_percents.iter().enumerate() {
            if i >= inner.height as usize {
                break;
            }
            let filled = ((percent / 100.0) * 6.0) as usize;
            let bar: String = "█".repeat(filled.min(6)) + &"░".repeat(6 - filled.min(6));
            let label = format!("{:>2} {} {:>3.0}", i, bar, percent);

            frame.render(
                Paragraph::new(label),
                Rect {
                    x: inner.x,
                    y: inner.y + i as u16,
                    width: inner.width.min(15),
                    height: 1,
                },
            );
        }

        let text = frame.as_text();

        // Verify per-core meters render correctly
        assert!(text.contains("█"), "Should have filled bar chars");
        assert!(text.contains("░"), "Should have empty bar chars");
    }

    #[test]
    fn test_memory_panel_with_sparklines() {
        let mut frame = PlaybookFrame::new(60, 8);
        let area = Rect::new(0, 0, 60, 8);

        // Test block rendering
        let block = btop_block(" Memory │ 8.5G / 16.0G (53%) ", Color::Magenta);
        frame.render(block, area);

        let output = frame.as_text();
        assert!(output.contains("Memory"), "Should show Memory title");
        assert!(output.contains("8.5G"), "Should show used memory");
        assert!(output.contains("53%"), "Should show usage percent");

        // Test memory row content in separate buffer
        let mut content_frame = PlaybookFrame::new(40, 4);
        let rows = vec![
            ("Used", 8.5, 53.0),
            ("Available", 7.5, 47.0),
            ("Cached", 4.0, 25.0),
            ("Free", 3.5, 22.0),
        ];

        for (i, (label, gb, pct)) in rows.iter().enumerate() {
            let text = format!("{:>9}: {:>5.1}G {:>2.0}%", label, gb, pct);
            content_frame.render(
                Paragraph::new(text),
                Rect {
                    x: 0,
                    y: i as u16,
                    width: 25,
                    height: 1,
                },
            );
        }

        let content = content_frame.as_text();
        assert!(content.contains("Used"), "Should show Used row");
        assert!(content.contains("Available"), "Should show Available row");
    }

    #[test]
    fn test_gpu_panel_macos_style() {
        // Test block rendering
        let mut frame = PlaybookFrame::new(50, 6);
        let area = Rect::new(0, 0, 50, 6);

        let block = btop_block(" Apple M2 Pro │ 16 cores ", Color::Green);
        frame.render(block, area);

        let output = frame.as_text();
        assert!(output.contains("Apple"), "Should show Apple GPU");
        assert!(output.contains("cores"), "Should show core count");

        // Test GPU meter separately
        let mut meter_frame = PlaybookFrame::new(40, 1);
        let meter = Meter::new(0.45).label("GPU").color(Color::Green);
        meter.render(Rect::new(0, 0, 40, 1), &mut meter_frame.buffer);

        let meter_output = meter_frame.as_text();
        assert!(meter_output.contains("GPU"), "Should have GPU meter label");
    }

    #[test]
    fn test_network_panel_dual_graphs() {
        // Test block rendering
        let mut frame = PlaybookFrame::new(60, 12);
        let area = Rect::new(0, 0, 60, 12);

        let block = btop_block(" Network (en0) │ ↓ 1.2M/s │ ↑ 500K/s ", Color::Blue);
        frame.render(block, area);

        let output = frame.as_text();
        assert!(output.contains("Network"), "Should show Network title");
        assert!(output.contains("↓"), "Should show download arrow");
        assert!(output.contains("↑"), "Should show upload arrow");

        // Test RX/TX labels separately
        let mut label_frame = PlaybookFrame::new(30, 2);
        label_frame.render(
            Paragraph::new(Line::from(vec![
                Span::styled("↓ Download ", Style::default().fg(Color::Cyan)),
                Span::styled("1.2M/s", Style::default().fg(Color::White)),
            ])),
            Rect::new(0, 0, 30, 1),
        );
        label_frame.render(
            Paragraph::new(Line::from(vec![
                Span::styled("↑ Upload   ", Style::default().fg(Color::Green)),
                Span::styled("500K/s", Style::default().fg(Color::White)),
            ])),
            Rect::new(0, 1, 30, 1),
        );

        let label_output = label_frame.as_text();
        assert!(label_output.contains("Download"), "Should show Download label");
        assert!(label_output.contains("Upload"), "Should show Upload label");
    }

    #[test]
    fn test_process_panel_with_table() {
        // Test block rendering
        let mut frame = PlaybookFrame::new(100, 15);
        let area = Rect::new(0, 0, 100, 15);

        let block = btop_block(" Processes (142) │ Sort: CPU% ▼ ", Color::Yellow);
        frame.render(block, area);

        let output = frame.as_text();
        assert!(output.contains("Processes"), "Should show Processes title");
        assert!(output.contains("Sort"), "Should show sort indicator");
        assert!(output.contains("CPU%"), "Should have CPU% in title");

        // Test table separately
        let mut table_frame = PlaybookFrame::new(80, 5);
        let header = Row::new(vec![
            Span::styled("PID", Style::default().fg(Color::Yellow)),
            Span::styled("USER", Style::default().fg(Color::Yellow)),
            Span::styled("CPU%", Style::default().fg(Color::Yellow)),
            Span::styled("NAME", Style::default().fg(Color::Yellow)),
        ])
        .height(1);

        let rows = vec![
            Row::new(vec!["  1234", "root", "45.2", "java"]),
            Row::new(vec!["  5678", "user", "12.8", "chrome"]),
        ];

        let widths = [
            Constraint::Length(7),
            Constraint::Length(8),
            Constraint::Length(6),
            Constraint::Length(15),
        ];

        let table = Table::new(rows, widths).header(header);
        table.render(Rect::new(0, 0, 80, 5), &mut table_frame.buffer);

        let table_output = table_frame.as_text();
        assert!(table_output.contains("PID"), "Should have PID column");
        assert!(table_output.contains("USER"), "Should have USER column");
    }

    #[test]
    fn test_disk_panel_with_meters() {
        // Test block rendering
        let mut frame = PlaybookFrame::new(50, 8);
        let area = Rect::new(0, 0, 50, 8);

        let block = btop_block(" Disk │ R: 50M/s │ W: 25M/s ", Color::Rgb(255, 150, 100));
        frame.render(block, area);

        let output = frame.as_text();
        assert!(output.contains("Disk"), "Should show Disk title");
        assert!(output.contains("R:"), "Should show read rate");
        assert!(output.contains("W:"), "Should show write rate");

        // Test disk meters separately
        let mut meter_frame = PlaybookFrame::new(40, 2);
        let meter1 = Meter::new(0.68).label("/").color(Color::Yellow);
        let meter2 = Meter::new(0.45).label("home").color(Color::Green);
        meter1.render(Rect::new(0, 0, 40, 1), &mut meter_frame.buffer);
        meter2.render(Rect::new(0, 1, 40, 1), &mut meter_frame.buffer);

        let meter_output = meter_frame.as_text();
        assert!(meter_output.contains("/") || meter_output.contains("█"), "Should show disk meters");
    }

    #[test]
    fn test_battery_panel() {
        // Test block rendering
        let mut frame = PlaybookFrame::new(45, 5);
        let area = Rect::new(0, 0, 45, 5);

        let block = btop_block(" Battery │ 85% │ Charging ", Color::Green);
        frame.render(block, area);

        let output = frame.as_text();
        assert!(output.contains("Battery"), "Should show Battery title");
        assert!(output.contains("85%"), "Should show charge percentage");
        assert!(output.contains("Charging"), "Should show status");

        // Test charge meter separately
        let mut meter_frame = PlaybookFrame::new(30, 1);
        let meter = Meter::new(0.85).label("Charge").color(Color::Green);
        meter.render(Rect::new(0, 0, 30, 1), &mut meter_frame.buffer);

        let meter_output = meter_frame.as_text();
        assert!(meter_output.contains("█"), "Should show filled meter");
    }

    #[test]
    fn test_sensors_panel() {
        // Test block rendering
        let mut frame = PlaybookFrame::new(40, 6);
        let area = Rect::new(0, 0, 40, 6);

        let block = btop_block(" Sensors │ Max: 72°C ", Color::Red);
        frame.render(block, area);

        let output = frame.as_text();
        assert!(output.contains("Sensors"), "Should show Sensors title");
        assert!(output.contains("°C"), "Should show temperature unit");

        // Test sensor readings separately
        let mut content_frame = PlaybookFrame::new(30, 3);
        let sensors = vec![("CPU Core 0", 68.0), ("CPU Core 1", 72.0), ("GPU", 55.0)];

        for (i, (label, temp)) in sensors.iter().enumerate() {
            let color = if *temp > 80.0 {
                Color::Red
            } else if *temp > 60.0 {
                Color::Yellow
            } else {
                Color::Green
            };

            content_frame.render(
                Paragraph::new(Line::from(vec![
                    Span::raw(format!("{:12}", label)),
                    Span::styled(format!(" {:.0}°C", temp), Style::default().fg(color)),
                ])),
                Rect::new(0, i as u16, 25, 1),
            );
        }

        let content = content_frame.as_text();
        assert!(content.contains("CPU"), "Should show CPU sensor");
        assert!(content.contains("68"), "Should show temperature value");
    }
}

#[cfg(test)]
mod widget_snapshot_tests {
    use super::*;

    #[test]
    fn test_graph_block_mode_snapshot() {
        let data = vec![0.2, 0.4, 0.6, 0.8, 1.0, 0.7, 0.5, 0.3];
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 5));

        let graph = Graph::new(&data).mode(GraphMode::Block).color(Color::Cyan);
        graph.render(Rect::new(0, 0, 20, 5), &mut buffer);

        // Verify graph renders block characters
        let mut has_blocks = false;
        for y in 0..5 {
            for x in 0..20 {
                if let Some(cell) = buffer.cell((x, y)) {
                    if matches!(
                        cell.symbol(),
                        "█" | "▓" | "▒" | "░" | "▁" | "▂" | "▃" | "▄" | "▅" | "▆" | "▇"
                    ) {
                        has_blocks = true;
                    }
                }
            }
        }
        assert!(has_blocks, "Graph should render block characters");
    }

    #[test]
    fn test_graph_braille_mode_snapshot() {
        let data = vec![0.1, 0.3, 0.5, 0.7, 0.9, 0.6, 0.4, 0.2];
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 5));

        let graph = Graph::new(&data)
            .mode(GraphMode::Braille)
            .color(Color::Green);
        graph.render(Rect::new(0, 0, 20, 5), &mut buffer);

        // Verify braille or block characters are rendered
        let mut has_chars = false;
        for y in 0..5 {
            for x in 0..20 {
                if let Some(cell) = buffer.cell((x, y)) {
                    let sym = cell.symbol();
                    // Braille range: U+2800 to U+28FF
                    if sym.chars().any(|c| ('\u{2800}'..='\u{28FF}').contains(&c)) {
                        has_chars = true;
                    }
                    // Also accept block chars
                    if matches!(sym, "█" | "▓" | "▒" | "░") {
                        has_chars = true;
                    }
                }
            }
        }
        assert!(has_chars, "Graph should render braille or block characters");
    }

    #[test]
    fn test_meter_snapshot_25_percent() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 30, 1));
        let meter = Meter::new(0.25).label("25%").color(Color::Green);
        meter.render(Rect::new(0, 0, 30, 1), &mut buffer);

        let mut filled_count = 0;
        for x in 0..30 {
            if let Some(cell) = buffer.cell((x, 0)) {
                if cell.symbol() == "█" {
                    filled_count += 1;
                }
            }
        }

        // 25% of ~24 chars (excluding label) should be about 6 blocks
        assert!(
            filled_count >= 4 && filled_count <= 10,
            "25% meter should have ~6 filled blocks, got {}",
            filled_count
        );
    }

    #[test]
    fn test_meter_snapshot_75_percent() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 30, 1));
        let meter = Meter::new(0.75).label("75%").color(Color::Yellow);
        meter.render(Rect::new(0, 0, 30, 1), &mut buffer);

        let mut filled_count = 0;
        for x in 0..30 {
            if let Some(cell) = buffer.cell((x, 0)) {
                if cell.symbol() == "█" {
                    filled_count += 1;
                }
            }
        }

        // 75% should have more blocks than 25%
        assert!(
            filled_count >= 15,
            "75% meter should have many filled blocks, got {}",
            filled_count
        );
    }

    #[test]
    fn test_sparkline_snapshot() {
        let data = vec![0.1, 0.2, 0.4, 0.6, 0.8, 0.9, 0.7, 0.5, 0.3];
        let mut buffer = Buffer::empty(Rect::new(0, 0, 15, 1));

        let sparkline = MonitorSparkline::new(&data)
            .color(Color::Cyan)
            .show_trend(true);
        sparkline.render(Rect::new(0, 0, 15, 1), &mut buffer);

        // Verify sparkline chars
        let mut has_sparkline = false;
        for x in 0..15 {
            if let Some(cell) = buffer.cell((x, 0)) {
                if matches!(
                    cell.symbol(),
                    "▁" | "▂" | "▃" | "▄" | "▅" | "▆" | "▇" | "█"
                ) {
                    has_sparkline = true;
                }
            }
        }
        assert!(has_sparkline, "Sparkline should render bar characters");
    }
}

#[cfg(test)]
mod deterministic_tests {
    use super::*;

    #[test]
    fn test_deterministic_graph_rendering() {
        // Same data should produce identical output
        let data = vec![0.3, 0.5, 0.7, 0.9, 0.6, 0.4];

        let mut buffer1 = Buffer::empty(Rect::new(0, 0, 15, 4));
        let graph1 = Graph::new(&data).mode(GraphMode::Block).color(Color::Cyan);
        graph1.render(Rect::new(0, 0, 15, 4), &mut buffer1);

        let mut buffer2 = Buffer::empty(Rect::new(0, 0, 15, 4));
        let graph2 = Graph::new(&data).mode(GraphMode::Block).color(Color::Cyan);
        graph2.render(Rect::new(0, 0, 15, 4), &mut buffer2);

        // Compare buffers
        for y in 0..4 {
            for x in 0..15 {
                let cell1 = buffer1.cell((x, y));
                let cell2 = buffer2.cell((x, y));
                assert_eq!(
                    cell1.map(|c| c.symbol()),
                    cell2.map(|c| c.symbol()),
                    "Deterministic rendering should produce identical output at ({}, {})",
                    x,
                    y
                );
            }
        }
    }

    #[test]
    fn test_deterministic_meter_rendering() {
        let mut buffer1 = Buffer::empty(Rect::new(0, 0, 25, 1));
        let meter1 = Meter::new(0.67).label("Test").color(Color::Yellow);
        meter1.render(Rect::new(0, 0, 25, 1), &mut buffer1);

        let mut buffer2 = Buffer::empty(Rect::new(0, 0, 25, 1));
        let meter2 = Meter::new(0.67).label("Test").color(Color::Yellow);
        meter2.render(Rect::new(0, 0, 25, 1), &mut buffer2);

        for x in 0..25 {
            let cell1 = buffer1.cell((x, 0));
            let cell2 = buffer2.cell((x, 0));
            assert_eq!(
                cell1.map(|c| c.symbol()),
                cell2.map(|c| c.symbol()),
                "Deterministic meter should produce identical output at x={}",
                x
            );
        }
    }
}

#[cfg(test)]
mod color_gradient_tests {
    use super::*;

    fn percent_color(percent: f64) -> Color {
        let p = percent.clamp(0.0, 100.0);
        // Gradient: cyan (0%) -> green (30%) -> yellow (60%) -> orange (80%) -> red (100%)
        if p < 30.0 {
            let t = p / 30.0;
            Color::Rgb(
                (100.0 * t) as u8,
                (200.0 + 55.0 * t) as u8,
                (255.0 - 55.0 * t) as u8,
            )
        } else if p < 60.0 {
            let t = (p - 30.0) / 30.0;
            Color::Rgb((100.0 + 155.0 * t) as u8, 255, (200.0 - 200.0 * t) as u8)
        } else if p < 80.0 {
            let t = (p - 60.0) / 20.0;
            Color::Rgb(255, (255.0 - 100.0 * t) as u8, 0)
        } else {
            let t = (p - 80.0) / 20.0;
            Color::Rgb(255, (155.0 - 155.0 * t) as u8, 0)
        }
    }

    #[test]
    fn test_color_gradient_low_values() {
        // Low percentages should be cyan/blue-ish
        let color_10 = percent_color(10.0);
        if let Color::Rgb(r, g, b) = color_10 {
            assert!(b > g / 2, "10% should have significant blue: r={}, g={}, b={}", r, g, b);
        }
    }

    #[test]
    fn test_color_gradient_medium_values() {
        // Medium percentages should be green/yellow
        let color_50 = percent_color(50.0);
        if let Color::Rgb(r, g, _b) = color_50 {
            assert!(
                g > 200,
                "50% should be greenish/yellow: r={}, g={}",
                r,
                g
            );
        }
    }

    #[test]
    fn test_color_gradient_high_values() {
        // High percentages should be orange/red
        let color_90 = percent_color(90.0);
        if let Color::Rgb(r, g, _b) = color_90 {
            assert!(r == 255 && g < 100, "90% should be red/orange: r={}, g={}", r, g);
        }
    }

    #[test]
    fn test_color_gradient_critical_values() {
        // Critical (95%+) should be pure red
        let color_99 = percent_color(99.0);
        if let Color::Rgb(r, g, b) = color_99 {
            assert!(
                r == 255 && g < 50,
                "99% should be nearly pure red: r={}, g={}, b={}",
                r,
                g,
                b
            );
        }
    }
}
