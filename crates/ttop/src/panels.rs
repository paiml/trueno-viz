//! Panel rendering for ttop.

use trueno_viz::monitor::ratatui::layout::Rect;
use trueno_viz::monitor::ratatui::style::{Modifier, Style};
use trueno_viz::monitor::ratatui::text::{Line, Span};
use trueno_viz::monitor::ratatui::widgets::{
    Block, Borders, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
};
use trueno_viz::monitor::ratatui::Frame;

use trueno_viz::monitor::collectors::process::ProcessState;
use trueno_viz::monitor::types::Collector;
use trueno_viz::monitor::widgets::{Graph, Meter, MonitorSparkline};

use crate::app::App;
use crate::theme::{self, borders, graph, percent_color, process_state, temp_color};

/// Helper to create a btop-style block with rounded corners
fn btop_block(title: &str, color: trueno_viz::monitor::ratatui::style::Color) -> Block<'_> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(borders::STYLE)
        .border_style(Style::default().fg(color))
}

/// Truncate a string to fit within max_len, adding "..." if truncated
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        s.chars().take(max_len).collect()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Draw CPU panel - btop-style with per-core meters and graph
pub fn draw_cpu(f: &mut Frame, app: &App, area: Rect) {
    let load = app.cpu.load_average();
    let freq = app.cpu.frequencies();
    let max_freq = freq.iter().map(|f| f.current_mhz).max().unwrap_or(0);
    let core_count = app.cpu.core_count();

    // Get max CPU temp if available
    let max_temp = app.sensors.max_temp();
    let temp_str = max_temp.map(|t| format!(" {:.0}°C", t)).unwrap_or_default();

    // Current CPU usage for title
    let cpu_pct = app.cpu_history.last().copied().unwrap_or(0.0) * 100.0;

    let title = format!(
        " CPU {:.0}% │ {} cores │ {:.1}GHz{} │ up {} │ LAV {:.2} ",
        cpu_pct,
        core_count,
        max_freq as f64 / 1000.0,
        temp_str,
        theme::format_uptime(app.cpu.uptime_secs()),
        load.one,
    );

    let block = btop_block(&title, borders::CPU);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 1 || inner.width < 10 {
        return;
    }

    // btop-style layout: per-core meters on left, graph on right
    // Calculate meter width needed for all cores
    let meter_bar_width = 10u16; // "C0 ████████"
    let cores_per_col = inner.height as usize;
    let num_meter_cols = core_count.div_ceil(cores_per_col);
    let meters_width = (num_meter_cols as u16 * meter_bar_width).min(inner.width / 2);

    // Draw per-core meters on left side
    let cpu_temps = app.sensors.cpu_temps();

    for (i, &percent) in app.per_core_percent.iter().enumerate() {
        let col = i / cores_per_col;
        let row = i % cores_per_col;

        let cell_x = inner.x + (col as u16) * meter_bar_width;
        let cell_y = inner.y + row as u16;

        if cell_x + meter_bar_width > inner.x + meters_width || cell_y >= inner.y + inner.height {
            continue;
        }

        let color = percent_color(percent);
        let core_temp = cpu_temps.get(i).map(|t| t.current);

        // btop-style meter: "C0 ████████" with gradient fill
        let bar_len = 6usize;
        let filled = ((percent / 100.0) * bar_len as f64) as usize;
        let bar: String =
            "█".repeat(filled.min(bar_len)) + &"░".repeat(bar_len - filled.min(bar_len));

        // Show temp if available, otherwise just percent
        let label = if let Some(t) = core_temp {
            format!("{:>2} {} {:>2.0}°", i, bar, t)
        } else {
            format!("{:>2} {} {:>3.0}", i, bar, percent)
        };

        f.render_widget(
            Paragraph::new(label).style(Style::default().fg(color)),
            Rect {
                x: cell_x,
                y: cell_y,
                width: meter_bar_width,
                height: 1,
            },
        );
    }

    // Draw graph on right side
    let graph_x = inner.x + meters_width + 1;
    let graph_width = inner.width.saturating_sub(meters_width + 1);

    if graph_width > 3 && !app.cpu_history.is_empty() {
        let graph_area = Rect {
            x: graph_x,
            y: inner.y,
            width: graph_width,
            height: inner.height,
        };
        let cpu_graph = Graph::new(&app.cpu_history)
            .color(graph::CPU)
            .mode(trueno_viz::monitor::widgets::GraphMode::Block);
        f.render_widget(cpu_graph, graph_area);
    }
}

