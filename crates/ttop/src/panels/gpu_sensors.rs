pub fn draw_gpu(f: &mut Frame, app: &App, area: Rect) {
    use trueno_viz::monitor::ratatui::style::Color;
    use trueno_viz::monitor::ratatui::text::{Line, Span};

    // Collect GPU info into a unified structure
    struct GpuDisplay {
        name: String,
        gpu_util: f64,
        vram_used: u64,
        vram_total: u64,
        vram_pct: f64,
        temp: f64,
        power: u32,
        power_limit: u32,
        clock_mhz: u32,
        history: Option<Vec<f64>>,
    }

    let mut gpus: Vec<GpuDisplay> = Vec::new();

    // Check for mock GPU data first (for testing)
    if !app.mock_gpus.is_empty() {
        for mock_gpu in &app.mock_gpus {
            let vram_pct = if mock_gpu.vram_total > 0 {
                mock_gpu.vram_used as f64 / mock_gpu.vram_total as f64
            } else {
                0.0
            };
            gpus.push(GpuDisplay {
                name: mock_gpu.name.clone(),
                gpu_util: mock_gpu.gpu_util,
                vram_used: mock_gpu.vram_used,
                vram_total: mock_gpu.vram_total,
                vram_pct,
                temp: mock_gpu.temperature,
                power: mock_gpu.power_watts,
                power_limit: mock_gpu.power_limit_watts,
                clock_mhz: mock_gpu.clock_mhz,
                history: Some(mock_gpu.history.clone()),
            });
        }
    }

    #[cfg(feature = "nvidia")]
    if gpus.is_empty() && app.nvidia_gpu.is_available() {
        for (i, gpu) in app.nvidia_gpu.gpus().iter().enumerate() {
            let vram_pct = if gpu.mem_total > 0 {
                gpu.mem_used as f64 / gpu.mem_total as f64
            } else {
                0.0
            };
            let history = app.nvidia_gpu.gpu_history(i).map(|h| {
                let (a, b) = h.as_slices();
                let mut v = a.to_vec();
                v.extend_from_slice(b);
                v
            });
            gpus.push(GpuDisplay {
                name: gpu.name.clone(),
                gpu_util: gpu.gpu_util,
                vram_used: gpu.mem_used,
                vram_total: gpu.mem_total,
                vram_pct,
                temp: gpu.temperature,
                power: gpu.power_mw / 1000,
                power_limit: gpu.power_limit_mw / 1000,
                clock_mhz: gpu.gpu_clock_mhz,
                history,
            });
        }
    }

    #[cfg(target_os = "linux")]
    if gpus.is_empty() && app.amd_gpu.is_available() {
        for (i, gpu) in app.amd_gpu.gpus().iter().enumerate() {
            let vram_pct = if gpu.vram_total > 0 {
                gpu.vram_used as f64 / gpu.vram_total as f64
            } else {
                0.0
            };
            let history = app.amd_gpu.gpu_history(i).map(|h| {
                let (a, b) = h.as_slices();
                let mut v = a.to_vec();
                v.extend_from_slice(b);
                v
            });
            gpus.push(GpuDisplay {
                name: gpu.name.clone(),
                gpu_util: gpu.gpu_util,
                vram_used: gpu.vram_used,
                vram_total: gpu.vram_total,
                vram_pct,
                temp: gpu.temperature,
                power: gpu.power_watts as u32,
                power_limit: if gpu.power_cap_watts > 0.0 { gpu.power_cap_watts as u32 } else { 300 },
                clock_mhz: gpu.gpu_clock_mhz as u32,
                history,
            });
        }
    }

    #[cfg(target_os = "macos")]
    if gpus.is_empty() && app.apple_gpu.is_available() {
        for (i, gpu) in app.apple_gpu.gpus().iter().enumerate() {
            let history = app.apple_gpu.util_history(i).map(|h| {
                let (a, b) = h.as_slices();
                let mut v = a.to_vec();
                v.extend_from_slice(b);
                v
            });
            gpus.push(GpuDisplay {
                name: gpu.name.clone(),
                gpu_util: gpu.gpu_util,
                vram_used: 0,
                vram_total: 0,
                vram_pct: 0.0, // Apple uses unified memory
                temp: 0.0,
                power: 0,
                power_limit: 0,
                clock_mhz: 0, // Apple doesn't expose clock via IOKit
                history,
            });
        }
    }

    // macOS fallback: detect AMD/Intel GPUs via system_profiler when Apple GPU collector fails
    #[cfg(target_os = "macos")]
    if gpus.is_empty() {
        if let Ok(output) = std::process::Command::new("system_profiler")
            .args(["SPDisplaysDataType", "-json"])
            .output()
        {
            if output.status.success() {
                if let Ok(json) = String::from_utf8(output.stdout) {
                    // Parse GPU names and VRAM from JSON output
                    for line in json.lines() {
                        let line = line.trim();
                        if line.contains("\"sppci_model\"") {
                            if let Some(name) = line.split(':').nth(1) {
                                let name = name.trim().trim_matches('"').trim_matches(',');
                                gpus.push(GpuDisplay {
                                    name: name.to_string(),
                                    gpu_util: 0.0,
                                    vram_used: 0,
                                    vram_total: 0,
                                    vram_pct: 0.0,
                                    temp: 0.0,
                                    power: 0,
                                    power_limit: 0,
                                    clock_mhz: 0,
                                    history: None,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // Build title showing GPU name and key stats
    let title = if gpus.len() > 1 {
        format!(" GPU ({} devices) ", gpus.len())
    } else if let Some(gpu) = gpus.first() {
        if gpu.temp > 0.0 {
            format!(" {} │ {}°C │ {}W ", gpu.name, gpu.temp as u32, gpu.power)
        } else {
            format!(" {} ", gpu.name)
        }
    } else {
        " GPU ".to_string()
    };

    let block = btop_block(&title, borders::GPU);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 2 || gpus.is_empty() {
        return;
    }

    let mut y = inner.y;

    // Reserve space for GPU processes at bottom
    let reserved_bottom = if app.gpu_process_analyzer.is_available() { 3u16 } else { 0 };
    let gpu_area_height = inner.height.saturating_sub(reserved_bottom);

    // Column layout: Label(5) | Bar(variable) | Value(10) | Sparkline(remaining)
    let label_col = 5u16;
    let value_col = 12u16;
    let sparkline_col = 20u16.min(inner.width / 4);
    let bar_width = inner.width.saturating_sub(label_col + value_col + sparkline_col + 2).max(10);

    for (i, gpu) in gpus.iter().enumerate() {
        if y >= inner.y + gpu_area_height {
            break;
        }

        // === ROW 1: GPU utilization with sparkline ===
        let label = if gpus.len() > 1 {
            format!("GPU{}", i)
        } else {
            "GPU".to_string()
        };

        let gpu_color = percent_color(gpu.gpu_util);
        let mut x = inner.x;
        let max_x = inner.x + inner.width;

        // Col 1: Label
        if x < max_x {
            let w = label_col.min(max_x.saturating_sub(x));
            f.render_widget(
                Paragraph::new(format!("{:<width$}", label, width = w as usize))
                    .style(Style::default().fg(Color::White)),
                Rect { x, y, width: w, height: 1 },
            );
            x += label_col;
        }

        // Col 2: Utilization bar
        if x < max_x {
            let w = bar_width.min(max_x.saturating_sub(x));
            let util_filled = ((gpu.gpu_util / 100.0) * w as f64) as usize;
            let util_empty = (w as usize).saturating_sub(util_filled);
            let bar_line = Line::from(vec![
                Span::styled("█".repeat(util_filled), Style::default().fg(gpu_color)),
                Span::styled("░".repeat(util_empty), Style::default().fg(Color::DarkGray)),
            ]);
            f.render_widget(
                Paragraph::new(bar_line),
                Rect { x, y, width: w, height: 1 },
            );
            x += bar_width + 1;
        }

        // Col 3: Percentage value
        if x < max_x {
            let w = value_col.min(max_x.saturating_sub(x));
            f.render_widget(
                Paragraph::new(format!("{:>5.1}%", gpu.gpu_util))
                    .style(Style::default().fg(gpu_color)),
                Rect { x, y, width: w, height: 1 },
            );
            x += value_col;
        }

        // Col 4: Sparkline (if history available)
        if x < max_x {
            if let Some(ref hist) = gpu.history {
                let w = sparkline_col.min(max_x.saturating_sub(x));
                if !hist.is_empty() && w > 3 {
                    let sparkline = MonitorSparkline::new(hist)
                        .color(gpu_color)
                        .show_trend(true);
                    f.render_widget(
                        sparkline,
                        Rect { x, y, width: w, height: 1 },
                    );
                }
            }
        }
        y += 1;

        // === ROW 2: VRAM bar (if available) ===
        if y < inner.y + gpu_area_height && gpu.vram_total > 0 {
            let vram_gb_used = gpu.vram_used as f64 / (1024.0 * 1024.0 * 1024.0);
            let vram_gb_total = gpu.vram_total as f64 / (1024.0 * 1024.0 * 1024.0);
            let vram_color = percent_color(gpu.vram_pct * 100.0);

            x = inner.x;

            // Col 1: Label
            if x < max_x {
                let w = label_col.min(max_x.saturating_sub(x));
                f.render_widget(
                    Paragraph::new(format!("{:<width$}", "VRAM", width = w as usize))
                        .style(Style::default().fg(Color::DarkGray)),
                    Rect { x, y, width: w, height: 1 },
                );
                x += label_col;
            }

            // Col 2: VRAM bar
            if x < max_x {
                let w = bar_width.min(max_x.saturating_sub(x));
                let vram_filled = ((gpu.vram_pct) * w as f64) as usize;
                let vram_empty = (w as usize).saturating_sub(vram_filled);
                let vram_bar = Line::from(vec![
                    Span::styled("█".repeat(vram_filled), Style::default().fg(vram_color)),
                    Span::styled("░".repeat(vram_empty), Style::default().fg(Color::DarkGray)),
                ]);
                f.render_widget(
                    Paragraph::new(vram_bar),
                    Rect { x, y, width: w, height: 1 },
                );
                x += bar_width + 1;
            }

            // Col 3: VRAM value
            if x < max_x {
                let w = (value_col + sparkline_col).min(max_x.saturating_sub(x));
                f.render_widget(
                    Paragraph::new(format!("{:.1}/{:.0}G", vram_gb_used, vram_gb_total))
                        .style(Style::default().fg(Color::White)),
                    Rect { x, y, width: w, height: 1 },
                );
            }
            y += 1;
        }

        // === ROW 3: Thermal + Power (if available) ===
        if y < inner.y + gpu_area_height && gpu.temp > 0.0 {
            let temp_color = temp_color(gpu.temp);
            let temp_pct = (gpu.temp / 100.0).min(1.0);

            // Power color
            let power_pct = if gpu.power_limit > 0 {
                (gpu.power as f64 / gpu.power_limit as f64 * 100.0).min(100.0)
            } else {
                0.0
            };
            let power_color = if power_pct > 90.0 {
                Color::Red
            } else if power_pct > 70.0 {
                Color::Yellow
            } else {
                Color::Green
            };

            x = inner.x;

            // Col 1: Label
            if x < max_x {
                let w = label_col.min(max_x.saturating_sub(x));
                f.render_widget(
                    Paragraph::new(format!("{:<width$}", "Temp", width = w as usize))
                        .style(Style::default().fg(Color::DarkGray)),
                    Rect { x, y, width: w, height: 1 },
                );
                x += label_col;
            }

            // Col 2: Temp bar (half width) + Power bar (half width)
            if x < max_x {
                let w = bar_width.min(max_x.saturating_sub(x));
                let half_bar = (w / 2) as usize;
                let temp_filled = (temp_pct * half_bar as f64) as usize;
                let temp_empty = half_bar.saturating_sub(temp_filled);

                let power_filled = ((power_pct / 100.0) * half_bar as f64) as usize;
                let power_empty = half_bar.saturating_sub(power_filled);

                let thermal_bar = Line::from(vec![
                    Span::styled("█".repeat(temp_filled), Style::default().fg(temp_color)),
                    Span::styled("░".repeat(temp_empty), Style::default().fg(Color::DarkGray)),
                    Span::styled("│", Style::default().fg(Color::DarkGray)),
                    Span::styled("█".repeat(power_filled), Style::default().fg(power_color)),
                    Span::styled("░".repeat(power_empty), Style::default().fg(Color::DarkGray)),
                ]);
                f.render_widget(
                    Paragraph::new(thermal_bar),
                    Rect { x, y, width: w, height: 1 },
                );
                x += bar_width + 1;
            }

            // Col 3: Temp + Power + Clock values
            if x < max_x {
                let values = if gpu.clock_mhz > 0 && gpu.power_limit > 0 {
                    format!(
                        "{}°C {:>3}W/{:>3}W {:>4}MHz",
                        gpu.temp as u32, gpu.power, gpu.power_limit, gpu.clock_mhz
                    )
                } else if gpu.power_limit > 0 {
                    format!("{}°C {:>3}W/{:>3}W", gpu.temp as u32, gpu.power, gpu.power_limit)
                } else if gpu.clock_mhz > 0 {
                    format!("{}°C {:>3}W {:>4}MHz", gpu.temp as u32, gpu.power, gpu.clock_mhz)
                } else {
                    format!("{}°C {:>3}W", gpu.temp as u32, gpu.power)
                };
                let w = (value_col + sparkline_col).min(max_x.saturating_sub(x));
                f.render_widget(
                    Paragraph::new(values).style(Style::default().fg(temp_color)),
                    Rect { x, y, width: w, height: 1 },
                );
            }
            y += 1;
        }

        // Add spacing between GPUs if multiple
        if gpus.len() > 1 && i < gpus.len() - 1 && y < inner.y + gpu_area_height {
            y += 1;
        }
    }

    // === GPU Processes Section (bottom) ===
    if y < inner.y + inner.height && app.gpu_process_analyzer.is_available() {
        let procs = app.gpu_process_analyzer.top_processes(3);
        if !procs.is_empty() {
            // Divider
            if y < inner.y + inner.height {
                let divider = "─".repeat(inner.width as usize);
                f.render_widget(
                    Paragraph::new(divider).style(Style::default().fg(Color::DarkGray)),
                    Rect { x: inner.x, y, width: inner.width, height: 1 },
                );
                y += 1;
            }

            // Show top GPU processes with enhanced display
            for proc in procs {
                if y >= inner.y + inner.height {
                    break;
                }

                // SM utilization color
                let sm_color = if proc.sm_util >= 50 {
                    Color::LightRed
                } else if proc.sm_util >= 20 {
                    Color::Yellow
                } else {
                    Color::Green
                };

                // Type badge color: Compute=Cyan, Graphics=Magenta
                let type_color = match proc.proc_type {
                    crate::analyzers::GpuProcType::Compute => Color::Cyan,
                    crate::analyzers::GpuProcType::Graphics => Color::Magenta,
                };

                // Memory bar (6 chars based on mem_util)
                let mem_bar_width = 6usize;
                let mem_filled = ((proc.mem_util as f64 / 100.0) * mem_bar_width as f64) as usize;
                let mem_empty = mem_bar_width.saturating_sub(mem_filled);
                let mem_color = if proc.mem_util >= 80 {
                    Color::Red
                } else if proc.mem_util >= 50 {
                    Color::Yellow
                } else {
                    Color::Green
                };

                // Encoder/Decoder indicators
                let enc_dec = if proc.enc_util > 0 && proc.dec_util > 0 {
                    format!("[E{}D{}]", proc.enc_util, proc.dec_util)
                } else if proc.enc_util > 0 {
                    format!("[E{}]", proc.enc_util)
                } else if proc.dec_util > 0 {
                    format!("[D{}]", proc.dec_util)
                } else {
                    String::new()
                };

                // GPU index for multi-GPU systems
                let gpu_str = if gpus.len() > 1 {
                    format!("{}", proc.gpu_idx)
                } else {
                    String::new()
                };

                // Build process line with columnar layout
                let mut spans = vec![
                    // Type badge (◼C or ◼G)
                    Span::styled(format!("◼{}", proc.proc_type), Style::default().fg(type_color)),
                ];

                // GPU index for multi-GPU
                if !gpu_str.is_empty() {
                    spans.push(Span::styled(
                        format!("{} ", gpu_str),
                        Style::default().fg(Color::DarkGray),
                    ));
                } else {
                    spans.push(Span::styled(" ", Style::default()));
                }

                spans.extend(vec![
                    // PID
                    Span::styled(format!("{:>5} ", proc.pid), Style::default().fg(Color::DarkGray)),
                    // SM utilization
                    Span::styled(format!("{:>2}%", proc.sm_util), Style::default().fg(sm_color)),
                    Span::styled(" ", Style::default()),
                    // Memory bar
                    Span::styled("█".repeat(mem_filled), Style::default().fg(mem_color)),
                    Span::styled("░".repeat(mem_empty), Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{:>2}%", proc.mem_util), Style::default().fg(mem_color)),
                    Span::styled(" ", Style::default()),
                ]);

                // Add encoder/decoder if present
                if !enc_dec.is_empty() {
                    spans.push(Span::styled(
                        format!("{} ", enc_dec),
                        Style::default().fg(Color::Yellow),
                    ));
                }

                // Calculate remaining space for command
                let gpu_width = if gpu_str.is_empty() { 0 } else { 2 };
                let fixed_width = 3 + gpu_width + 6 + 3 + 1 + mem_bar_width + 3 + 1 + enc_dec.len() + if enc_dec.is_empty() { 0 } else { 1 };
                let cmd_width = (inner.width as usize).saturating_sub(fixed_width);

                // Command name
                spans.push(Span::styled(
                    truncate_str(&proc.command, cmd_width),
                    Style::default().fg(Color::White),
                ));

                f.render_widget(
                    Paragraph::new(Line::from(spans)),
                    Rect { x: inner.x, y, width: inner.width, height: 1 },
                );
                y += 1;
            }
        }
    }
}

/// Draw Battery panel
pub fn draw_battery(f: &mut Frame, app: &App, area: Rect) {
    // Check for mock battery data first (for testing)
    if let Some(mock_bat) = &app.mock_battery {
        let status = if mock_bat.charging { "Charging" } else { "Discharging" };
        let title = format!(" Battery │ {:.0}% │ {} ", mock_bat.percent, status);
        let block = btop_block(&title, borders::BATTERY);
        let inner = block.inner(area);
        f.render_widget(block, area);

        if inner.height < 1 {
            return;
        }

        let color = percent_color(100.0 - mock_bat.percent);
        let meter = Meter::new(mock_bat.percent / 100.0).label("Charge").color(color);
        f.render_widget(
            meter,
            Rect {
                x: inner.x,
                y: inner.y,
                width: inner.width,
                height: 1,
            },
        );

        if inner.height > 1 {
            let time_str = if let Some(mins) = mock_bat.time_remaining_mins {
                if mock_bat.charging {
                    format!("Time to full: {}h {}m", mins / 60, mins % 60)
                } else {
                    format!("Time remaining: {}h {}m", mins / 60, mins % 60)
                }
            } else {
                String::new()
            };

            if !time_str.is_empty() {
                f.render_widget(
                    Paragraph::new(time_str),
                    Rect {
                        x: inner.x,
                        y: inner.y + 1,
                        width: inner.width,
                        height: 1,
                    },
                );
            }
        }

        // Additional battery info for larger displays
        if inner.height > 2 {
            let info = format!(
                "Power: {:.1}W │ Health: {:.0}% │ Cycles: {}",
                mock_bat.power_watts, mock_bat.health_percent, mock_bat.cycle_count
            );
            f.render_widget(
                Paragraph::new(info),
                Rect {
                    x: inner.x,
                    y: inner.y + 2,
                    width: inner.width,
                    height: 1,
                },
            );
        }
        return;
    }

    // Real battery data path
    let batteries = app.battery.batteries();
    let battery = batteries.first();

    let (charge, status) = battery
        .map(|b| (b.capacity as f64, format!("{:?}", b.state)))
        .unwrap_or((0.0, "Unknown".to_string()));

    let title = format!(" Battery │ {:.0}% │ {} ", charge, status);

    let block = btop_block(&title, borders::BATTERY);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 1 || battery.is_none() {
        return;
    }

    let bat = battery.expect("checked above");
    let charge_pct = bat.capacity as f64;
    let color = percent_color(100.0 - charge_pct); // Invert for battery
    let meter = Meter::new(charge_pct / 100.0).label("Charge").color(color);
    f.render_widget(
        meter,
        Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        },
    );

    // Time remaining
    if inner.height > 1 {
        let time_str = if let Some(secs) = bat.time_to_empty {
            let mins = secs / 60;
            format!("Time remaining: {}h {}m", mins / 60, mins % 60)
        } else if let Some(secs) = bat.time_to_full {
            let mins = secs / 60;
            format!("Time to full: {}h {}m", mins / 60, mins % 60)
        } else {
            String::new()
        };

        if !time_str.is_empty() {
            f.render_widget(
                Paragraph::new(time_str),
                Rect {
                    x: inner.x,
                    y: inner.y + 1,
                    width: inner.width,
                    height: 1,
                },
            );
        }
    }
}

/// Draw Sensors/Temperature panel with health analysis
pub fn draw_sensors(f: &mut Frame, app: &App, area: Rect) {
    use trueno_viz::monitor::ratatui::style::Color;
    use crate::analyzers::{SensorHealth, SensorType};
    use crate::app::MockSensorType;

    // Check for mock sensor data first (for testing)
    if !app.mock_sensors.is_empty() {
        let mock_temps: Vec<_> = app.mock_sensors.iter()
            .filter(|s| s.sensor_type == MockSensorType::Temperature)
            .collect();
        let max_temp = mock_temps.iter().map(|s| s.value).fold(0.0f64, |a, b| a.max(b));

        let title = format!(" Sensors │ Max: {:.0}°C │ {} readings ", max_temp, app.mock_sensors.len());
        let block = btop_block(&title, borders::SENSORS);
        let inner = block.inner(area);
        f.render_widget(block, area);

        if inner.height < 1 {
            return;
        }

        for (i, sensor) in app.mock_sensors.iter().take(inner.height as usize).enumerate() {
            let label: String = sensor.label.chars().take(10).collect();

            let (color, health_symbol) = match sensor.sensor_type {
                MockSensorType::Temperature => {
                    let headroom = sensor.crit.map(|c| c - sensor.value).unwrap_or(100.0);
                    if headroom < 10.0 {
                        (Color::Red, "● ")
                    } else if headroom < 25.0 {
                        (Color::Yellow, "◐ ")
                    } else {
                        (Color::Green, "○ ")
                    }
                }
                MockSensorType::Fan => (Color::Cyan, "◎ "),
                MockSensorType::Voltage => (Color::Blue, "⚡ "),
                MockSensorType::Power => (Color::Magenta, "⚡ "),
            };

            let unit = match sensor.sensor_type {
                MockSensorType::Temperature => "°C",
                MockSensorType::Fan => "RPM",
                MockSensorType::Voltage => "V",
                MockSensorType::Power => "W",
            };

            let line = Line::from(vec![
                Span::styled(health_symbol, Style::default().fg(color)),
                Span::styled(format!("{label:10}"), Style::default()),
                Span::styled(format!(" {:>6.1}{}", sensor.value, unit), Style::default().fg(color)),
            ]);

            f.render_widget(
                Paragraph::new(line),
                Rect {
                    x: inner.x,
                    y: inner.y + i as u16,
                    width: inner.width,
                    height: 1,
                },
            );
        }
        return;
    }

    // Real sensor data path
    let temps = app.sensors.readings();
    let max_temp = app.sensors.max_temp().unwrap_or(0.0);

    // Get thermal summary from health analyzer
    let health_summary = app.sensor_health.thermal_summary();
    let headroom_str = health_summary
        .map(|(_, hr, _)| format!(" │ Δ{:.0}°", hr))
        .unwrap_or_default();

    // Check for any critical sensors
    let critical_indicator = if app.sensor_health.any_critical() { " ⚠" } else { "" };

    let title = format!(" Sensors │ Max: {:.0}°C{}{} ", max_temp, headroom_str, critical_indicator);

    let block = btop_block(&title, borders::SENSORS);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 1 {
        return;
    }

    // Get sensor health readings
    let health_readings = app.sensor_health.by_health();

    // Show temperature readings with health indicators
    for (i, temp) in temps.iter().take(inner.height as usize).enumerate() {
        let label: String = temp.label.chars().take(10).collect();
        let color = temp_color(temp.current);

        // Find matching health reading for this sensor
        let health_info = health_readings.values()
            .flatten()
            .find(|r| r.sensor_type == SensorType::Temperature && r.label.contains(&temp.label[..temp.label.len().min(6)]));

        let mut spans = Vec::new();

        // Health indicator
        if let Some(info) = health_info {
            let health_color = match info.health {
                SensorHealth::Healthy => Color::Green,
                SensorHealth::Warning => Color::Yellow,
                SensorHealth::Critical => Color::Red,
                SensorHealth::Stale => Color::DarkGray,
                SensorHealth::Dead => Color::DarkGray,
            };
            spans.push(Span::styled(
                format!("{} ", info.health.symbol()),
                Style::default().fg(health_color),
            ));
        } else {
            spans.push(Span::raw("  "));
        }

        // Label
        spans.push(Span::styled(format!("{label:10}"), Style::default()));

        // Temperature value
        spans.push(Span::styled(
            format!(" {:>5.0}°C", temp.current),
            Style::default().fg(color),
        ));

        // Drift indicator (if significant)
        if let Some(info) = health_info {
            if let Some(drift) = info.drift_rate {
                if drift.abs() > 1.0 {
                    let drift_color = if drift > 0.0 { Color::Red } else { Color::Cyan };
                    let arrow = if drift > 0.0 { "↑" } else { "↓" };
                    spans.push(Span::styled(
                        format!(" {}{:.1}/m", arrow, drift.abs()),
                        Style::default().fg(drift_color),
                    ));
                }
            }

            // Outlier marker
            if info.is_outlier {
                spans.push(Span::styled(" !", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)));
            }
        }

        let line = Line::from(spans);
        f.render_widget(
            Paragraph::new(line),
            Rect {
                x: inner.x,
                y: inner.y + i as u16,
                width: inner.width,
                height: 1,
            },
        );
    }
}

/// Draw compact sensors panel (single line)
pub fn draw_sensors_compact(f: &mut Frame, app: &App, area: Rect) {
    use trueno_viz::monitor::ratatui::style::Color;
    use crate::analyzers::SensorType;

    let readings = app.sensor_health.get_cached_readings();
    let max_temp = app.sensors.max_temp().unwrap_or(0.0);

    let title = format!(" Sensors │ {:.0}°C ", max_temp);

    let block = btop_block(&title, borders::SENSORS);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 1 || inner.width < 10 {
        return;
    }

    // Filter to most useful sensors, prioritize temps
    let mut display_sensors: Vec<_> = readings.iter()
        .filter(|r| r.sensor_type == SensorType::Temperature || r.sensor_type == SensorType::Fan)
        .collect();

    // Sort: temps first, then by value descending
    display_sensors.sort_by(|a, b| {
        let type_order = |t: &SensorType| match t {
            SensorType::Temperature => 0,
            SensorType::Fan => 1,
            _ => 2,
        };
        type_order(&a.sensor_type).cmp(&type_order(&b.sensor_type))
            .then(b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal))
    });

    let bar_width = 4usize;

    for (i, sensor) in display_sensors.iter().take(inner.height as usize).enumerate() {
        let y = inner.y + i as u16;

        // Type letter: C=CPU, G=GPU, D=Disk, M=Mobo, F=Fan
        let label_lower = sensor.label.to_lowercase();
        let (type_char, type_color) = if label_lower.contains("cpu") || label_lower.contains("core") || label_lower.contains("package") {
            ('C', Color::Rgb(220, 120, 80))   // CPU - orange
        } else if label_lower.contains("gpu") || label_lower.contains("edge") || label_lower.contains("junction") {
            ('G', Color::Rgb(120, 200, 80))   // GPU - green
        } else if label_lower.contains("nvme") || label_lower.contains("composite") || label_lower.contains("ssd") {
            ('D', Color::Rgb(80, 160, 220))   // Disk - blue
        } else if label_lower.contains("fan") {
            ('F', Color::Rgb(180, 180, 220))  // Fan - light purple
        } else {
            ('M', Color::Rgb(180, 180, 140))  // Mobo/other - tan
        };

        // Calculate bar fill based on temp-to-critical ratio
        let max_val = sensor.crit.or(sensor.max).unwrap_or(100.0);
        let ratio = (sensor.value / max_val).min(1.0);
        let fill = (ratio * bar_width as f64).round() as usize;

        // Top color: current temp (green -> yellow -> red)
        let top_color = if sensor.value >= 85.0 {
            Color::Rgb(220, 60, 60)    // Red - hot
        } else if sensor.value >= 70.0 {
            Color::Rgb(220, 180, 60)   // Yellow - warm
        } else {
            Color::Rgb(80, 180, 100)   // Green - cool
        };

        // Bottom color: trend (green=stable, red=rising, blue=cooling)
        let bottom_color = match sensor.drift_rate {
            Some(drift) if drift > 2.0 => Color::Rgb(220, 80, 80),   // Rising fast - red
            Some(drift) if drift > 0.5 => Color::Rgb(220, 180, 80),  // Rising - yellow
            Some(drift) if drift < -2.0 => Color::Rgb(80, 140, 220), // Cooling fast - blue
            Some(drift) if drift < -0.5 => Color::Rgb(100, 180, 200),// Cooling - cyan
            _ => Color::Rgb(80, 180, 100),                           // Stable - green
        };

        let bar: String = "▄".repeat(fill.min(bar_width));
        let empty: String = " ".repeat(bar_width.saturating_sub(fill));

        // Value display
        let value_str = if sensor.sensor_type == SensorType::Fan {
            format!("{:.0}", sensor.value)  // RPM, no unit (too wide)
        } else {
            format!("{:.0}°", sensor.value)
        };

        // Label (truncate to fit)
        let name_width = (inner.width as usize).saturating_sub(bar_width + 8);
        let label: String = sensor.label.chars().take(name_width).collect();

        let line = Line::from(vec![
            Span::styled(String::from(type_char), Style::default().fg(type_color)),
            Span::styled(bar, Style::default().fg(bottom_color).bg(top_color)),
            Span::styled(empty, Style::default().fg(Color::Rgb(40, 40, 45))),
            Span::styled(format!("{:>4}", value_str), Style::default().fg(Color::Rgb(180, 180, 160))),
            Span::styled(format!(" {}", label), Style::default().fg(Color::Rgb(160, 165, 175))),
        ]);

        f.render_widget(
            Paragraph::new(line),
            Rect { x: inner.x, y, width: inner.width, height: 1 },
        );
    }
}

/// Draw PSI (Pressure Stall Information) panel
