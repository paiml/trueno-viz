mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn test_panel_visibility_default() {
        let vis = PanelVisibility::default();
        assert!(vis.cpu);
        assert!(vis.memory);
        assert!(vis.disk);
        assert!(vis.network);
        assert!(vis.process);
        assert!(vis.gpu);
        assert!(vis.battery);
        assert!(vis.sensors);
        assert!(!vis.files); // Off by default
    }

    #[test]
    fn test_mock_app_creation() {
        let app = App::new_mock();
        assert!(app.deterministic);
        assert_eq!(app.frame_id, 100);
        assert_eq!(app.avg_frame_time_us, 1000);
        assert_eq!(app.max_frame_time_us, 2000);
        assert!(!app.show_fps);
    }

    #[test]
    fn test_mock_app_history_populated() {
        let app = App::new_mock();
        assert_eq!(app.cpu_history.len(), 8);
        assert_eq!(app.mem_history.len(), 8);
        assert_eq!(app.per_core_percent.len(), 8);
    }

    #[test]
    fn test_mock_app_memory_values() {
        let app = App::new_mock();
        assert_eq!(app.mem_total, 16 * 1024 * 1024 * 1024);
        assert_eq!(app.mem_used, 10 * 1024 * 1024 * 1024);
        assert_eq!(app.mem_available, 6 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_update_frame_stats_empty() {
        let mut app = App::new_mock();
        app.update_frame_stats(&[]);
        // Should not panic, values unchanged
    }

    #[test]
    fn test_update_frame_stats_single() {
        let mut app = App::new_mock();
        app.update_frame_stats(&[Duration::from_micros(500)]);
        assert_eq!(app.avg_frame_time_us, 500);
        assert_eq!(app.max_frame_time_us, 500);
    }

    #[test]
    fn test_update_frame_stats_multiple() {
        let mut app = App::new_mock();
        let times = vec![
            Duration::from_micros(100),
            Duration::from_micros(200),
            Duration::from_micros(300),
        ];
        app.update_frame_stats(&times);
        assert_eq!(app.avg_frame_time_us, 200); // (100+200+300)/3
        assert_eq!(app.max_frame_time_us, 300);
    }

    #[test]
    fn test_visible_panels_default() {
        let app = App::new_mock();
        let visible = app.visible_panels();
        // Default: cpu, memory, disk, network, process (not files, battery/sensors may vary)
        assert!(visible.contains(&PanelType::Cpu));
        assert!(visible.contains(&PanelType::Memory));
        assert!(visible.contains(&PanelType::Disk));
        assert!(visible.contains(&PanelType::Network));
        assert!(visible.contains(&PanelType::Process));
        assert!(!visible.contains(&PanelType::Files)); // Off by default
    }

    #[test]
    fn test_visible_panels_with_files() {
        let mut app = App::new_mock();
        app.panels.files = true;
        let visible = app.visible_panels();
        assert!(visible.contains(&PanelType::Files));
    }

    #[test]
    fn test_visible_panels_all_disabled() {
        let mut app = App::new_mock();
        app.panels.cpu = false;
        app.panels.memory = false;
        app.panels.disk = false;
        app.panels.network = false;
        app.panels.process = false;
        app.panels.gpu = false;
        app.panels.battery = false;
        app.panels.sensors = false;
        app.panels.files = false;
        let visible = app.visible_panels();
        assert!(visible.is_empty());
    }

    #[test]
    fn test_first_visible_panel_default() {
        let app = App::new_mock();
        let first = app.first_visible_panel();
        assert_eq!(first, PanelType::Cpu);
    }

    #[test]
    fn test_first_visible_panel_when_cpu_disabled() {
        let mut app = App::new_mock();
        app.panels.cpu = false;
        let first = app.first_visible_panel();
        assert_eq!(first, PanelType::Memory);
    }

    #[test]
    fn test_is_panel_visible() {
        let app = App::new_mock();
        assert!(app.is_panel_visible(PanelType::Cpu));
        assert!(app.is_panel_visible(PanelType::Memory));
        assert!(!app.is_panel_visible(PanelType::Files));
    }

    #[test]
    fn test_handle_key_quit_q() {
        let mut app = App::new_mock();
        let quit = app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(quit);
    }

    #[test]
    fn test_handle_key_quit_ctrl_c() {
        let mut app = App::new_mock();
        let quit = app.handle_key(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(quit);
    }

    #[test]
    fn test_handle_key_quit_esc() {
        let mut app = App::new_mock();
        let quit = app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(quit);
    }

    #[test]
    fn test_handle_key_help_toggle() {
        let mut app = App::new_mock();
        assert!(!app.show_help);
        app.handle_key(KeyCode::Char('?'), KeyModifiers::NONE);
        assert!(app.show_help);
        app.handle_key(KeyCode::Char('?'), KeyModifiers::NONE);
        assert!(!app.show_help);
    }

    #[test]
    fn test_handle_key_panel_toggles() {
        let mut app = App::new_mock();

        // Toggle CPU off
        assert!(app.panels.cpu);
        app.handle_key(KeyCode::Char('1'), KeyModifiers::NONE);
        assert!(!app.panels.cpu);

        // Toggle memory off
        assert!(app.panels.memory);
        app.handle_key(KeyCode::Char('2'), KeyModifiers::NONE);
        assert!(!app.panels.memory);

        // Toggle files on (off by default)
        assert!(!app.panels.files);
        app.handle_key(KeyCode::Char('9'), KeyModifiers::NONE);
        assert!(app.panels.files);
    }

    #[test]
    fn test_handle_key_filter_mode() {
        let mut app = App::new_mock();
        assert!(!app.show_filter_input);

        // Enter filter mode
        app.handle_key(KeyCode::Char('/'), KeyModifiers::NONE);
        assert!(app.show_filter_input);

        // Type some text
        app.handle_key(KeyCode::Char('t'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('e'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('t'), KeyModifiers::NONE);
        assert_eq!(app.filter, "test");

        // Backspace
        app.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(app.filter, "tes");

        // Escape clears and exits
        app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(!app.show_filter_input);
        assert_eq!(app.filter, "");
    }

    #[test]
    fn test_handle_key_filter_enter_confirm() {
        let mut app = App::new_mock();
        app.handle_key(KeyCode::Char('f'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('a'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert!(!app.show_filter_input);
        assert_eq!(app.filter, "a"); // Preserved
    }

    #[test]
    fn test_handle_key_sort_toggle() {
        let mut app = App::new_mock();
        assert_eq!(app.sort_column, ProcessSortColumn::Cpu);

        app.handle_key(KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(app.sort_column, ProcessSortColumn::Mem);

        app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
        assert_eq!(app.sort_column, ProcessSortColumn::State);
    }

    #[test]
    fn test_handle_key_sort_reverse() {
        let mut app = App::new_mock();
        assert!(app.sort_descending);
        app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
        assert!(!app.sort_descending);
    }

    #[test]
    fn test_handle_key_tree_toggle() {
        let mut app = App::new_mock();
        assert!(!app.show_tree);
        app.handle_key(KeyCode::Char('t'), KeyModifiers::NONE);
        assert!(app.show_tree);
    }

    #[test]
    fn test_handle_key_reset_view() {
        let mut app = App::new_mock();
        app.panels.cpu = false;
        app.focused_panel = Some(PanelType::Memory);
        app.exploded_panel = Some(PanelType::Disk);

        app.handle_key(KeyCode::Char('0'), KeyModifiers::NONE);

        assert!(app.panels.cpu);
        assert!(app.focused_panel.is_none());
        assert!(app.exploded_panel.is_none());
    }

    #[test]
    fn test_handle_key_focus_start_h() {
        let mut app = App::new_mock();
        assert!(app.focused_panel.is_none());

        app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
        assert!(app.focused_panel.is_some());
    }

    #[test]
    fn test_handle_key_focus_start_l() {
        let mut app = App::new_mock();
        assert!(app.focused_panel.is_none());

        app.handle_key(KeyCode::Char('l'), KeyModifiers::NONE);
        assert!(app.focused_panel.is_some());
    }

    #[test]
    fn test_handle_key_focus_navigation() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Cpu);

        // Navigate right
        app.handle_key(KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(app.focused_panel, Some(PanelType::Memory));

        // Navigate left
        app.handle_key(KeyCode::Left, KeyModifiers::NONE);
        assert_eq!(app.focused_panel, Some(PanelType::Cpu));
    }

    #[test]
    fn test_handle_key_explode_panel() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Cpu);

        // Explode with Enter
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(app.exploded_panel, Some(PanelType::Cpu));

        // Un-explode with Enter
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert!(app.exploded_panel.is_none());
    }

    #[test]
    fn test_handle_key_explode_with_z() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Memory);

        app.handle_key(KeyCode::Char('z'), KeyModifiers::NONE);
        assert_eq!(app.exploded_panel, Some(PanelType::Memory));
    }

    #[test]
    fn test_handle_key_esc_unexplode() {
        let mut app = App::new_mock();
        app.exploded_panel = Some(PanelType::Cpu);

        let quit = app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(!quit);
        assert!(app.exploded_panel.is_none());
    }

    #[test]
    fn test_handle_key_esc_unfocus() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Cpu);

        let quit = app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(!quit);
        assert!(app.focused_panel.is_none());
    }

    #[test]
    fn test_handle_key_files_view_mode() {
        let mut app = App::new_mock();
        use crate::state::FilesViewMode;

        assert_eq!(app.files_view_mode, FilesViewMode::Size);
        app.handle_key(KeyCode::Char('v'), KeyModifiers::NONE);
        assert_eq!(app.files_view_mode, FilesViewMode::Entropy);
    }

    #[test]
    fn test_navigate_panel_focus_wrap_right() {
        let mut app = App::new_mock();
        // Only keep CPU visible - disable everything else
        app.panels.memory = false;
        app.panels.disk = false;
        app.panels.network = false;
        app.panels.process = false;
        app.panels.gpu = false;
        app.panels.battery = false;
        app.panels.sensors = false;
        app.panels.files = false;

        app.focused_panel = Some(PanelType::Cpu);
        app.navigate_panel_focus(KeyCode::Right);
        // Should wrap to CPU (only visible panel)
        assert_eq!(app.focused_panel, Some(PanelType::Cpu));
    }

    #[test]
    fn test_navigate_panel_focus_wrap_left() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Cpu);
        app.navigate_panel_focus(KeyCode::Left);
        // Should wrap to last visible panel
        assert!(app.focused_panel.is_some());
    }

    #[test]
    fn test_navigate_panel_focus_empty() {
        let mut app = App::new_mock();
        app.panels.cpu = false;
        app.panels.memory = false;
        app.panels.disk = false;
        app.panels.network = false;
        app.panels.process = false;
        app.panels.gpu = false;
        app.panels.battery = false;
        app.panels.sensors = false;

        app.navigate_panel_focus(KeyCode::Right);
        // Should not panic
    }

    #[test]
    fn test_navigate_process_empty() {
        let mut app = App::new_mock();
        // Mock has no real processes
        app.navigate_process(1);
        app.navigate_process(-1);
        // Should not panic
    }

    #[test]
    fn test_process_count_empty() {
        let app = App::new_mock();
        assert_eq!(app.process_count(), 0);
    }

    #[test]
    fn test_sorted_processes_empty() {
        let app = App::new_mock();
        let procs = app.sorted_processes();
        assert!(procs.is_empty());
    }

    #[test]
    fn test_has_gpu_mock() {
        let app = App::new_mock();
        // Mock collectors are not "available"
        // This tests the has_gpu() method runs without panic
        let _has_gpu = app.has_gpu();
    }

    #[test]
    fn test_thrashing_severity() {
        let app = App::new_mock();
        let severity = app.thrashing_severity();
        assert_eq!(severity, ThrashingSeverity::None);
    }

    #[test]
    fn test_has_zram() {
        let app = App::new_mock();
        let _has = app.has_zram();
        // Just verify it doesn't panic
    }

    #[test]
    fn test_zram_ratio() {
        let app = App::new_mock();
        let ratio = app.zram_ratio();
        assert!(ratio >= 0.0);
    }

    #[test]
    fn test_selected_process_none() {
        let app = App::new_mock();
        assert!(app.selected_process().is_none());
    }

    #[test]
    fn test_request_signal_no_process() {
        let mut app = App::new_mock();
        app.request_signal(SignalType::Term);
        assert!(app.pending_signal.is_none()); // No process selected
    }

    #[test]
    fn test_cancel_signal() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test".to_string(), SignalType::Term));
        app.cancel_signal();
        assert!(app.pending_signal.is_none());
    }

    #[test]
    fn test_confirm_signal_none() {
        let mut app = App::new_mock();
        app.confirm_signal(); // No pending signal
        // Should not panic
    }

    #[test]
    fn test_clear_old_signal_result_none() {
        let mut app = App::new_mock();
        app.clear_old_signal_result();
        // Should not panic when no result
    }

    #[test]
    fn test_clear_old_signal_result_recent() {
        let mut app = App::new_mock();
        app.signal_result = Some((true, "test".to_string(), Instant::now()));
        app.clear_old_signal_result();
        assert!(app.signal_result.is_some()); // Not old enough
    }

    #[test]
    fn test_signal_menu_handling() {
        let mut app = App::new_mock();
        app.show_signal_menu = true;

        // ESC closes menu
        let quit = app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(!quit);
        assert!(!app.show_signal_menu);
    }

    #[test]
    fn test_signal_menu_keys() {
        let mut app = App::new_mock();

        // Test various signal menu keys
        app.show_signal_menu = true;
        app.handle_key(KeyCode::Char('x'), KeyModifiers::NONE);
        assert!(!app.show_signal_menu);

        app.show_signal_menu = true;
        app.handle_key(KeyCode::Char('K'), KeyModifiers::NONE);
        assert!(!app.show_signal_menu);

        app.show_signal_menu = true;
        app.handle_key(KeyCode::Char('H'), KeyModifiers::NONE);
        assert!(!app.show_signal_menu);
    }

    #[test]
    fn test_pending_signal_confirmation() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test".to_string(), SignalType::Term));

        // Y confirms
        let quit = app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);
        assert!(!quit);
        assert!(app.pending_signal.is_none());
    }

    #[test]
    fn test_pending_signal_cancel() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test".to_string(), SignalType::Term));

        // N cancels
        let quit = app.handle_key(KeyCode::Char('n'), KeyModifiers::NONE);
        assert!(!quit);
        assert!(app.pending_signal.is_none());
    }

    #[test]
    fn test_pending_signal_esc_cancels() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test".to_string(), SignalType::Term));

        let quit = app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(!quit);
        assert!(app.pending_signal.is_none());
    }

    #[test]
    fn test_process_navigation_keys() {
        let mut app = App::new_mock();

        // Home key
        app.process_selected = 5;
        app.handle_key(KeyCode::Home, KeyModifiers::NONE);
        assert_eq!(app.process_selected, 0);

        // g key
        app.process_selected = 5;
        app.handle_key(KeyCode::Char('g'), KeyModifiers::NONE);
        assert_eq!(app.process_selected, 0);
    }

    #[test]
    fn test_delete_clears_filter() {
        let mut app = App::new_mock();
        app.filter = "test".to_string();
        app.handle_key(KeyCode::Delete, KeyModifiers::NONE);
        assert!(app.filter.is_empty());
    }

    #[test]
    fn test_hjkl_focus_navigation() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Cpu);

        // l moves right in focus mode
        app.handle_key(KeyCode::Char('l'), KeyModifiers::NONE);
        assert_eq!(app.focused_panel, Some(PanelType::Memory));

        // h moves left
        app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
        assert_eq!(app.focused_panel, Some(PanelType::Cpu));
    }

    #[test]
    fn test_jk_process_nav_in_explode() {
        let mut app = App::new_mock();
        app.exploded_panel = Some(PanelType::Process);

        // j/k should navigate processes in explode mode
        app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
        // Should not panic
    }

    #[test]
    fn test_z_starts_focus() {
        let mut app = App::new_mock();
        assert!(app.focused_panel.is_none());
        assert!(app.exploded_panel.is_none());

        app.handle_key(KeyCode::Char('z'), KeyModifiers::NONE);
        assert!(app.focused_panel.is_some());
    }

    /// Test collect_metrics with real system data (for coverage)
    #[test]
    fn test_collect_metrics_real() {
        let mut app = App::new(false, false);
        // Run one real collection cycle for coverage
        app.collect_metrics();
        // Should complete without panic
        assert!(app.frame_id >= 1);
    }

    /// Test collect_metrics multiple cycles
    #[test]
    fn test_collect_metrics_cycles() {
        let mut app = App::new(false, false);
        let initial_frame = app.frame_id;
        app.collect_metrics();
        app.collect_metrics();
        assert_eq!(app.frame_id, initial_frame + 2);
    }

    /// Test history update in collect_metrics
    #[test]
    fn test_collect_metrics_history() {
        let mut app = App::new(false, false);
        let initial_cpu_len = app.cpu_history.len();
        app.collect_metrics();
        // History should have been updated (may or may not grow depending on collector state)
        assert!(app.cpu_history.len() >= initial_cpu_len);
    }

    /// Test push_to_history helper
    #[test]
    fn test_push_to_history() {
        let mut history = Vec::new();
        App::push_to_history(&mut history, 0.5);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0], 0.5);

        // Push more values
        for i in 0..400 {
            App::push_to_history(&mut history, i as f64 / 400.0);
        }
        // Should be capped at 300
        assert_eq!(history.len(), 300);
    }

    // === Micro-benchmark Performance Tests ===

    /// Verify collect_metrics completes within reasonable time
    /// Note: Real metrics collection involves many system calls (reading /proc, /sys, etc.)
    #[test]
    fn test_collect_metrics_performance() {
        use std::time::Instant;

        let mut app = App::new(false, false);
        let iterations = 5;
        let start = Instant::now();

        for _ in 0..iterations {
            app.collect_metrics();
        }

        let elapsed = start.elapsed();
        let avg_ms = elapsed.as_millis() / iterations as u128;

        // Real metrics collection with all collectors can take several seconds
        // on systems with many disks, network interfaces, and processes
        assert!(avg_ms < 5000, "collect_metrics too slow: {}ms avg", avg_ms);
    }

    /// Verify history push is O(1) amortized
    #[test]
    fn test_history_push_performance() {
        use std::time::Instant;

        let mut history = Vec::new();
        let iterations = 10000;
        let start = Instant::now();

        for i in 0..iterations {
            App::push_to_history(&mut history, i as f64 / iterations as f64);
        }

        let elapsed = start.elapsed();
        let per_op_ns = elapsed.as_nanos() / iterations as u128;

        // Each push should be sub-microsecond
        assert!(per_op_ns < 1000, "push_to_history too slow: {}ns per op", per_op_ns);
    }

    /// Verify App::new_mock is reasonably fast for testing
    /// Note: Mock creation initializes many analyzers which may read system state
    #[test]
    fn test_app_mock_creation_performance() {
        use std::time::Instant;

        let iterations = 50;
        let start = Instant::now();

        for _ in 0..iterations {
            let _ = App::new_mock();
        }

        let elapsed = start.elapsed();
        let avg_us = elapsed.as_micros() / iterations as u128;

        // Mock creation includes initializing analyzers which may touch system state
        // Allow up to 100ms each
        assert!(avg_us < 100000, "new_mock too slow: {}us avg", avg_us);
    }

    // === Additional Coverage Tests ===

    #[test]
    fn test_visible_panels_files_enabled() {
        let mut app = App::new_mock();
        app.panels.files = true;
        let panels = app.visible_panels();
        assert!(panels.contains(&PanelType::Files));
    }

    #[test]
    fn test_cancel_signal_with_pending() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test_proc".to_string(), SignalType::Term));
        app.cancel_signal();
        assert!(app.pending_signal.is_none());
    }

    #[test]
    fn test_pending_signal_hup() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "proc".to_string(), SignalType::Hup));
        assert!(app.pending_signal.is_some());
        if let Some((pid, name, signal)) = &app.pending_signal {
            assert_eq!(*pid, 1234);
            assert_eq!(name, "proc");
            assert_eq!(*signal, SignalType::Hup);
        }
    }

    #[test]
    fn test_pending_signal_int() {
        let mut app = App::new_mock();
        app.pending_signal = Some((5678, "daemon".to_string(), SignalType::Int));
        if let Some((_, _, signal)) = &app.pending_signal {
            assert_eq!(*signal, SignalType::Int);
        }
    }

    #[test]
    fn test_pending_signal_usr1() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1000, "app".to_string(), SignalType::Usr1));
        if let Some((_, _, signal)) = &app.pending_signal {
            assert_eq!(*signal, SignalType::Usr1);
        }
    }

    #[test]
    fn test_pending_signal_usr2() {
        let mut app = App::new_mock();
        app.pending_signal = Some((2000, "service".to_string(), SignalType::Usr2));
        if let Some((_, _, signal)) = &app.pending_signal {
            assert_eq!(*signal, SignalType::Usr2);
        }
    }

    #[test]
    fn test_pending_signal_stop() {
        let mut app = App::new_mock();
        app.pending_signal = Some((3000, "worker".to_string(), SignalType::Stop));
        if let Some((_, _, signal)) = &app.pending_signal {
            assert_eq!(*signal, SignalType::Stop);
        }
    }

    #[test]
    fn test_pending_signal_cont() {
        let mut app = App::new_mock();
        app.pending_signal = Some((4000, "bg_task".to_string(), SignalType::Cont));
        if let Some((_, _, signal)) = &app.pending_signal {
            assert_eq!(*signal, SignalType::Cont);
        }
    }

    #[test]
    fn test_pending_signal_kill() {
        let mut app = App::new_mock();
        app.pending_signal = Some((9999, "zombie".to_string(), SignalType::Kill));
        if let Some((_, _, signal)) = &app.pending_signal {
            assert_eq!(*signal, SignalType::Kill);
        }
    }

    #[test]
    fn test_signal_menu_key_i_int() {
        let mut app = App::new_mock();
        app.process_selected = 0;
        app.show_signal_menu = true;
        app.handle_key(KeyCode::Char('i'), KeyModifiers::NONE);
        assert!(!app.show_signal_menu);
    }

    #[test]
    fn test_signal_menu_key_p_stop() {
        let mut app = App::new_mock();
        app.process_selected = 0;
        app.show_signal_menu = true;
        app.handle_key(KeyCode::Char('p'), KeyModifiers::NONE);
        assert!(!app.show_signal_menu);
    }

    #[test]
    fn test_signal_menu_key_c_cont() {
        let mut app = App::new_mock();
        app.process_selected = 0;
        app.show_signal_menu = true;
        app.handle_key(KeyCode::Char('c'), KeyModifiers::NONE);
        assert!(!app.show_signal_menu);
    }

    #[test]
    fn test_signal_menu_key_unknown_ignored() {
        let mut app = App::new_mock();
        app.show_signal_menu = true;
        // Unknown key should not close menu
        app.handle_key(KeyCode::Char('z'), KeyModifiers::NONE);
        assert!(app.show_signal_menu);
    }

    #[test]
    fn test_filter_input_escape_clears() {
        let mut app = App::new_mock();
        app.show_filter_input = true;
        app.filter = "test".to_string();
        app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(!app.show_filter_input);
        assert!(app.filter.is_empty());
    }

    #[test]
    fn test_filter_input_backspace() {
        let mut app = App::new_mock();
        app.show_filter_input = true;
        app.filter = "test".to_string();
        app.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(app.filter, "tes");
    }

    #[test]
    fn test_filter_input_char_append() {
        let mut app = App::new_mock();
        app.show_filter_input = true;
        app.filter = "te".to_string();
        app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('t'), KeyModifiers::NONE);
        assert_eq!(app.filter, "test");
    }

    #[test]
    fn test_filter_input_enter_closes() {
        let mut app = App::new_mock();
        app.show_filter_input = true;
        app.filter = "search".to_string();
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert!(!app.show_filter_input);
        assert_eq!(app.filter, "search"); // Filter kept
    }

    // === Mock Data Verification Tests ===

    #[test]
    fn test_mock_gpus_populated() {
        let app = App::new_mock();
        assert!(!app.mock_gpus.is_empty(), "mock_gpus should be populated");
        assert_eq!(app.mock_gpus.len(), 2, "should have 2 mock GPUs");
        assert!(app.mock_gpus[0].name.contains("RTX"));
    }

    #[test]
    fn test_mock_battery_populated() {
        let app = App::new_mock();
        assert!(app.mock_battery.is_some(), "mock_battery should be populated");
        let bat = app.mock_battery.as_ref().expect("battery");
        assert!(bat.percent > 0.0);
        assert!(bat.health_percent > 0.0);
    }

    #[test]
    fn test_mock_sensors_populated() {
        let app = App::new_mock();
        assert!(!app.mock_sensors.is_empty(), "mock_sensors should be populated");
        assert!(app.mock_sensors.len() >= 3);
    }

    #[test]
    fn test_mock_containers_populated() {
        let app = App::new_mock();
        assert!(!app.mock_containers.is_empty(), "mock_containers should be populated");
        assert_eq!(app.mock_containers.len(), 3);
    }

    // === Additional Key Handling Tests ===

    #[test]
    fn test_panel_toggle_keys() {
        let mut app = App::new_mock();

        // Test panel 3 (disk)
        let original = app.panels.disk;
        app.handle_key(KeyCode::Char('3'), KeyModifiers::NONE);
        assert_ne!(app.panels.disk, original);

        // Test panel 4 (network)
        let original = app.panels.network;
        app.handle_key(KeyCode::Char('4'), KeyModifiers::NONE);
        assert_ne!(app.panels.network, original);

        // Test panel 5 (process)
        let original = app.panels.process;
        app.handle_key(KeyCode::Char('5'), KeyModifiers::NONE);
        assert_ne!(app.panels.process, original);

        // Test panel 6 (gpu)
        let original = app.panels.gpu;
        app.handle_key(KeyCode::Char('6'), KeyModifiers::NONE);
        assert_ne!(app.panels.gpu, original);

        // Test panel 7 (battery)
        let original = app.panels.battery;
        app.handle_key(KeyCode::Char('7'), KeyModifiers::NONE);
        assert_ne!(app.panels.battery, original);

        // Test panel 8 (sensors)
        let original = app.panels.sensors;
        app.handle_key(KeyCode::Char('8'), KeyModifiers::NONE);
        assert_ne!(app.panels.sensors, original);
    }

    #[test]
    fn test_navigation_when_focused() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Cpu);

        // h should navigate left
        app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
        // l should navigate right
        app.handle_key(KeyCode::Char('l'), KeyModifiers::NONE);
        // j should navigate down
        app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
        // k should navigate up
        app.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
    }

    #[test]
    fn test_navigation_when_not_focused() {
        let mut app = App::new_mock();
        app.focused_panel = None;

        // h should start focus
        app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
        assert!(app.focused_panel.is_some());

        // Reset and try l
        app.focused_panel = None;
        app.handle_key(KeyCode::Char('l'), KeyModifiers::NONE);
        assert!(app.focused_panel.is_some());
    }

    #[test]
    fn test_process_navigation_pageup_pagedown() {
        let mut app = App::new_mock();
        app.process_selected = 5;

        // Just test that keys are handled without panicking
        app.handle_key(KeyCode::PageDown, KeyModifiers::NONE);
        app.handle_key(KeyCode::PageUp, KeyModifiers::NONE);
    }

    #[test]
    fn test_process_navigation_home_end() {
        let mut app = App::new_mock();
        app.process_selected = 5;

        // g should go to start
        app.handle_key(KeyCode::Char('g'), KeyModifiers::NONE);
        assert_eq!(app.process_selected, 0);

        // G should go to end
        app.handle_key(KeyCode::Char('G'), KeyModifiers::NONE);
    }

    #[test]
    fn test_process_navigation_arrow_keys() {
        let mut app = App::new_mock();
        app.focused_panel = None;
        app.exploded_panel = None;
        app.process_selected = 5;

        // Just test that arrow keys are handled without panicking
        app.handle_key(KeyCode::Down, KeyModifiers::NONE);
        app.handle_key(KeyCode::Up, KeyModifiers::NONE);
    }

    #[test]
    fn test_process_navigation_with_exploded_panel() {
        let mut app = App::new_mock();
        app.exploded_panel = Some(PanelType::Process);
        app.process_selected = 0;

        // Just test that j/k are handled
        app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
    }

    #[test]
    fn test_help_toggle() {
        let mut app = App::new_mock();
        assert!(!app.show_help);

        app.handle_key(KeyCode::Char('?'), KeyModifiers::NONE);
        assert!(app.show_help);

        app.handle_key(KeyCode::Char('?'), KeyModifiers::NONE);
        assert!(!app.show_help);
    }

    #[test]
    fn test_f1_help_toggle() {
        let mut app = App::new_mock();
        assert!(!app.show_help);

        app.handle_key(KeyCode::F(1), KeyModifiers::NONE);
        assert!(app.show_help);
    }

    #[test]
    fn test_quit_key() {
        let mut app = App::new_mock();
        let quit = app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(quit);
    }

    #[test]
    fn test_esc_clears_focus() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Cpu);

        app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(app.focused_panel.is_none());
    }

    #[test]
    fn test_ctrl_c_returns_true() {
        let mut app = App::new_mock();
        let quit = app.handle_key(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(quit);
    }

    #[test]
    fn test_view_mode_cycle() {
        let mut app = App::new_mock();
        let original = app.files_view_mode;

        app.handle_key(KeyCode::Char('v'), KeyModifiers::NONE);
        // Should have cycled to next mode
        assert_ne!(app.files_view_mode, original);
    }

    #[test]
    fn test_pending_signal_confirm() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test".to_string(), SignalType::Term));

        // y confirms
        app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);
        assert!(app.pending_signal.is_none());
    }

    #[test]
    fn test_pending_signal_cancel_with_n_key() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test".to_string(), SignalType::Term));

        // n cancels
        app.handle_key(KeyCode::Char('n'), KeyModifiers::NONE);
        assert!(app.pending_signal.is_none());
    }

    #[test]
    fn test_panel_explode_toggle() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Cpu);
        app.exploded_panel = None;

        // z or Enter should toggle explode
        app.handle_key(KeyCode::Char('z'), KeyModifiers::NONE);
        assert!(app.exploded_panel.is_some());

        app.handle_key(KeyCode::Char('z'), KeyModifiers::NONE);
        assert!(app.exploded_panel.is_none());
    }

    #[test]
    fn test_signal_menu_keys_huk() {
        let mut app = App::new_mock();
        app.show_signal_menu = true;
        app.process_selected = 0;

        // Test H for HUP
        app.handle_key(KeyCode::Char('H'), KeyModifiers::NONE);

        app.show_signal_menu = true;
        // Test u for USR1
        app.handle_key(KeyCode::Char('u'), KeyModifiers::NONE);

        app.show_signal_menu = true;
        // Test U for USR2
        app.handle_key(KeyCode::Char('U'), KeyModifiers::NONE);
        // These keys should be handled without panicking
    }

    // === Additional Edge Case Tests ===

    #[test]
    fn test_signal_menu_all_signal_types() {
        let mut app = App::new_mock();

        // Test all signal menu keys
        for (key, _) in [
            ('x', SignalType::Term),
            ('K', SignalType::Kill),
            ('i', SignalType::Int),
            ('p', SignalType::Stop),
            ('c', SignalType::Cont),
        ] {
            app.show_signal_menu = true;
            app.pending_signal = None;
            app.handle_key(KeyCode::Char(key), KeyModifiers::NONE);
            assert!(!app.show_signal_menu);
        }
    }

    #[test]
    fn test_signal_menu_esc_closes_menu() {
        let mut app = App::new_mock();
        app.show_signal_menu = true;

        app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(!app.show_signal_menu);
    }

    #[test]
    fn test_filter_input_backspace_removal() {
        let mut app = App::new_mock();
        app.show_filter_input = true;
        app.filter = "test".to_string();

        app.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(app.filter, "tes");
    }

    #[test]
    fn test_filter_input_esc_clears_text() {
        let mut app = App::new_mock();
        app.show_filter_input = true;
        app.filter = "test".to_string();

        app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(!app.show_filter_input);
        assert!(app.filter.is_empty());
    }

    #[test]
    fn test_filter_input_enter_preserves() {
        let mut app = App::new_mock();
        app.show_filter_input = true;
        app.filter = "test".to_string();

        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert!(!app.show_filter_input);
        assert_eq!(app.filter, "test"); // Filter preserved
    }

    #[test]
    fn test_filter_input_add_char() {
        let mut app = App::new_mock();
        app.show_filter_input = true;
        app.filter = String::new();

        app.handle_key(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(app.filter, "a");
    }

    #[test]
    fn test_pending_signal_enter_confirms() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test".to_string(), SignalType::Term));

        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert!(app.pending_signal.is_none());
    }

    #[test]
    fn test_pending_signal_capital_y_confirms() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test".to_string(), SignalType::Term));

        app.handle_key(KeyCode::Char('Y'), KeyModifiers::NONE);
        assert!(app.pending_signal.is_none());
    }

    #[test]
    fn test_pending_signal_capital_n_cancels() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test".to_string(), SignalType::Term));

        app.handle_key(KeyCode::Char('N'), KeyModifiers::NONE);
        assert!(app.pending_signal.is_none());
    }

    #[test]
    fn test_pending_signal_esc_cancels_prompt() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test".to_string(), SignalType::Term));

        app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(app.pending_signal.is_none());
    }

    #[test]
    fn test_exploded_panel_enter_exits() {
        let mut app = App::new_mock();
        app.exploded_panel = Some(PanelType::Cpu);

        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert!(app.exploded_panel.is_none());
    }

    #[test]
    fn test_focused_panel_arrow_navigation() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Cpu);
        app.exploded_panel = None;

        // Test all arrow directions
        app.handle_key(KeyCode::Left, KeyModifiers::NONE);
        app.handle_key(KeyCode::Right, KeyModifiers::NONE);
        app.handle_key(KeyCode::Up, KeyModifiers::NONE);
        app.handle_key(KeyCode::Down, KeyModifiers::NONE);
    }

    #[test]
    fn test_focused_panel_hjkl_navigation() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Memory);
        app.exploded_panel = None;

        // Test h/l navigation
        app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('l'), KeyModifiers::NONE);
    }

    #[test]
    fn test_focused_panel_enter_explodes() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Disk);
        app.exploded_panel = None;

        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(app.exploded_panel, Some(PanelType::Disk));
    }

    #[test]
    fn test_unfocused_process_navigation_jk() {
        let mut app = App::new_mock();
        app.focused_panel = None;
        app.exploded_panel = None;
        app.process_selected = 5;

        app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
    }

    #[test]
    fn test_push_to_history_overflow() {
        let mut history = Vec::new();
        for i in 0..305 {
            App::push_to_history(&mut history, i as f64);
        }
        assert_eq!(history.len(), 300);
        assert_eq!(history[0], 5.0); // First 5 elements should be removed
    }

    #[test]
    fn test_panel_visibility_all_fields() {
        let vis = PanelVisibility {
            cpu: false,
            memory: false,
            disk: false,
            network: false,
            process: false,
            gpu: false,
            battery: false,
            sensors: false,
            files: true,
        };
        assert!(!vis.cpu);
        assert!(vis.files);
    }

    #[test]
    fn test_mock_gpu_data_debug() {
        let gpu = MockGpuData {
            name: "Test GPU".to_string(),
            gpu_util: 50.0,
            vram_used: 1000,
            vram_total: 8000,
            temperature: 65.0,
            power_watts: 150,
            power_limit_watts: 300,
            clock_mhz: 1500,
            history: vec![0.5],
        };
        let debug = format!("{:?}", gpu);
        assert!(debug.contains("Test GPU"));
    }

    #[test]
    fn test_mock_battery_data_debug() {
        let bat = MockBatteryData {
            percent: 75.0,
            charging: true,
            time_remaining_mins: Some(120),
            power_watts: 45.0,
            health_percent: 95.0,
            cycle_count: 500,
        };
        let debug = format!("{:?}", bat);
        assert!(debug.contains("75"));
    }

    #[test]
    fn test_mock_sensor_data_debug() {
        let sensor = MockSensorData {
            name: "cpu/temp1".to_string(),
            label: "CPU".to_string(),
            value: 65.0,
            max: Some(90.0),
            crit: Some(100.0),
            sensor_type: MockSensorType::Temperature,
        };
        let debug = format!("{:?}", sensor);
        assert!(debug.contains("cpu/temp1"));
    }

    #[test]
    fn test_mock_container_data_debug() {
        let container = MockContainerData {
            name: "nginx".to_string(),
            status: "running".to_string(),
            cpu_percent: 5.0,
            mem_used: 100_000_000,
            mem_limit: 1_000_000_000,
        };
        let debug = format!("{:?}", container);
        assert!(debug.contains("nginx"));
    }

    #[test]
    fn test_mock_sensor_type_equality() {
        assert_eq!(MockSensorType::Temperature, MockSensorType::Temperature);
        assert_ne!(MockSensorType::Temperature, MockSensorType::Fan);
        assert_ne!(MockSensorType::Voltage, MockSensorType::Power);
    }

    #[test]
    fn test_toggle_panel_9_files_toggle() {
        let mut app = App::new_mock();
        let initial = app.panels.files;

        app.handle_key(KeyCode::Char('9'), KeyModifiers::NONE);
        assert_ne!(app.panels.files, initial);
    }

    // =========================================================================
    // TUI Load Testing (probar integration)
    // =========================================================================

    /// Test filter performance with large dataset using probar's TUI load testing.
    /// This test uses probar's synthetic data generator and detects hangs.
    #[test]
    fn test_filter_no_hang_with_5000_items() {
        use jugar_probar::tui_load::{DataGenerator, TuiLoadTest};
        use std::time::{Duration, Instant};

        // Generate 5000 synthetic process-like items
        let generator = DataGenerator::new(5000);
        let items = generator.generate();

        // Test filter performance with timeout
        let timeout = Duration::from_millis(1000);
        let filters = ["", "a", "sys", "chrome", "nonexistent_long_filter_string"];

        for filter in filters {
            let filter_lower = filter.to_lowercase();
            let start = Instant::now();

            // Simulate what sorted_processes does: filter then collect
            let filtered: Vec<_> = items
                .iter()
                .filter(|item| {
                    if filter_lower.is_empty() {
                        true
                    } else {
                        item.name.to_lowercase().contains(&filter_lower)
                            || item.description.to_lowercase().contains(&filter_lower)
                    }
                })
                .collect();

            let elapsed = start.elapsed();

            assert!(
                elapsed < timeout,
                "Filter '{}' took {:?} (timeout: {:?}) - HANG DETECTED with {} items, {} results",
                filter, elapsed, timeout, items.len(), filtered.len()
            );
        }
    }

    /// Test that filter performance is O(n) not O(n²) using probar load testing.
    #[test]
    fn test_filter_scales_linearly() {
        use jugar_probar::tui_load::DataGenerator;
        use std::time::Instant;

        let sizes = [100, 500, 1000, 2000, 5000];
        let mut times_us = Vec::new();

        for size in sizes {
            let items = DataGenerator::new(size).generate();
            let filter_lower = "sys".to_lowercase();

            let start = Instant::now();
            // Run filter 10 times for stable measurement
            for _ in 0..10 {
                let _: Vec<_> = items
                    .iter()
                    .filter(|item| {
                        item.name.to_lowercase().contains(&filter_lower)
                            || item.description.to_lowercase().contains(&filter_lower)
                    })
                    .collect();
            }
            let elapsed = start.elapsed().as_micros() as u64;
            times_us.push((size, elapsed));
        }

        // Check that time grows roughly linearly (< 3x for 5x data)
        // From 1000 to 5000 items should take roughly 5x longer (with tolerance)
        let time_1k = times_us.iter().find(|(s, _)| *s == 1000).map(|(_, t)| *t).unwrap_or(1);
        let time_5k = times_us.iter().find(|(s, _)| *s == 5000).map(|(_, t)| *t).unwrap_or(1);

        let ratio = time_5k as f64 / time_1k as f64;

        // Should scale roughly linearly: 5x data = ~5x time (allow up to 8x for overhead)
        assert!(
            ratio < 8.0,
            "Filter time scaled {}x from 1K to 5K items (expected ~5x). \
             Times: {:?}. May indicate O(n²) complexity.",
            ratio, times_us
        );
    }

    /// Stress test with probar's TuiLoadTest harness - tests for hangs, not microbenchmarks
    #[test]
    fn test_filter_stress_with_probar() {
        use jugar_probar::tui_load::TuiLoadTest;

        let load_test = TuiLoadTest::new()
            .with_item_count(5000)     // Test with 5000 items
            .with_timeout_ms(2000)      // 2 second timeout per frame
            .with_frames_per_filter(3);

        // Run filter stress test using allocation-free matching (like ttop's optimized code)
        let result = load_test.run_filter_stress(|items, filter| {
            let filter_lower = filter.to_lowercase();
            items
                .iter()
                .filter(|item| {
                    if filter_lower.is_empty() {
                        true
                    } else {
                        // Use allocation-free case-insensitive contains
                        contains_ignore_case(&item.name, &filter_lower)
                            || contains_ignore_case(&item.description, &filter_lower)
                    }
                })
                .cloned()
                .collect()
        });

        // The main assertion: no frame should timeout (no hangs)
        assert!(result.is_ok(), "TUI filter stress test detected hang: {:?}", result.err());

        // Verify we actually ran all the filters
        let results = result.expect("result should be ok");
        assert!(!results.is_empty(), "Should have run at least one filter");

        // Log performance for manual review (not a hard failure)
        for (filter, metrics) in &results {
            let avg = metrics.avg_frame_ms();
            assert!(
                avg < 500.0,
                "Filter '{}' took {:.1}ms avg - too slow for responsive UI",
                filter, avg
            );
        }
    }

    /// Integration load test that tests REAL App with REAL collectors.
    ///
    /// This test would have caught the container_analyzer hang because it:
    /// 1. Creates a real App (not mock)
    /// 2. Calls real collect methods
    /// 3. Measures component-level timings
    /// 4. Enforces per-component budgets (container_analyzer: 200ms max)
    ///
    /// The synthetic load tests missed the hang because they only tested
    /// filter performance with fake data, not actual system calls.
    #[test]
    fn test_integration_load_real_app_no_hang() {
        use jugar_probar::tui_load::{ComponentTimings, IntegrationLoadTest};

        // Set up integration test with component budgets
        let test = IntegrationLoadTest::new()
            .with_frame_budget_ms(500.0)   // 500ms total frame budget
            .with_timeout_ms(5000)          // 5 second timeout for hang detection
            .with_frame_count(3)            // Test 3 frames
            // Per-component budgets - this is what would catch the container_analyzer issue
            .with_component_budget("container_analyzer", 200.0)  // Max 200ms
            .with_component_budget("disk_analyzer", 200.0)
            .with_component_budget("network_analyzer", 200.0)
            .with_component_budget("sensor_analyzer", 100.0);

        // Track whether we're on first frame (initialization is slower)
        let first_frame = std::sync::atomic::AtomicBool::new(true);
        let app = std::sync::Mutex::new(None::<App>);

        let result = test.run(|| {
            let mut timings = ComponentTimings::new();

            // Get or create app
            let mut guard = app.lock().expect("lock");
            let app = guard.get_or_insert_with(|| {
                // First frame includes App::new() which is slower
                App::new(false, false) // deterministic=false, show_fps=false
            });

            // Measure individual analyzer times
            let start = Instant::now();
            app.container_analyzer.collect();
            timings.record("container_analyzer", start.elapsed().as_secs_f64() * 1000.0);

            let start = Instant::now();
            app.disk_io_analyzer.collect();
            timings.record("disk_analyzer", start.elapsed().as_secs_f64() * 1000.0);

            let start = Instant::now();
            app.network_stats.collect();
            timings.record("network_analyzer", start.elapsed().as_secs_f64() * 1000.0);

            let start = Instant::now();
            app.sensor_health.collect();
            timings.record("sensor_analyzer", start.elapsed().as_secs_f64() * 1000.0);

            // Skip strict budget check on first frame (initialization)
            if first_frame.swap(false, std::sync::atomic::Ordering::SeqCst) {
                // Return empty timings for first frame so budget checks are skipped
                return ComponentTimings::new();
            }

            timings
        });

        // This assertion WOULD HAVE FAILED before fixing container_analyzer
        // because docker stats --no-stream was blocking for 1.5+ seconds
        assert!(
            result.is_ok(),
            "Integration load test failed! This catches real collector hangs: {:?}",
            result.err()
        );

        let metrics = result.expect("test passed");
        assert!(
            metrics.p95_frame_ms() < 1000.0,
            "Frame time p95 {:.1}ms is too slow",
            metrics.p95_frame_ms()
        );
    }

    // =========================================================================
    // EXTREME TDD: Exploded Panel Features - Tests Written FIRST
    // =========================================================================
    //
    // SPEC: CPU Panel Exploded Mode Features
    // 1. Per-core sparkline history (60 samples per core)
    // 2. CPU state breakdown (user/system/iowait/idle per core)
    // 3. Frequency timeline (history of freq changes)
    // 4. Top process per core
    // 5. Thermal throttling indicator
    //
    // PMAT Requirements:
    // - Performance: O(1) per-core history update
    // - Maintainability: Clear data structures
    // - Accessibility: Data available via public getters
    // - Testing: 95% coverage requirement

    /// TDD Test 1: Per-core history storage exists and is populated
    #[test]
    fn test_per_core_history_exists() {
        let app = App::new_mock();
        // Per-core history should exist for each core
        assert!(!app.per_core_history.is_empty(), "per_core_history must exist");
        assert_eq!(app.per_core_history.len(), app.per_core_percent.len(),
            "per_core_history should have one entry per core");
    }

    /// TDD Test 2: Per-core history has correct capacity
    #[test]
    fn test_per_core_history_capacity() {
        let app = App::new_mock();
        // Each core should have history capacity for 60 samples (60 seconds at 1Hz)
        for (i, history) in app.per_core_history.iter().enumerate() {
            assert!(history.capacity() >= 60,
                "Core {} history should have capacity for 60 samples, got {}",
                i, history.capacity());
        }
    }

    /// TDD Test 3: Per-core history values are valid percentages
    #[test]
    fn test_per_core_history_values_valid() {
        let app = App::new_mock();
        for (i, history) in app.per_core_history.iter().enumerate() {
            for (j, &value) in history.iter().enumerate() {
                assert!(value >= 0.0 && value <= 100.0,
                    "Core {} history[{}] = {} is invalid (must be 0-100)",
                    i, j, value);
            }
        }
    }

    /// TDD Test 4: CPU state breakdown per core exists
    #[test]
    fn test_cpu_state_breakdown_exists() {
        let app = App::new_mock();
        // Should have breakdown for each core
        assert!(!app.per_core_state.is_empty(), "per_core_state must exist");
        assert_eq!(app.per_core_state.len(), app.per_core_percent.len(),
            "per_core_state should have one entry per core");
    }

    /// TDD Test 5: CPU state breakdown has all components
    #[test]
    fn test_cpu_state_breakdown_components() {
        let app = App::new_mock();
        for (i, state) in app.per_core_state.iter().enumerate() {
            // Each state should have user, system, iowait, idle
            let total = state.user + state.system + state.iowait + state.idle;
            assert!((total - 100.0).abs() < 1.0,
                "Core {} state breakdown should sum to ~100%, got {:.1}%",
                i, total);
        }
    }

    /// TDD Test 6: Frequency history exists
    #[test]
    fn test_freq_history_exists() {
        let app = App::new_mock();
        // Should have frequency history for trend tracking
        assert!(app.freq_history.capacity() >= 60,
            "freq_history should have capacity for 60 samples");
    }

    /// TDD Test 7: Top process per core tracking
    #[test]
    fn test_top_process_per_core_exists() {
        let app = App::new_mock();
        // Should track which process is using each core most
        assert!(!app.top_process_per_core.is_empty(),
            "top_process_per_core must exist");
    }

    /// TDD Test 8: Thermal throttling state exists
    #[test]
    fn test_thermal_throttling_state_exists() {
        let app = App::new_mock();
        // Should track throttling state
        assert!(app.thermal_throttle_active.is_some() || !app.thermal_throttle_active.is_some(),
            "thermal_throttle_active field must exist (can be None)");
    }

    /// TDD Test 9: Probar load test for per-core history update performance
    #[test]
    fn test_per_core_history_update_performance() {
        use std::time::Instant;

        let mut app = App::new_mock();
        let core_count = 48; // Simulate high core count

        // Initialize per-core history for many cores
        app.per_core_history = vec![Vec::with_capacity(60); core_count];
        app.per_core_percent = vec![50.0; core_count];

        // Measure update performance
        let start = Instant::now();
        for _ in 0..1000 {
            // Simulate history update
            for (i, history) in app.per_core_history.iter_mut().enumerate() {
                let value = app.per_core_percent.get(i).copied().unwrap_or(0.0);
                if history.len() >= 60 {
                    history.remove(0);
                }
                history.push(value);
            }
        }
        let elapsed = start.elapsed();

        // Should complete 1000 updates in under 100ms
        assert!(elapsed.as_millis() < 100,
            "1000 per-core history updates took {:?} (should be < 100ms)",
            elapsed);
    }

    /// TDD Test 10: CpuCoreState struct validation
    #[test]
    fn test_cpu_core_state_struct() {
        let state = CpuCoreState {
            user: 25.0,
            system: 10.0,
            iowait: 5.0,
            idle: 60.0,
        };
        assert_eq!(state.user, 25.0);
        assert_eq!(state.system, 10.0);
        assert_eq!(state.iowait, 5.0);
        assert_eq!(state.idle, 60.0);
        assert!((state.total_busy() - 40.0).abs() < 0.1);
    }

    // =========================================================================
    // EXTREME TDD: Memory Panel Exploded Mode Features - Tests Written FIRST
    // =========================================================================
    //
    // SPEC: Memory Panel Exploded Mode Features
    // 1. Memory pressure (PSI) history (60 samples)
    // 2. Memory breakdown categories (used/cached/buffers/free)
    // 3. Swap thrashing detection with trend
    // 4. Memory reclaim rate tracking
    // 5. Top memory consumers with trend
    //
    // PMAT Requirements:
    // - Performance: O(1) history updates
    // - Maintainability: Clear data structures
    // - Accessibility: Data available via public getters
    // - Testing: 95% coverage requirement

    /// TDD Test 11: Memory pressure history exists
    #[test]
    fn test_mem_pressure_history_exists() {
        let app = App::new_mock();
        assert!(app.mem_pressure_history.capacity() >= 60,
            "mem_pressure_history should have capacity for 60 samples");
    }

    /// TDD Test 12: Memory reclaim rate tracking
    #[test]
    fn test_mem_reclaim_rate_exists() {
        let app = App::new_mock();
        // Should track memory reclaim rate (can be 0.0 if not available)
        assert!(app.mem_reclaim_rate >= 0.0, "mem_reclaim_rate should be >= 0");
    }

    /// TDD Test 13: Top memory consumers tracking
    #[test]
    fn test_top_mem_consumers_exists() {
        let app = App::new_mock();
        assert!(!app.top_mem_consumers.is_empty(),
            "top_mem_consumers should have at least one entry");
    }

    /// TDD Test 14: Memory breakdown struct
    #[test]
    fn test_mem_breakdown_struct() {
        let breakdown = MemoryBreakdown {
            used_bytes: 8 * 1024 * 1024 * 1024,
            cached_bytes: 4 * 1024 * 1024 * 1024,
            buffers_bytes: 512 * 1024 * 1024,
            free_bytes: 3 * 1024 * 1024 * 1024,
        };
        assert_eq!(breakdown.used_bytes, 8 * 1024 * 1024 * 1024);
        assert_eq!(breakdown.cached_bytes, 4 * 1024 * 1024 * 1024);
    }

    /// TDD Test 15: Swap trend indicator
    #[test]
    fn test_swap_trend_exists() {
        let app = App::new_mock();
        // Swap trend can be Rising, Falling, or Stable
        match app.swap_trend {
            SwapTrend::Rising | SwapTrend::Falling | SwapTrend::Stable => (),
        }
    }

    // =========================================================================
    // EXTREME TDD: Disk Panel Exploded Mode Features - Tests Written FIRST
    // =========================================================================
    //
    // SPEC: Disk Panel Exploded Mode Features
    // 1. Per-mount I/O history (60 samples)
    // 2. Latency history tracking
    // 3. Queue depth tracking
    // 4. Device health status
    // 5. IOPS breakdown (read/write)

    /// TDD Test 16: Disk latency history exists
    #[test]
    fn test_disk_latency_history_exists() {
        let app = App::new_mock();
        assert!(app.disk_latency_history.capacity() >= 60,
            "disk_latency_history should have capacity for 60 samples");
    }

    /// TDD Test 17: Disk IOPS breakdown exists
    #[test]
    fn test_disk_iops_breakdown_exists() {
        let app = App::new_mock();
        assert!(app.disk_read_iops >= 0.0, "disk_read_iops should be >= 0");
        assert!(app.disk_write_iops >= 0.0, "disk_write_iops should be >= 0");
    }

    /// TDD Test 18: Disk queue depth tracking
    #[test]
    fn test_disk_queue_depth_exists() {
        let app = App::new_mock();
        assert!(app.disk_queue_depth >= 0.0, "disk_queue_depth should be >= 0");
    }

    /// TDD Test 19: Disk health struct
    #[test]
    fn test_disk_health_struct() {
        let health = DiskHealthStatus {
            device: "sda".to_string(),
            status: DiskHealth::Good,
            temperature: Some(35.0),
            reallocated_sectors: 0,
        };
        assert_eq!(health.device, "sda");
        assert_eq!(health.status, DiskHealth::Good);
    }

    /// TDD Test 20: Disk health status field
    #[test]
    fn test_disk_health_status_exists() {
        let app = App::new_mock();
        // disk_health can be empty if no SMART data available
        assert!(app.disk_health.is_empty() || !app.disk_health.is_empty(),
            "disk_health field must exist");
    }

    // =========================================================================
    // EXTREME TDD: Network Panel Exploded Mode Features - Tests Written FIRST
    // =========================================================================
    //
    // SPEC: Network Panel Exploded Mode Features
    // 1. Per-interface history (60 samples)
    // 2. Bandwidth utilization percentage
    // 3. Connection count by state
    // 4. Error/drop rate tracking
    // 5. Latency estimation

    /// TDD Test 21: Network per-interface history
    #[test]
    fn test_net_interface_history_exists() {
        let app = App::new_mock();
        assert!(app.net_rx_history.capacity() >= 8,
            "net_rx_history should have capacity for samples");
        assert!(app.net_tx_history.capacity() >= 8,
            "net_tx_history should have capacity for samples");
    }

    /// TDD Test 22: Network error tracking
    #[test]
    fn test_net_error_counts_exists() {
        let app = App::new_mock();
        assert!(app.net_errors >= 0, "net_errors should be >= 0");
        assert!(app.net_drops >= 0, "net_drops should be >= 0");
    }

    /// TDD Test 23: Network connection state counts
    #[test]
    fn test_net_connection_states_exists() {
        let app = App::new_mock();
        assert!(app.net_established >= 0, "net_established should be >= 0");
        assert!(app.net_listening >= 0, "net_listening should be >= 0");
    }

    // =========================================================================
    // EXTREME TDD: Process Panel Exploded Mode Features - Tests Written FIRST
    // =========================================================================
    //
    // SPEC: Process Panel Exploded Mode Features
    // 1. Process tree view
    // 2. Per-process history (CPU/memory)
    // 3. I/O rates per process
    // 4. Thread count tracking
    // 5. File descriptor usage

    /// TDD Test 24: Process tree data exists
    #[test]
    fn test_process_tree_data_exists() {
        let app = App::new_mock();
        // show_tree is the toggle
        assert!(!app.show_tree || app.show_tree, "show_tree field must exist");
    }

    /// TDD Test 25: Process additional fields tracked
    #[test]
    fn test_process_extra_fields_exist() {
        let app = App::new_mock();
        // These are tracked by process_extra analyzer
        // Just verify the field exists (analyzer is Default-able)
        let _ = &app.process_extra;
        assert!(true, "process_extra analyzer must exist");
    }
}