/// Draw Memory panel - btop style, adaptive to available space
/// Enhanced with swap thrashing detection (Denning 1968), ZRAM monitoring, and PSI
pub fn draw_memory(f: &mut Frame, app: &App, area: Rect) {
    use crate::analyzers::PressureLevel;

    let total_gb = app.mem_total as f64 / (1024.0 * 1024.0 * 1024.0);
    let used_gb = app.mem_used as f64 / (1024.0 * 1024.0 * 1024.0);
    let _available_gb = app.mem_available as f64 / (1024.0 * 1024.0 * 1024.0);
    let cached_gb = app.mem_cached as f64 / (1024.0 * 1024.0 * 1024.0);
    let free_gb = app.mem_free as f64 / (1024.0 * 1024.0 * 1024.0);
    let swap_used_gb = app.swap_used as f64 / (1024.0 * 1024.0 * 1024.0);

    // Calculate percentages
    let used_pct = if app.mem_total > 0 {
        (app.mem_used as f64 / app.mem_total as f64) * 100.0
    } else {
        0.0
    };
    let _avail_pct = if app.mem_total > 0 {
        (app.mem_available as f64 / app.mem_total as f64) * 100.0
    } else {
        0.0
    };
    let cached_pct = if app.mem_total > 0 {
        (app.mem_cached as f64 / app.mem_total as f64) * 100.0
    } else {
        0.0
    };
    let free_pct = if app.mem_total > 0 {
        (app.mem_free as f64 / app.mem_total as f64) * 100.0
    } else {
        0.0
    };
    let swap_pct = if app.swap_total > 0 {
        (app.swap_used as f64 / app.swap_total as f64) * 100.0
    } else {
        0.0
    };

    // ZRAM info if available
    let zram_info = if app.has_zram() {
        format!(" │ ZRAM:{:.1}x", app.zram_ratio())
    } else {
        String::new()
    };

    let title = format!(
        " Memory │ {used_gb:.1}G / {total_gb:.1}G ({used_pct:.0}%){zram_info} "
    );

    let block = btop_block(&title, borders::MEMORY);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 1 || inner.width < 10 {
        return;
    }

    use trueno_viz::monitor::ratatui::style::Color;
    use trueno_viz::monitor::ratatui::text::{Line, Span};

    // For very small panels, just show a meter
    if inner.height < 3 {
        let meter = Meter::new(used_pct / 100.0)
            .label(format!("{:.1}G/{:.1}G", used_gb, total_gb))
            .color(percent_color(used_pct));
        f.render_widget(meter, inner);
        return;
    }

    let mut y = inner.y;

    // === LINE 1: Stacked memory bar ===
    // [████████████░░░░░░░░░░░░░░░] Used|Cached|Free
    {
        let bar_width = inner.width as usize;

        // Calculate segment widths (used includes buffers, then cached, then free)
        let used_actual_pct = if app.mem_total > 0 {
            ((app.mem_total - app.mem_available) as f64 / app.mem_total as f64) * 100.0
        } else { 0.0 };

        let used_chars = ((used_actual_pct / 100.0) * bar_width as f64) as usize;
        let cached_chars = ((cached_pct / 100.0) * bar_width as f64) as usize;
        let free_chars = bar_width.saturating_sub(used_chars + cached_chars);

        let mut bar_spans = Vec::new();

        // Used segment (red/yellow based on pressure)
        let used_color = percent_color(used_actual_pct);
        if used_chars > 0 {
            bar_spans.push(Span::styled("█".repeat(used_chars), Style::default().fg(used_color)));
        }

        // Cached segment (cyan)
        if cached_chars > 0 {
            bar_spans.push(Span::styled("█".repeat(cached_chars), Style::default().fg(Color::Cyan)));
        }

        // Free segment (dark/dim)
        if free_chars > 0 {
            bar_spans.push(Span::styled("░".repeat(free_chars), Style::default().fg(Color::DarkGray)));
        }

        f.render_widget(
            Paragraph::new(Line::from(bar_spans)),
            Rect { x: inner.x, y, width: inner.width, height: 1 },
        );
        y += 1;
    }

    // === LINES 2+: Memory rows ===
    struct MemRow<'a> {
        label: &'static str,
        value_gb: f64,
        total_gb: Option<f64>,
        pct: f64,
        history: &'a [f64],
        color: Color,
    }

    let mut rows: Vec<MemRow> = vec![
        MemRow {
            label: "Used",
            value_gb: used_gb,
            total_gb: None,
            pct: used_pct,
            history: &app.mem_history,
            color: percent_color(used_pct),
        },
        MemRow {
            label: "Cached",
            value_gb: cached_gb,
            total_gb: None,
            pct: cached_pct,
            history: &app.mem_cached_history,
            color: Color::Cyan,
        },
        MemRow {
            label: "Free",
            value_gb: free_gb,
            total_gb: None,
            pct: free_pct,
            history: &app.mem_free_history,
            color: Color::Blue,
        },
    ];

    // Insert swap after Used (position 1) if exists
    if app.swap_total > 0 {
        let swap_total_gb = app.swap_total as f64 / (1024.0 * 1024.0 * 1024.0);
        let swap_color = if swap_pct > 50.0 {
            Color::Red
        } else if swap_pct > 10.0 {
            Color::Yellow
        } else {
            Color::Green
        };
        rows.insert(1, MemRow {
            label: "Swap",
            value_gb: swap_used_gb,
            total_gb: Some(swap_total_gb),
            pct: swap_pct,
            history: &app.swap_history,
            color: swap_color,
        });
    }

    // Reserve lines for PSI (1) and Top consumers (1) at bottom
    let reserved_bottom = 2;
    let available_for_rows = (inner.y + inner.height).saturating_sub(y + reserved_bottom) as usize;
    let rows_to_show = rows.len().min(available_for_rows);

    for row in rows.iter().take(rows_to_show) {
        let label_part = if let Some(total) = row.total_gb {
            format!("{:>6}: {:>3.0}/{:.0}G {:>2.0}", row.label, row.value_gb, total, row.pct)
        } else {
            format!("{:>6}: {:>5.1}G {:>2.0}", row.label, row.value_gb, row.pct)
        };
        let label_width = label_part.len() as u16 + 1;
        let sparkline_width = inner.width.saturating_sub(label_width);

        f.render_widget(
            Paragraph::new(label_part).style(Style::default().fg(row.color)),
            Rect { x: inner.x, y, width: label_width, height: 1 },
        );

        if sparkline_width > 3 && !row.history.is_empty() {
            let sparkline = MonitorSparkline::new(row.history)
                .color(row.color)
                .show_trend(true);
            f.render_widget(
                sparkline,
                Rect { x: inner.x + label_width, y, width: sparkline_width, height: 1 },
            );
        }
        y += 1;
    }

    // === PSI Row ===
    if y < inner.y + inner.height && app.psi_analyzer.is_available() {
        let psi = &app.psi_analyzer;
        let level_color = |level: PressureLevel| -> Color {
            match level {
                PressureLevel::None => Color::DarkGray,
                PressureLevel::Low => Color::Green,
                PressureLevel::Medium => Color::Yellow,
                PressureLevel::High => Color::LightRed,
                PressureLevel::Critical => Color::Red,
            }
        };

        let cpu_lvl = psi.cpu_level();
        let mem_lvl = psi.memory_level();
        let io_lvl = psi.io_level();

        let pressure_line = Line::from(vec![
            Span::styled("PSI ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}{:>4.1}", cpu_lvl.symbol(), psi.cpu.some_avg10),
                         Style::default().fg(level_color(cpu_lvl))),
            Span::styled(" cpu ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}{:>4.1}", mem_lvl.symbol(), psi.memory.some_avg10),
                         Style::default().fg(level_color(mem_lvl))),
            Span::styled(" mem ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}{:>4.1}", io_lvl.symbol(), psi.io.some_avg10),
                         Style::default().fg(level_color(io_lvl))),
            Span::styled(" io", Style::default().fg(Color::DarkGray)),
        ]);

        f.render_widget(
            Paragraph::new(pressure_line),
            Rect { x: inner.x, y, width: inner.width, height: 1 },
        );
        y += 1;
    }

    // === Top Memory Consumers Row ===
    if y < inner.y + inner.height {
        // Get top 3 processes by memory
        let mut procs: Vec<_> = app.process.processes().values().collect();
        procs.sort_by(|a, b| b.mem_bytes.cmp(&a.mem_bytes));

        let mut spans = vec![Span::styled("Top:", Style::default().fg(Color::DarkGray))];

        for proc in procs.iter().take(3) {
            let mem_gb = proc.mem_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            let name: String = proc.name.chars().take(10).collect();
            spans.push(Span::raw(" "));
            spans.push(Span::styled(name, Style::default().fg(Color::White)));
            spans.push(Span::styled(
                format!(" {:.1}G", mem_gb),
                Style::default().fg(Color::Magenta),
            ));
            spans.push(Span::styled(" │", Style::default().fg(Color::DarkGray)));
        }

        // Remove trailing separator
        if !procs.is_empty() {
            spans.pop();
        }

        f.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect { x: inner.x, y, width: inner.width, height: 1 },
        );
    }
}

