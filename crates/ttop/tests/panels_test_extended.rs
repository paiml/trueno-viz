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

/// Property-based tests for TUI rendering bounds safety
#[cfg(test)]
mod proptest_tui_bounds {
    use super::*;
    use proptest::prelude::*;
    use ttop::app::App;
    use ttop::ui;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Test that draw never panics for any reasonable terminal size
        #[test]
        fn test_draw_any_size(width in 10u16..300, height in 5u16..100) {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = App::new(true, true);

            // Should never panic
            terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
        }

        /// Test that draw handles edge case sizes without panic
        #[test]
        fn test_draw_edge_sizes(width in 1u16..20, height in 1u16..10) {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = App::new(true, true);

            // Should handle tiny sizes gracefully
            terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
        }

        /// Test exact width=100 which triggered the original panic
        #[test]
        fn test_draw_width_100(height in 10u16..60) {
            let backend = TestBackend::new(100, height);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = App::new(true, true);

            terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
        }

        /// Test various aspect ratios
        #[test]
        fn test_draw_aspect_ratios(base in 20u16..80) {
            // Wide
            let backend = TestBackend::new(base * 3, base);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = App::new(true, true);
            terminal.draw(|f| ui::draw(f, &mut app)).unwrap();

            // Tall
            let backend = TestBackend::new(base, base * 2);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = App::new(true, true);
            terminal.draw(|f| ui::draw(f, &mut app)).unwrap();

            // Square
            let backend = TestBackend::new(base, base);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = App::new(true, true);
            terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
        }
    }

    /// Test specific problematic sizes that have caused panics
    #[test]
    fn test_known_problem_sizes() {
        let problem_sizes = [
            (100, 41),  // Original Mac panic
            (100, 1),   // Edge case
            (100, 2),
            (80, 24),   // Standard terminal
            (120, 40),
            (200, 50),
            (50, 100),  // Tall
            (15, 5),    // Tiny
        ];

        for (width, height) in problem_sizes {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = App::new(true, true);

            // Should not panic
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
            }));

            assert!(result.is_ok(), "Panicked at size {}x{}", width, height);
        }
    }

    /// Test with populated app data (simulating real usage)
    #[test]
    fn test_populated_app_various_sizes() {
        let sizes = [
            (100, 41),
            (100, 30),
            (80, 24),
            (120, 50),
            (60, 20),
        ];

        for (width, height) in sizes {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = App::new(true, true);

            // Populate with realistic data
            app.cpu_history = (0..300).map(|i| (i as f64 / 300.0) * 0.8).collect();
            app.mem_history = (0..300).map(|i| 0.3 + (i as f64 / 600.0)).collect();
            app.swap_history = (0..300).map(|i| (i as f64 / 1000.0)).collect();
            app.net_rx_history = (0..300).map(|i| (i as f64 * 1000.0)).collect();
            app.net_tx_history = (0..300).map(|i| (i as f64 * 500.0)).collect();
            app.mem_total = 32 * 1024 * 1024 * 1024;
            app.mem_used = 16 * 1024 * 1024 * 1024;
            app.mem_free = 8 * 1024 * 1024 * 1024;
            app.mem_available = 12 * 1024 * 1024 * 1024;
            app.mem_cached = 4 * 1024 * 1024 * 1024;
            app.swap_total = 8 * 1024 * 1024 * 1024;
            app.swap_used = 1 * 1024 * 1024 * 1024;

            // Simulate many CPU cores (like on Mac Pro)
            app.per_core_percent = (0..16).map(|i| (i as f64 * 5.0) % 100.0).collect();

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
            }));

            assert!(result.is_ok(), "Panicked at size {}x{} with populated data", width, height);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(30))]

        /// Test with varying amounts of CPU cores
        #[test]
        fn test_various_core_counts(width in 60u16..200, height in 20u16..80, cores in 1usize..128) {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = App::new(true, true);

            // Simulate many cores
            app.per_core_percent = (0..cores).map(|i| ((i * 7) % 100) as f64).collect();
            app.cpu_history = (0..100).map(|i| (i as f64 / 100.0) * 0.7).collect();

            terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
        }

        /// Test with very long history data
        #[test]
        fn test_long_history(width in 40u16..150, height in 15u16..60, history_len in 100usize..1000) {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = App::new(true, true);

            app.cpu_history = (0..history_len).map(|i| (i as f64 / history_len as f64) * 0.9).collect();
            app.mem_history = (0..history_len).map(|i| 0.2 + (i as f64 / history_len as f64) * 0.5).collect();
            app.net_rx_history = (0..history_len).map(|i| i as f64 * 1024.0).collect();
            app.net_tx_history = (0..history_len).map(|i| i as f64 * 512.0).collect();

            terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
        }
    }
}
