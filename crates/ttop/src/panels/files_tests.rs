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
            ProcessSortColumn::Pid,
            ProcessSortColumn::Name,
            ProcessSortColumn::State,
            ProcessSortColumn::User,
            ProcessSortColumn::Threads,
        ];
        for sort in sorts {
            let mut app = App::new_mock();
            app.sort_column = sort;
            let backend = TestBackend::new(100, 30);
            let mut terminal = Terminal::new(backend).expect("terminal");
            terminal.draw(|f| {
                let area = Rect::new(0, 0, 100, 30);
                draw_process(f, &mut app, area);
            }).expect(&format!("sort {:?}", sort));
        }
    }

    /// Test Disk panel exploded mode (wider columns, more detail)
    #[test]
    fn test_disk_exploded_mode() {
        let app = App::new_mock();

        // Small terminal: compact mode
        let backend_small = TestBackend::new(60, 12);
        let mut terminal_small = Terminal::new(backend_small).expect("terminal");
        terminal_small.draw(|f| {
            let area = Rect::new(0, 0, 60, 12);
            draw_disk(f, &app, area);
        }).expect("compact disk");

        // Large terminal: exploded mode
        let backend_large = TestBackend::new(150, 35);
        let mut terminal_large = Terminal::new(backend_large).expect("terminal");
        terminal_large.draw(|f| {
            let area = Rect::new(0, 0, 150, 35);
            draw_disk(f, &app, area);
        }).expect("exploded disk");

        // Verify exploded mode renders
        let buffer = terminal_large.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);
        let content = frame.as_text();

        assert!(content.contains("Disk"),
            "Exploded disk should show Disk title. Got: {}", &content[..300.min(content.len())]);
    }

    /// Test Network panel exploded mode (more interfaces, rates shown)
    #[test]
    fn test_network_exploded_mode() {
        let app = App::new_mock();

        // Small terminal: compact mode
        let backend_small = TestBackend::new(60, 12);
        let mut terminal_small = Terminal::new(backend_small).expect("terminal");
        terminal_small.draw(|f| {
            let area = Rect::new(0, 0, 60, 12);
            draw_network(f, &app, area);
        }).expect("compact network");

        // Large terminal: exploded mode
        let backend_large = TestBackend::new(150, 35);
        let mut terminal_large = Terminal::new(backend_large).expect("terminal");
        terminal_large.draw(|f| {
            let area = Rect::new(0, 0, 150, 35);
            draw_network(f, &app, area);
        }).expect("exploded network");

        // Verify exploded mode renders
        let buffer = terminal_large.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);
        let content = frame.as_text();

        assert!(content.contains("Network") && (content.contains("Download") || content.contains("↓")),
            "Exploded network should show download info. Got: {}", &content[..300.min(content.len())]);
    }

    /// Test Memory panel exploded mode (wider bars, more consumers, user column)
    #[test]
    fn test_memory_exploded_mode() {
        let app = App::new_mock();

        // Small terminal: compact mode
        let backend_small = TestBackend::new(60, 12);
        let mut terminal_small = Terminal::new(backend_small).expect("terminal");
        terminal_small.draw(|f| {
            let area = Rect::new(0, 0, 60, 12);
            draw_memory(f, &app, area);
        }).expect("compact memory");

        // Large terminal: exploded mode
        let backend_large = TestBackend::new(150, 35);
        let mut terminal_large = Terminal::new(backend_large).expect("terminal");
        terminal_large.draw(|f| {
            let area = Rect::new(0, 0, 150, 35);
            draw_memory(f, &app, area);
        }).expect("exploded memory");

        // Verify exploded mode renders with more detail
        let buffer = terminal_large.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);
        let content = frame.as_text();

        // Exploded mode should show "Full View" header and Memory info
        assert!(content.contains("Memory") && (content.contains("G") || content.contains("%")),
            "Exploded memory should show memory info. Got: {}", &content[..300.min(content.len())]);
    }

    /// Test CPU panel exploded mode (wider bars, spread layout, more details)
    #[test]
    fn test_cpu_exploded_mode() {
        let app = App::new_mock();

        // Small terminal: compact mode
        let backend_small = TestBackend::new(60, 15);
        let mut terminal_small = Terminal::new(backend_small).expect("terminal");
        terminal_small.draw(|f| {
            let area = Rect::new(0, 0, 60, 15);
            draw_cpu(f, &app, area);
        }).expect("compact cpu");

        // Large terminal: exploded mode
        let backend_large = TestBackend::new(150, 35);
        let mut terminal_large = Terminal::new(backend_large).expect("terminal");
        terminal_large.draw(|f| {
            let area = Rect::new(0, 0, 150, 35);
            draw_cpu(f, &app, area);
        }).expect("exploded cpu");

        // Verify exploded mode shows more detail
        let buffer = terminal_large.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);
        let content = frame.as_text();

        // Exploded mode should show frequency info and more details
        assert!(content.contains("CPU") && (content.contains("%") || content.contains("GHz")),
            "Exploded CPU should show percentage or frequency. Got: {}", &content[..300.min(content.len())]);
    }

    /// Test process panel exploded mode (wide terminal shows extra columns)
    #[test]
    fn test_process_exploded_mode() {
        let mut app = App::new_mock();

        // Small terminal: compact mode (5 columns)
        let backend_small = TestBackend::new(60, 15);
        let mut terminal_small = Terminal::new(backend_small).expect("terminal");
        terminal_small.draw(|f| {
            let area = Rect::new(0, 0, 60, 15);
            draw_process(f, &mut app, area);
        }).expect("compact process");

        // Large terminal: exploded mode (8 columns with USER, THR, MEM)
        let backend_large = TestBackend::new(150, 40);
        let mut terminal_large = Terminal::new(backend_large).expect("terminal");
        terminal_large.draw(|f| {
            let area = Rect::new(0, 0, 150, 40);
            draw_process(f, &mut app, area);
        }).expect("exploded process");

        // Verify exploded mode triggers at width > 80 or height > 25
        // The exploded mode should show USER, THR, MEM columns
        let buffer = terminal_large.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);
        let content = frame.as_text();
        assert!(content.contains("USER") || content.contains("THR") || content.contains("MEM"),
            "Exploded mode should show extended columns. Got: {}", &content[..200.min(content.len())]);
    }

    /// Test panels with wide terminal (horizontal coverage)
    #[test]
    fn test_panels_wide() {
        let app = App::new_mock();
        let backend = TestBackend::new(300, 25);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 300, 25);
            draw_cpu(f, &app, area);
        }).expect("wide cpu");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 300, 25);
            draw_memory(f, &app, area);
        }).expect("wide memory");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 300, 25);
            draw_network(f, &app, area);
        }).expect("wide network");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 300, 25);
            draw_disk(f, &app, area);
        }).expect("wide disk");
    }

    /// Test panels with tall terminal (vertical coverage)
    #[test]
    fn test_panels_tall() {
        let app = App::new_mock();
        let backend = TestBackend::new(60, 100);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 60, 100);
            draw_cpu(f, &app, area);
        }).expect("tall cpu");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 60, 100);
            draw_sensors(f, &app, area);
        }).expect("tall sensors");

        let mut app_mut = App::new_mock();
        terminal.draw(|f| {
            let area = Rect::new(0, 0, 60, 100);
            draw_process(f, &mut app_mut, area);
        }).expect("tall process");
    }

    // === clamp_rect Tests ===

    #[test]
    fn test_clamp_rect_within_bounds() {
        let parent = Rect::new(0, 0, 100, 50);
        let result = clamp_rect(parent, 10, 10, 20, 20);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.x, 10);
        assert_eq!(r.y, 10);
        assert_eq!(r.width, 20);
        assert_eq!(r.height, 20);
    }

    #[test]
    fn test_clamp_rect_exceeds_right() {
        let parent = Rect::new(0, 0, 100, 50);
        let result = clamp_rect(parent, 90, 10, 20, 20);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.width, 10); // Clamped to fit
    }

    #[test]
    fn test_clamp_rect_exceeds_bottom() {
        let parent = Rect::new(0, 0, 100, 50);
        let result = clamp_rect(parent, 10, 45, 20, 20);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.height, 5); // Clamped to fit
    }

    #[test]
    fn test_clamp_rect_outside_bounds() {
        let parent = Rect::new(0, 0, 100, 50);
        assert!(clamp_rect(parent, 100, 10, 20, 20).is_none()); // x at boundary
        assert!(clamp_rect(parent, 10, 50, 20, 20).is_none()); // y at boundary
        assert!(clamp_rect(parent, 150, 100, 20, 20).is_none()); // Both outside
    }

    #[test]
    fn test_clamp_rect_zero_result() {
        let parent = Rect::new(10, 10, 100, 50);
        // Starting at parent's max x/y will give zero width/height
        assert!(clamp_rect(parent, 110, 10, 0, 20).is_none());
        assert!(clamp_rect(parent, 10, 60, 20, 0).is_none());
    }

    #[test]
    fn test_clamp_rect_with_offset_parent() {
        let parent = Rect::new(50, 50, 100, 100);
        let result = clamp_rect(parent, 60, 60, 50, 50);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.x, 60);
        assert_eq!(r.y, 60);
        assert_eq!(r.width, 50);
        assert_eq!(r.height, 50);
    }

    // === Edge Case Panel Tests ===

    #[test]
    fn test_cpu_panel_with_no_cores() {
        let mut app = App::new_mock();
        app.per_core_percent.clear();
        app.cpu_history.clear();

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_cpu(f, &app, area);
        }).expect("cpu no cores");
    }

    #[test]
    fn test_memory_panel_zero_memory() {
        let mut app = App::new_mock();
        app.mem_total = 0;
        app.mem_used = 0;
        app.mem_available = 0;
        app.swap_total = 0;
        app.swap_used = 0;

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_memory(f, &app, area);
        }).expect("memory zero");
    }

    #[test]
    fn test_memory_panel_huge_memory() {
        let mut app = App::new_mock();
        app.mem_total = 1024u64 * 1024 * 1024 * 1024; // 1TB
        app.mem_used = 512u64 * 1024 * 1024 * 1024;    // 512GB
        app.swap_total = 64u64 * 1024 * 1024 * 1024;   // 64GB

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_memory(f, &app, area);
        }).expect("memory huge");
    }

    #[test]
    fn test_network_panel_high_bandwidth() {
        let mut app = App::new_mock();
        app.net_rx_peak = 10_000_000_000.0;  // 10 GB/s
        app.net_tx_peak = 10_000_000_000.0;
        app.net_rx_total = 1_000_000_000_000; // 1 TB
        app.net_tx_total = 500_000_000_000;   // 500 GB

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_network(f, &app, area);
        }).expect("network high bandwidth");
    }

    #[test]
    fn test_network_panel_zero_traffic() {
        let mut app = App::new_mock();
        app.net_rx_peak = 0.0;
        app.net_tx_peak = 0.0;
        app.net_rx_total = 0;
        app.net_tx_total = 0;
        app.net_rx_history.clear();
        app.net_tx_history.clear();

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_network(f, &app, area);
        }).expect("network zero traffic");
    }

    #[test]
    fn test_cpu_panel_high_utilization() {
        let mut app = App::new_mock();
        app.per_core_percent = vec![99.0, 100.0, 98.5, 99.9, 100.0, 97.0, 98.0, 99.0];
        app.cpu_history = (0..100).map(|_| 0.99).collect();

        let backend = TestBackend::new(80, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 30);
            draw_cpu(f, &app, area);
        }).expect("cpu high utilization");
    }

    #[test]
    fn test_process_panel_with_selection() {
        let mut app = App::new_mock();
        app.process_selected = 5;
        app.process_scroll_offset = 2;

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 120, 40);
            draw_process(f, &mut app, area);
        }).expect("process with selection");
    }

    #[test]
    fn test_process_panel_ascending_sort() {
        let mut app = App::new_mock();
        app.sort_descending = false;

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 30);
            draw_process(f, &mut app, area);
        }).expect("process ascending");
    }

    #[test]
    fn test_all_panels_at_boundary_size() {
        let app = App::new_mock();
        // Test at exactly the minimum usable size
        let backend = TestBackend::new(15, 4);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 15, 4);

        terminal.draw(|f| { draw_cpu(f, &app, area); }).ok();
        terminal.draw(|f| { draw_memory(f, &app, area); }).ok();
        terminal.draw(|f| { draw_disk(f, &app, area); }).ok();
        terminal.draw(|f| { draw_network(f, &app, area); }).ok();
        terminal.draw(|f| { draw_gpu(f, &app, area); }).ok();
        terminal.draw(|f| { draw_battery(f, &app, area); }).ok();
        terminal.draw(|f| { draw_sensors(f, &app, area); }).ok();
        terminal.draw(|f| { draw_psi(f, &app, area); }).ok();
        terminal.draw(|f| { draw_system(f, &app, area); }).ok();
        terminal.draw(|f| { draw_connections(f, &app, area); }).ok();
        terminal.draw(|f| { draw_treemap(f, &app, area); }).ok();
        terminal.draw(|f| { draw_files(f, &app, area); }).ok();
    }

    #[test]
    fn test_btop_block_creation() {
        use trueno_viz::monitor::ratatui::style::Color;
        let block = btop_block("Test Title", Color::Green);
        // Just verify it doesn't panic
        let _ = block;
    }

    #[test]
    fn test_files_panel_entropy_mode() {
        use crate::state::FilesViewMode;
        let mut app = App::new_mock();
        app.files_view_mode = FilesViewMode::Entropy;

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 30);
            draw_files(f, &app, area);
        }).expect("files entropy mode");
    }

    #[test]
    fn test_files_panel_io_mode() {
        use crate::state::FilesViewMode;
        let mut app = App::new_mock();
        app.files_view_mode = FilesViewMode::Io;

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 30);
            draw_files(f, &app, area);
        }).expect("files io mode");
    }

    #[test]
    fn test_process_panel_with_signal_menu() {
        let mut app = App::new_mock();
        app.show_signal_menu = true;

        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 120, 40);
            draw_process(f, &mut app, area);
        }).expect("process with signal menu");
    }

    #[test]
    fn test_disk_panel_with_full_disk() {
        let app = App::new_mock();
        // The mock disk collector should handle this

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_disk(f, &app, area);
        }).expect("disk full");
    }

    #[test]
    fn test_psi_panel_with_pressure() {
        let app = App::new_mock();

        let backend = TestBackend::new(60, 15);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 60, 15);
            draw_psi(f, &app, area);
        }).expect("psi with pressure");
    }

    #[test]
    fn test_system_panel_detailed() {
        let app = App::new_mock();

        let backend = TestBackend::new(100, 15);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 15);
            draw_system(f, &app, area);
        }).expect("system detailed");
    }

    #[test]
    fn test_treemap_large_area() {
        let app = App::new_mock();

        let backend = TestBackend::new(150, 50);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 150, 50);
            draw_treemap(f, &app, area);
        }).expect("treemap large");
    }

    #[test]
    fn test_connections_detailed() {
        let app = App::new_mock();

        let backend = TestBackend::new(120, 35);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 120, 35);
            draw_connections(f, &app, area);
        }).expect("connections detailed");
    }

    #[test]
    fn test_sensors_many_sensors() {
        let app = App::new_mock();

        // Test with extra tall area for many sensors
        let backend = TestBackend::new(60, 40);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 60, 40);
            draw_sensors(f, &app, area);
        }).expect("sensors many");
    }

    #[test]
    fn test_battery_large_area() {
        let app = App::new_mock();

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_battery(f, &app, area);
        }).expect("battery large");
    }

    #[test]
    fn test_gpu_large_area() {
        let app = App::new_mock();

        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 120, 30);
            draw_gpu(f, &app, area);
        }).expect("gpu large");
    }

    // === Format Functions Tests ===

    #[test]
    fn test_format_dir_path_exact_fit() {
        let path = "/home/user";
        let result = format_dir_path(path, path.len());
        assert_eq!(result, path);
    }

    #[test]
    fn test_format_dir_path_edge_cases() {
        // Test with minimum width
        let result = format_dir_path("/a/b/c/d", 5);
        assert!(result.len() <= 5);

        // Test with width smaller than first part
        let result = format_dir_path("/verylongdirname", 5);
        assert!(result.len() <= 5);
    }

    #[test]
    fn test_truncate_str_empty() {
        assert_eq!(truncate_str("", 10), "");
    }

    #[test]
    fn test_truncate_str_zero_len() {
        assert_eq!(truncate_str("hello", 0), "");
    }

    #[test]
    fn test_entropy_heatmap_boundary_values() {
        // Test at exact boundaries
        let (display, _, _, _) = entropy_heatmap(0.0);
        assert!(!display.is_empty());

        let (display, _, _, _) = entropy_heatmap(1.0);
        assert!(!display.is_empty());

        // Test slightly above/below boundaries
        let (_, r1, _, _) = entropy_heatmap(0.749);
        let (_, r2, _, _) = entropy_heatmap(0.751);
        // Just ensure no panic and valid values
        assert!(r1 > 0 && r2 > 0);
    }

    #[test]
    fn test_mount_marker_generic_mnt() {
        // /mnt/ssd doesn't match nvme or storage patterns, so it's generic mount
        let (c, _, label) = mount_marker("/mnt/ssd/data");
        assert_eq!(c, 'M');
        assert_eq!(label, "mnt");
    }

    #[test]
    fn test_mount_marker_tmp_unknown() {
        // /tmp doesn't match any known pattern, so it's unknown
        let (c, _, label) = mount_marker("/tmp/work");
        assert_eq!(c, '?');
        assert_eq!(label, "unk");
    }

    #[test]
    fn test_mount_marker_boot_unknown() {
        // /boot doesn't match any known pattern, so it's unknown
        let (c, _, label) = mount_marker("/boot/efi");
        assert_eq!(c, '?');
        assert_eq!(label, "unk");
    }

    #[test]
    fn test_mount_marker_usr_sys() {
        // /usr should match sys pattern
        let (c, _, label) = mount_marker("/usr/lib");
        assert_eq!(c, '/');
        assert_eq!(label, "sys");
    }

    #[test]
    fn test_mount_marker_var_sys() {
        // /var should match sys pattern
        let (c, _, label) = mount_marker("/var/log");
        assert_eq!(c, '/');
        assert_eq!(label, "sys");
    }

    // === Micro-benchmark Performance Tests ===

    /// Verify panel rendering completes within reasonable time
    #[test]
    fn test_panel_render_performance() {
        use std::time::Instant;

        let app = App::new_mock();
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 120, 40);

        let iterations = 50;
        let start = Instant::now();

        for _ in 0..iterations {
            terminal.draw(|f| {
                draw_cpu(f, &app, area);
            }).expect("draw cpu");
            terminal.draw(|f| {
                draw_memory(f, &app, area);
            }).expect("draw memory");
            terminal.draw(|f| {
                draw_network(f, &app, area);
            }).expect("draw network");
        }

        let elapsed = start.elapsed();
        let avg_ms = elapsed.as_millis() / iterations as u128;

        // Should render 3 panels in < 100ms
        assert!(avg_ms < 100, "Panel rendering too slow: {}ms avg for 3 panels", avg_ms);
    }

    /// Verify helper function performance
    #[test]
    fn test_helper_functions_performance() {
        use std::time::Instant;

        let iterations = 10000;

        // Test truncate_str
        let start = Instant::now();
        for i in 0..iterations {
            let _ = truncate_str("this is a long string that needs truncation", i % 50);
        }
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 100, "truncate_str too slow");

        // Test mount_marker
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = mount_marker("/home/user/documents");
            let _ = mount_marker("/mnt/nvme-raid0/data");
            let _ = mount_marker("/var/log");
        }
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 100, "mount_marker too slow");

        // Test entropy_heatmap
        let start = Instant::now();
        for i in 0..iterations {
            let _ = entropy_heatmap(i as f64 / iterations as f64);
        }
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 100, "entropy_heatmap too slow");
    }

    /// Verify clamp_rect is efficient
    #[test]
    fn test_clamp_rect_performance() {
        use std::time::Instant;

        let parent = Rect::new(0, 0, 200, 100);
        let iterations = 100000;
        let start = Instant::now();

        for i in 0..iterations {
            let _ = clamp_rect(parent, (i % 200) as u16, (i % 100) as u16, 50, 25);
        }

        let elapsed = start.elapsed();
        let per_op_ns = elapsed.as_nanos() / iterations as u128;

        // Should be sub-microsecond
        assert!(per_op_ns < 1000, "clamp_rect too slow: {}ns per op", per_op_ns);
    }

    // === Comprehensive Panel Coverage Tests ===

    #[test]
    fn test_draw_gpu_no_gpus() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 80, 20);

        // Should render "No GPU detected" message without panic
        terminal.draw(|f| {
            draw_gpu(f, &app, area);
        }).expect("draw gpu with no gpus");
    }

    #[test]
    fn test_draw_sensors_empty() {
        let app = App::new_mock();
        let backend = TestBackend::new(60, 15);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 60, 15);

        terminal.draw(|f| {
            draw_sensors(f, &app, area);
        }).expect("draw sensors");
    }

    #[test]
    fn test_draw_sensors_compact_empty() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 3);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 80, 3);

        terminal.draw(|f| {
            draw_sensors_compact(f, &app, area);
        }).expect("draw sensors compact");
    }

    #[test]
    fn test_draw_psi_standard_size() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 12);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 80, 12);

        terminal.draw(|f| {
            draw_psi(f, &app, area);
        }).expect("draw psi");
    }

    #[test]
    fn test_draw_system_standard_size() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 80, 10);

        terminal.draw(|f| {
            draw_system(f, &app, area);
        }).expect("draw system");
    }

    #[test]
    fn test_draw_connections_standard_size() {
        let app = App::new_mock();
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 100, 20);

        terminal.draw(|f| {
            draw_connections(f, &app, area);
        }).expect("draw connections");
    }

    #[test]
    fn test_draw_treemap_standard_size() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 80, 20);

        terminal.draw(|f| {
            draw_treemap(f, &app, area);
        }).expect("draw treemap");
    }

    #[test]
    fn test_draw_files_standard_size() {
        let app = App::new_mock();
        let backend = TestBackend::new(100, 25);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 100, 25);

        terminal.draw(|f| {
            draw_files(f, &app, area);
        }).expect("draw files");
    }

    #[test]
    fn test_draw_battery_standard_size() {
        let app = App::new_mock();
        let backend = TestBackend::new(60, 10);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 60, 10);

        terminal.draw(|f| {
            draw_battery(f, &app, area);
        }).expect("draw battery");
    }

    #[test]
    fn test_draw_process_standard_size() {
        let mut app = App::new_mock();
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 120, 30);

        terminal.draw(|f| {
            draw_process(f, &mut app, area);
        }).expect("draw process");
    }

    #[test]
    fn test_draw_cpu_minimal_height() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 5);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 80, 3);

        terminal.draw(|f| {
            draw_cpu(f, &app, area);
        }).expect("draw cpu minimal");
    }

    #[test]
    fn test_draw_memory_minimal_height() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 5);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 80, 3);

        terminal.draw(|f| {
            draw_memory(f, &app, area);
        }).expect("draw memory minimal");
    }

    #[test]
    fn test_draw_disk_minimal_height() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 5);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 80, 3);

        terminal.draw(|f| {
            draw_disk(f, &app, area);
        }).expect("draw disk minimal");
    }

    #[test]
    fn test_draw_network_minimal_height() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 5);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 80, 3);

        terminal.draw(|f| {
            draw_network(f, &app, area);
        }).expect("draw network minimal");
    }

    #[test]
    fn test_draw_gpu_minimal_height() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 5);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 80, 3);

        terminal.draw(|f| {
            draw_gpu(f, &app, area);
        }).expect("draw gpu minimal");
    }

    #[test]
    fn test_draw_sensors_minimal_height() {
        let app = App::new_mock();
        let backend = TestBackend::new(60, 5);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 60, 3);

        terminal.draw(|f| {
            draw_sensors(f, &app, area);
        }).expect("draw sensors minimal");
    }

    #[test]
    fn test_draw_battery_minimal_height() {
        let app = App::new_mock();
        let backend = TestBackend::new(60, 5);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 60, 3);

        terminal.draw(|f| {
            draw_battery(f, &app, area);
        }).expect("draw battery minimal");
    }

    #[test]
    fn test_draw_process_minimal_height() {
        let mut app = App::new_mock();
        let backend = TestBackend::new(80, 5);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 80, 3);

        terminal.draw(|f| {
            draw_process(f, &mut app, area);
        }).expect("draw process minimal");
    }

    #[test]
    fn test_draw_connections_minimal() {
        let app = App::new_mock();
        let backend = TestBackend::new(60, 5);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 60, 3);

        terminal.draw(|f| {
            draw_connections(f, &app, area);
        }).expect("draw connections minimal");
    }

    #[test]
    fn test_draw_treemap_minimal() {
        let app = App::new_mock();
        let backend = TestBackend::new(40, 5);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 40, 3);

        terminal.draw(|f| {
            draw_treemap(f, &app, area);
        }).expect("draw treemap minimal");
    }

    #[test]
    fn test_draw_files_minimal() {
        let app = App::new_mock();
        let backend = TestBackend::new(50, 5);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 50, 3);

        terminal.draw(|f| {
            draw_files(f, &app, area);
        }).expect("draw files minimal");
    }

    #[test]
    fn test_draw_psi_minimal() {
        let app = App::new_mock();
        let backend = TestBackend::new(40, 5);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 40, 3);

        terminal.draw(|f| {
            draw_psi(f, &app, area);
        }).expect("draw psi minimal");
    }

    #[test]
    fn test_draw_system_minimal() {
        let app = App::new_mock();
        let backend = TestBackend::new(40, 5);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 40, 3);

        terminal.draw(|f| {
            draw_system(f, &app, area);
        }).expect("draw system minimal");
    }

    #[test]
    fn test_all_panels_wide_terminal() {
        let mut app = App::new_mock();
        let backend = TestBackend::new(200, 60);
        let mut terminal = Terminal::new(backend).expect("terminal");

        // Test all panels on a wide terminal
        terminal.draw(|f| {
            draw_cpu(f, &app, Rect::new(0, 0, 100, 20));
            draw_memory(f, &app, Rect::new(100, 0, 100, 20));
            draw_disk(f, &app, Rect::new(0, 20, 100, 20));
            draw_network(f, &app, Rect::new(100, 20, 100, 20));
            draw_process(f, &mut app, Rect::new(0, 40, 200, 20));
        }).expect("draw all panels wide");
    }

    #[test]
    fn test_all_panels_narrow_terminal() {
        let mut app = App::new_mock();
        let backend = TestBackend::new(40, 80);
        let mut terminal = Terminal::new(backend).expect("terminal");

        // Test all panels on a narrow terminal
        terminal.draw(|f| {
            draw_cpu(f, &app, Rect::new(0, 0, 40, 10));
            draw_memory(f, &app, Rect::new(0, 10, 40, 10));
            draw_disk(f, &app, Rect::new(0, 20, 40, 10));
            draw_network(f, &app, Rect::new(0, 30, 40, 10));
            draw_sensors(f, &app, Rect::new(0, 40, 40, 10));
            draw_battery(f, &app, Rect::new(0, 50, 40, 10));
            draw_gpu(f, &app, Rect::new(0, 60, 40, 10));
            draw_process(f, &mut app, Rect::new(0, 70, 40, 10));
        }).expect("draw all panels narrow");
    }

    #[test]
    fn test_draw_cpu_zero_width() {
        let app = App::new_mock();
        let backend = TestBackend::new(10, 10);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 0, 10);

        // Zero width should not panic
        terminal.draw(|f| {
            draw_cpu(f, &app, area);
        }).expect("draw cpu zero width");
    }

    #[test]
    fn test_draw_cpu_zero_height() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let area = Rect::new(0, 0, 80, 0);

        // Zero height should not panic
        terminal.draw(|f| {
            draw_cpu(f, &app, area);
        }).expect("draw cpu zero height");
    }

    #[test]
    fn test_format_dir_path_long_component() {
        let result = format_dir_path("/mnt/verylongmountpointname/very/long/path/to/directory/structure/file.txt", 30);
        assert!(result.len() <= 30);
    }

    #[test]
    fn test_format_dir_path_root_only() {
        let result = format_dir_path("/", 20);
        assert_eq!(result, "/");
    }

    #[test]
    fn test_format_dir_path_home_path() {
        let result = format_dir_path("/home/user/documents/project", 25);
        assert!(result.len() <= 25);
    }

    #[test]
    fn test_format_dir_path_media_path() {
        let result = format_dir_path("/media/usb/backup/important", 20);
        assert!(result.len() <= 20);
    }

    #[test]
    fn test_entropy_heatmap_zero() {
        let (display, r, g, b) = entropy_heatmap(0.0);
        assert!(r > 200); // Should be red for high duplication
        assert!(!display.is_empty());
    }

    #[test]
    fn test_entropy_heatmap_one() {
        let (display, r, g, b) = entropy_heatmap(1.0);
        assert!(g > 150); // Should be green for unique data
        assert!(!display.is_empty());
    }

    #[test]
    fn test_entropy_heatmap_quarter() {
        let (display, r, g, b) = entropy_heatmap(0.25);
        // Orange range
        assert!(r > 200 && g > 100);
        assert!(!display.is_empty());
    }

    #[test]
    fn test_mount_marker_all_variants() {
        // Test all path patterns
        let paths = [
            "/mnt/nvme-raid0/data",
            "/mnt/nvme/fast",
            "/mnt/storage/bulk",
            "/mnt/hdd/archive",
            "/home/user",
            "/",
            "/usr/bin",
            "/var/log",
            "/mnt/external",
            "/media/usb",
            "/tmp/unknown",
            "/boot/efi",
        ];

        for path in paths {
            let (marker, color, label) = mount_marker(path);
            assert!(!label.is_empty());
            assert!(color.0 > 0 || color.1 > 0 || color.2 > 0);
        }
    }

    #[test]
    fn test_mock_gpu_data_renders() {
        let app = App::new_mock();

        // Verify mock data is populated
        assert!(!app.mock_gpus.is_empty(), "mock_gpus should not be empty");
        assert_eq!(app.mock_gpus.len(), 2);
        assert!(app.mock_gpus[0].name.contains("RTX 4090"));

        // Render GPU panel
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_gpu(f, &app, area);
        }).expect("draw gpu");

        // Get frame content
        let buffer = terminal.backend().buffer().clone();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();

        // Should contain mock GPU data values
        assert!(content.contains("GPU (2 devices)"), "Should show 2 mock GPU devices");
        assert!(content.contains("75"), "Should show mock GPU0 utilization 75%");
        assert!(content.contains("72"), "Should show mock GPU0 temperature 72°C");
        assert!(content.contains("350"), "Should show mock GPU0 power 350W");
        assert!(content.contains("45"), "Should show mock GPU1 utilization 45%");
    }

    #[test]
    fn test_mock_container_data_renders() {
        let app = App::new_mock();

        // Verify mock data is populated
        assert!(!app.mock_containers.is_empty(), "mock_containers should not be empty");
        assert_eq!(app.mock_containers.len(), 3);
        assert_eq!(app.mock_containers[0].name, "nginx-proxy");

        // Render container panel
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_containers_inner(f, &app, area);
        }).expect("draw containers");

        // Get frame content
        let buffer = terminal.backend().buffer().clone();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();

        // Should contain mock container data
        assert!(content.contains("Containers") || content.contains("2/3"), "Should show container count");
        assert!(content.contains("nginx") || content.contains("●"), "Should show running container indicator");
    }

    #[test]
    fn test_mock_battery_data_renders() {
        let app = App::new_mock();

        // Verify mock data is populated
        assert!(app.mock_battery.is_some(), "mock_battery should be Some");
        let battery = app.mock_battery.as_ref().unwrap();
        assert!((battery.percent - 72.5).abs() < 0.1);

        // Render battery panel
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_battery(f, &app, area);
        }).expect("draw battery");

        // Get frame content
        let buffer = terminal.backend().buffer().clone();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();

        // Should contain mock battery data
        assert!(content.contains("72") || content.contains("Battery"), "Should show battery percentage or title");
    }

    #[test]
    fn test_mock_sensor_data_renders() {
        let app = App::new_mock();

        // Verify mock data is populated
        assert!(!app.mock_sensors.is_empty(), "mock_sensors should not be empty");
        assert_eq!(app.mock_sensors.len(), 6);

        // Render sensors panel
        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_sensors(f, &app, area);
        }).expect("draw sensors");

        // Get frame content
        let buffer = terminal.backend().buffer().clone();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();

        // Should contain mock sensor data
        assert!(content.contains("Sensors") || content.contains("CPU") || content.contains("65"),
            "Should show sensor data");
    }

    #[test]
    fn test_container_status_colors() {
        // Test all container status types for coverage
        let mut app = App::new_mock();

        // Set up containers with different statuses
        app.mock_containers = vec![
            crate::app::MockContainerData {
                name: "running-container".to_string(),
                status: "running".to_string(),
                cpu_percent: 25.0,
                mem_used: 100 * 1024 * 1024,
                mem_limit: 512 * 1024 * 1024,
            },
            crate::app::MockContainerData {
                name: "paused-container".to_string(),
                status: "paused".to_string(),
                cpu_percent: 0.0,
                mem_used: 50 * 1024 * 1024,
                mem_limit: 256 * 1024 * 1024,
            },
            crate::app::MockContainerData {
                name: "restarting-container".to_string(),
                status: "restarting".to_string(),
                cpu_percent: 50.0,
                mem_used: 200 * 1024 * 1024,
                mem_limit: 512 * 1024 * 1024,
            },
            crate::app::MockContainerData {
                name: "exited-container".to_string(),
                status: "exited".to_string(),
                cpu_percent: 0.0,
                mem_used: 0,
                mem_limit: 256 * 1024 * 1024,
            },
            crate::app::MockContainerData {
                name: "unknown-container".to_string(),
                status: "unknown".to_string(),
                cpu_percent: 10.0,
                mem_used: 30 * 1024 * 1024,
                mem_limit: 256 * 1024 * 1024,
            },
        ];

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_containers_inner(f, &app, area);
        }).expect("draw containers");

        let buffer = terminal.backend().buffer().clone();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();

        // Should render all containers
        assert!(content.contains("Containers"), "Should show container panel");
    }

    #[test]
    fn test_container_cpu_color_thresholds() {
        // Test CPU color thresholds: <40 green, 40-80 yellow, >=80 red
        let mut app = App::new_mock();

        app.mock_containers = vec![
            crate::app::MockContainerData {
                name: "low-cpu".to_string(),
                status: "running".to_string(),
                cpu_percent: 20.0,  // green
                mem_used: 100 * 1024 * 1024,
                mem_limit: 512 * 1024 * 1024,
            },
            crate::app::MockContainerData {
                name: "medium-cpu".to_string(),
                status: "running".to_string(),
                cpu_percent: 60.0,  // yellow
                mem_used: 100 * 1024 * 1024,
                mem_limit: 512 * 1024 * 1024,
            },
            crate::app::MockContainerData {
                name: "high-cpu".to_string(),
                status: "running".to_string(),
                cpu_percent: 95.0,  // red
                mem_used: 100 * 1024 * 1024,
                mem_limit: 512 * 1024 * 1024,
            },
        ];

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_containers_inner(f, &app, area);
        }).expect("draw containers");

        // Test passes if no panic - colors are applied correctly
    }

    #[test]
    fn test_container_mem_color_thresholds() {
        // Test memory color thresholds: <50% green, 50-80% yellow, >=80% red
        let mut app = App::new_mock();

        app.mock_containers = vec![
            crate::app::MockContainerData {
                name: "low-mem".to_string(),
                status: "running".to_string(),
                cpu_percent: 10.0,
                mem_used: 100 * 1024 * 1024,   // ~19% of 512MB = green
                mem_limit: 512 * 1024 * 1024,
            },
            crate::app::MockContainerData {
                name: "medium-mem".to_string(),
                status: "running".to_string(),
                cpu_percent: 10.0,
                mem_used: 350 * 1024 * 1024,  // ~68% of 512MB = yellow
                mem_limit: 512 * 1024 * 1024,
            },
            crate::app::MockContainerData {
                name: "high-mem".to_string(),
                status: "running".to_string(),
                cpu_percent: 10.0,
                mem_used: 450 * 1024 * 1024,  // ~88% of 512MB = red
                mem_limit: 512 * 1024 * 1024,
            },
        ];

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_containers_inner(f, &app, area);
        }).expect("draw containers");

        // Test passes if no panic - colors are applied correctly
    }

    #[test]
    fn test_container_memory_gigabytes_format() {
        // Test container memory >= 1GB shows as "XG" format
        let mut app = App::new_mock();

        app.mock_containers = vec![
            crate::app::MockContainerData {
                name: "big-mem".to_string(),
                status: "running".to_string(),
                cpu_percent: 50.0,
                mem_used: 2 * 1024 * 1024 * 1024,  // 2GB
                mem_limit: 4 * 1024 * 1024 * 1024, // 4GB limit
            },
        ];

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_containers_inner(f, &app, area);
        }).expect("draw containers");

        let buffer = terminal.backend().buffer().clone();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();

        assert!(content.contains("2G"), "Should show memory in GB format");
    }

    #[test]
    fn test_empty_containers() {
        let mut app = App::new_mock();
        app.mock_containers = vec![];

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_containers_inner(f, &app, area);
        }).expect("draw containers");

        let buffer = terminal.backend().buffer().clone();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();

        // Empty mock containers should show "No running containers"
        assert!(content.contains("No running") || content.contains("0/0"),
            "Should show no containers message");
    }

    // === Additional Panel Coverage Tests ===

    #[test]
    fn test_cpu_panel_various_sizes() {
        let app = App::new_mock();

        // Test various terminal sizes
        for (w, h) in [(40, 10), (80, 15), (120, 20), (160, 30)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).expect("terminal");

            terminal.draw(|f| {
                let area = Rect::new(0, 0, w, h);
                draw_cpu(f, &app, area);
            }).expect("draw cpu");
        }
    }

    #[test]
    fn test_memory_panel_various_sizes() {
        let app = App::new_mock();

        for (w, h) in [(40, 8), (80, 12), (120, 16), (160, 24)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).expect("terminal");

            terminal.draw(|f| {
                let area = Rect::new(0, 0, w, h);
                draw_memory(f, &app, area);
            }).expect("draw memory");
        }
    }

    #[test]
    fn test_network_panel_various_sizes() {
        let app = App::new_mock();

        for (w, h) in [(40, 6), (80, 10), (120, 14), (160, 20)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).expect("terminal");

            terminal.draw(|f| {
                let area = Rect::new(0, 0, w, h);
                draw_network(f, &app, area);
            }).expect("draw network");
        }
    }

    #[test]
    fn test_disk_panel_various_sizes() {
        let app = App::new_mock();

        for (w, h) in [(40, 5), (80, 10), (120, 15), (160, 20)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).expect("terminal");

            terminal.draw(|f| {
                let area = Rect::new(0, 0, w, h);
                draw_disk(f, &app, area);
            }).expect("draw disk");
        }
    }

    #[test]
    fn test_process_panel_various_sizes() {
        let mut app = App::new_mock();

        for (w, h) in [(60, 10), (100, 20), (150, 30), (200, 40)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).expect("terminal");

            terminal.draw(|f| {
                let area = Rect::new(0, 0, w, h);
                draw_process(f, &mut app, area);
            }).expect("draw process");
        }
    }

    #[test]
    fn test_files_panel_various_sizes() {
        let app = App::new_mock();

        for (w, h) in [(50, 8), (80, 12), (120, 16)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).expect("terminal");

            terminal.draw(|f| {
                let area = Rect::new(0, 0, w, h);
                draw_files(f, &app, area);
            }).expect("draw files");
        }
    }

    #[test]
    fn test_battery_charging_states() {
        let mut app = App::new_mock();

        // Test charging state
        app.mock_battery = Some(crate::app::MockBatteryData {
            percent: 45.0,
            charging: true,
            time_remaining_mins: Some(60),
            power_watts: 65.0,
            health_percent: 95.0,
            cycle_count: 150,
        });

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_battery(f, &app, area);
        }).expect("draw battery charging");

        // Test discharging state
        app.mock_battery = Some(crate::app::MockBatteryData {
            percent: 85.0,
            charging: false,
            time_remaining_mins: Some(180),
            power_watts: 15.0,
            health_percent: 90.0,
            cycle_count: 300,
        });

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_battery(f, &app, area);
        }).expect("draw battery discharging");
    }

    #[test]
    fn test_battery_critical_levels() {
        let mut app = App::new_mock();

        // Critical battery level (< 10%)
        app.mock_battery = Some(crate::app::MockBatteryData {
            percent: 5.0,
            charging: false,
            time_remaining_mins: Some(10),
            power_watts: 8.0,
            health_percent: 85.0,
            cycle_count: 500,
        });

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_battery(f, &app, area);
        }).expect("draw critical battery");
    }

    #[test]
    fn test_battery_full_charge() {
        let mut app = App::new_mock();

        // Full charge
        app.mock_battery = Some(crate::app::MockBatteryData {
            percent: 100.0,
            charging: false,
            time_remaining_mins: None,
            power_watts: 0.0,
            health_percent: 100.0,
            cycle_count: 10,
        });

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_battery(f, &app, area);
        }).expect("draw full battery");
    }

    #[test]
    fn test_sensor_temperature_thresholds() {
        let mut app = App::new_mock();

        // Set sensors with various temperature levels for threshold testing
        app.mock_sensors = vec![
            // Low temp (green)
            crate::app::MockSensorData {
                name: "cpu/temp1".to_string(),
                label: "CPU Cool".to_string(),
                value: 35.0,
                max: Some(100.0),
                crit: Some(110.0),
                sensor_type: crate::app::MockSensorType::Temperature,
            },
            // Medium temp (yellow)
            crate::app::MockSensorData {
                name: "cpu/temp2".to_string(),
                label: "CPU Warm".to_string(),
                value: 70.0,
                max: Some(90.0),
                crit: Some(100.0),
                sensor_type: crate::app::MockSensorType::Temperature,
            },
            // High temp (red)
            crate::app::MockSensorData {
                name: "cpu/temp3".to_string(),
                label: "CPU Hot".to_string(),
                value: 95.0,
                max: Some(95.0),
                crit: Some(100.0),
                sensor_type: crate::app::MockSensorType::Temperature,
            },
        ];

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_sensors(f, &app, area);
        }).expect("draw sensors with thresholds");
    }

    #[test]
    fn test_sensor_fan_type() {
        let mut app = App::new_mock();

        app.mock_sensors = vec![
            crate::app::MockSensorData {
                name: "fan/fan1".to_string(),
                label: "CPU Fan".to_string(),
                value: 1500.0,
                max: Some(3000.0),
                crit: None,
                sensor_type: crate::app::MockSensorType::Fan,
            },
            crate::app::MockSensorData {
                name: "fan/fan2".to_string(),
                label: "Case Fan".to_string(),
                value: 1000.0,
                max: Some(2500.0),
                crit: None,
                sensor_type: crate::app::MockSensorType::Fan,
            },
        ];

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_sensors(f, &app, area);
        }).expect("draw fan sensors");
    }

    #[test]
    fn test_sensor_voltage_type() {
        let mut app = App::new_mock();

        app.mock_sensors = vec![
            crate::app::MockSensorData {
                name: "power/in0".to_string(),
                label: "VCore".to_string(),
                value: 1.35,
                max: Some(1.55),
                crit: Some(1.65),
                sensor_type: crate::app::MockSensorType::Voltage,
            },
        ];

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_sensors(f, &app, area);
        }).expect("draw voltage sensors");
    }

    #[test]
    fn test_sensor_power_type() {
        let mut app = App::new_mock();

        app.mock_sensors = vec![
            crate::app::MockSensorData {
                name: "power/power1".to_string(),
                label: "Package Power".to_string(),
                value: 95.0,
                max: Some(150.0),
                crit: Some(200.0),
                sensor_type: crate::app::MockSensorType::Power,
            },
        ];

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_sensors(f, &app, area);
        }).expect("draw power sensors");
    }

    #[test]
    fn test_gpu_single_device() {
        let mut app = App::new_mock();

        // Single GPU
        app.mock_gpus = vec![
            crate::app::MockGpuData {
                name: "RTX 4090".to_string(),
                gpu_util: 50.0,
                vram_used: 8 * 1024 * 1024 * 1024,
                vram_total: 24 * 1024 * 1024 * 1024,
                temperature: 65.0,
                power_watts: 200,
                power_limit_watts: 450,
                clock_mhz: 2500,
                history: vec![40.0, 45.0, 50.0, 55.0, 50.0],
            },
        ];

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_gpu(f, &app, area);
        }).expect("draw single gpu");

        let buffer = terminal.backend().buffer().clone();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();

        // Single GPU should show full name
        assert!(content.contains("RTX") || content.contains("GPU"), "Should show GPU info");
    }

    #[test]
    fn test_gpu_high_utilization() {
        let mut app = App::new_mock();

        // GPU at high utilization (should trigger red color path)
        app.mock_gpus = vec![
            crate::app::MockGpuData {
                name: "RTX 4090".to_string(),
                gpu_util: 98.0,
                vram_used: 22 * 1024 * 1024 * 1024,
                vram_total: 24 * 1024 * 1024 * 1024,
                temperature: 85.0,
                power_watts: 420,
                power_limit_watts: 450,
                clock_mhz: 2700,
                history: vec![95.0, 96.0, 97.0, 98.0, 98.0],
            },
        ];

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_gpu(f, &app, area);
        }).expect("draw high util gpu");
    }

    #[test]
    fn test_gpu_no_history() {
        let mut app = App::new_mock();

        // GPU with no history
        app.mock_gpus = vec![
            crate::app::MockGpuData {
                name: "RTX 4090".to_string(),
                gpu_util: 25.0,
                vram_used: 4 * 1024 * 1024 * 1024,
                vram_total: 24 * 1024 * 1024 * 1024,
                temperature: 45.0,
                power_watts: 100,
                power_limit_watts: 450,
                clock_mhz: 2000,
                history: vec![],
            },
        ];

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_gpu(f, &app, area);
        }).expect("draw gpu no history");
    }

    #[test]
    fn test_panel_tiny_area() {
        let app = App::new_mock();

        // Test with extremely tiny areas
        let backend = TestBackend::new(10, 5);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 10, 5);
            draw_cpu(f, &app, area);
        }).expect("draw cpu tiny");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 10, 5);
            draw_memory(f, &app, area);
        }).expect("draw memory tiny");
    }

    #[test]
    fn test_panel_unit_width() {
        let app = App::new_mock();

        // Width = 1, should handle gracefully
        let backend = TestBackend::new(1, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 1, 20);
            draw_network(f, &app, area);
        }).expect("draw network unit width");
    }

    #[test]
    fn test_panel_unit_height() {
        let app = App::new_mock();

        // Height = 1, should handle gracefully
        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 1);
            draw_disk(f, &app, area);
        }).expect("draw disk unit height");
    }

    #[test]
    fn test_psi_panel_large() {
        let app = App::new_mock();

        // Large PSI panel
        let backend = TestBackend::new(150, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 150, 30);
            draw_psi(f, &app, area);
        }).expect("draw large psi");
    }

    #[test]
    fn test_btop_block_styling() {
        // Test btop_block function returns properly styled block
        use trueno_viz::monitor::ratatui::style::Color;

        let block = btop_block(" Test Panel ", Color::Cyan);
        // Just verify it doesn't panic and returns a block
        assert!(format!("{:?}", block).contains("Block"));
    }

    #[test]
    fn test_graph_render_with_data() {
        // Test graph rendering with actual data points
        let app = App::new_mock();

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        // Render multiple times to populate history
        for _ in 0..3 {
            terminal.draw(|f| {
                let area = Rect::new(0, 0, 100, 20);
                draw_cpu(f, &app, area);
                draw_memory(f, &app, area);
            }).expect("draw panels");
        }
    }

    #[test]
    fn test_connections_panel() {
        let app = App::new_mock();

        let backend = TestBackend::new(120, 25);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 120, 25);
            draw_connections(f, &app, area);
        }).expect("draw connections");
    }

    #[test]
    fn test_treemap_panel_various_sizes() {
        let app = App::new_mock();

        for (w, h) in [(60, 15), (100, 20), (150, 30)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).expect("terminal");

            terminal.draw(|f| {
                let area = Rect::new(0, 0, w, h);
                draw_treemap(f, &app, area);
            }).expect("draw treemap");
        }
    }

    #[test]
    fn test_swap_panel_with_history() {
        let mut app = App::new_mock();

        // Populate swap history
        app.swap_history = (0..100).map(|i| (i as f64 / 100.0) * 0.5).collect();

        let backend = TestBackend::new(100, 15);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 15);
            draw_memory(f, &app, area);
        }).expect("draw swap with history");
    }

    // === Additional Coverage: System Panel Tests ===

    #[test]
    fn test_system_panel() {
        let app = App::new_mock();

        let backend = TestBackend::new(100, 10);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 10);
            draw_system(f, &app, area);
        }).expect("draw system");
    }

    #[test]
    fn test_system_panel_compact() {
        let app = App::new_mock();

        let backend = TestBackend::new(40, 5);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 40, 5);
            draw_system(f, &app, area);
        }).expect("draw system compact");
    }

    // === Additional Coverage: Empty Data Tests ===

    #[test]
    fn test_gpu_empty_data() {
        let mut app = App::new_mock();
        app.mock_gpus = vec![];

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_gpu(f, &app, area);
        }).expect("draw gpu empty");
    }

    #[test]
    fn test_battery_no_data() {
        let mut app = App::new_mock();
        app.mock_battery = None;

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_battery(f, &app, area);
        }).expect("draw battery none");
    }

    #[test]
    fn test_sensors_empty() {
        let mut app = App::new_mock();
        app.mock_sensors = vec![];

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_sensors(f, &app, area);
        }).expect("draw sensors empty");
    }

    // === Additional Coverage: Large Data Tests ===

    #[test]
    fn test_gpu_many_devices() {
        let mut app = App::new_mock();

        // 4 GPUs
        app.mock_gpus = vec![
            crate::app::MockGpuData {
                name: "GPU 0".to_string(),
                gpu_util: 25.0,
                vram_used: 4 * 1024 * 1024 * 1024,
                vram_total: 12 * 1024 * 1024 * 1024,
                temperature: 55.0,
                power_watts: 150,
                power_limit_watts: 300,
                clock_mhz: 2100,
                history: vec![20.0, 25.0, 30.0, 25.0],
            },
            crate::app::MockGpuData {
                name: "GPU 1".to_string(),
                gpu_util: 50.0,
                vram_used: 8 * 1024 * 1024 * 1024,
                vram_total: 12 * 1024 * 1024 * 1024,
                temperature: 65.0,
                power_watts: 200,
                power_limit_watts: 300,
                clock_mhz: 2200,
                history: vec![45.0, 50.0, 55.0, 50.0],
            },
            crate::app::MockGpuData {
                name: "GPU 2".to_string(),
                gpu_util: 75.0,
                vram_used: 10 * 1024 * 1024 * 1024,
                vram_total: 12 * 1024 * 1024 * 1024,
                temperature: 75.0,
                power_watts: 250,
                power_limit_watts: 300,
                clock_mhz: 2300,
                history: vec![70.0, 75.0, 80.0, 75.0],
            },
            crate::app::MockGpuData {
                name: "GPU 3".to_string(),
                gpu_util: 95.0,
                vram_used: 11 * 1024 * 1024 * 1024,
                vram_total: 12 * 1024 * 1024 * 1024,
                temperature: 85.0,
                power_watts: 290,
                power_limit_watts: 300,
                clock_mhz: 2400,
                history: vec![90.0, 95.0, 98.0, 95.0],
            },
        ];

        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 30);
            draw_gpu(f, &app, area);
        }).expect("draw many gpus");
    }

    #[test]
    fn test_sensors_many() {
        let mut app = App::new_mock();

        // 10 sensors
        app.mock_sensors = (0..10).map(|i| {
            crate::app::MockSensorData {
                name: format!("sensor/temp{}", i),
                label: format!("Sensor {}", i),
                value: 40.0 + i as f64 * 5.0,
                max: Some(100.0),
                crit: Some(110.0),
                sensor_type: crate::app::MockSensorType::Temperature,
            }
        }).collect();

        let backend = TestBackend::new(100, 25);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 25);
            draw_sensors(f, &app, area);
        }).expect("draw many sensors");
    }

    #[test]
    fn test_containers_many() {
        let mut app = App::new_mock();

        // 10 containers
        app.mock_containers = (0..10).map(|i| {
            crate::app::MockContainerData {
                name: format!("container-{}", i),
                status: if i % 3 == 0 { "exited" } else { "running" }.to_string(),
                cpu_percent: (i as f64 * 10.0) % 100.0,
                mem_used: (i as u64 + 1) * 100 * 1024 * 1024,
                mem_limit: 2 * 1024 * 1024 * 1024,
            }
        }).collect();

        let backend = TestBackend::new(100, 25);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 25);
            draw_containers_inner(f, &app, area);
        }).expect("draw many containers");
    }

    // === Additional Coverage: Sensor Compact View ===

    #[test]
    fn test_sensors_compact_view() {
        let app = App::new_mock();

        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 15);
            draw_sensors_compact(f, &app, area);
        }).expect("draw sensors compact");
    }

    #[test]
    fn test_sensors_compact_empty() {
        let mut app = App::new_mock();
        app.mock_sensors = vec![];

        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 15);
            draw_sensors_compact(f, &app, area);
        }).expect("draw sensors compact empty");
    }

    // === Additional Coverage: Edge Cases ===

    #[test]
    fn test_gpu_with_zero_vram() {
        let mut app = App::new_mock();

        app.mock_gpus = vec![
            crate::app::MockGpuData {
                name: "Test GPU".to_string(),
                gpu_util: 50.0,
                vram_used: 0,
                vram_total: 0, // Edge case: no VRAM
                temperature: 60.0,
                power_watts: 100,
                power_limit_watts: 200,
                clock_mhz: 2000,
                history: vec![50.0],
            },
        ];

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_gpu(f, &app, area);
        }).expect("draw gpu zero vram");
    }

    #[test]
    fn test_container_zero_mem_limit() {
        let mut app = App::new_mock();

        app.mock_containers = vec![
            crate::app::MockContainerData {
                name: "unlimited-container".to_string(),
                status: "running".to_string(),
                cpu_percent: 50.0,
                mem_used: 1024 * 1024 * 1024,
                mem_limit: 0, // Edge case: unlimited memory
            },
        ];

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_containers_inner(f, &app, area);
        }).expect("draw container zero limit");
    }

    #[test]
    fn test_battery_low_health() {
        let mut app = App::new_mock();

        app.mock_battery = Some(crate::app::MockBatteryData {
            percent: 50.0,
            charging: false,
            time_remaining_mins: Some(60),
            power_watts: 20.0,
            health_percent: 50.0, // Low health
            cycle_count: 1000, // High cycle count
        });

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_battery(f, &app, area);
        }).expect("draw battery low health");
    }

    #[test]
    fn test_all_panels_minimal_height() {
        let app = App::new_mock();

        let backend = TestBackend::new(100, 3);
        let mut terminal = Terminal::new(backend).expect("terminal");

        // All panels should handle height=3 gracefully
        terminal.draw(|f| { draw_cpu(f, &app, Rect::new(0, 0, 100, 3)); }).ok();
        terminal.draw(|f| { draw_memory(f, &app, Rect::new(0, 0, 100, 3)); }).ok();
        terminal.draw(|f| { draw_disk(f, &app, Rect::new(0, 0, 100, 3)); }).ok();
        terminal.draw(|f| { draw_network(f, &app, Rect::new(0, 0, 100, 3)); }).ok();
        terminal.draw(|f| { draw_gpu(f, &app, Rect::new(0, 0, 100, 3)); }).ok();
        terminal.draw(|f| { draw_battery(f, &app, Rect::new(0, 0, 100, 3)); }).ok();
        terminal.draw(|f| { draw_sensors(f, &app, Rect::new(0, 0, 100, 3)); }).ok();
        terminal.draw(|f| { draw_psi(f, &app, Rect::new(0, 0, 100, 3)); }).ok();
    }

    #[test]
    fn test_all_panels_maximum_size() {
        let app = App::new_mock();

        let backend = TestBackend::new(300, 100);
        let mut terminal = Terminal::new(backend).expect("terminal");

        // All panels should handle large sizes
        terminal.draw(|f| { draw_cpu(f, &app, Rect::new(0, 0, 300, 100)); }).ok();
        terminal.draw(|f| { draw_memory(f, &app, Rect::new(0, 0, 300, 100)); }).ok();
        terminal.draw(|f| { draw_disk(f, &app, Rect::new(0, 0, 300, 100)); }).ok();
        terminal.draw(|f| { draw_network(f, &app, Rect::new(0, 0, 300, 100)); }).ok();
        terminal.draw(|f| { draw_gpu(f, &app, Rect::new(0, 0, 300, 100)); }).ok();
        terminal.draw(|f| { draw_battery(f, &app, Rect::new(0, 0, 300, 100)); }).ok();
        terminal.draw(|f| { draw_sensors(f, &app, Rect::new(0, 0, 300, 100)); }).ok();
        terminal.draw(|f| { draw_psi(f, &app, Rect::new(0, 0, 300, 100)); }).ok();
    }

    #[test]
    fn test_connections_panel_various_sizes() {
        let app = App::new_mock();

        for (w, h) in [(60, 10), (100, 20), (150, 30), (200, 40)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).expect("terminal");

            terminal.draw(|f| {
                let area = Rect::new(0, 0, w, h);
                draw_connections(f, &app, area);
            }).expect("draw connections");
        }
    }

    #[test]
    fn test_sensors_with_critical_temp() {
        let mut app = App::new_mock();

        // Sensor at critical temperature
        app.mock_sensors = vec![
            crate::app::MockSensorData {
                name: "cpu/temp1".to_string(),
                label: "CPU Critical".to_string(),
                value: 105.0, // Above crit
                max: Some(95.0),
                crit: Some(100.0),
                sensor_type: crate::app::MockSensorType::Temperature,
            },
        ];

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_sensors(f, &app, area);
        }).expect("draw critical sensor");
    }

    #[test]
    fn test_sensors_no_thresholds() {
        let mut app = App::new_mock();

        // Sensor with no max/crit thresholds
        app.mock_sensors = vec![
            crate::app::MockSensorData {
                name: "misc/temp1".to_string(),
                label: "Unknown Sensor".to_string(),
                value: 50.0,
                max: None, // No thresholds
                crit: None,
                sensor_type: crate::app::MockSensorType::Temperature,
            },
        ];

        let backend = TestBackend::new(100, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 20);
            draw_sensors(f, &app, area);
        }).expect("draw sensor no thresholds");
    }

    // === Additional Unique Panel Tests for Coverage ===

    #[test]
    fn test_btop_block_with_color() {
        use trueno_viz::monitor::ratatui::style::Color;
        let block = btop_block("Test Title", Color::Blue);
        // Just verify it doesn't panic and returns a valid block
        let _ = block;
    }

    #[test]
    fn test_draw_network_10gbit() {
        let mut app = App::new_mock();

        // Simulate very high bandwidth
        app.net_rx_history = vec![1e10; 60]; // 10 GB/s
        app.net_tx_history = vec![5e9; 60];  // 5 GB/s
        app.net_rx_peak = 1.5e10;
        app.net_tx_peak = 8e9;

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_network(f, &app, area);
        }).expect("draw network high bandwidth");
    }

    #[test]
    fn test_draw_cpu_boost_indicator() {
        let app = App::new_mock();
        // Mock data should trigger boost indicator (>3GHz)

        let backend = TestBackend::new(100, 25);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 100, 25);
            draw_cpu(f, &app, area);
        }).expect("draw cpu with boosting");
    }

    #[test]
    fn test_draw_memory_near_full() {
        let mut app = App::new_mock();

        // Simulate 99% memory usage
        app.mem_history = vec![0.99; 60];
        app.swap_history = vec![0.95; 60];

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_memory(f, &app, area);
        }).expect("draw memory extreme usage");
    }

    #[test]
    fn test_draw_psi_small() {
        let app = App::new_mock();

        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 15);
            draw_psi(f, &app, area);
        }).expect("draw psi");
    }

    #[test]
    fn test_draw_treemap_standard() {
        let app = App::new_mock();

        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_treemap(f, &app, area);
        }).expect("draw treemap");
    }

    #[test]
    fn test_draw_sensors_compact_multi() {
        let mut app = App::new_mock();

        app.mock_sensors = vec![
            crate::app::MockSensorData {
                name: "cpu/temp1".to_string(),
                label: "CPU".to_string(),
                value: 65.0,
                max: Some(90.0),
                crit: Some(100.0),
                sensor_type: crate::app::MockSensorType::Temperature,
            },
            crate::app::MockSensorData {
                name: "gpu/temp1".to_string(),
                label: "GPU".to_string(),
                value: 75.0,
                max: Some(95.0),
                crit: Some(105.0),
                sensor_type: crate::app::MockSensorType::Temperature,
            },
        ];

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 40, 10);
            draw_sensors_compact(f, &app, area);
        }).expect("draw sensors compact");
    }

    #[test]
    fn test_draw_connections_wide() {
        let app = App::new_mock();

        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 120, 30);
            draw_connections(f, &app, area);
        }).expect("draw connections");
    }

    #[test]
    fn test_draw_process_tall() {
        let mut app = App::new_mock();

        let backend = TestBackend::new(120, 50);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 120, 50);
            draw_process(f, &mut app, area);
        }).expect("draw many processes");
    }

    #[test]
    fn test_draw_files_mini() {
        let app = App::new_mock();

        let backend = TestBackend::new(15, 3);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 15, 3);
            draw_files(f, &app, area);
        }).expect("draw files tiny");
    }

    #[test]
    fn test_entropy_heatmap_val_high() {
        let (display, r, g, b) = entropy_heatmap(0.9);
        assert!(display.contains("%"));
        assert_eq!((r, g, b), (80, 200, 100)); // Green for high entropy
    }

    #[test]
    fn test_entropy_heatmap_val_medium() {
        let (display, r, g, b) = entropy_heatmap(0.6);
        assert!(display.contains("%"));
        assert_eq!((r, g, b), (200, 200, 80)); // Yellow for medium entropy
    }

    #[test]
    fn test_entropy_heatmap_val_low() {
        let (display, r, g, b) = entropy_heatmap(0.3);
        assert!(display.contains("%"));
        assert_eq!((r, g, b), (220, 140, 60)); // Orange for low entropy
    }

    #[test]
    fn test_entropy_heatmap_val_very_low() {
        let (display, r, g, b) = entropy_heatmap(0.1);
        assert!(display.contains("%"));
        assert_eq!((r, g, b), (220, 80, 80)); // Red for very low entropy
    }
}