/// Draw Disk panel - enhanced with Little's Law latency estimation
/// and Ruemmler & Wilkes (1994) workload classification
pub fn draw_disk(f: &mut Frame, app: &App, area: Rect) {
    let mounts = app.disk.mounts();
    let rates = app.disk.rates();

    // Calculate total I/O rates
    let total_read: f64 = rates.values().map(|r| r.read_bytes_per_sec).sum();
    let total_write: f64 = rates.values().map(|r| r.write_bytes_per_sec).sum();

    // Get I/O latency estimate from disk_io_analyzer (Little's Law)
    let latency_info = if let Some(device) = app.disk_io_analyzer.primary_device() {
        let latency = app.disk_io_analyzer.estimated_latency_ms(&device);
        let workload = app.disk_io_analyzer.workload_type(&device);
        if latency > 0.1 {
            format!(" │ {:.1}ms {}", latency, workload.description())
        } else {
            format!(" │ {}", workload.description())
        }
    } else {
        // Always show overall workload even without specific device
        let workload = app.disk_io_analyzer.overall_workload();
        format!(" │ {}", workload.description())
    };

    let title = format!(
        " Disk │ R: {} │ W: {}{} ",
        theme::format_bytes_rate(total_read),
        theme::format_bytes_rate(total_write),
        latency_info
    );

    let block = btop_block(&title, borders::DISK);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 1 {
        return;
    }

    // Use 2 lines per mount if we have space, otherwise 1
    let lines_per_mount = if inner.height >= (mounts.len() as u16 * 2) { 2 } else { 1 };
    let max_mounts = (inner.height as usize) / lines_per_mount.max(1) as usize;

    let mut y = inner.y;
    for mount in mounts.iter().take(max_mounts) {
        if mount.total_bytes == 0 || y >= inner.y + inner.height {
            continue;
        }

        let used_pct = mount.usage_percent();
        let total_gb = mount.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let used_gb = mount.used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

        // Short mount point label
        let label: String = if mount.mount_point == "/" {
            "/".to_string()
        } else {
            mount
                .mount_point
                .rsplit('/')
                .next()
                .unwrap_or(&mount.mount_point)
                .chars()
                .take(8)
                .collect()
        };

        // Find I/O rate for this device
        let device_name = mount.device.rsplit('/').next().unwrap_or("");
        let io_info = rates.get(device_name).or_else(|| {
            let base: String = device_name
                .chars()
                .take_while(|c| !c.is_ascii_digit())
                .collect();
            rates.get(&base)
        });

        let color = percent_color(used_pct);

        if lines_per_mount >= 2 && y + 1 < inner.y + inner.height {
            // Two-line format: more visual
            // Line 1: label with big bar
            let meter = Meter::new(used_pct / 100.0).label(&label).color(color);
            f.render_widget(meter, Rect { x: inner.x, y, width: inner.width, height: 1 });
            y += 1;

            // Line 2: size info + I/O sparklines
            let size_str = if total_gb >= 1000.0 {
                format!("{:.1}T/{:.1}T", used_gb / 1024.0, total_gb / 1024.0)
            } else if total_gb >= 1.0 {
                format!("{:.0}G/{:.0}G", used_gb, total_gb)
            } else {
                format!("{:.0}M/{:.0}M", used_gb * 1024.0, total_gb * 1024.0)
            };

            // Get base device name for disk_io_analyzer (strip partition number)
            // For nvme0n1p5, base is nvme0n1. For sda1, base is sda
            let base_device: String = if device_name.contains("nvme") {
                // nvme0n1p5 -> nvme0n1 (strip pN)
                device_name.split('p').next().unwrap_or(device_name).to_string()
            } else {
                // sda1 -> sda (strip digits at end)
                device_name
                    .chars()
                    .take_while(|c| !c.is_ascii_digit())
                    .collect()
            };

            // Get I/O histories from disk_io_analyzer
            let read_history = app.disk_io_analyzer.device_read_history(&base_device);
            let write_history = app.disk_io_analyzer.device_write_history(&base_device);

            // Render size + I/O rates as label, then sparklines
            let rate_str = if let Some(io) = io_info {
                format!("R:{} W:{}",
                    theme::format_bytes_rate(io.read_bytes_per_sec),
                    theme::format_bytes_rate(io.write_bytes_per_sec))
            } else {
                String::new()
            };
            let label_str = format!("  {} {}", size_str, rate_str);
            let label_width = label_str.len() as u16;
            let sparkline_width = inner.width.saturating_sub(label_width + 1);

            // Render label
            f.render_widget(
                Paragraph::new(label_str)
                    .style(Style::default().fg(trueno_viz::monitor::ratatui::style::Color::DarkGray)),
                Rect { x: inner.x, y, width: label_width, height: 1 },
            );

            // Render I/O sparkline (read + write combined or stacked)
            if sparkline_width > 4 {
                if let Some(ref rh) = read_history {
                    if !rh.is_empty() {
                        // Show read sparkline in cyan
                        let sparkline = MonitorSparkline::new(rh)
                            .color(trueno_viz::monitor::ratatui::style::Color::Cyan)
                            .show_trend(false);
                        f.render_widget(
                            sparkline,
                            Rect { x: inner.x + label_width, y, width: sparkline_width / 2, height: 1 },
                        );
                    }
                }
                if let Some(ref wh) = write_history {
                    if !wh.is_empty() {
                        // Show write sparkline in magenta
                        let sparkline = MonitorSparkline::new(wh)
                            .color(trueno_viz::monitor::ratatui::style::Color::Magenta)
                            .show_trend(false);
                        f.render_widget(
                            sparkline,
                            Rect { x: inner.x + label_width + sparkline_width / 2, y, width: sparkline_width / 2, height: 1 },
                        );
                    }
                }
            }
            y += 1;
        } else {
            // Single-line compact format
            let size_str = if total_gb >= 1000.0 {
                format!("{:.1}T", total_gb / 1024.0)
            } else {
                format!("{:.0}G", total_gb)
            };
            let compact_label = format!("{} {}", label, size_str);
            let meter = Meter::new(used_pct / 100.0).label(&compact_label).color(color);
            f.render_widget(meter, Rect { x: inner.x, y, width: inner.width, height: 1 });
            y += 1;
        }
    }
}

