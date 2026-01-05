//! Panel rendering for ttop.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
};
use ratatui::Frame;

use trueno_viz::monitor::collectors::process::ProcessState;
use trueno_viz::monitor::types::Collector;
use trueno_viz::monitor::widgets::{Graph, Meter, MonitorSparkline};

use crate::app::App;
use crate::theme::{self, borders, graph, percent_color, process_state, temp_color};

/// Helper to create a btop-style block with rounded corners
fn btop_block(title: &str, color: ratatui::style::Color) -> Block<'_> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(borders::STYLE)
        .border_style(Style::default().fg(color))
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
pub fn draw_memory(f: &mut Frame, app: &App, area: Rect) {
    let total_gb = app.mem_total as f64 / (1024.0 * 1024.0 * 1024.0);
    let used_gb = app.mem_used as f64 / (1024.0 * 1024.0 * 1024.0);
    let available_gb = app.mem_available as f64 / (1024.0 * 1024.0 * 1024.0);
    let cached_gb = app.mem_cached as f64 / (1024.0 * 1024.0 * 1024.0);
    let free_gb = app.mem_free as f64 / (1024.0 * 1024.0 * 1024.0);
    let swap_used_gb = app.swap_used as f64 / (1024.0 * 1024.0 * 1024.0);

    // Calculate percentages
    let used_pct = if app.mem_total > 0 {
        (app.mem_used as f64 / app.mem_total as f64) * 100.0
    } else {
        0.0
    };
    let avail_pct = if app.mem_total > 0 {
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

    let title = format!(" Memory │ {used_gb:.1}G / {total_gb:.1}G ({used_pct:.0}%) ");

    let block = btop_block(&title, borders::MEMORY);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 1 || inner.width < 10 {
        return;
    }

    // Adaptive layout based on available height
    // Priority: Used/Available/Cached/Free/Swap rows with sparklines

    struct MemRow<'a> {
        label: &'static str,
        value_gb: f64,
        pct: f64,
        history: &'a [f64],
        color: ratatui::style::Color,
    }

    let mut rows: Vec<MemRow> = vec![
        MemRow {
            label: "Used",
            value_gb: used_gb,
            pct: used_pct,
            history: &app.mem_history,
            color: percent_color(used_pct),
        },
        MemRow {
            label: "Available",
            value_gb: available_gb,
            pct: avail_pct,
            history: &app.mem_available_history,
            color: ratatui::style::Color::Green,
        },
        MemRow {
            label: "Cached",
            value_gb: cached_gb,
            pct: cached_pct,
            history: &app.mem_cached_history,
            color: ratatui::style::Color::Cyan,
        },
        MemRow {
            label: "Free",
            value_gb: free_gb,
            pct: free_pct,
            history: &app.mem_free_history,
            color: ratatui::style::Color::Blue,
        },
    ];

    // Add swap if exists
    if app.swap_total > 0 {
        let swap_color = if swap_pct > 50.0 {
            ratatui::style::Color::Red
        } else if swap_pct > 10.0 {
            ratatui::style::Color::Yellow
        } else {
            ratatui::style::Color::Green
        };
        rows.push(MemRow {
            label: "Swap",
            value_gb: swap_used_gb,
            pct: swap_pct,
            history: &app.swap_history,
            color: swap_color,
        });
    }

    // Calculate how many rows we can show
    let available_rows = inner.height as usize;
    let rows_to_show = rows.len().min(available_rows);

    // For very small panels, just show a meter
    if inner.height < 3 {
        let meter = Meter::new(used_pct / 100.0)
            .label(format!("{:.1}G/{:.1}G", used_gb, total_gb))
            .color(percent_color(used_pct));
        f.render_widget(meter, inner);
        return;
    }

    let mut y = inner.y;
    for row in rows.iter().take(rows_to_show) {
        // Compact format: "Used: 85.4G 68 ▁▂▃▄▅▆▇█→"
        let label_part = format!("{:>9}: {:>5.1}G {:>2.0}", row.label, row.value_gb, row.pct);
        let label_width = label_part.len() as u16 + 1;
        let sparkline_width = inner.width.saturating_sub(label_width);

        // Draw label
        f.render_widget(
            Paragraph::new(label_part).style(Style::default().fg(row.color)),
            Rect {
                x: inner.x,
                y,
                width: label_width,
                height: 1,
            },
        );

        // Draw sparkline
        if sparkline_width > 3 && !row.history.is_empty() {
            let sparkline = MonitorSparkline::new(row.history)
                .color(row.color)
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
}

/// Draw Disk panel
pub fn draw_disk(f: &mut Frame, app: &App, area: Rect) {
    let mounts = app.disk.mounts();
    let rates = app.disk.rates();

    // Calculate total I/O rates
    let total_read: f64 = rates.values().map(|r| r.read_bytes_per_sec).sum();
    let total_write: f64 = rates.values().map(|r| r.write_bytes_per_sec).sum();

    let title = format!(
        " Disk │ R: {} │ W: {} ",
        theme::format_bytes_rate(total_read),
        theme::format_bytes_rate(total_write)
    );

    let block = btop_block(&title, borders::DISK);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 1 {
        return;
    }

    // Each mount gets 2 lines: meter + I/O info (if space)
    let lines_per_mount = if inner.height >= (mounts.len() as u16 * 2) {
        2
    } else {
        1
    };
    let max_mounts = (inner.height as usize) / lines_per_mount.max(1) as usize;

    let mut y = inner.y;
    for mount in mounts.iter().take(max_mounts) {
        if mount.total_bytes == 0 || y >= inner.y + inner.height {
            continue;
        }

        let used_pct = mount.usage_percent();
        let total_gb = mount.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let used_gb = mount.used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

        // Truncate mount point for label
        let label: String = if mount.mount_point == "/" {
            "/".to_string()
        } else {
            mount
                .mount_point
                .rsplit('/')
                .next()
                .unwrap_or(&mount.mount_point)
                .chars()
                .take(6)
                .collect()
        };

        let color = percent_color(used_pct);
        let meter = Meter::new(used_pct / 100.0).label(&label).color(color);
        f.render_widget(
            meter,
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
        y += 1;

        // Show capacity info on second line if space
        if lines_per_mount >= 2 && y < inner.y + inner.height {
            // Find I/O rate for this device
            let device_name = mount.device.rsplit('/').next().unwrap_or("");
            let io_info = rates.get(device_name).or_else(|| {
                // Try without partition number (e.g., sda1 -> sda)
                let base: String = device_name
                    .chars()
                    .take_while(|c| !c.is_ascii_digit())
                    .collect();
                rates.get(&base)
            });

            let io_str = if let Some(io) = io_info {
                format!(
                    "  {:.1}G/{:.1}G ({:.0}%) │ R:{} W:{}",
                    used_gb,
                    total_gb,
                    used_pct,
                    theme::format_bytes_rate(io.read_bytes_per_sec),
                    theme::format_bytes_rate(io.write_bytes_per_sec)
                )
            } else {
                format!("  {:.1}G / {:.1}G ({:.0}%)", used_gb, total_gb, used_pct)
            };

            let info_line =
                Paragraph::new(io_str).style(Style::default().fg(ratatui::style::Color::DarkGray));
            f.render_widget(
                info_line,
                Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 1,
                },
            );
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
                    .fg(ratatui::style::Color::White)
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
                    .fg(ratatui::style::Color::White)
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
                Style::default().fg(ratatui::style::Color::DarkGray),
            ),
            Span::styled("↓ ", Style::default().fg(graph::NETWORK_RX)),
            Span::styled(
                theme::format_bytes(app.net_rx_total),
                Style::default().fg(ratatui::style::Color::White),
            ),
            Span::styled(" │ ", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled("↑ ", Style::default().fg(graph::NETWORK_TX)),
            Span::styled(
                theme::format_bytes(app.net_tx_total),
                Style::default().fg(ratatui::style::Color::White),
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

/// Draw GPU panel
pub fn draw_gpu(f: &mut Frame, app: &App, area: Rect) {
    let mut title = " GPU ".to_string();
    let mut gpu_util = 0.0;
    let mut gpu_temp = 0.0;
    let mut gpu_power = 0;
    let mut vram_pct = 0.0;

    #[cfg(feature = "nvidia")]
    if app.nvidia_gpu.is_available() {
        let gpus = app.nvidia_gpu.gpus();
        if let Some(gpu) = gpus.first() {
            title = format!(
                " {} │ {}°C │ {}W ",
                gpu.name,
                gpu.temperature as u32,
                gpu.power_mw / 1000
            );
            gpu_util = gpu.gpu_util;
            gpu_temp = gpu.temperature;
            gpu_power = gpu.power_mw / 1000;
            if gpu.mem_total > 0 {
                vram_pct = gpu.mem_used as f64 / gpu.mem_total as f64;
            }
        }
    }

    #[cfg(target_os = "linux")]
    if app.amd_gpu.is_available() {
        let gpus = app.amd_gpu.gpus();
        if let Some(gpu) = gpus.first() {
            title = format!(
                " {} │ {}°C │ {:.0}W ",
                gpu.name, gpu.temperature as u32, gpu.power_watts
            );
            gpu_util = gpu.gpu_util;
            gpu_temp = gpu.temperature;
            gpu_power = gpu.power_watts as u32;
            if gpu.vram_total > 0 {
                vram_pct = gpu.vram_used as f64 / gpu.vram_total as f64;
            }
        }
    }

    let block = btop_block(&title, borders::GPU);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 3 {
        return;
    }

    // GPU utilization meter
    let gpu_color = percent_color(gpu_util);
    let gpu_meter = Meter::new(gpu_util / 100.0).label("GPU").color(gpu_color);
    f.render_widget(
        gpu_meter,
        Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        },
    );

    // VRAM meter
    if inner.height > 1 {
        let vram_color = percent_color(vram_pct * 100.0);
        let vram_meter = Meter::new(vram_pct).label("VRAM").color(vram_color);
        f.render_widget(
            vram_meter,
            Rect {
                x: inner.x,
                y: inner.y + 1,
                width: inner.width,
                height: 1,
            },
        );
    }

    // Temperature and power info
    if inner.height > 2 {
        let info = format!("Temp: {}°C │ Power: {}W", gpu_temp as u32, gpu_power);
        let info_para = Paragraph::new(info).style(Style::default().fg(temp_color(gpu_temp)));
        f.render_widget(
            info_para,
            Rect {
                x: inner.x,
                y: inner.y + 2,
                width: inner.width,
                height: 1,
            },
        );
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
                    Style::default().fg(ratatui::style::Color::Blue),
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
                    Style::default().fg(ratatui::style::Color::White),
                ),
                Span::styled(
                    p.cmdline.chars().take(50).collect::<String>(),
                    Style::default().fg(ratatui::style::Color::DarkGray),
                ),
            ])
        })
        .collect();

    let widths = [
        ratatui::layout::Constraint::Length(7),  // PID
        ratatui::layout::Constraint::Length(9),  // USER
        ratatui::layout::Constraint::Length(2),  // State
        ratatui::layout::Constraint::Length(5),  // CPU%
        ratatui::layout::Constraint::Length(5),  // CPU bar
        ratatui::layout::Constraint::Length(6),  // MEM%
        ratatui::layout::Constraint::Length(16), // NAME (with tree)
        ratatui::layout::Constraint::Min(20),    // COMMAND
    ];

    let mut table_state = ratatui::widgets::TableState::default();
    table_state.select(Some(app.process_selected));

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(
            Style::default()
                .bg(ratatui::style::Color::DarkGray)
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