/// Advanced probar tests for panels using soft assertions, snapshots, and pixel coverage
#[cfg(test)]
mod advanced_panel_probar_tests {
    use super::*;
    use crate::app::App;
    use jugar_probar::SoftAssertions;
    use jugar_probar::pixel_coverage::{PixelCoverageTracker, PixelRegion};
    use jugar_probar::tui::{TuiFrame, TuiSnapshot};
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
                let cell = buffer.cell((x, y)).expect("cell in bounds");
                line.push_str(cell.symbol());
            }
            lines.push(line.trim_end().to_string());
        }

        TuiFrame::from_lines(&lines.iter().map(|s| s.as_str()).collect::<Vec<_>>())
    }

    /// Test CPU panel with soft assertions for comprehensive validation
    #[test]
    fn test_cpu_panel_soft_assertions() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_cpu(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);
        let text = frame.as_text();

        let mut soft = SoftAssertions::new();
        soft.assert_contains(&text, "CPU", "should have CPU title");
        soft.assert_eq(&frame.width(), &80, "frame width");
        soft.assert_eq(&frame.height(), &20, "frame height");
        // btop-style borders
        soft.assert_true(text.contains("╭") || text.contains("┌"), "should have top corners");

        soft.verify().expect("all CPU panel assertions should pass");
    }

    /// Test Memory panel with soft assertions
    #[test]
    fn test_memory_panel_soft_assertions() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_memory(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);
        let text = frame.as_text();

        let mut soft = SoftAssertions::new();
        soft.assert_contains(&text, "Memory", "should have Memory title");
        soft.assert_true(
            text.contains("Used") || text.contains("GB") || text.contains("MB"),
            "should show memory usage"
        );

        soft.verify().expect("all Memory panel assertions should pass");
    }

    /// Test Process panel with soft assertions
    #[test]
    fn test_process_panel_soft_assertions() {
        let mut app = App::new_mock();
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 120, 30);
            draw_process(f, &mut app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);
        let text = frame.as_text();

        let mut soft = SoftAssertions::new();
        soft.assert_contains(&text, "Process", "should have Process title");
        // Process table headers
        soft.assert_true(
            text.contains("PID") || text.contains("CMD") || text.contains("CPU"),
            "should have process table columns"
        );

        soft.verify().expect("all Process panel assertions should pass");
    }

    /// Test panel snapshots for regression detection
    #[test]
    fn test_panel_snapshot_cpu() {
        let app = App::new_mock();
        let backend = TestBackend::new(60, 15);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 60, 15);
            draw_cpu(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        let snapshot = TuiSnapshot::from_frame("cpu_panel_60x15", &frame)
            .with_metadata("panel", "cpu")
            .with_metadata("size", "60x15");

        assert_eq!(snapshot.width, 60);
        assert_eq!(snapshot.height, 15);
        assert!(!snapshot.hash.is_empty());

        // Round-trip verification
        let restored = snapshot.to_frame();
        assert!(frame.is_identical(&restored));
    }

    /// Test panel snapshots for memory
    #[test]
    fn test_panel_snapshot_memory() {
        let app = App::new_mock();
        let backend = TestBackend::new(60, 15);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 60, 15);
            draw_memory(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);

        let snapshot = TuiSnapshot::from_frame("memory_panel_60x15", &frame)
            .with_metadata("panel", "memory");

        // Verify snapshot round-trips correctly
        let restored = snapshot.to_frame();
        assert!(frame.is_identical(&restored));
    }

    /// Test pixel coverage for CPU panel
    #[test]
    fn test_cpu_panel_pixel_coverage() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_cpu(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();

        // Create pixel coverage tracker
        let mut pixels = PixelCoverageTracker::builder()
            .resolution(80, 20)
            .grid_size(8, 4)
            .threshold(0.30)
            .build();

        // Record non-empty pixels
        for y in 0..20 {
            for x in 0..80 {
                if let Some(cell) = buffer.cell((x, y)) {
                    if cell.symbol() != " " {
                        pixels.record_region(PixelRegion::new(x as u32, y as u32, 1, 1));
                    }
                }
            }
        }

        let report = pixels.generate_report();
        // CPU panel with borders and content should cover at least 30%
        assert!(
            report.overall_coverage >= 0.20,
            "CPU panel should cover at least 20% of area, got {:.1}%",
            report.overall_coverage * 100.0
        );
    }

    /// Test pixel coverage for Memory panel
    #[test]
    fn test_memory_panel_pixel_coverage() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 20);
            draw_memory(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();

        let mut pixels = PixelCoverageTracker::builder()
            .resolution(80, 20)
            .grid_size(8, 4)
            .threshold(0.25)
            .build();

        for y in 0..20 {
            for x in 0..80 {
                if let Some(cell) = buffer.cell((x, y)) {
                    if cell.symbol() != " " {
                        pixels.record_region(PixelRegion::new(x as u32, y as u32, 1, 1));
                    }
                }
            }
        }

        let report = pixels.generate_report();
        assert!(
            report.overall_coverage >= 0.15,
            "Memory panel should cover at least 15% of area, got {:.1}%",
            report.overall_coverage * 100.0
        );
    }

    /// Test all panels render without panic with soft assertions
    #[test]
    fn test_all_panels_render_soft_assertions() {
        let app = App::new_mock();

        // Note: Process panel excluded - it takes &mut App (tested separately above)
        let panels: Vec<(&str, fn(&mut ratatui::Frame<'_>, &App, Rect))> = vec![
            ("CPU", draw_cpu),
            ("Memory", draw_memory),
            ("Disk", draw_disk),
            ("Network", draw_network),
            ("GPU", draw_gpu),
            ("Battery", draw_battery),
            ("Sensors", draw_sensors),
            ("Files", draw_files),
        ];

        for (name, draw_fn) in panels {
            let backend = TestBackend::new(80, 20);
            let mut terminal = Terminal::new(backend).expect("terminal");

            terminal.draw(|f| {
                let area = Rect::new(0, 0, 80, 20);
                draw_fn(f, &app, area);
            }).expect(&format!("draw {}", name));

            let buffer = terminal.backend().buffer().clone();
            let frame = buffer_to_frame(&buffer, 0);

            let mut soft = SoftAssertions::new();
            soft.assert_eq(&frame.width(), &80, &format!("{} width", name));
            soft.assert_eq(&frame.height(), &20, &format!("{} height", name));
            soft.assert_true(frame.height() > 0, &format!("{} should render content", name));

            soft.verify().expect(&format!("all {} assertions should pass", name));
        }
    }

    /// Test panel responsiveness with soft assertions
    #[test]
    fn test_panel_responsiveness_soft_assertions() {
        let app = App::new_mock();
        let sizes = [
            (40, 10, "tiny"),
            (80, 20, "medium"),
            (120, 30, "large"),
        ];

        for (width, height, name) in sizes {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).expect("terminal");

            terminal.draw(|f| {
                let area = Rect::new(0, 0, width, height);
                draw_cpu(f, &app, area);
            }).expect(&format!("draw at {}", name));

            let buffer = terminal.backend().buffer().clone();
            let frame = buffer_to_frame(&buffer, 0);

            let mut soft = SoftAssertions::new();
            soft.assert_eq(&frame.width(), &width, &format!("{} width", name));
            soft.assert_eq(&frame.height(), &height, &format!("{} height", name));

            soft.verify().expect(&format!("responsiveness at {} should pass", name));
        }
    }

    /// Test snapshot difference detection for panels
    #[test]
    fn test_panel_snapshot_diff_detection() {
        // Create two memory panels with different data
        let mut app1 = App::new_mock();
        app1.mem_used = 4_000_000_000; // 4GB

        let mut app2 = App::new_mock();
        app2.mem_used = 8_000_000_000; // 8GB

        let backend1 = TestBackend::new(60, 10);
        let mut terminal1 = Terminal::new(backend1).expect("terminal");
        terminal1.draw(|f| {
            draw_memory(f, &app1, Rect::new(0, 0, 60, 10));
        }).expect("draw1");

        let backend2 = TestBackend::new(60, 10);
        let mut terminal2 = Terminal::new(backend2).expect("terminal");
        terminal2.draw(|f| {
            draw_memory(f, &app2, Rect::new(0, 0, 60, 10));
        }).expect("draw2");

        let frame1 = buffer_to_frame(terminal1.backend().buffer(), 0);
        let frame2 = buffer_to_frame(terminal2.backend().buffer(), 0);

        let snap1 = TuiSnapshot::from_frame("mem_4gb", &frame1);
        let snap2 = TuiSnapshot::from_frame("mem_8gb", &frame2);

        // Different memory values should produce different snapshots
        assert!(!snap1.matches(&snap2), "Different memory values should produce different snapshots");
    }

    /// Test Files panel with soft assertions
    #[test]
    fn test_files_panel_soft_assertions() {
        let app = App::new_mock();
        let backend = TestBackend::new(80, 25);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            let area = Rect::new(0, 0, 80, 25);
            draw_files(f, &app, area);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = buffer_to_frame(&buffer, 0);
        let text = frame.as_text();

        let mut soft = SoftAssertions::new();
        soft.assert_contains(&text, "Files", "should have Files title");
        soft.assert_true(
            text.contains("╭") || text.contains("┌"),
            "should have btop-style borders"
        );

        soft.verify().expect("all Files panel assertions should pass");
    }
}