/// Draw Network panel - btop style with dual graphs and session totals
pub fn draw_network(f: &mut Frame, app: &App, area: Rect) {
    let iface = app.network.current_interface().unwrap_or("none");
    let (rx_rate, tx_rate) = app
        .network
        .current_interface()
        .and_then(|i| app.network.all_rates().get(i))
        .map(|r| (r.rx_bytes_per_sec, r.tx_bytes_per_sec))
        .unwrap_or((0.0, 0.0));

    let title = format!(
        " Network ({}) │ ↓ {} │ ↑ {} ",
        iface,
        theme::format_bytes_rate(rx_rate),
        theme::format_bytes_rate(tx_rate)
    );

    let block = btop_block(&title, borders::NETWORK);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 2 {
        return;
    }

    // Calculate layout:
    // - Row for RX info with sparkline (1 line)
    // - RX Graph (variable)
    // - Row for TX info with sparkline (1 line)
    // - TX Graph (variable)
    // - Session totals row (1 line) if space

    let show_totals = inner.height >= 8;
    let info_lines = 2; // RX + TX info rows
    let totals_line = if show_totals { 1 } else { 0 };
    let graph_total = inner.height.saturating_sub(info_lines + totals_line);
    let half_height = graph_total / 2;

    let mut y = inner.y;

    // RX info line with rate and sparkline
    {
        let label_width = 16u16;
        let sparkline_width = inner.width.saturating_sub(label_width);

        let rx_label = Line::from(vec![
            Span::styled("↓ Download ", Style::default().fg(graph::NETWORK_RX)),
            Span::styled(
                theme::format_bytes_rate(rx_rate),
                Style::default()
                    .fg(trueno_viz::monitor::ratatui::style::Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        f.render_widget(
            Paragraph::new(rx_label),
            Rect {
                x: inner.x,
                y,
                width: label_width,
                height: 1,
            },
        );

        // RX sparkline
        if sparkline_width > 2 && !app.net_rx_history.is_empty() {
            let sparkline = MonitorSparkline::new(&app.net_rx_history)
                .color(graph::NETWORK_RX)
                .show_trend(true);
            f.render_widget(
                sparkline,
                Rect {
                    x: inner.x + label_width,
                    y,
                    width: sparkline_width,
                    height: 1,
                },
            );
        }
        y += 1;
    }

    // Download graph
    if half_height > 0 {
        let rx_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: half_height,
        };
        let rx_data: Vec<f64> = if app.net_rx_history.is_empty() {
            vec![0.0]
        } else {
            app.net_rx_history.clone()
        };
        let rx_graph = Graph::new(&rx_data).color(graph::NETWORK_RX);
        f.render_widget(rx_graph, rx_area);
        y += half_height;
    }

    // TX info line with rate and sparkline
    {
        let label_width = 16u16;
        let sparkline_width = inner.width.saturating_sub(label_width);

        let tx_label = Line::from(vec![
            Span::styled("↑ Upload   ", Style::default().fg(graph::NETWORK_TX)),
            Span::styled(
                theme::format_bytes_rate(tx_rate),
                Style::default()
                    .fg(trueno_viz::monitor::ratatui::style::Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        f.render_widget(
            Paragraph::new(tx_label),
            Rect {
                x: inner.x,
                y,
                width: label_width,
                height: 1,
            },
        );

        // TX sparkline
        if sparkline_width > 2 && !app.net_tx_history.is_empty() {
            let sparkline = MonitorSparkline::new(&app.net_tx_history)
                .color(graph::NETWORK_TX)
                .show_trend(true);
            f.render_widget(
                sparkline,
                Rect {
                    x: inner.x + label_width,
                    y,
                    width: sparkline_width,
                    height: 1,
                },
            );
        }
        y += 1;
    }

    // Upload graph (inverted)
    let remaining_for_graph = if show_totals {
        (inner.y + inner.height - 1).saturating_sub(y)
    } else {
        (inner.y + inner.height).saturating_sub(y)
    };

    if remaining_for_graph > 0 {
        let tx_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: remaining_for_graph,
        };
        let tx_data: Vec<f64> = if app.net_tx_history.is_empty() {
            vec![0.0]
        } else {
            app.net_tx_history.clone()
        };
        let tx_graph = Graph::new(&tx_data).color(graph::NETWORK_TX).inverted(true);
        f.render_widget(tx_graph, tx_area);
        y += remaining_for_graph;
    }

    // Session totals
    if show_totals && y < inner.y + inner.height {
        let totals_line = Line::from(vec![
            Span::styled(
                "Session: ",
                Style::default().fg(trueno_viz::monitor::ratatui::style::Color::DarkGray),
            ),
            Span::styled("↓ ", Style::default().fg(graph::NETWORK_RX)),
            Span::styled(
                theme::format_bytes(app.net_rx_total),
                Style::default().fg(trueno_viz::monitor::ratatui::style::Color::White),
            ),
            Span::styled(" │ ", Style::default().fg(trueno_viz::monitor::ratatui::style::Color::DarkGray)),
            Span::styled("↑ ", Style::default().fg(graph::NETWORK_TX)),
            Span::styled(
                theme::format_bytes(app.net_tx_total),
                Style::default().fg(trueno_viz::monitor::ratatui::style::Color::White),
            ),
        ]);
        f.render_widget(
            Paragraph::new(totals_line),
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
    }
}

/// Draw GPU panel - supports multiple GPUs
pub fn draw_gpu(f: &mut Frame, app: &App, area: Rect) {
    use trueno_viz::monitor::ratatui::style::Color;

    // Collect GPU info into a unified structure
    struct GpuDisplay {
        name: String,
        gpu_util: f64,
        vram_pct: f64,
        temp: f64,
        power: u32,
    }

    let mut gpus: Vec<GpuDisplay> = Vec::new();

    #[cfg(feature = "nvidia")]
    if app.nvidia_gpu.is_available() {
        for gpu in app.nvidia_gpu.gpus() {
            let vram_pct = if gpu.mem_total > 0 {
                gpu.mem_used as f64 / gpu.mem_total as f64
            } else {
                0.0
            };
            gpus.push(GpuDisplay {
                name: gpu.name.clone(),
                gpu_util: gpu.gpu_util,
                vram_pct,
                temp: gpu.temperature,
                power: gpu.power_mw / 1000,
            });
        }
    }

    #[cfg(target_os = "linux")]
    if app.amd_gpu.is_available() {
        for gpu in app.amd_gpu.gpus() {
            let vram_pct = if gpu.vram_total > 0 {
                gpu.vram_used as f64 / gpu.vram_total as f64
            } else {
                0.0
            };
            gpus.push(GpuDisplay {
                name: gpu.name.clone(),
                gpu_util: gpu.gpu_util,
                vram_pct,
                temp: gpu.temperature,
                power: gpu.power_watts as u32,
            });
        }
    }

    #[cfg(target_os = "macos")]
    if app.apple_gpu.is_available() {
        for gpu in app.apple_gpu.gpus() {
            gpus.push(GpuDisplay {
                name: gpu.name.clone(),
                gpu_util: gpu.gpu_util,
                vram_pct: 0.0, // Apple GPUs don't report VRAM via IOKit
                temp: 0.0,
                power: 0,
            });
        }
    }

    // Build title showing GPU count
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

    // Calculate rows per GPU: util + vram + info = 3 rows each (or 2 if tight)
    let rows_per_gpu = if inner.height as usize >= gpus.len() * 3 {
        3
    } else if inner.height as usize >= gpus.len() * 2 {
        2
    } else {
        1
    };

    let mut y_offset = 0u16;

    for (i, gpu) in gpus.iter().enumerate() {
        if y_offset >= inner.height {
            break;
        }

        // GPU label for multi-GPU systems
        let label = if gpus.len() > 1 {
            format!("GPU{}", i)
        } else {
            "GPU".to_string()
        };

        // GPU utilization meter
        let gpu_color = percent_color(gpu.gpu_util);
        let gpu_meter = Meter::new(gpu.gpu_util / 100.0)
            .label(&label)
            .color(gpu_color);
        f.render_widget(
            gpu_meter,
            Rect {
                x: inner.x,
                y: inner.y + y_offset,
                width: inner.width,
                height: 1,
            },
        );
        y_offset += 1;

        // VRAM meter (if space and VRAM data available)
        if rows_per_gpu >= 2 && y_offset < inner.height && gpu.vram_pct > 0.0 {
            let vram_label = if gpus.len() > 1 {
                format!("VRM{}", i)
            } else {
                "VRAM".to_string()
            };
            let vram_color = percent_color(gpu.vram_pct * 100.0);
            let vram_meter = Meter::new(gpu.vram_pct)
                .label(&vram_label)
                .color(vram_color);
            f.render_widget(
                vram_meter,
                Rect {
                    x: inner.x,
                    y: inner.y + y_offset,
                    width: inner.width,
                    height: 1,
                },
            );
            y_offset += 1;
        }

        // Temperature and power info (if space)
        if rows_per_gpu >= 3 && y_offset < inner.height && gpu.temp > 0.0 {
            let info = if gpus.len() > 1 {
                format!("GPU{}: {}°C │ {}W", i, gpu.temp as u32, gpu.power)
            } else {
                format!("Temp: {}°C │ Power: {}W", gpu.temp as u32, gpu.power)
            };
            let info_para = Paragraph::new(info).style(Style::default().fg(temp_color(gpu.temp)));
            f.render_widget(
                info_para,
                Rect {
                    x: inner.x,
                    y: inner.y + y_offset,
                    width: inner.width,
                    height: 1,
                },
            );
            y_offset += 1;
        }
    }

    // GPU Processes (if space available)
    if y_offset < inner.height && app.gpu_process_analyzer.is_available() {
        let procs = app.gpu_process_analyzer.top_processes(4);
        if !procs.is_empty() {
            // Add a divider line if we have room
            if y_offset + 1 < inner.height {
                let divider = "─".repeat(inner.width as usize);
                f.render_widget(
                    Paragraph::new(divider).style(Style::default().fg(Color::DarkGray)),
                    Rect {
                        x: inner.x,
                        y: inner.y + y_offset,
                        width: inner.width,
                        height: 1,
                    },
                );
                y_offset += 1;
            }

            // Show top GPU processes
            for proc in procs {
                if y_offset >= inner.height {
                    break;
                }

                // Color based on SM utilization
                let color = if proc.sm_util >= 50 {
                    Color::LightRed
                } else if proc.sm_util >= 20 {
                    Color::LightYellow
                } else {
                    Color::DarkGray
                };

                // Type indicator and process info
                let proc_line = format!(
                    "{} {:>3}% {:>3}% {}",
                    proc.proc_type,
                    proc.sm_util,
                    proc.mem_util,
                    truncate_str(&proc.command, (inner.width as usize).saturating_sub(12))
                );

                f.render_widget(
                    Paragraph::new(proc_line).style(Style::default().fg(color)),
                    Rect {
                        x: inner.x,
                        y: inner.y + y_offset,
                        width: inner.width,
                        height: 1,
                    },
                );
                y_offset += 1;
            }
        }
    }
}

/// Draw Battery panel
pub fn draw_battery(f: &mut Frame, app: &App, area: Rect) {
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

/// Draw Sensors/Temperature panel
pub fn draw_sensors(f: &mut Frame, app: &App, area: Rect) {
    let temps = app.sensors.readings();
    let max_temp = app.sensors.max_temp().unwrap_or(0.0);

    let title = format!(" Sensors │ Max: {:.0}°C ", max_temp);

    let block = btop_block(&title, borders::SENSORS);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 1 {
        return;
    }

    // Show temperature readings
    for (i, temp) in temps.iter().take(inner.height as usize).enumerate() {
        let label: String = temp.label.chars().take(12).collect();
        let color = temp_color(temp.current);
        let line = Line::from(vec![
            Span::styled(format!("{label:12}"), Style::default()),
            Span::styled(
                format!(" {:.0}°C", temp.current),
                Style::default().fg(color),
            ),
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
}

/// Draw compact sensors panel (single line)
pub fn draw_sensors_compact(f: &mut Frame, app: &App, area: Rect) {
    use trueno_viz::monitor::ratatui::style::Color;

    let temps = app.sensors.readings();

    // Find key temperatures: CPU, GPU, NVMe
    let cpu_temp = temps.iter()
        .find(|t| t.label.to_lowercase().contains("cpu") || t.label.to_lowercase().contains("core"))
        .map(|t| t.current);
    let gpu_temp = temps.iter()
        .find(|t| t.label.to_lowercase().contains("gpu") || t.label.to_lowercase().contains("edge"))
        .map(|t| t.current);
    let nvme_temp = temps.iter()
        .find(|t| t.label.to_lowercase().contains("nvme") || t.label.to_lowercase().contains("composite"))
        .map(|t| t.current);

    let max_temp = app.sensors.max_temp().unwrap_or(0.0);
    let title = format!(" Sensors │ {:.0}°C ", max_temp);

    let block = btop_block(&title, borders::SENSORS);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 1 || inner.width < 10 {
        return;
    }

    // Build compact sensor line
    let mut spans = Vec::new();

    if let Some(t) = cpu_temp {
        spans.push(Span::styled("CPU ", Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled(format!("{:.0}°", t), Style::default().fg(temp_color(t))));
        spans.push(Span::raw("  "));
    }
    if let Some(t) = gpu_temp {
        spans.push(Span::styled("GPU ", Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled(format!("{:.0}°", t), Style::default().fg(temp_color(t))));
        spans.push(Span::raw("  "));
    }
    if let Some(t) = nvme_temp {
        spans.push(Span::styled("NVMe ", Style::default().fg(Color::DarkGray)));
        spans.push(Span::styled(format!("{:.0}°", t), Style::default().fg(temp_color(t))));
    }

    f.render_widget(
        Paragraph::new(Line::from(spans)),
        Rect { x: inner.x, y: inner.y, width: inner.width, height: 1 },
    );
}

/// Draw PSI (Pressure Stall Information) panel
pub fn draw_psi(f: &mut Frame, app: &App, area: Rect) {
    use trueno_viz::monitor::ratatui::style::Color;
    use crate::analyzers::PressureLevel;

    let psi = &app.psi_analyzer;

    let overall = psi.overall_level();
    let title = format!(" Pressure │ {} ", overall.symbol());

    // Color based on overall pressure
    let border_color = match overall {
        PressureLevel::None => Color::Rgb(80, 120, 80),
        PressureLevel::Low => Color::Rgb(120, 120, 80),
        PressureLevel::Medium => Color::Rgb(180, 140, 60),
        PressureLevel::High => Color::Rgb(200, 100, 60),
        PressureLevel::Critical => Color::Rgb(200, 60, 60),
    };

    let block = btop_block(&title, border_color);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 1 || !psi.is_available() {
        return;
    }

    // Helper to get color for pressure level
    let level_color = |level: PressureLevel| -> Color {
        match level {
            PressureLevel::None => Color::DarkGray,
            PressureLevel::Low => Color::Green,
            PressureLevel::Medium => Color::Yellow,
            PressureLevel::High => Color::LightRed,
            PressureLevel::Critical => Color::Red,
        }
    };

    // Show CPU, Memory, I/O pressure
    let cpu_lvl = psi.cpu_level();
    let mem_lvl = psi.memory_level();
    let io_lvl = psi.io_level();

    let line = Line::from(vec![
        Span::styled("CPU ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{} {:>4.1}%", cpu_lvl.symbol(), psi.cpu.some_avg10),
                     Style::default().fg(level_color(cpu_lvl))),
        Span::raw("  "),
        Span::styled("MEM ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{} {:>4.1}%", mem_lvl.symbol(), psi.memory.some_avg10),
                     Style::default().fg(level_color(mem_lvl))),
        Span::raw("  "),
        Span::styled("I/O ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{} {:>4.1}%", io_lvl.symbol(), psi.io.some_avg10),
                     Style::default().fg(level_color(io_lvl))),
    ]);

    f.render_widget(
        Paragraph::new(line),
        Rect { x: inner.x, y: inner.y, width: inner.width, height: 1 },
    );

    // If more space, show "full" stall percentages on second line
    if inner.height >= 2 {
        let full_line = Line::from(vec![
            Span::styled("Full stall: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("CPU {:.1}%", psi.cpu.full_avg10),
                         Style::default().fg(if psi.cpu.full_avg10 > 5.0 { Color::Yellow } else { Color::DarkGray })),
            Span::raw("  "),
            Span::styled(format!("MEM {:.1}%", psi.memory.full_avg10),
                         Style::default().fg(if psi.memory.full_avg10 > 5.0 { Color::Yellow } else { Color::DarkGray })),
            Span::raw("  "),
            Span::styled(format!("I/O {:.1}%", psi.io.full_avg10),
                         Style::default().fg(if psi.io.full_avg10 > 5.0 { Color::Yellow } else { Color::DarkGray })),
        ]);

        f.render_widget(
            Paragraph::new(full_line),
            Rect { x: inner.x, y: inner.y + 1, width: inner.width, height: 1 },
        );
    }
}

/// Draw combined System panel: Sensors + Containers stacked vertically
/// (PSI is now shown in the Memory panel)
pub fn draw_system(f: &mut Frame, app: &App, area: Rect) {
    use trueno_viz::monitor::ratatui::layout::{Layout, Direction, Constraint};

    // Determine what components are available
    let has_sensors = app.sensors.is_available();
    let has_containers = app.container_analyzer.is_available();

    // Calculate heights
    let sensor_height = if has_sensors { 3 } else { 0 }; // border + 1 line
    let container_height = area.height.saturating_sub(sensor_height);

    let mut constraints = Vec::new();
    if has_sensors {
        constraints.push(Constraint::Length(sensor_height));
    }
    if has_containers && container_height > 2 {
        constraints.push(Constraint::Min(3));
    }

    if constraints.is_empty() {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut chunk_idx = 0;

    // Sensors (compact)
    if has_sensors && chunk_idx < chunks.len() {
        draw_sensors_compact(f, app, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // Containers
    if has_containers && chunk_idx < chunks.len() && container_height > 2 {
        draw_containers_inner(f, app, chunks[chunk_idx]);
    }
}

/// Draw Container/Docker panel (internal)
fn draw_containers_inner(f: &mut Frame, app: &App, area: Rect) {
    use trueno_viz::monitor::ratatui::style::Color;

    let analyzer = &app.container_analyzer;

    let running = analyzer.running_count();
    let total = analyzer.total_count();

    let title = if total > 0 {
        format!(" Containers │ {}/{} ", running, total)
    } else {
        " Containers ".to_string()
    };

    let block = btop_block(&title, Color::Rgb(80, 140, 180));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 1 {
        return;
    }

    if !analyzer.is_available() {
        f.render_widget(
            Paragraph::new("Docker not available").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    let containers = analyzer.top_containers(inner.height as usize);

    if containers.is_empty() {
        f.render_widget(
            Paragraph::new("No running containers").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    for (i, c) in containers.iter().enumerate() {
        if i as u16 >= inner.height {
            break;
        }

        // Color based on CPU usage
        let color = if c.cpu_pct >= 50.0 {
            Color::LightRed
        } else if c.cpu_pct >= 10.0 {
            Color::Yellow
        } else {
            Color::Green
        };

        // Format memory
        let mem_str = if c.mem_used >= 1024 * 1024 * 1024 {
            format!("{:.1}G", c.mem_used as f64 / (1024.0 * 1024.0 * 1024.0))
        } else {
            format!("{:.0}M", c.mem_used as f64 / (1024.0 * 1024.0))
        };

        // Truncate name to fit
        let max_name_len = (inner.width as usize).saturating_sub(18);
        let name = truncate_str(&c.name, max_name_len);

        let line = Line::from(vec![
            Span::styled(c.status.symbol(), Style::default().fg(Color::Green)),
            Span::raw(" "),
            Span::styled(format!("{:>5.1}%", c.cpu_pct), Style::default().fg(color)),
            Span::raw(" "),
            Span::styled(format!("{:>5}", mem_str), Style::default().fg(Color::Cyan)),
            Span::raw(" "),
            Span::styled(name, Style::default().fg(Color::White)),
        ]);

        f.render_widget(
            Paragraph::new(line),
            Rect { x: inner.x, y: inner.y + i as u16, width: inner.width, height: 1 },
        );
    }
}

/// Draw Process panel - btop style with mini CPU bars and optional tree view
pub fn draw_process(f: &mut Frame, app: &mut App, area: Rect) {
    let sorted = app.sorted_processes();
    let count = sorted.len();

    let sort_indicator = app.sort_column.name();
    let direction = if app.sort_descending { "▼" } else { "▲" };
    let filter_info = if !app.filter.is_empty() {
        format!(" │ Filter: \"{}\"", app.filter)
    } else {
        String::new()
    };
    let tree_info = if app.show_tree { " │ 🌲 Tree" } else { "" };

    let title = format!(
        " Processes ({}) │ Sort: {} {}{}{} ",
        count, sort_indicator, direction, filter_info, tree_info
    );

    let block = btop_block(&title, borders::PROCESS);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 2 {
        return;
    }

    // Header with mini bar column
    let header_cells = ["PID", "USER", "S", "CPU", "", "MEM%", "NAME", "COMMAND"];
    let header = Row::new(header_cells.iter().map(|h| {
        let style = if *h == app.sort_column.name()
            || (*h == "S" && app.sort_column == crate::state::ProcessSortColumn::State)
            || (*h == "CPU" && app.sort_column == crate::state::ProcessSortColumn::Cpu)
        {
            Style::default()
                .fg(borders::PROCESS)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default()
                .fg(borders::PROCESS)
                .add_modifier(Modifier::BOLD)
        };
        Span::styled(*h, style)
    }))
    .height(1);

    // Helper function to create mini CPU bar
    fn mini_cpu_bar(percent: f64) -> String {
        // 5-char bar using block chars
        let filled = ((percent / 100.0) * 5.0) as usize;
        let partial = ((percent / 100.0) * 5.0 - filled as f64) * 8.0;
        let partial_char = match partial as usize {
            0 => ' ',
            1 => '▏',
            2 => '▎',
            3 => '▍',
            4 => '▌',
            5 => '▋',
            6 => '▊',
            7 => '▉',
            _ => '█',
        };
        let mut bar = "█".repeat(filled.min(5));
        if filled < 5 {
            bar.push(partial_char);
            while bar.chars().count() < 5 {
                bar.push(' ');
            }
        }
        bar
    }

    // Build tree structure if tree view enabled
    let tree_prefixes: std::collections::HashMap<u32, String> = if app.show_tree {
        let tree = app.process.build_tree();
        let mut prefixes = std::collections::HashMap::new();

        fn build_prefixes(
            tree: &std::collections::BTreeMap<u32, Vec<u32>>,
            prefixes: &mut std::collections::HashMap<u32, String>,
            parent: u32,
            prefix: &str,
            _is_last: bool,
        ) {
            if let Some(children) = tree.get(&parent) {
                let count = children.len();
                for (i, &child) in children.iter().enumerate() {
                    let is_last_child = i == count - 1;
                    let branch = if is_last_child { "└─" } else { "├─" };
                    let child_prefix = format!("{}{}", prefix, branch);
                    prefixes.insert(child, child_prefix.clone());

                    let next_prefix = if is_last_child {
                        format!("{}  ", prefix)
                    } else {
                        format!("{}│ ", prefix)
                    };
                    build_prefixes(tree, prefixes, child, &next_prefix, is_last_child);
                }
            }
        }

        // Start from init processes (ppid = 0 or 1)
        build_prefixes(&tree, &mut prefixes, 0, "", false);
        build_prefixes(&tree, &mut prefixes, 1, "", false);
        prefixes
    } else {
        std::collections::HashMap::new()
    };

    // Rows with mini CPU bars
    let rows: Vec<Row> = sorted
        .iter()
        .map(|p| {
            let state_color = match p.state {
                ProcessState::Running => process_state::RUNNING,
                ProcessState::Sleeping => process_state::SLEEPING,
                ProcessState::DiskWait => process_state::DISK_WAIT,
                ProcessState::Zombie => process_state::ZOMBIE,
                ProcessState::Stopped => process_state::STOPPED,
                _ => process_state::UNKNOWN,
            };

            let cpu_color = percent_color(p.cpu_percent);
            let mem_color = percent_color(p.mem_percent);

            // Tree prefix for name if tree view enabled
            let tree_prefix = tree_prefixes.get(&p.pid).cloned().unwrap_or_default();
            let name_with_tree = if app.show_tree {
                format!(
                    "{}{}",
                    tree_prefix,
                    p.name
                        .chars()
                        .take(15 - tree_prefix.chars().count())
                        .collect::<String>()
                )
            } else {
                p.name.chars().take(15).collect()
            };

            Row::new(vec![
                Span::styled(format!("{:>6}", p.pid), Style::default()),
                Span::styled(
                    format!("{:8}", p.user.chars().take(8).collect::<String>()),
                    Style::default().fg(trueno_viz::monitor::ratatui::style::Color::Blue),
                ),
                Span::styled(
                    format!("{}", p.state.as_char()),
                    Style::default().fg(state_color),
                ),
                Span::styled(
                    format!("{:>4.0}", p.cpu_percent),
                    Style::default().fg(cpu_color),
                ),
                Span::styled(mini_cpu_bar(p.cpu_percent), Style::default().fg(cpu_color)),
                Span::styled(
                    format!("{:>5.1}", p.mem_percent),
                    Style::default().fg(mem_color),
                ),
                Span::styled(
                    format!("{:15}", name_with_tree),
                    Style::default().fg(trueno_viz::monitor::ratatui::style::Color::White),
                ),
                Span::styled(
                    p.cmdline.chars().take(50).collect::<String>(),
                    Style::default().fg(trueno_viz::monitor::ratatui::style::Color::DarkGray),
                ),
            ])
        })
        .collect();

    let widths = [
        trueno_viz::monitor::ratatui::layout::Constraint::Length(7),  // PID
        trueno_viz::monitor::ratatui::layout::Constraint::Length(9),  // USER
        trueno_viz::monitor::ratatui::layout::Constraint::Length(2),  // State
        trueno_viz::monitor::ratatui::layout::Constraint::Length(5),  // CPU%
        trueno_viz::monitor::ratatui::layout::Constraint::Length(5),  // CPU bar
        trueno_viz::monitor::ratatui::layout::Constraint::Length(6),  // MEM%
        trueno_viz::monitor::ratatui::layout::Constraint::Length(16), // NAME (with tree)
        trueno_viz::monitor::ratatui::layout::Constraint::Min(20),    // COMMAND
    ];

    let mut table_state = trueno_viz::monitor::ratatui::widgets::TableState::default();
    table_state.select(Some(app.process_selected));

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(
            Style::default()
                .bg(trueno_viz::monitor::ratatui::style::Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(table, inner, &mut table_state);

    // Scrollbar
    if count > inner.height as usize {
        let mut scroll_state = ScrollbarState::default()
            .content_length(count)
            .position(app.process_selected);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));
        let scrollbar_area = Rect {
            x: area.x + area.width - 1,
            y: area.y + 1,
            width: 1,
            height: area.height - 2,
        };
        f.render_stateful_widget(scrollbar, scrollbar_area, &mut scroll_state);
    }
}

/// Draw Network Connections panel - Little Snitch style
pub fn draw_connections(f: &mut Frame, app: &App, area: Rect) {
    use crate::analyzers::{ConnState, Protocol};

    let conns = app.connection_analyzer.connections();
    let active_count = conns.iter().filter(|c| c.state == ConnState::Established).count();
    let listen_count = conns.iter().filter(|c| c.state == ConnState::Listen).count();

    let title = format!(" Connections │ {} active │ {} listen ", active_count, listen_count);

    let block = btop_block(&title, borders::NETWORK);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 2 {
        return;
    }

    // Sort: established first, then by remote port
    let mut sorted_conns: Vec<_> = conns.iter().collect();
    sorted_conns.sort_by(|a, b| {
        match (a.state == ConnState::Established, b.state == ConnState::Established) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.remote_port.cmp(&b.remote_port),
        }
    });

    // Header
    let header = Row::new(vec![
        Span::styled("PROTO", Style::default().fg(borders::NETWORK).add_modifier(Modifier::BOLD)),
        Span::styled("LOCAL", Style::default().fg(borders::NETWORK).add_modifier(Modifier::BOLD)),
        Span::styled("REMOTE", Style::default().fg(borders::NETWORK).add_modifier(Modifier::BOLD)),
        Span::styled("ST", Style::default().fg(borders::NETWORK).add_modifier(Modifier::BOLD)),
        Span::styled("PROCESS", Style::default().fg(borders::NETWORK).add_modifier(Modifier::BOLD)),
    ]).height(1);

    // Connection rows
    let rows: Vec<Row> = sorted_conns
        .iter()
        .take(inner.height.saturating_sub(1) as usize)
        .map(|conn| {
            let proto_str = match conn.protocol {
                Protocol::Tcp => "TCP",
                Protocol::Udp => "UDP",
            };
            let proto_color = match conn.protocol {
                Protocol::Tcp => trueno_viz::monitor::ratatui::style::Color::Cyan,
                Protocol::Udp => trueno_viz::monitor::ratatui::style::Color::Yellow,
            };

            let state_color = match conn.state {
                ConnState::Established => trueno_viz::monitor::ratatui::style::Color::Green,
                ConnState::Listen => trueno_viz::monitor::ratatui::style::Color::Blue,
                ConnState::TimeWait | ConnState::CloseWait => trueno_viz::monitor::ratatui::style::Color::Yellow,
                _ => trueno_viz::monitor::ratatui::style::Color::DarkGray,
            };

            // Get process name for this connection
            let proc_name = app.connection_analyzer
                .process_for_connection(conn)
                .map(|(_, name)| name.to_string())
                .unwrap_or_else(|| "-".to_string());

            // Format addresses (truncate if needed)
            let local = format!(":{}", conn.local_port);
            let remote = if conn.remote_ip.is_unspecified() {
                "*".to_string()
            } else {
                format!("{}:{}", conn.remote_ip, conn.remote_port)
            };

            Row::new(vec![
                Span::styled(proto_str, Style::default().fg(proto_color)),
                Span::styled(local, Style::default().fg(trueno_viz::monitor::ratatui::style::Color::White)),
                Span::styled(format!("{:>21}", remote), Style::default().fg(trueno_viz::monitor::ratatui::style::Color::White)),
                Span::styled(format!("{}", conn.state.as_char()), Style::default().fg(state_color)),
                Span::styled(proc_name.chars().take(12).collect::<String>(), Style::default().fg(trueno_viz::monitor::ratatui::style::Color::Magenta)),
            ])
        })
        .collect();

    let widths = [
        trueno_viz::monitor::ratatui::layout::Constraint::Length(5),  // PROTO
        trueno_viz::monitor::ratatui::layout::Constraint::Length(7),  // LOCAL
        trueno_viz::monitor::ratatui::layout::Constraint::Length(22), // REMOTE
        trueno_viz::monitor::ratatui::layout::Constraint::Length(3),  // ST
        trueno_viz::monitor::ratatui::layout::Constraint::Min(8),     // PROCESS
    ];

    let table = Table::new(rows, widths).header(header);
    f.render_widget(table, inner);
}

/// Draw Storage Treemap panel - Pareto-style minimalist design
/// Uses monochromatic warm palette with clear visual hierarchy
pub fn draw_treemap(f: &mut Frame, app: &App, area: Rect) {
    use trueno_viz::monitor::ratatui::style::Color;

    let total_size = app.treemap_analyzer.total_size();
    let scanning = app.treemap_analyzer.is_scanning();

    let size_str = if total_size >= 1024 * 1024 * 1024 * 1024 {
        format!("{:.1}T", total_size as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0))
    } else if total_size >= 1024 * 1024 * 1024 {
        format!("{:.1}G", total_size as f64 / (1024.0 * 1024.0 * 1024.0))
    } else {
        format!("{:.0}M", total_size as f64 / (1024.0 * 1024.0))
    };

    let title = if scanning {
        " Files │ scanning... ".to_string()
    } else {
        format!(" Files │ {} ", size_str)
    };

    let block = btop_block(&title, Color::Rgb(180, 140, 100));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 2 || inner.width < 4 {
        return;
    }

    let layout = app.treemap_analyzer.layout(inner.width as f64, inner.height as f64);

    if layout.is_empty() {
        let msg = if scanning { "Scanning..." } else { "No large files" };
        f.render_widget(
            Paragraph::new(msg).style(Style::default().fg(Color::Rgb(100, 100, 100))),
            inner,
        );
        return;
    }

    // Monochromatic warm palette - Pareto: biggest = most saturated
    // Gradient from warm amber to cool slate based on rank
    let total_files = layout.len();

    // Grid: (char, fg_color, bg_color)
    let mut grid: Vec<Vec<(char, Color, Color)>> = vec![
        vec![(' ', Color::Rgb(40, 40, 45), Color::Rgb(25, 25, 30)); inner.width as usize];
        inner.height as usize
    ];

    // Render rectangles with Pareto coloring (rank-based intensity)
    for (idx, (rect, name)) in layout.iter().enumerate() {
        let x1 = rect.x.floor() as usize;
        let y1 = rect.y.floor() as usize;
        let x2 = (rect.x + rect.w).ceil() as usize;
        let y2 = (rect.y + rect.h).ceil() as usize;
        let rw = x2.saturating_sub(x1);
        let rh = y2.saturating_sub(y1);

        // Pareto gradient: top items are warm/bright, lower items fade to cool/dim
        let rank_ratio = idx as f64 / total_files.max(1) as f64;
        let (fill, border, text_color) = pareto_colors(rank_ratio);

        // Fill rectangle
        let y_end = y2.min(inner.height as usize);
        let x_end = x2.min(inner.width as usize);
        for (row_idx, row) in grid.iter_mut().enumerate().take(y_end).skip(y1) {
            for (col_idx, cell) in row.iter_mut().enumerate().take(x_end).skip(x1) {
                let is_edge = col_idx == x1 || row_idx == y1;
                if is_edge {
                    *cell = ('▌', border, Color::Rgb(20, 20, 25));
                } else {
                    *cell = (' ', fill, fill);
                }
            }
        }

        // Labels only on blocks large enough (minimalist)
        if rw >= 8 && rh >= 2 {
            // Format size compactly
            let size_str = if rect.size >= 1024 * 1024 * 1024 {
                format!("{:.1}G", rect.size as f64 / (1024.0 * 1024.0 * 1024.0))
            } else {
                format!("{}M", rect.size / (1024 * 1024))
            };

            // Single line: "name size"
            let max_name = rw.saturating_sub(size_str.len() + 3);
            let short_name: String = name.chars().take(max_name).collect();
            let label = format!("{} {}", short_name, size_str);

            let label_y = y1 + rh / 2;
            let label_x = x1 + 1;

            if label_y < inner.height as usize {
                for (i, ch) in label.chars().enumerate() {
                    let px = label_x + i;
                    if px < x2.saturating_sub(1) && px < inner.width as usize {
                        grid[label_y][px] = (ch, text_color, Color::Rgb(15, 15, 18));
                    }
                }
            }
        } else if rw >= 4 && rh >= 1 {
            // Tiny: just size
            let size_str = if rect.size >= 1024 * 1024 * 1024 {
                format!("{:.0}G", rect.size as f64 / (1024.0 * 1024.0 * 1024.0))
            } else {
                format!("{}M", rect.size / (1024 * 1024))
            };
            let label_y = y1 + rh / 2;
            for (i, ch) in size_str.chars().take(rw - 1).enumerate() {
                let px = x1 + 1 + i;
                if px < x2 && px < inner.width as usize && label_y < inner.height as usize {
                    grid[label_y][px] = (ch, text_color, Color::Rgb(15, 15, 18));
                }
            }
        }
    }

    // Render
    for (row_idx, row) in grid.iter().enumerate() {
        let spans: Vec<Span> = row.iter().map(|(ch, fg, bg)| {
            Span::styled(ch.to_string(), Style::default().fg(*fg).bg(*bg))
        }).collect();

        f.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect { x: inner.x, y: inner.y + row_idx as u16, width: inner.width, height: 1 },
        );
    }
}

/// Pareto color scheme: warm amber for top items, cool slate for bottom
fn pareto_colors(rank_ratio: f64) -> (trueno_viz::monitor::ratatui::style::Color, trueno_viz::monitor::ratatui::style::Color, trueno_viz::monitor::ratatui::style::Color) {
    use trueno_viz::monitor::ratatui::style::Color;

    // Top 20% (Pareto vital few): warm amber/orange
    // Middle 30%: muted gold
    // Bottom 50%: cool blue-gray

    if rank_ratio < 0.2 {
        // Vital few - warm amber
        let intensity = 1.0 - (rank_ratio / 0.2);
        let r = 180 + (40.0 * intensity) as u8;
        let g = 100 + (30.0 * intensity) as u8;
        let b = 50;
        (
            Color::Rgb(r / 3, g / 3, b / 3),           // fill (dark)
            Color::Rgb(r, g, b),                        // border (bright)
            Color::Rgb(255, 240, 220),                  // text (warm white)
        )
    } else if rank_ratio < 0.5 {
        // Useful many - muted gold/tan
        let t = (rank_ratio - 0.2) / 0.3;
        let r = 140 - (30.0 * t) as u8;
        let g = 120 - (30.0 * t) as u8;
        let b = 80 - (20.0 * t) as u8;
        (
            Color::Rgb(r / 3, g / 3, b / 3),
            Color::Rgb(r, g, b),
            Color::Rgb(220, 210, 190),
        )
    } else {
        // Trivial many - cool slate
        let t = (rank_ratio - 0.5) / 0.5;
        let r = 70 - (20.0 * t) as u8;
        let g = 80 - (20.0 * t) as u8;
        let b = 100 - (20.0 * t) as u8;
        (
            Color::Rgb(r / 3, g / 3, b / 3),
            Color::Rgb(r, g, b),
            Color::Rgb(160, 165, 175),
        )
    }
}
