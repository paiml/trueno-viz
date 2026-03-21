pub fn draw_disk(f: &mut Frame, app: &App, area: Rect) {
    use trueno_viz::monitor::ratatui::style::Color;
    use trueno_viz::monitor::ratatui::text::{Line, Span};
    use crate::analyzers::PressureLevel;

    let mounts = app.disk.mounts();
    let rates = app.disk.rates();

    // Calculate total I/O rates
    let total_read: f64 = rates.values().map(|r| r.read_bytes_per_sec).sum();
    let total_write: f64 = rates.values().map(|r| r.write_bytes_per_sec).sum();
    let total_iops = app.disk_io_analyzer.total_iops();

    // Get workload type
    let workload = app.disk_io_analyzer.overall_workload();

    // Get entropy gauge
    let entropy_gauge = app.disk_entropy.system_gauge();

    // Detect exploded mode early for title (account for borders)
    let is_exploded = area.width > 82 || area.height > 22;
    let exploded_info = if is_exploded { " │ ▣ FULL" } else { "" };

    // IOPS breakdown and queue depth for exploded mode
    let iops_detail = if is_exploded && (app.disk_read_iops > 0.0 || app.disk_write_iops > 0.0) {
        format!(" │ R:{:.0} W:{:.0}", app.disk_read_iops, app.disk_write_iops)
    } else {
        String::new()
    };

    let queue_info = if is_exploded && app.disk_queue_depth > 0.1 {
        format!(" │ Q:{:.1}", app.disk_queue_depth)
    } else {
        String::new()
    };

    // Disk health summary for exploded mode
    let health_info = if is_exploded && !app.disk_health.is_empty() {
        let worst_health = app.disk_health.iter()
            .map(|h| h.status)
            .max_by_key(|s| match s {
                DiskHealth::Critical => 3,
                DiskHealth::Warning => 2,
                DiskHealth::Good => 1,
                DiskHealth::Unknown => 0,
            })
            .unwrap_or(DiskHealth::Unknown);
        format!(" {}", worst_health.symbol())
    } else {
        String::new()
    };

    let title = format!(
        " Disk │ R: {} │ W: {} │ {:.0} IOPS{}{}{} │ {} │ E:{}{} ",
        theme::format_bytes_rate(total_read),
        theme::format_bytes_rate(total_write),
        total_iops,
        iops_detail,
        queue_info,
        health_info,
        workload.description(),
        entropy_gauge,
        exploded_info
    );

    let block = btop_block(&title, borders::DISK);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 1 {
        return;
    }

    let mut y = inner.y;

    // === LINE 1: Latency gauge bar ===
    if let Some(device) = app.disk_io_analyzer.primary_device() {
        let latency = app.disk_io_analyzer.estimated_latency_ms(&device);

        // Latency color: green <5ms, yellow 5-20ms, red >20ms
        let latency_color = if latency < 5.0 {
            Color::Green
        } else if latency < 20.0 {
            Color::Yellow
        } else {
            Color::Red
        };

        // Latency bar (max 100ms for scale)
        let latency_pct = (latency / 100.0).min(1.0);
        let bar_width = inner.width.saturating_sub(20) as usize;
        let filled = (latency_pct * bar_width as f64) as usize;
        let empty = bar_width.saturating_sub(filled);

        let latency_line = Line::from(vec![
            Span::styled("Latency ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:>5.1}ms ", latency), Style::default().fg(latency_color)),
            Span::styled("█".repeat(filled), Style::default().fg(latency_color)),
            Span::styled("░".repeat(empty), Style::default().fg(Color::DarkGray)),
        ]);

        f.render_widget(
            Paragraph::new(latency_line),
            Rect { x: inner.x, y, width: inner.width, height: 1 },
        );
        y += 1;
    }

    // Reserve 1 line for I/O PSI at bottom
    let reserved_bottom = 1u16;
    let remaining_height = (inner.y + inner.height).saturating_sub(y + reserved_bottom);
    let max_mounts = remaining_height as usize;

    // Column layout: Name | Size | Bar(variable) | I/O Rate | Sparkline(rest)
    // In exploded mode: scale columns to fill available width
    let (name_col, size_col, io_col) = if is_exploded {
        // Scale columns with terminal width
        let name_w = (inner.width / 6).max(15).min(30);
        let size_w = (inner.width / 12).max(8).min(14);
        let io_w = (inner.width / 6).max(14).min(24);
        (name_w, size_w, io_w)
    } else {
        (10u16, 6u16, 14u16)
    };
    // Bar takes remaining space - will be larger in exploded mode
    let bar_width = inner.width.saturating_sub(name_col + size_col + io_col + 4).max(10);
    let sparkline_width = inner.width.saturating_sub(name_col + size_col + bar_width + io_col + 4);

    for mount in mounts.iter().take(max_mounts) {
        if mount.total_bytes == 0 || y >= inner.y + inner.height - reserved_bottom {
            continue;
        }

        let used_pct = mount.usage_percent();
        let total_gb = mount.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

        // Short mount point label (fixed width)
        let label: String = if mount.mount_point == "/" {
            "/".to_string()
        } else {
            mount
                .mount_point
                .rsplit('/')
                .next()
                .unwrap_or(&mount.mount_point)
                .chars()
                .take(name_col as usize - 1)
                .collect()
        };

        // Find I/O rate for this device
        // Mount device: /dev/disk3s1 (macOS) or /dev/sda1 (Linux) or /dev/nvme0n1p1
        let device_name = mount.device.rsplit('/').next().unwrap_or("");
        let base_device: String = if device_name.contains("nvme") {
            // nvme0n1p1 -> nvme0n1 (strip partition)
            device_name.split('p').next().unwrap_or(device_name).to_string()
        } else if device_name.starts_with("disk") && device_name.contains('s') {
            // macOS: disk3s1 -> disk3 (strip slice suffix)
            device_name.split('s').next().unwrap_or(device_name).to_string()
        } else {
            // Linux: sda1 -> sda (strip partition number)
            device_name.chars().take_while(|c| !c.is_ascii_digit()).collect()
        };

        // macOS APFS: synthesized disk1 backed by physical disk0, disk3 backed by disk2
        // Try exact match, then base device, then physical backing disk
        let io_info = rates.get(device_name)
            .or_else(|| rates.get(&base_device))
            .or_else(|| {
                // macOS: disk1 -> disk0, disk3 -> disk2 (synthesized -> physical)
                if let Some(num_str) = base_device.strip_prefix("disk") {
                    let disk_num: u32 = num_str.parse().unwrap_or(0);
                    if disk_num > 0 {
                        // Synthesized containers (odd: 1,3,5) map to physical (even: 0,2,4)
                        let physical = format!("disk{}", disk_num.saturating_sub(1));
                        rates.get(&physical)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .or_else(|| rates.get("disk0")); // Final fallback to primary disk

        let color = percent_color(used_pct);

        // Size string
        let size_str = if total_gb >= 1000.0 {
            format!("{:.1}T", total_gb / 1024.0)
        } else {
            format!("{:.0}G", total_gb)
        };

        // I/O rate string
        let io_str = if let Some(io) = io_info {
            let total_rate = io.read_bytes_per_sec + io.write_bytes_per_sec;
            if total_rate > 1000.0 {
                theme::format_bytes_rate(total_rate).to_string()
            } else {
                "—".to_string()
            }
        } else {
            "—".to_string()
        };

        // Build the row with proper columns (with bounds checking)
        let mut x = inner.x;
        let max_x = inner.x + inner.width;

        // Col 1: Name
        if x < max_x {
            let w = name_col.min(max_x.saturating_sub(x));
            f.render_widget(
                Paragraph::new(format!("{:<width$}", label, width = w as usize))
                    .style(Style::default().fg(Color::White)),
                Rect { x, y, width: w, height: 1 },
            );
            x += name_col;
        }

        // Col 2: Size
        if x < max_x {
            let w = size_col.min(max_x.saturating_sub(x));
            f.render_widget(
                Paragraph::new(format!("{:>width$}", size_str, width = w as usize))
                    .style(Style::default().fg(Color::DarkGray)),
                Rect { x, y, width: w, height: 1 },
            );
            x += size_col + 1;
        }

        // Col 3: Usage bar
        if x < max_x {
            let w = bar_width.min(max_x.saturating_sub(x));
            let filled = ((used_pct / 100.0) * w as f64) as usize;
            let empty = (w as usize).saturating_sub(filled);
            let bar_line = Line::from(vec![
                Span::styled("█".repeat(filled), Style::default().fg(color)),
                Span::styled("░".repeat(empty), Style::default().fg(Color::DarkGray)),
            ]);
            f.render_widget(
                Paragraph::new(bar_line),
                Rect { x, y, width: w, height: 1 },
            );
            x += bar_width + 1;
        }

        // Col 4: Percentage + Entropy indicator
        if x < max_x {
            let entropy_char = app.disk_entropy
                .get_mount_entropy(&mount.mount_point)
                .map(|e| e.indicator())
                .unwrap_or('·');
            let entropy_color = match entropy_char {
                '●' => Color::Green,   // High entropy (unique)
                '◐' => Color::Yellow,  // Medium entropy
                '○' => Color::Red,     // Low entropy (dupes)
                _ => Color::DarkGray,
            };
            let w = 5u16.min(max_x.saturating_sub(x));
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(format!("{:>3.0}%", used_pct), Style::default().fg(color)),
                    Span::styled(format!("{}", entropy_char), Style::default().fg(entropy_color)),
                ])),
                Rect { x, y, width: w, height: 1 },
            );
            x += 6;
        }

        // Col 5: I/O rate
        if x < max_x {
            let w = 8u16.min(max_x.saturating_sub(x));
            f.render_widget(
                Paragraph::new(format!("{:>8}", io_str))
                    .style(Style::default().fg(Color::Cyan)),
                Rect { x, y, width: w, height: 1 },
            );
            x += 9;
        }

        // Col 6: I/O sparkline (if space and history available)
        if x < max_x && sparkline_width > 3 {
            let w = sparkline_width.min(max_x.saturating_sub(x));
            if w > 3 {
                let read_history = app.disk_io_analyzer.device_read_history(&base_device);
                if let Some(ref rh) = read_history {
                    if !rh.is_empty() {
                        let sparkline = MonitorSparkline::new(rh)
                            .color(Color::Cyan)
                            .show_trend(false);
                        f.render_widget(
                            sparkline,
                            Rect { x, y, width: w, height: 1 },
                        );
                    }
                }
            }
        }

        y += 1;
    }

    // === I/O PSI Row at bottom ===
    if y < inner.y + inner.height && app.psi_analyzer.is_available() {
        let psi = &app.psi_analyzer;
        let io_lvl = psi.io_level();

        let level_color = |level: PressureLevel| -> Color {
            match level {
                PressureLevel::None => Color::DarkGray,
                PressureLevel::Low => Color::Green,
                PressureLevel::Medium => Color::Yellow,
                PressureLevel::High => Color::LightRed,
                PressureLevel::Critical => Color::Red,
            }
        };

        // Show I/O pressure with both "some" and "full" stall percentages
        let io_line = Line::from(vec![
            Span::styled("I/O Pressure ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}{:>5.1}%", io_lvl.symbol(), psi.io.some_avg10),
                Style::default().fg(level_color(io_lvl)),
            ),
            Span::styled(" some  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:>5.1}%", psi.io.full_avg10),
                Style::default().fg(if psi.io.full_avg10 > 5.0 { Color::Yellow } else { Color::DarkGray }),
            ),
            Span::styled(" full", Style::default().fg(Color::DarkGray)),
        ]);

        f.render_widget(
            Paragraph::new(io_line),
            Rect { x: inner.x, y, width: inner.width, height: 1 },
        );
        y += 1;
    }

    // === Expand to fill remaining height with top active processes ===
    let remaining_height = (inner.y + inner.height).saturating_sub(y) as usize;
    if remaining_height > 1 {
        // Show top CPU processes as proxy for I/O activity
        let mut procs: Vec<_> = app.process.processes().values().collect();
        procs.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(std::cmp::Ordering::Equal));

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("── Top Active Processes ", Style::default().fg(Color::DarkGray)),
                Span::styled("─".repeat((inner.width as usize).saturating_sub(24)), Style::default().fg(Color::DarkGray)),
            ])),
            Rect { x: inner.x, y, width: inner.width, height: 1 },
        );
        y += 1;

        let procs_to_show = (remaining_height - 1).min(procs.len());
        for proc in procs.iter().take(procs_to_show) {
            if y >= inner.y + inner.height {
                break;
            }

            if proc.cpu_percent < 0.1 {
                continue; // Skip idle processes
            }

            let name: String = proc.name.chars().take(20).collect();
            let mem_mb = proc.mem_bytes as f64 / (1024.0 * 1024.0);
            let line = Line::from(vec![
                Span::styled(format!("{:>6} ", proc.pid), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{:<20} ", name), Style::default().fg(Color::White)),
                Span::styled(format!("CPU:{:>5.1}% ", proc.cpu_percent), Style::default().fg(percent_color(proc.cpu_percent))),
                Span::styled(format!("MEM:{:>7.1}M ", mem_mb), Style::default().fg(Color::Magenta)),
            ]);

            f.render_widget(
                Paragraph::new(line),
                Rect { x: inner.x, y, width: inner.width, height: 1 },
            );
            y += 1;
        }
    }
}

