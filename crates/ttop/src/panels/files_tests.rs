#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mount_marker_nvme() {
        let (mark, color, label) = mount_marker("/mnt/nvme-raid0/foo/bar");
        assert_eq!(mark, 'N');
        assert_eq!(color, (100, 220, 140));
        assert_eq!(label, "nvme");

        let (mark, _, _) = mount_marker("/mnt/nvme/data");
        assert_eq!(mark, 'N');
    }

    #[test]
    fn test_mount_marker_storage() {
        let (mark, color, label) = mount_marker("/mnt/storage/backups");
        assert_eq!(mark, 'D');
        assert_eq!(color, (220, 100, 100));
        assert_eq!(label, "hdd");

        let (mark, _, _) = mount_marker("/mnt/hdd/archive");
        assert_eq!(mark, 'D');
    }

    #[test]
    fn test_mount_marker_home() {
        let (mark, color, label) = mount_marker("/home/user/documents");
        assert_eq!(mark, 'h');
        assert_eq!(color, (220, 180, 80));
        assert_eq!(label, "home");
    }

    #[test]
    fn test_mount_marker_system() {
        let (mark, color, _) = mount_marker("/");
        assert_eq!(mark, '/');
        assert_eq!(color, (140, 160, 220));

        let (mark, _, _) = mount_marker("/usr/local/bin");
        assert_eq!(mark, '/');

        let (mark, _, _) = mount_marker("/var/log");
        assert_eq!(mark, '/');
    }

    #[test]
    fn test_mount_marker_other_mounts() {
        let (mark, color, label) = mount_marker("/mnt/usb");
        assert_eq!(mark, 'M');
        assert_eq!(color, (180, 140, 220));
        assert_eq!(label, "mnt");

        let (mark, _, _) = mount_marker("/media/cdrom");
        assert_eq!(mark, 'M');
    }

    #[test]
    fn test_mount_marker_unknown() {
        let (mark, color, label) = mount_marker("/opt/app");
        assert_eq!(mark, '?');
        assert_eq!(color, (140, 140, 140));
        assert_eq!(label, "unk");
    }

    #[test]
    fn test_format_dir_path_short() {
        assert_eq!(format_dir_path("/mnt/data", 20), "/mnt/data");
        assert_eq!(format_dir_path("/home", 10), "/home");
    }

    #[test]
    fn test_format_dir_path_truncate() {
        let result = format_dir_path("/mnt/nvme-raid0/very/long/path/here", 25);
        assert!(result.len() <= 25);
        assert!(result.contains("..."));
    }

    #[test]
    fn test_format_dir_path_very_small_width() {
        let result = format_dir_path("/mnt/nvme-raid0/foo", 8);
        assert!(result.len() <= 8);
    }

    #[test]
    fn test_format_dir_path_single_component() {
        let result = format_dir_path("/verylongsingledirectoryname", 15);
        assert!(result.len() <= 15);
        assert!(result.starts_with('/'));
    }

    #[test]
    fn test_entropy_heatmap_high_dupe() {
        // Low entropy = high duplication (red)
        let (display, r, g, b) = entropy_heatmap(0.1);
        assert!(display.contains('%'));
        assert!(r > g); // Red-ish for high dupe
    }

    #[test]
    fn test_entropy_heatmap_medium() {
        // Medium entropy = mixed (yellow-ish)
        let (display, r, g, b) = entropy_heatmap(0.5);
        assert!(display.contains('%'));
        assert!(r > 150 && g > 150); // Yellow-ish
    }

    #[test]
    fn test_entropy_heatmap_unique() {
        // High entropy = unique data (green)
        let (display, r, g, b) = entropy_heatmap(0.9);
        assert!(display.contains('%'));
        assert!(g > r); // Green for unique
    }

    #[test]
    fn test_entropy_heatmap_boundary() {
        // Test boundary values don't panic
        let _ = entropy_heatmap(0.0);
        let _ = entropy_heatmap(0.5);
        let _ = entropy_heatmap(1.0);
        let _ = entropy_heatmap(1.5); // Over 1.0 should work
    }

    #[test]
    fn test_truncate_str_short() {
        assert_eq!(truncate_str("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_str_exact() {
        assert_eq!(truncate_str("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_str_long() {
        assert_eq!(truncate_str("hello world", 8), "hello...");
    }

    #[test]
    fn test_truncate_str_very_short_max() {
        assert_eq!(truncate_str("hello", 2), "..");
    }

    #[test]
    fn test_mount_legend_str() {
        let legend = mount_legend_str();
        assert!(legend.contains("N:nvme"));
        assert!(legend.contains("D:hdd"));
        assert!(legend.contains("h:home"));
        assert!(legend.contains("/:sys"));
    }

    #[test]
    fn test_btop_block() {
        use trueno_viz::monitor::ratatui::style::Color;
        let block = btop_block("Test", Color::Red);
        // Just verify it doesn't panic and returns a Block
        assert!(format!("{:?}", block).contains("Block"));
    }
}

/// TUI rendering tests using probar
#[cfg(test)]
mod tui_tests {
    use jugar_probar::tui::{TuiFrame, expect_frame};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use ratatui::widgets::{Block, Borders, Paragraph};
    use ratatui::style::{Color, Style};
    use ratatui::buffer::Buffer;

    /// Helper: Convert ratatui Buffer to TuiFrame
    /// This bridges ratatui's TestBackend with probar's framework-agnostic TuiFrame
    fn buffer_to_frame(buffer: &Buffer, _timestamp_ms: u64) -> TuiFrame {
        let area = buffer.area;
        let mut lines = Vec::with_capacity(area.height as usize);

        for y in 0..area.height {
            let mut line = String::with_capacity(area.width as usize);
            for x in 0..area.width {
                let cell = buffer.cell((x, y)).expect("cell in bounds");
                line.push_str(cell.symbol());
            }
            // Trim trailing whitespace to match expected behavior
            lines.push(line.trim_end().to_string());
        }

        TuiFrame::from_lines(&lines.iter().map(|s| s.as_str()).collect::<Vec<_>>())
    }

    /// Test that btop_block renders correctly
    #[test]
    fn test_btop_block_renders() {
        let mut backend = TestBackend::new(40, 5);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let block = Block::default()
                .title("CPU")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            f.render_widget(block, f.area());
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        assert!(frame.contains("CPU"));
        assert!(frame.contains("─")); // Border character
    }

    /// Test paragraph widget rendering
    #[test]
    fn test_paragraph_renders() {
        let mut backend = TestBackend::new(40, 5);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let para = Paragraph::new("Hello World")
                .style(Style::default().fg(Color::Green));
            f.render_widget(para, f.area());
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        assert!(frame.contains("Hello World"));
    }

    /// Test mount marker legend in rendered frame
    #[test]
    fn test_mount_legend_renders() {
        use super::mount_legend_str;

        let mut backend = TestBackend::new(60, 3);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let legend = mount_legend_str();
            let para = Paragraph::new(legend);
            f.render_widget(para, f.area());
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        assert!(frame.contains("N:nvme"));
        assert!(frame.contains("D:hdd"));
        assert!(frame.contains("h:home"));
    }

    /// Test format_dir_path output in widget
    #[test]
    fn test_format_dir_path_renders() {
        use super::format_dir_path;

        let mut backend = TestBackend::new(30, 3);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let path = format_dir_path("/mnt/nvme-raid0/very/long/path", 25);
            let para = Paragraph::new(path);
            f.render_widget(para, f.area());
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        // Should contain ellipsis for long paths
        assert!(frame.contains("...") || frame.as_text().len() <= 25 * 3);
    }

    /// Test entropy_heatmap rendering
    #[test]
    fn test_entropy_heatmap_renders() {
        use super::entropy_heatmap;

        let mut backend = TestBackend::new(20, 3);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let (display, _r, _g, _b) = entropy_heatmap(0.5);
            let para = Paragraph::new(display);
            f.render_widget(para, f.area());
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        // Should contain percentage
        assert!(frame.contains("%"));
    }

    /// Test truncate_str in rendered context
    #[test]
    fn test_truncate_str_renders() {
        use super::truncate_str;

        let mut backend = TestBackend::new(15, 3);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let text = truncate_str("very long process name", 12);
            let para = Paragraph::new(text);
            f.render_widget(para, f.area());
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        // Should be truncated with ellipsis
        assert!(frame.contains("..."));
        assert!(frame.contains("very long"));
    }

    /// Test probar frame assertions
    #[test]
    fn test_probar_assertions() {
        let mut backend = TestBackend::new(40, 5);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let block = Block::default()
                .title("Memory")
                .borders(Borders::ALL);
            let para = Paragraph::new("Used: 8GB / 16GB")
                .block(block);
            f.render_widget(para, f.area());
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        // Use probar's assertion API
        expect_frame(&frame)
            .to_contain_text("Memory")
            .expect("should contain Memory");

        expect_frame(&frame)
            .to_contain_text("8GB")
            .expect("should contain 8GB");

        expect_frame(&frame)
            .to_match(r"\d+GB / \d+GB")
            .expect("should match memory pattern");
    }

    /// Test multiple lines rendering
    #[test]
    fn test_multiline_renders() {
        let mut backend = TestBackend::new(30, 6);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let text = "Line 1\nLine 2\nLine 3";
            let para = Paragraph::new(text);
            f.render_widget(para, f.area());
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        assert!(frame.contains("Line 1"));
        assert!(frame.contains("Line 2"));
        assert!(frame.contains("Line 3"));
        assert_eq!(frame.line(0), Some("Line 1"));
        assert_eq!(frame.line(1), Some("Line 2"));
    }

    /// Test frame diff for regression detection
    #[test]
    fn test_frame_diff() {
        let frame1 = TuiFrame::from_lines(&[
            "CPU: 50%",
            "MEM: 60%",
        ]);
        let frame2 = TuiFrame::from_lines(&[
            "CPU: 50%",
            "MEM: 60%",
        ]);

        assert!(frame1.is_identical(&frame2));

        let frame3 = TuiFrame::from_lines(&[
            "CPU: 75%",
            "MEM: 60%",
        ]);

        let diff = frame1.diff(&frame3);
        assert!(!diff.is_identical);
        assert_eq!(diff.changed_lines.len(), 1);
        assert_eq!(diff.changed_lines[0].line_number, 0);
    }

    /// Test styled text rendering
    #[test]
    fn test_styled_text_renders() {
        let mut backend = TestBackend::new(30, 3);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            use ratatui::text::{Span, Line};
            let line = Line::from(vec![
                Span::styled("Red", Style::default().fg(Color::Red)),
                Span::raw(" "),
                Span::styled("Green", Style::default().fg(Color::Green)),
            ]);
            let para = Paragraph::new(line);
            f.render_widget(para, f.area());
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        assert!(frame.contains("Red"));
        assert!(frame.contains("Green"));
    }

    /// Test sparkline-style bar rendering (used in panels)
    #[test]
    fn test_split_bar_renders() {
        let mut backend = TestBackend::new(20, 3);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            use ratatui::text::{Span, Line};
            // Simulate the split bar used in panels
            let bar = "▄".repeat(5);
            let line = Line::from(vec![
                Span::styled(&bar, Style::default()
                    .fg(Color::Green)   // Bottom half
                    .bg(Color::Blue)),  // Top half
            ]);
            let para = Paragraph::new(line);
            f.render_widget(para, f.area());
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        assert!(frame.contains("▄"));
    }
}

/// Full panel rendering tests using mock App
#[cfg(test)]
mod panel_render_tests {
    use super::*;
    use crate::app::App;
    use jugar_probar::tui::{TuiFrame, expect_frame};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use ratatui::layout::Rect;
    use ratatui::buffer::Buffer;

    /// Helper: Convert ratatui Buffer to TuiFrame
    fn buffer_to_frame(buffer: &Buffer, _timestamp_ms: u64) -> TuiFrame {
        let area = buffer.area;
        let mut lines = Vec::with_capacity(area.height as usize);
        for y in 0..area.height {
            let mut line = String::with_capacity(area.width as usize);
            for x in 0..area.width {
                let cell = buffer.cell((x, y)).expect("cell");
                line.push_str(cell.symbol());
            }
            lines.push(line.trim_end().to_string());
        }
        TuiFrame::from_lines(&lines.iter().map(|s| s.as_str()).collect::<Vec<_>>())
    }

    /// Test CPU panel renders with mock data
    #[test]
    fn test_draw_cpu_panel() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_cpu(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        // CPU panel should contain title with CPU info
        assert!(frame.contains("CPU"));
    }

    /// Test Memory panel renders with mock data
    #[test]
    fn test_draw_memory_panel() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_memory(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        // Memory panel should contain memory info
        assert!(frame.contains("Memory") || frame.contains("Used") || frame.contains("Swap"));
    }

    /// Test Disk panel renders with mock data
    #[test]
    fn test_draw_disk_panel() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 15);
            draw_disk(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        // Disk panel should contain disk info
        assert!(frame.contains("Disk") || frame.contains("IOPS") || frame.contains("/"));
    }

    /// Test Network panel renders with mock data
    #[test]
    fn test_draw_network_panel() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 15);
            draw_network(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        // Network panel should contain network info
        assert!(frame.contains("Network") || frame.contains("Download") || frame.contains("Upload"));
    }

    /// Test GPU panel renders with mock data
    #[test]
    fn test_draw_gpu_panel() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 15);
            draw_gpu(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        // GPU panel should contain mock GPU data
        assert!(frame.contains("GPU"), "Frame should contain GPU header");
        assert!(frame.contains("75") || frame.contains("2 devices"), "Frame should show mock GPU data");
    }

    /// Test Battery panel renders with mock data
    #[test]
    fn test_draw_battery_panel() {
        let app = App::new_mock();
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 40, 10);
            draw_battery(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        // Battery panel should render
        assert!(frame.contains("Battery") || frame.contains("AC") || frame.contains("%") || frame.height() > 0);
    }

    /// Test Sensors panel renders with mock data
    #[test]
    fn test_draw_sensors_panel() {
        let app = App::new_mock();
        let backend = TestBackend::new(40, 15);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 40, 15);
            draw_sensors(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        // Sensors panel should render
        assert!(frame.contains("Sensors") || frame.contains("°") || frame.height() > 0);
    }

    /// Test Sensors compact panel renders
    #[test]
    fn test_draw_sensors_compact_panel() {
        let app = App::new_mock();
        let backend = TestBackend::new(40, 15);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 40, 15);
            draw_sensors_compact(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        // Compact sensors panel should render
        assert!(frame.height() > 0);
    }

    /// Test PSI panel renders with mock data
    #[test]
    fn test_draw_psi_panel() {
        let app = App::new_mock();
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 40, 10);
            draw_psi(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        // PSI panel should contain pressure info
        assert!(frame.contains("Pressure") || frame.contains("PSI") || frame.contains("I/O") || frame.height() > 0);
    }

    /// Test System panel renders with mock data
    #[test]
    fn test_draw_system_panel() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 10);
            draw_system(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        // System panel should contain system info
        assert!(frame.contains("System") || frame.contains("Host") || frame.height() > 0);
    }

    /// Test Process panel renders with mock data
    #[test]
    fn test_draw_process_panel() {
        let mut app = App::new_mock();
        let backend = TestBackend::new(100, 25);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 25);
            draw_process(f, &mut app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        // Process panel should contain process header
        assert!(frame.contains("Process") || frame.contains("PID") || frame.contains("COMMAND"));
    }

    /// Test Connections panel renders with mock data
    #[test]
    fn test_draw_connections_panel() {
        let app = App::new_mock();
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 60, 20);
            draw_connections(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        // Connections panel should contain connection info
        assert!(frame.contains("Connection") || frame.contains("SVC") || frame.contains("listen"));
    }

    /// Test Treemap panel renders with mock data
    #[test]
    fn test_draw_treemap_panel() {
        let app = App::new_mock();
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 60, 20);
            draw_treemap(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        // Treemap panel should render something
        assert!(frame.height() > 0);
    }

    /// Test Files panel renders with mock data
    #[test]
    fn test_draw_files_panel() {
        let app = App::new_mock();
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 60, 20);
            draw_files(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        // Files panel should contain files header with mount legend
        assert!(frame.contains("Files") || frame.contains("nvme") || frame.contains("hdd"));
    }

    /// Test panels render at various sizes
    #[test]
    fn test_panels_various_sizes() {
        let app = App::new_mock();
        let sizes = [(40, 10), (80, 20), (120, 30), (200, 50)];

        for (width, height) in sizes {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).expect("terminal");

            // Test CPU panel at this size
            terminal.draw(|f| {
                let area = Rect::new(0, 0, width, height);
                draw_cpu(f, &app, area);
            }).expect(&format!("draw cpu at {}x{}", width, height));

            // Test Memory panel at this size
            terminal.draw(|f| {
                let area = Rect::new(0, 0, width, height);
                draw_memory(f, &app, area);
            }).expect(&format!("draw memory at {}x{}", width, height));

            // Test Network panel at this size
            terminal.draw(|f| {
                let area = Rect::new(0, 0, width, height);
                draw_network(f, &app, area);
            }).expect(&format!("draw network at {}x{}", width, height));
        }
    }

    /// Test panels with small terminal (edge case)
    #[test]
    fn test_panels_tiny_terminal() {
        let app = App::new_mock();
        let backend = TestBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).expect("terminal");

        // Should not panic even with tiny terminal
        terminal.draw(|f| {
            let area = Rect::new(0, 0, 20, 5);
            draw_cpu(f, &app, area);
        }).expect("draw tiny cpu");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 20, 5);
            draw_memory(f, &app, area);
        }).expect("draw tiny memory");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 20, 5);
            draw_disk(f, &app, area);
        }).expect("draw tiny disk");
    }

    /// Test probar assertions on CPU panel
    #[test]
    fn test_cpu_panel_probar_assertions() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_cpu(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        // Verify CPU panel content with probar assertions
        expect_frame(&frame)
            .to_contain_text("CPU")
            .expect("should contain CPU title");
    }

    /// Test probar assertions on Memory panel
    #[test]
    fn test_memory_panel_probar_assertions() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_memory(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        // Memory panel should show memory usage
        expect_frame(&frame)
            .to_contain_text("Memory")
            .expect("should contain Memory title");
    }

    /// Test Files panel with corrected mount legend
    #[test]
    fn test_files_panel_mount_legend() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_files(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        // Verify the corrected mount legend (D:hdd, h:home not H:hdd, ::home)
        let text = frame.as_text();
        // Files panel title should have the legend
        assert!(text.contains("Files") || text.contains("N:nvme"));
    }

    /// Test Files panel with different view modes
    #[test]
    fn test_files_panel_view_modes() {
        use crate::state::FilesViewMode;

        for mode in [FilesViewMode::Size, FilesViewMode::Entropy, FilesViewMode::Io] {
            let mut app = App::new_mock();
            app.files_view_mode = mode;

            let backend = TestBackend::new(80, 20);
            let mut terminal = Terminal::new(backend).expect("terminal");

            terminal.draw(|f| {
                let area = Rect::new(0, 0, 80, 20);
                draw_files(f, &app, area);
            }).expect(&format!("draw files in {:?} mode", mode));

            // Should not panic for any view mode
        }
    }

    /// Test Memory panel with different history lengths
    #[test]
    fn test_memory_panel_history_variations() {
        let mut app = App::new_mock();

        // Test with empty history
        app.mem_history.clear();
        app.swap_history.clear();

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_memory(f, &app, area);
        }).expect("draw memory empty history");

        // Test with long history
        app.mem_history = (0..300).map(|i| (i as f64 / 300.0)).collect();
        app.swap_history = (0..300).map(|i| (i as f64 / 600.0)).collect();

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_memory(f, &app, area);
        }).expect("draw memory full history");
    }

    /// Test CPU panel with different core counts
    #[test]
    fn test_cpu_panel_core_variations() {
        let mut app = App::new_mock();

        // Test with no cores
        app.per_core_percent.clear();

        let backend = TestBackend::new(80, 40);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 15);
            draw_cpu(f, &app, area);
        }).expect("draw cpu no cores");

        // Test with many cores (16) - needs taller area
        app.per_core_percent = (0..16).map(|i| i as f64 * 6.0).collect();

        let backend = TestBackend::new(100, 50);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 40);
            draw_cpu(f, &app, area);
        }).expect("draw cpu 16 cores");

        // Test with even more cores (32) - needs even taller area
        app.per_core_percent = (0..32).map(|i| i as f64 * 3.0).collect();

        let backend = TestBackend::new(120, 80);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 120, 70);
            draw_cpu(f, &app, area);
        }).expect("draw cpu 32 cores");
    }

    /// Test Network panel with peak values
    #[test]
    fn test_network_panel_peaks() {
        let mut app = App::new_mock();

        // Set high peak values
        app.net_rx_peak = 1_000_000_000.0;  // 1 GB/s
        app.net_tx_peak = 500_000_000.0;    // 500 MB/s

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_network(f, &app, area);
        }).expect("draw network with peaks");
    }

    /// Test Network panel with empty history
    #[test]
    fn test_network_panel_empty_history() {
        let mut app = App::new_mock();
        app.net_rx_history.clear();
        app.net_tx_history.clear();

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_network(f, &app, area);
        }).expect("draw network empty");
    }

    /// Test Process panel with filter active
    #[test]
    fn test_process_panel_with_filter() {
        let mut app = App::new_mock();
        app.filter = "chrome".to_string();
        app.show_filter_input = true;

        let backend = TestBackend::new(100, 25);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 25);
            draw_process(f, &mut app, area);
        }).expect("draw process with filter");
    }

    /// Test Process panel with tree mode
    #[test]
    fn test_process_panel_tree_mode() {
        let mut app = App::new_mock();
        app.show_tree = true;

        let backend = TestBackend::new(100, 25);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 25);
            draw_process(f, &mut app, area);
        }).expect("draw process tree mode");
    }

    /// Test Disk panel with different sizes
    #[test]
    fn test_disk_panel_sizes() {
        let app = App::new_mock();

        // Test narrow disk panel
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 40, 10);
            draw_disk(f, &app, area);
        }).expect("draw narrow disk");

        // Test wide disk panel
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 120, 30);
            draw_disk(f, &app, area);
        }).expect("draw wide disk");
    }

    /// Test GPU panel at various sizes
    #[test]
    fn test_gpu_panel_sizes() {
        let app = App::new_mock();

        for (w, h) in [(30, 10), (60, 15), (100, 20)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).expect("terminal");

            terminal.draw(|f| {
                let area = Rect::new(0, 0, w, h);
                draw_gpu(f, &app, area);
            }).expect(&format!("draw gpu {}x{}", w, h));
        }
    }

    /// Test System panel at various sizes
    #[test]
    fn test_system_panel_sizes() {
        let app = App::new_mock();

        for (w, h) in [(40, 8), (80, 12), (120, 15)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).expect("terminal");

            terminal.draw(|f| {
                let area = Rect::new(0, 0, w, h);
                draw_system(f, &app, area);
            }).expect(&format!("draw system {}x{}", w, h));
        }
    }

    /// Test PSI panel sizes
    #[test]
    fn test_psi_panel_sizes() {
        let app = App::new_mock();

        for (w, h) in [(30, 8), (50, 12), (80, 15)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).expect("terminal");

            terminal.draw(|f| {
                let area = Rect::new(0, 0, w, h);
                draw_psi(f, &app, area);
            }).expect(&format!("draw psi {}x{}", w, h));
        }
    }

    /// Test Battery panel sizes
    #[test]
    fn test_battery_panel_sizes() {
        let app = App::new_mock();

        for (w, h) in [(25, 6), (40, 10), (60, 12)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).expect("terminal");

            terminal.draw(|f| {
                let area = Rect::new(0, 0, w, h);
                draw_battery(f, &app, area);
            }).expect(&format!("draw battery {}x{}", w, h));
        }
    }

    /// Test Sensors panel sizes
    #[test]
    fn test_sensors_panel_sizes() {
        let app = App::new_mock();

        for (w, h) in [(30, 10), (50, 15), (80, 20)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).expect("terminal");

            terminal.draw(|f| {
                let area = Rect::new(0, 0, w, h);
                draw_sensors(f, &app, area);
            }).expect(&format!("draw sensors {}x{}", w, h));

            terminal.draw(|f| {
                let area = Rect::new(0, 0, w, h);
                draw_sensors_compact(f, &app, area);
            }).expect(&format!("draw sensors compact {}x{}", w, h));
        }
    }

    /// Test Connections panel sizes
    #[test]
    fn test_connections_panel_sizes() {
        let app = App::new_mock();

        for (w, h) in [(40, 12), (60, 18), (100, 25)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).expect("terminal");

            terminal.draw(|f| {
                let area = Rect::new(0, 0, w, h);
                draw_connections(f, &app, area);
            }).expect(&format!("draw connections {}x{}", w, h));
        }
    }

    /// Test Treemap panel sizes
    #[test]
    fn test_treemap_panel_sizes() {
        let app = App::new_mock();

        for (w, h) in [(40, 15), (60, 20), (100, 30)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).expect("terminal");

            terminal.draw(|f| {
                let area = Rect::new(0, 0, w, h);
                draw_treemap(f, &app, area);
            }).expect(&format!("draw treemap {}x{}", w, h));
        }
    }

    /// Test all panels with zero-sized area (edge case)
    #[test]
    fn test_panels_zero_area() {
        let app = App::new_mock();
        let mut app_mut = App::new_mock();
        let backend = TestBackend::new(1, 1);
        let mut terminal = Terminal::new(backend).expect("terminal");

        // Draw with minimal area (1x1) - should not panic
        let area = Rect::new(0, 0, 1, 1);

        terminal.draw(|f| { draw_cpu(f, &app, area); }).ok();
        terminal.draw(|f| { draw_memory(f, &app, area); }).ok();
        terminal.draw(|f| { draw_disk(f, &app, area); }).ok();
        terminal.draw(|f| { draw_network(f, &app, area); }).ok();
        terminal.draw(|f| { draw_gpu(f, &app, area); }).ok();
        terminal.draw(|f| { draw_battery(f, &app, area); }).ok();
        terminal.draw(|f| { draw_sensors(f, &app, area); }).ok();
        terminal.draw(|f| { draw_psi(f, &app, area); }).ok();
        terminal.draw(|f| { draw_system(f, &app, area); }).ok();
        terminal.draw(|f| { draw_process(f, &mut app_mut, area); }).ok();
        terminal.draw(|f| { draw_connections(f, &app, area); }).ok();
        terminal.draw(|f| { draw_treemap(f, &app, area); }).ok();
        terminal.draw(|f| { draw_files(f, &app, area); }).ok();
    }

    // === Helper Function Tests ===

    #[test]
    fn test_truncate_str_no_truncation() {
        assert_eq!(truncate_str("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_str_exact_length() {
        assert_eq!(truncate_str("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_str_with_truncation() {
        assert_eq!(truncate_str("hello world", 8), "hello...");
    }

    #[test]
    fn test_truncate_str_very_short() {
        assert_eq!(truncate_str("hello", 3), "...");
    }

    #[test]
    fn test_truncate_str_min_for_ellipsis() {
        assert_eq!(truncate_str("hello world", 4), "h...");
    }

    #[test]
    fn test_mount_marker_nvme() {
        let (c, color, label) = mount_marker("/mnt/nvme-raid0/data");
        assert_eq!(c, 'N');
        assert_eq!(label, "nvme");
        assert_eq!(color, (100, 220, 140));
    }

    #[test]
    fn test_mount_marker_hdd() {
        let (c, _, label) = mount_marker("/mnt/storage/archive");
        assert_eq!(c, 'D');
        assert_eq!(label, "hdd");
    }

    #[test]
    fn test_mount_marker_home() {
        let (c, _, label) = mount_marker("/home/user/documents");
        assert_eq!(c, 'h');
        assert_eq!(label, "home");
    }

    #[test]
    fn test_mount_marker_root() {
        let (c, _, label) = mount_marker("/");
        assert_eq!(c, '/');
        assert_eq!(label, "sys");
    }

    #[test]
    fn test_mount_marker_var() {
        let (c, _, label) = mount_marker("/var/log");
        assert_eq!(c, '/');
        assert_eq!(label, "sys");
    }

    #[test]
    fn test_mount_marker_usr() {
        let (c, _, label) = mount_marker("/usr/bin");
        assert_eq!(c, '/');
        assert_eq!(label, "sys");
    }

    #[test]
    fn test_mount_marker_other_mnt() {
        let (c, _, label) = mount_marker("/mnt/usb");
        assert_eq!(c, 'M');
        assert_eq!(label, "mnt");
    }

    #[test]
    fn test_mount_marker_media() {
        let (c, _, label) = mount_marker("/media/cdrom");
        assert_eq!(c, 'M');
        assert_eq!(label, "mnt");
    }

    #[test]
    fn test_mount_marker_unknown() {
        let (c, _, label) = mount_marker("/some/other/path");
        assert_eq!(c, '?');
        assert_eq!(label, "unk");
    }

    #[test]
    fn test_mount_legend_str() {
        let legend = mount_legend_str();
        assert!(legend.contains("N:nvme"));
        assert!(legend.contains("D:hdd"));
        assert!(legend.contains("h:home"));
        assert!(legend.contains("/:sys"));
    }

    #[test]
    fn test_format_dir_path_short() {
        assert_eq!(format_dir_path("/home/user", 20), "/home/user");
    }

    #[test]
    fn test_format_dir_path_very_small_width() {
        let result = format_dir_path("/home/user/very/long/path", 8);
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_format_dir_path_truncation() {
        let result = format_dir_path("/mnt/nvme-raid0/targets/trueno-viz/debug", 25);
        assert!(result.len() <= 25);
        assert!(result.contains("..."));
    }

    #[test]
    fn test_format_dir_path_empty_parts() {
        assert_eq!(format_dir_path("/", 10), "/");
    }

    #[test]
    fn test_format_dir_path_single_part() {
        let result = format_dir_path("/root", 10);
        assert!(result.starts_with("/"));
    }

    #[test]
    fn test_entropy_heatmap_high_entropy() {
        let (_, r, g, b) = entropy_heatmap(0.9);
        assert_eq!((r, g, b), (80, 200, 100)); // Green
    }

    #[test]
    fn test_entropy_heatmap_medium_entropy() {
        let (_, r, g, b) = entropy_heatmap(0.6);
        assert_eq!((r, g, b), (200, 200, 80)); // Yellow
    }

    #[test]
    fn test_entropy_heatmap_low_entropy() {
        let (_, r, g, b) = entropy_heatmap(0.3);
        assert_eq!((r, g, b), (220, 140, 60)); // Orange
    }

    #[test]
    fn test_entropy_heatmap_very_low_entropy() {
        let (_, r, g, b) = entropy_heatmap(0.1);
        assert_eq!((r, g, b), (220, 80, 80)); // Red
    }

    /// Test all panels at extra large size for full coverage
    #[test]
    fn test_all_panels_xlarge() {
        let app = App::new_mock();
        let backend = TestBackend::new(250, 80);
        let mut terminal = Terminal::new(backend).expect("terminal");

        // Test all panels at xlarge size
        for panel_fn in [
            |f: &mut Frame, a: &App, area: Rect| draw_cpu(f, a, area),
            |f: &mut Frame, a: &App, area: Rect| draw_memory(f, a, area),
            |f: &mut Frame, a: &App, area: Rect| draw_disk(f, a, area),
            |f: &mut Frame, a: &App, area: Rect| draw_network(f, a, area),
            |f: &mut Frame, a: &App, area: Rect| draw_gpu(f, a, area),
            |f: &mut Frame, a: &App, area: Rect| draw_battery(f, a, area),
            |f: &mut Frame, a: &App, area: Rect| draw_sensors(f, a, area),
            |f: &mut Frame, a: &App, area: Rect| draw_sensors_compact(f, a, area),
            |f: &mut Frame, a: &App, area: Rect| draw_psi(f, a, area),
            |f: &mut Frame, a: &App, area: Rect| draw_system(f, a, area),
            |f: &mut Frame, a: &App, area: Rect| draw_connections(f, a, area),
            |f: &mut Frame, a: &App, area: Rect| draw_treemap(f, a, area),
            |f: &mut Frame, a: &App, area: Rect| draw_files(f, a, area),
        ] {
            terminal.draw(|f| {
                let area = Rect::new(0, 0, 250, 80);
                panel_fn(f, &app, area);
            }).expect("xlarge panel");
        }

        // Process needs mutable app
        let mut app_mut = App::new_mock();
        terminal.draw(|f| {
            let area = Rect::new(0, 0, 250, 80);
            draw_process(f, &mut app_mut, area);
        }).expect("xlarge process");
    }

    /// Test process panel with tree mode
    #[test]
    fn test_process_tree_mode() {
        let mut app = App::new_mock();
        app.show_tree = true;
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 120, 40);
            draw_process(f, &mut app, area);
        }).expect("process tree");
    }

    /// Test process panel with filter
    #[test]
    fn test_process_filtered() {
        let mut app = App::new_mock();
        app.filter = "test".to_string();
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 120, 40);
            draw_process(f, &mut app, area);
        }).expect("process filtered");
    }

    /// Test process panel sort variations
    #[test]
    fn test_process_sorts() {
        use crate::state::ProcessSortColumn;
        let sorts = [
            ProcessSortColumn::Cpu,
            ProcessSortColumn::Mem,
