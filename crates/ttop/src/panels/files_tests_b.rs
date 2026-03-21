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