/// Draw Network panel - btop style with dual graphs, peaks, and connection stats
pub fn draw_network(f: &mut Frame, app: &App, area: Rect) {
    use std::time::Instant;
    use trueno_viz::monitor::ratatui::style::Color;

    let iface = app.network.current_interface().unwrap_or("none");
    let (rx_rate, tx_rate) = app
        .network
        .current_interface()
        .and_then(|i| app.network.all_rates().get(i))
        .map(|r| (r.rx_bytes_per_sec, r.tx_bytes_per_sec))
        .unwrap_or((0.0, 0.0));

    // Detect exploded mode early for title (account for borders)
    let is_exploded = area.width > 82 || area.height > 22;
    let exploded_info = if is_exploded { " │ ▣ FULL" } else { "" };

    // Connection counts for exploded mode
    let conn_info = if is_exploded && (app.net_established > 0 || app.net_listening > 0) {
        format!(" │ Est:{} Lis:{}", app.net_established, app.net_listening)
    } else {
        String::new()
    };

    // Error/drop info for exploded mode
    let error_info = if is_exploded && (app.net_errors > 0 || app.net_drops > 0) {
        format!(" │ Err:{} Drop:{}", app.net_errors, app.net_drops)
    } else {
        String::new()
    };

    let title = format!(
        " Network ({}) │ ↓ {} │ ↑ {}{}{}{} ",
        iface,
        theme::format_bytes_rate(rx_rate),
        theme::format_bytes_rate(tx_rate),
        conn_info,
        error_info,
        exploded_info
    );

    let block = btop_block(&title, borders::NETWORK);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 2 {
        return;
    }

    // Calculate layout based on available height
    // - Multi-interface row (1 line) if multiple interfaces
    // - RX info with sparkline (1 line)
    // - RX Graph (variable)
    // - TX info with sparkline (1 line)
    // - TX Graph (variable)
    // - Bottom rows: totals/peaks, connection stats, top consumers (up to 3 lines)

    let all_rates = app.network.all_rates();
    let interfaces: Vec<_> = all_rates.keys().collect();
    let show_multi_iface = interfaces.len() > 1 && inner.height >= 10;
    // More rows for expanded stats (protocol, errors, consumers) - lowered thresholds for visibility
    let bottom_row_count = if inner.height >= 10 { 4 } else if inner.height >= 8 { 3 } else if inner.height >= 6 { 2 } else if inner.height >= 4 { 1 } else { 0 };

    let info_lines = 2; // RX + TX info rows
    let multi_iface_line = if show_multi_iface { 1 } else { 0 };
    let graph_total = inner.height.saturating_sub(info_lines + multi_iface_line + bottom_row_count as u16);
    let half_height = graph_total / 2;

    let mut y = inner.y;

    // === Multi-Interface Summary Row ===
    if show_multi_iface {
        let mut spans = vec![Span::styled("Ifaces ", Style::default().fg(Color::DarkGray))];

        // Show more interfaces in exploded mode - scale with width
        let max_ifaces = if is_exploded { (inner.width as usize / 15).max(6).min(12) } else { 4 };
        let name_len = if is_exploded { (inner.width as usize / 12).max(10).min(20) } else { 8 };

        for (i, iface_name) in interfaces.iter().take(max_ifaces).enumerate() {
            if let Some(rates) = all_rates.get(*iface_name) {
                let total_rate = rates.rx_bytes_per_sec + rates.tx_bytes_per_sec;
                // Mini activity indicator (0-8 scale based on rate)
                let activity = if total_rate > 100_000_000.0 {
                    "▇"
                } else if total_rate > 10_000_000.0 {
                    "▅"
                } else if total_rate > 1_000_000.0 {
                    "▃"
                } else if total_rate > 100_000.0 {
                    "▂"
                } else if total_rate > 1000.0 {
                    "▁"
                } else {
                    "░"
                };

                let is_current = Some(iface_name.as_str()) == app.network.current_interface();
                let name_color = if is_current { Color::Cyan } else { Color::White };

                if i > 0 {
                    spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
                }

                // Truncate interface name
                let short_name: String = iface_name.chars().take(name_len).collect();
                spans.push(Span::styled(short_name, Style::default().fg(name_color)));
                // In exploded mode, show rate next to activity indicator
                if is_exploded {
                    spans.push(Span::styled(
                        format!(" {}", theme::format_bytes_rate(total_rate)),
                        Style::default().fg(Color::DarkGray)
                    ));
                }
                spans.push(Span::styled(activity, Style::default().fg(graph::NETWORK_RX)));
            }
        }

        f.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect { x: inner.x, y, width: inner.width, height: 1 },
        );
        y += 1;
    }

    // === RX info line with rate and sparkline ===
    {
        let label_width = 16u16.min(inner.width);
        let sparkline_width = inner.width.saturating_sub(label_width);

        let rx_label = Line::from(vec![
            Span::styled("↓ Download ", Style::default().fg(graph::NETWORK_RX)),
            Span::styled(
                theme::format_bytes_rate(rx_rate),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
        ]);
        if let Some(label_rect) = clamp_rect(inner, inner.x, y, label_width, 1) {
            f.render_widget(Paragraph::new(rx_label), label_rect);
        }

        if sparkline_width > 2 && !app.net_rx_history.is_empty() {
            if let Some(spark_rect) = clamp_rect(inner, inner.x + label_width, y, sparkline_width, 1) {
                let sparkline = MonitorSparkline::new(&app.net_rx_history)
                    .color(graph::NETWORK_RX)
                    .show_trend(true);
                f.render_widget(sparkline, spark_rect);
            }
        }
        y += 1;
    }

    // === Download graph ===
    if half_height > 0 {
        let rx_area = Rect { x: inner.x, y, width: inner.width, height: half_height };
        let rx_data: Vec<f64> = if app.net_rx_history.is_empty() {
            vec![0.0]
        } else {
            app.net_rx_history.clone()
        };
        let rx_graph = Graph::new(&rx_data).color(graph::NETWORK_RX);
        f.render_widget(rx_graph, rx_area);
        y += half_height;
    }

    // === TX info line with rate and sparkline ===
    {
        let label_width = 16u16.min(inner.width);
        let sparkline_width = inner.width.saturating_sub(label_width);

        let tx_label = Line::from(vec![
            Span::styled("↑ Upload   ", Style::default().fg(graph::NETWORK_TX)),
            Span::styled(
                theme::format_bytes_rate(tx_rate),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
        ]);
        if let Some(label_rect) = clamp_rect(inner, inner.x, y, label_width, 1) {
            f.render_widget(Paragraph::new(tx_label), label_rect);
        }

        if sparkline_width > 2 && !app.net_tx_history.is_empty() {
            if let Some(spark_rect) = clamp_rect(inner, inner.x + label_width, y, sparkline_width, 1) {
                let sparkline = MonitorSparkline::new(&app.net_tx_history)
                    .color(graph::NETWORK_TX)
                    .show_trend(true);
                f.render_widget(sparkline, spark_rect);
            }
        }
        y += 1;
    }

    // === Upload graph (inverted) ===
    let remaining_for_graph = (inner.y + inner.height)
        .saturating_sub(y)
        .saturating_sub(bottom_row_count as u16);

    if remaining_for_graph > 0 {
        let tx_area = Rect { x: inner.x, y, width: inner.width, height: remaining_for_graph };
        let tx_data: Vec<f64> = if app.net_tx_history.is_empty() {
            vec![0.0]
        } else {
            app.net_tx_history.clone()
        };
        let tx_graph = Graph::new(&tx_data).color(graph::NETWORK_TX).inverted(true);
        f.render_widget(tx_graph, tx_area);
        y += remaining_for_graph;
    }

    // === Bottom Row 1: Session totals + Peak rates ===
    if bottom_row_count >= 1 && y < inner.y + inner.height {
        // Format peak time as "Xm ago" or "Xs ago"
        let format_ago = |instant: Instant| -> String {
            let secs = instant.elapsed().as_secs();
            if secs >= 60 {
                format!("{}m", secs / 60)
            } else {
                format!("{}s", secs)
            }
        };

        let mut spans = vec![
            Span::styled("Session ", Style::default().fg(Color::DarkGray)),
            Span::styled("↓", Style::default().fg(graph::NETWORK_RX)),
            Span::styled(
                theme::format_bytes(app.net_rx_total),
                Style::default().fg(Color::White),
            ),
            Span::styled(" ↑", Style::default().fg(graph::NETWORK_TX)),
            Span::styled(
                theme::format_bytes(app.net_tx_total),
                Style::default().fg(Color::White),
            ),
        ];

        // Add peak rates if we have them
        if app.net_rx_peak > 0.0 || app.net_tx_peak > 0.0 {
            spans.push(Span::styled(" │ Peak ", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled("↓", Style::default().fg(graph::NETWORK_RX)));
            spans.push(Span::styled(
                format!("{} ({})", theme::format_bytes_rate(app.net_rx_peak), format_ago(app.net_rx_peak_time)),
                Style::default().fg(Color::White),
            ));
            spans.push(Span::styled(" ↑", Style::default().fg(graph::NETWORK_TX)));
            spans.push(Span::styled(
                format!("{} ({})", theme::format_bytes_rate(app.net_tx_peak), format_ago(app.net_tx_peak_time)),
                Style::default().fg(Color::White),
            ));
        }

        f.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect { x: inner.x, y, width: inner.width, height: 1 },
        );
        y += 1;
    }

    // === Bottom Row 2: Protocol Stats (TCP/UDP/ICMP) with Errors/Latency ===
    #[cfg(target_os = "linux")]
    if bottom_row_count >= 2 && y < inner.y + inner.height {
        let stats = &app.network_stats;
        let proto = &stats.protocol_stats;

        // Protocol counts
        let mut spans = vec![
            Span::styled("TCP ", Style::default().fg(Color::Cyan)),
            Span::styled(format!("{}", proto.tcp_established), Style::default().fg(Color::Green)),
            Span::styled("/", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", proto.tcp_listen), Style::default().fg(Color::Cyan)),
        ];

        // Show problematic states if any
        let problem_states = proto.tcp_time_wait + proto.tcp_close_wait;
        if problem_states > 0 {
            spans.push(Span::styled(
                format!(" ({}tw)", proto.tcp_time_wait + proto.tcp_close_wait),
                Style::default().fg(Color::Yellow),
            ));
        }

        spans.push(Span::styled(" UDP ", Style::default().fg(Color::Magenta)));
        spans.push(Span::styled(format!("{}", proto.udp_sockets), Style::default().fg(Color::White)));

        if proto.icmp_sockets > 0 {
            spans.push(Span::styled(" ICMP ", Style::default().fg(Color::Blue)));
            spans.push(Span::styled(format!("{}", proto.icmp_sockets), Style::default().fg(Color::White)));
        }

        // Latency gauge
        spans.push(Span::styled(" │ RTT ", Style::default().fg(Color::DarkGray)));
        let gauge = stats.latency_gauge();
        let gauge_color = if stats.tcp_perf.rtt_ms < 25.0 { Color::Green } else if stats.tcp_perf.rtt_ms < 50.0 { Color::Yellow } else { Color::Red };
        spans.push(Span::styled(gauge, Style::default().fg(gauge_color)));

        // Retransmission rate if significant
        if stats.tcp_perf.retrans_rate > 0.001 {
            spans.push(Span::styled(
                format!(" {:.1}%re", stats.tcp_perf.retrans_rate * 100.0),
                Style::default().fg(Color::Yellow),
            ));
        }

        f.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect { x: inner.x, y, width: inner.width, height: 1 },
        );
        y += 1;
    }

    // Fallback for non-Linux: use connection analyzer
    #[cfg(not(target_os = "linux"))]
    if bottom_row_count >= 2 && y < inner.y + inner.height {
        use crate::analyzers::ConnState;

        let conns = app.connection_analyzer.connections();
        let established = conns.iter().filter(|c| c.state == ConnState::Established).count();
        let listen = conns.iter().filter(|c| c.state == ConnState::Listen).count();
        let time_wait = conns.iter().filter(|c| c.state == ConnState::TimeWait).count();
        let close_wait = conns.iter().filter(|c| c.state == ConnState::CloseWait).count();

        let conn_line = Line::from(vec![
            Span::styled("Conn ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", established), Style::default().fg(Color::Green)),
            Span::styled(" estab ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}", listen), Style::default().fg(Color::Cyan)),
            Span::styled(" listen", Style::default().fg(Color::DarkGray)),
            if time_wait > 0 || close_wait > 0 {
                Span::styled(
                    format!(" │ {} tw {} cw", time_wait, close_wait),
                    Style::default().fg(Color::Yellow),
                )
            } else {
                Span::raw("")
            },
        ]);

        f.render_widget(
            Paragraph::new(conn_line),
            Rect { x: inner.x, y, width: inner.width, height: 1 },
        );
        y += 1;
    }

    // === Bottom Row 3: Interface Errors (Linux) ===
    #[cfg(target_os = "linux")]
    if bottom_row_count >= 3 && y < inner.y + inner.height {
        let stats = &app.network_stats;
        let (rx_errs, tx_errs) = stats.total_errors();
        let (rx_delta, tx_delta) = stats.total_error_deltas();

        let mut spans = vec![
            Span::styled("Errs ", Style::default().fg(Color::DarkGray)),
        ];

        // RX errors with delta
        let rx_color = if rx_delta > 0 { Color::Red } else if rx_errs > 0 { Color::Yellow } else { Color::Green };
        spans.push(Span::styled("↓", Style::default().fg(graph::NETWORK_RX)));
        spans.push(Span::styled(format!("{}", rx_errs), Style::default().fg(rx_color)));
        if rx_delta > 0 {
            spans.push(Span::styled(format!(" (+{})", rx_delta), Style::default().fg(Color::Red)));
        }

        // TX errors with delta
        spans.push(Span::styled(" ↑", Style::default().fg(graph::NETWORK_TX)));
        let tx_color = if tx_delta > 0 { Color::Red } else if tx_errs > 0 { Color::Yellow } else { Color::Green };
        spans.push(Span::styled(format!("{}", tx_errs), Style::default().fg(tx_color)));
        if tx_delta > 0 {
            spans.push(Span::styled(format!(" (+{})", tx_delta), Style::default().fg(Color::Red)));
        }

        // Queue stats if there's backlog
        let queues = &stats.queue_stats;
        if queues.total_rx_queue > 0 || queues.total_tx_queue > 0 {
            spans.push(Span::styled(" │ Q ", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(
                format!("rx:{} tx:{}",
                    theme::format_bytes(queues.total_rx_queue),
                    theme::format_bytes(queues.total_tx_queue)),
                Style::default().fg(Color::Yellow),
            ));
        }

        // SYN backlog pressure warning
        if queues.syn_backlog_pressure {
            spans.push(Span::styled(" SYN!", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)));
        }

        f.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect { x: inner.x, y, width: inner.width, height: 1 },
        );
        y += 1;
    }

    // === Bottom Row 4: Top Network Consumers ===
    if bottom_row_count >= 4 && y < inner.y + inner.height {
        // Get processes sorted by network activity (we'll use connections as proxy)
        let conns = app.connection_analyzer.connections();
        let mut proc_conn_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

        for conn in conns.iter() {
            if let Some((_, name)) = app.connection_analyzer.process_for_connection(conn) {
                *proc_conn_counts.entry(name.to_string()).or_insert(0) += 1;
            }
        }

        let mut sorted_procs: Vec<_> = proc_conn_counts.iter().collect();
        sorted_procs.sort_by(|a, b| b.1.cmp(a.1));

        let mut spans = vec![Span::styled("Top ", Style::default().fg(Color::DarkGray))];

        for (i, (name, count)) in sorted_procs.iter().take(3).enumerate() {
            if i > 0 {
                spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
            }
            spans.push(Span::styled(
                format!("{}", count),
                Style::default().fg(Color::Yellow),
            ));
            spans.push(Span::styled(
                format!(" {}", truncate_str(name, 10)),
                Style::default().fg(Color::White),
            ));
        }

        if !sorted_procs.is_empty() {
            f.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect { x: inner.x, y, width: inner.width, height: 1 },
            );
            y += 1;
        }
    }

    // === Expand to fill remaining height with connection list ===
    use crate::analyzers::connections::{ConnState, Protocol};

    let remaining_height = (inner.y + inner.height).saturating_sub(y) as usize;
    if remaining_height > 1 {
        let conns = app.connection_analyzer.connections();

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("── Active Connections ", Style::default().fg(Color::DarkGray)),
                Span::styled("─".repeat((inner.width as usize).saturating_sub(22)), Style::default().fg(Color::DarkGray)),
            ])),
            Rect { x: inner.x, y, width: inner.width, height: 1 },
        );
        y += 1;

        // Sort connections: ESTABLISHED first, then by remote port
        let mut sorted_conns: Vec<_> = conns.iter().collect();
        sorted_conns.sort_by(|a, b| {
            // ESTABLISHED connections first
            let a_est = a.state == ConnState::Established;
            let b_est = b.state == ConnState::Established;
            match (b_est, a_est) {
                (true, false) => std::cmp::Ordering::Greater,
                (false, true) => std::cmp::Ordering::Less,
                _ => a.remote_port.cmp(&b.remote_port),
            }
        });

        let conns_to_show = (remaining_height - 1).min(sorted_conns.len());
        for conn in sorted_conns.iter().take(conns_to_show) {
            if y >= inner.y + inner.height {
                break;
            }

            let state_color = match conn.state {
                ConnState::Established => Color::Green,
                ConnState::Listen => Color::Cyan,
                ConnState::TimeWait | ConnState::CloseWait => Color::Yellow,
                ConnState::SynSent | ConnState::SynRecv => Color::Magenta,
                _ => Color::DarkGray,
            };

            let state_str = match conn.state {
                ConnState::Established => "ESTABLISHED",
                ConnState::Listen => "LISTEN",
                ConnState::TimeWait => "TIME_WAIT",
                ConnState::CloseWait => "CLOSE_WAIT",
                ConnState::SynSent => "SYN_SENT",
                ConnState::SynRecv => "SYN_RECV",
                ConnState::FinWait1 => "FIN_WAIT1",
                ConnState::FinWait2 => "FIN_WAIT2",
                ConnState::Closing => "CLOSING",
                ConnState::LastAck => "LAST_ACK",
                ConnState::Close => "CLOSE",
                ConnState::Unknown => "UNKNOWN",
            };

            let proto_str = match conn.protocol {
                Protocol::Tcp => "TCP",
                Protocol::Udp => "UDP",
            };

            let proc_info = app.connection_analyzer.process_for_connection(conn)
                .map(|(pid, name)| format!("{} ({})", truncate_str(name, 12), pid))
                .unwrap_or_else(|| "-".to_string());

            let remote_addr = conn.remote_addr();
            let remote_str = if remote_addr.is_empty() || remote_addr == "*" {
                format!("*:{}", conn.local_port)
            } else {
                format!("{}:{}", truncate_str(&remote_addr, 15), conn.remote_port)
            };

            let line = Line::from(vec![
                Span::styled(format!("{:>5} ", conn.local_port), Style::default().fg(Color::Cyan)),
                Span::styled(format!("{:<4} ", proto_str), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{:<11} ", state_str), Style::default().fg(state_color)),
                Span::styled(format!("{:<22} ", remote_str), Style::default().fg(Color::White)),
                Span::styled(proc_info, Style::default().fg(Color::Magenta)),
            ]);

            f.render_widget(
                Paragraph::new(line),
                Rect { x: inner.x, y, width: inner.width, height: 1 },
            );
            y += 1;
        }
    }
}

/// Draw GPU panel - enhanced with sparklines, thermal gauges, and process info
