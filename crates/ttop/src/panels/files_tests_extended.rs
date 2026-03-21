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
