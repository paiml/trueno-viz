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

use crate::app::{App, DiskHealth};
use crate::theme::{self, borders, graph, percent_color, process_state, temp_color};

/// Helper to create a btop-style block with rounded corners
fn btop_block(title: &str, color: trueno_viz::monitor::ratatui::style::Color) -> Block<'_> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(borders::STYLE)
        .border_style(Style::default().fg(color))
}

/// Truncate a string to fit within max_len, adding "..." if truncated.
/// Delegates to batuta-common.
fn truncate_str(s: &str, max_len: usize) -> String {
    batuta_common::display::truncate_str(s, max_len)
}

/// Create a bounds-safe Rect that doesn't exceed parent boundaries
/// Returns None if the rect would be entirely outside parent bounds
fn clamp_rect(parent: Rect, x: u16, y: u16, width: u16, height: u16) -> Option<Rect> {
    let max_x = parent.x + parent.width;
    let max_y = parent.y + parent.height;

    // If starting position is outside parent, skip
    if x >= max_x || y >= max_y {
        return None;
    }

    // Clamp width and height to parent boundaries
    let clamped_width = width.min(max_x.saturating_sub(x));
    let clamped_height = height.min(max_y.saturating_sub(y));

    if clamped_width == 0 || clamped_height == 0 {
        return None;
    }

    Some(Rect { x, y, width: clamped_width, height: clamped_height })
}

/// Draw CPU panel - btop-style with per-core meters, graph, load gauge, and top consumers
pub fn draw_cpu(f: &mut Frame, app: &App, area: Rect) {
    use trueno_viz::monitor::ratatui::style::Color;
    use trueno_viz::monitor::ratatui::text::{Line, Span};

    let load = app.cpu.load_average();
    let freq = app.cpu.frequencies();
    let max_freq = freq.iter().map(|f| f.current_mhz).max().unwrap_or(0);
    let min_freq = freq.iter().map(|f| f.current_mhz).min().unwrap_or(0);
    let core_count = app.cpu.core_count();

    // Get max CPU temp if available
    let max_temp = app.sensors.max_temp();
    let temp_str = max_temp.map(|t| format!(" {:.0}°C", t)).unwrap_or_default();

    // Current CPU usage for title
    let cpu_pct = app.cpu_history.last().copied().unwrap_or(0.0) * 100.0;

    // Detect boost state (if current freq > base freq, we're boosting)
    let is_boosting = max_freq > 3000; // Rough heuristic: > 3GHz likely boosting

    // Detect exploded mode early for title (account for borders)
    let is_exploded = area.width > 82 || area.height > 22;
    let exploded_info = if is_exploded { " │ ▣ FULL" } else { "" };

    // Thermal throttling indicator
    let throttle_str = match app.thermal_throttle_active {
        Some(true) => " 🔥THROTTLE",
        Some(false) => "",
        None => "",
    };

    let title = format!(
        " CPU {:.0}% │ {} cores │ {:.1}GHz{}{}{} │ up {} │ LAV {:.2}{} ",
        cpu_pct,
        core_count,
        max_freq as f64 / 1000.0,
        if is_boosting { "⚡" } else { "" },
        temp_str,
        throttle_str,
        theme::format_uptime(app.cpu.uptime_secs()),
        load.one,
        exploded_info,
    );

    let block = btop_block(&title, borders::CPU);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 1 || inner.width < 10 {
        return;
    }

    // Reserve bottom rows for load gauge and top consumers
    let reserved_bottom = if is_exploded { 4u16 } else { 2u16 };
    let core_area_height = inner.height.saturating_sub(reserved_bottom);

    // btop-style layout: per-core meters on left, graph on right
    // In exploded mode: FILL THE ENTIRE WIDTH with meters and graph
    let (meter_bar_width, bar_len, max_cores_per_col, meters_width) = if is_exploded {
        // Calculate how many cores per column based on height
        let max_per_col = (core_area_height as usize).max(1);  // Avoid divide by zero
        let num_cols = core_count.div_ceil(max_per_col).max(1);

        // Use 70% of width for meters, 30% for graph
        let target_meters_width = (inner.width * 7 / 10) as usize;

        // Calculate bar width to fill the meter area
        let bar_width = (target_meters_width / num_cols).max(25).min(50) as u16;
        let bar_chars = (bar_width as usize).saturating_sub(12).max(12);  // At least 12 char bars

        let actual_meters_width = (num_cols as u16 * bar_width).min(inner.width * 8 / 10);

        (bar_width, bar_chars, max_per_col, actual_meters_width)
    } else {
        let cores_per_col = core_area_height as usize;
        let num_cols = if cores_per_col > 0 { core_count.div_ceil(cores_per_col) } else { 1 };
        let meters_w = (num_cols as u16 * 10).min(inner.width / 2);
        (10u16, 6usize, usize::MAX, meters_w)
    };

    let cores_per_col = (core_area_height as usize).min(max_cores_per_col);

    // Draw per-core meters on left side
    let cpu_temps = app.sensors.cpu_temps();

    for (i, &percent) in app.per_core_percent.iter().enumerate() {
        if cores_per_col == 0 {
            break;
        }
        let col = i / cores_per_col;
        let row = i % cores_per_col;

        let cell_x = inner.x + (col as u16) * meter_bar_width;
        let cell_y = inner.y + row as u16;

        if cell_x + meter_bar_width > inner.x + meters_width || cell_y >= inner.y + core_area_height
        {
            continue;
        }

        let color = percent_color(percent);
        let core_temp = cpu_temps.get(i).map(|t| t.current);

        let filled = ((percent / 100.0) * bar_len as f64) as usize;
        let bar: String =
            "█".repeat(filled.min(bar_len)) + &"░".repeat(bar_len - filled.min(bar_len));

        // Exploded mode: show state breakdown with colored bars
        if is_exploded {
            let temp_str = core_temp.map(|t| format!("{:>2.0}°", t)).unwrap_or_default();

            // Get CPU state breakdown for this core
            let state = app.per_core_state.get(i);

            let mut spans = vec![
                Span::styled(format!("{:>2} ", i), Style::default().fg(Color::DarkGray)),
                Span::styled(&bar, Style::default().fg(color)),
                Span::styled(format!(" {:>3.0}%", percent), Style::default().fg(color)),
            ];

            if !temp_str.is_empty() {
                spans.push(Span::styled(format!(" {}", temp_str), Style::default().fg(Color::Cyan)));
            }

            // Add state breakdown: u/s/io colored mini-text
            if let Some(s) = state {
                spans.push(Span::styled(" │", Style::default().fg(Color::DarkGray)));
                // User time in green
                if s.user > 0.5 {
                    spans.push(Span::styled(format!("u{:.0}", s.user), Style::default().fg(Color::Green)));
                }
                // System time in yellow
                if s.system > 0.5 {
                    spans.push(Span::styled(format!(" s{:.0}", s.system), Style::default().fg(Color::Yellow)));
                }
                // IOWait in red (only show if significant)
                if s.iowait > 1.0 {
                    spans.push(Span::styled(format!(" io{:.0}", s.iowait), Style::default().fg(Color::Red)));
                }
            }

            f.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect {
                    x: cell_x,
                    y: cell_y,
                    width: meter_bar_width,
                    height: 1,
                },
            );
        } else {
            // Normal mode: simple label
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
    }

    // Draw graph on right side
    let graph_x = inner.x + meters_width + 1;
    let graph_width = inner.width.saturating_sub(meters_width + 1);

    if graph_width > 3 && !app.cpu_history.is_empty() && core_area_height > 0 {
        let graph_area = Rect {
            x: graph_x,
            y: inner.y,
            width: graph_width,
            height: core_area_height,
        };
        let cpu_graph = Graph::new(&app.cpu_history)
            .color(graph::CPU)
            .mode(trueno_viz::monitor::widgets::GraphMode::Block);
        f.render_widget(cpu_graph, graph_area);
    }

    // === Bottom Row 1: Load Average Gauge + Frequency ===
    let load_y = inner.y + core_area_height;
    if load_y < inner.y + inner.height {
        // Load average visualization (normalized to core count)
        let load_normalized = load.one / core_count as f64;
        let load_color = if load_normalized > 1.0 {
            Color::Red
        } else if load_normalized > 0.7 {
            Color::Yellow
        } else {
            Color::Green
        };

        // Load trend arrows
        let trend_1_5 = if load.one > load.five {
            "↑"
        } else if load.one < load.five {
            "↓"
        } else {
            "→"
        };
        let trend_5_15 = if load.five > load.fifteen {
            "↑"
        } else if load.five < load.fifteen {
            "↓"
        } else {
            "→"
        };

        // Load bar (0-2x cores = 100%)
        let load_bar_width = 10usize;
        let load_pct = (load_normalized / 2.0).min(1.0);
        let load_filled = (load_pct * load_bar_width as f64) as usize;
        let load_empty = load_bar_width.saturating_sub(load_filled);

        // Frequency range
        let freq_str = if min_freq != max_freq && min_freq > 0 {
            format!(
                "{:.1}-{:.1}GHz",
                min_freq as f64 / 1000.0,
                max_freq as f64 / 1000.0
            )
        } else {
            format!("{:.1}GHz", max_freq as f64 / 1000.0)
        };

        let load_line = Line::from(vec![
            Span::styled("Load ", Style::default().fg(Color::DarkGray)),
            Span::styled("█".repeat(load_filled), Style::default().fg(load_color)),
            Span::styled("░".repeat(load_empty), Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(" {:.2}{} ", load.one, trend_1_5),
                Style::default().fg(load_color),
            ),
            Span::styled(
                format!("{:.2}{} ", load.five, trend_5_15),
                Style::default().fg(Color::White),
            ),
            Span::styled(format!("{:.2}", load.fifteen), Style::default().fg(Color::DarkGray)),
            Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
            Span::styled("Freq ", Style::default().fg(Color::DarkGray)),
            Span::styled(freq_str, Style::default().fg(if is_boosting { Color::Cyan } else { Color::White })),
            if is_boosting {
                Span::styled(" ⚡", Style::default().fg(Color::Yellow))
            } else {
                Span::raw("")
            },
        ]);

        f.render_widget(
            Paragraph::new(load_line),
            Rect {
                x: inner.x,
                y: load_y,
                width: inner.width,
                height: 1,
            },
        );
    }

    // === Bottom Row 2: Top CPU Consumers ===
    let consumers_y = inner.y + core_area_height + 1;
    if consumers_y < inner.y + inner.height {
        let mut top_procs: Vec<_> = app.process.processes().values().collect();
        top_procs.sort_by(|a, b| {
            b.cpu_percent
                .partial_cmp(&a.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut spans = vec![Span::styled("Top ", Style::default().fg(Color::DarkGray))];

        // Show more consumers in exploded mode
        let max_consumers = if is_exploded { 5 } else { 3 };
        let name_len = if is_exploded { 16 } else { 12 };

        for (i, proc) in top_procs.iter().take(max_consumers).enumerate() {
            if proc.cpu_percent < 0.1 {
                continue;
            }

            let cpu_color = if proc.cpu_percent > 50.0 {
                Color::Red
            } else if proc.cpu_percent > 20.0 {
                Color::Yellow
            } else {
                Color::Green
            };

            if i > 0 {
                spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
            }

            spans.push(Span::styled(
                format!("{:.0}%", proc.cpu_percent),
                Style::default().fg(cpu_color),
            ));
            spans.push(Span::styled(
                format!(" {}", truncate_str(&proc.name, name_len)),
                Style::default().fg(Color::White),
            ));
        }

        if spans.len() > 1 {
            f.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect {
                    x: inner.x,
                    y: consumers_y,
                    width: inner.width,
                    height: 1,
                },
            );
        }
    }

    // === Bottom Row 3 (exploded only): Per-core frequency summary ===
    if is_exploded {
        let freq_y = inner.y + core_area_height + 2;
        if freq_y < inner.y + inner.height && !freq.is_empty() {
            let avg_freq: u64 = freq.iter().map(|f| f.current_mhz).sum::<u64>() / freq.len() as u64;
            let freq_spread = max_freq.saturating_sub(min_freq);

            let freq_line = Line::from(vec![
                Span::styled("Freq ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("avg {:.2}GHz", avg_freq as f64 / 1000.0), Style::default().fg(Color::White)),
                Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("range {:.2}-{:.2}GHz", min_freq as f64 / 1000.0, max_freq as f64 / 1000.0), Style::default().fg(Color::Cyan)),
                Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("spread {}MHz", freq_spread), Style::default().fg(if freq_spread > 1000 { Color::Yellow } else { Color::DarkGray })),
            ]);

            f.render_widget(
                Paragraph::new(freq_line),
                Rect {
                    x: inner.x,
                    y: freq_y,
                    width: inner.width,
                    height: 1,
                },
            );
        }

        // === Bottom Row 4 (exploded only): Top process per core ===
        let proc_y = inner.y + core_area_height + 3;
        if proc_y < inner.y + inner.height && !app.top_process_per_core.is_empty() {
            let mut spans = vec![Span::styled("Per-core ", Style::default().fg(Color::DarkGray))];

            // Show top process for first few cores
            let max_shown = ((inner.width as usize - 10) / 20).min(app.top_process_per_core.len()).min(8);
            for (i, proc) in app.top_process_per_core.iter().take(max_shown).enumerate() {
                if i > 0 {
                    spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
                }

                let cpu_color = if proc.cpu_percent > 50.0 {
                    Color::Red
                } else if proc.cpu_percent > 20.0 {
                    Color::Yellow
                } else {
                    Color::Green
                };

                spans.push(Span::styled(format!("c{}", i), Style::default().fg(Color::DarkGray)));
                spans.push(Span::styled(
                    format!(":{:.0}%", proc.cpu_percent),
                    Style::default().fg(cpu_color),
                ));
                spans.push(Span::styled(
                    format!(" {}", truncate_str(&proc.name, 8)),
                    Style::default().fg(Color::White),
                ));
            }

            f.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect {
                    x: inner.x,
                    y: proc_y,
                    width: inner.width,
                    height: 1,
                },
            );
        }
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

    // Detect exploded mode early for title (account for borders)
    let is_exploded = area.width > 82 || area.height > 22;
    let exploded_info = if is_exploded { " │ ▣ FULL" } else { "" };

    // Swap trend indicator
    let swap_trend_info = if app.swap_total > 0 && swap_pct > 5.0 {
        format!(" Swap{}", app.swap_trend.symbol())
    } else {
        String::new()
    };

    // Memory reclaim rate (if significant)
    let reclaim_info = if app.mem_reclaim_rate > 100.0 {
        format!(" │ Reclaim:{:.0}p/s", app.mem_reclaim_rate)
    } else {
        String::new()
    };

    let title = format!(
        " Memory │ {used_gb:.1}G / {total_gb:.1}G ({used_pct:.0}%){swap_trend_info}{zram_info}{reclaim_info}{exploded_info} "
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

    // ZRAM details row (after swap) - only if ZRAM is active
    let zram_stats: Vec<_> = app.swap_analyzer.zram_stats().iter().filter(|z| z.is_active()).collect();
    let zram_total_orig: u64 = zram_stats.iter().map(|z| z.orig_data_size).sum();
    let zram_total_compr: u64 = zram_stats.iter().map(|z| z.compr_data_size).sum();

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
        // Clamp label width to inner bounds
        let label_width = (label_part.len() as u16 + 1).min(inner.width);
        let sparkline_width = inner.width.saturating_sub(label_width);

        if let Some(label_rect) = clamp_rect(inner, inner.x, y, label_width, 1) {
            f.render_widget(
                Paragraph::new(label_part).style(Style::default().fg(row.color)),
                label_rect,
            );
        }

        if sparkline_width > 3 && !row.history.is_empty() {
            if let Some(spark_rect) = clamp_rect(inner, inner.x + label_width, y, sparkline_width, 1) {
                let sparkline = MonitorSparkline::new(row.history)
                    .color(row.color)
                    .show_trend(true);
                f.render_widget(sparkline, spark_rect);
            }
        }
        y += 1;
    }

    // === ZRAM Row (conditional) ===
    if y < inner.y + inner.height && zram_total_orig > 0 {
        let orig_gb = zram_total_orig as f64 / (1024.0 * 1024.0 * 1024.0);
        let compr_gb = zram_total_compr as f64 / (1024.0 * 1024.0 * 1024.0);
        let ratio = if zram_total_compr > 0 {
            zram_total_orig as f64 / zram_total_compr as f64
        } else {
            1.0
        };

        // Format based on size (GB vs TB)
        let orig_str = if orig_gb >= 1000.0 {
            format!("{:.1}T", orig_gb / 1024.0)
        } else {
            format!("{:.0}G", orig_gb)
        };
        let compr_str = if compr_gb >= 1000.0 {
            format!("{:.1}T", compr_gb / 1024.0)
        } else {
            format!("{:.0}G", compr_gb)
        };

        let zram_line = Line::from(vec![
            Span::styled("  ZRAM ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}→{} ", orig_str, compr_str), Style::default().fg(Color::Magenta)),
            Span::styled(format!("{:.1}x", ratio), Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" {}", zram_stats.first().map(|z| z.comp_algorithm.as_str()).unwrap_or("?")),
                         Style::default().fg(Color::DarkGray)),
        ]);

        f.render_widget(
            Paragraph::new(zram_line),
            Rect { x: inner.x, y, width: inner.width, height: 1 },
        );
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

    // === Top Memory Consumers - expand to fill available height ===
    let remaining_height = (inner.y + inner.height).saturating_sub(y) as usize;
    if remaining_height > 0 {
        // Get top processes by memory - show more when we have more space
        let mut procs: Vec<_> = app.process.processes().values().collect();
        procs.sort_by(|a, b| b.mem_bytes.cmp(&a.mem_bytes));

        // Exploded mode settings - scale with available width
        let (name_width, bar_width, compact_count) = if is_exploded {
            // Scale bar width to fill ~50% of available space
            let bar_w = (inner.width as usize / 3).max(20).min(60);
            let name_w = (inner.width as usize / 5).max(20).min(40);
            (name_w, bar_w, remaining_height.min(15))  // Show many more processes
        } else {
            (20usize, 20usize, 3usize)
        };

        // First line: compact "Top:" format
        if remaining_height == 1 {
            let mut spans = vec![Span::styled("Top:", Style::default().fg(Color::DarkGray))];
            for proc in procs.iter().take(compact_count) {
                let mem_gb = proc.mem_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let name: String = proc.name.chars().take(if is_exploded { 15 } else { 10 }).collect();
                spans.push(Span::raw(" "));
                spans.push(Span::styled(name, Style::default().fg(Color::White)));
                spans.push(Span::styled(format!(" {:.1}G", mem_gb), Style::default().fg(Color::Magenta)));
                spans.push(Span::styled(" │", Style::default().fg(Color::DarkGray)));
            }
            if !procs.is_empty() {
                spans.pop();
            }
            f.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect { x: inner.x, y, width: inner.width, height: 1 },
            );
        } else {
            // Multiple lines available - show detailed process list
            let header_suffix = if is_exploded { " (Full View) " } else { " " };
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(format!("── Top Memory Consumers{}", header_suffix), Style::default().fg(Color::DarkGray)),
                    Span::styled("─".repeat((inner.width as usize).saturating_sub(24 + header_suffix.len())), Style::default().fg(Color::DarkGray)),
                ])),
                Rect { x: inner.x, y, width: inner.width, height: 1 },
            );
            y += 1;

            let procs_to_show = (remaining_height - 1).min(procs.len());
            for proc in procs.iter().take(procs_to_show) {
                if y >= inner.y + inner.height {
                    break;
                }
                let mem_gb = proc.mem_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let mem_pct = if app.mem_total > 0 {
                    (proc.mem_bytes as f64 / app.mem_total as f64) * 100.0
                } else {
                    0.0
                };

                let name: String = proc.name.chars().take(name_width).collect();

                // Exploded mode: show user and thread count too, scale bar to fill remaining width
                let line = if is_exploded {
                    let user: String = proc.user.chars().take(10).collect();
                    // Calculate remaining width for bar after fixed columns
                    // PID(8) + USER(11) + NAME(name_width+1) + MEM(8) + PCT(7) = fixed
                    let fixed_cols = 8 + 11 + name_width + 1 + 8 + 7;
                    let remaining_bar = (inner.width as usize).saturating_sub(fixed_cols).max(10);
                    let filled = ((mem_pct / 100.0) * remaining_bar as f64) as usize;
                    let bar = "█".repeat(filled.min(remaining_bar)) + &"░".repeat(remaining_bar.saturating_sub(filled));

                    Line::from(vec![
                        Span::styled(format!("{:>7} ", proc.pid), Style::default().fg(Color::DarkGray)),
                        Span::styled(format!("{:<10} ", user), Style::default().fg(Color::Cyan)),
                        Span::styled(format!("{:<width$} ", name, width = name_width), Style::default().fg(Color::White)),
                        Span::styled(format!("{:>6.1}G ", mem_gb), Style::default().fg(Color::Magenta)),
                        Span::styled(format!("{:>5.1}% ", mem_pct), Style::default().fg(percent_color(mem_pct))),
                        Span::styled(bar, Style::default().fg(percent_color(mem_pct))),
                    ])
                } else {
                    let filled = ((mem_pct / 100.0) * bar_width as f64) as usize;
                    let bar = "█".repeat(filled.min(bar_width)) + &"░".repeat(bar_width.saturating_sub(filled));
                    Line::from(vec![
                        Span::styled(format!("{:>6} ", proc.pid), Style::default().fg(Color::DarkGray)),
                        Span::styled(format!("{:<20} ", name), Style::default().fg(Color::White)),
                        Span::styled(format!("{:>6.1}G ", mem_gb), Style::default().fg(Color::Magenta)),
                        Span::styled(format!("{:>5.1}% ", mem_pct), Style::default().fg(percent_color(mem_pct))),
                        Span::styled(bar, Style::default().fg(percent_color(mem_pct))),
                    ])
                };

                f.render_widget(
                    Paragraph::new(line),
                    Rect { x: inner.x, y, width: inner.width, height: 1 },
                );
                y += 1;
            }
        }
    }
}

/// Draw Disk panel - enhanced with Little's Law latency estimation
/// and Ruemmler & Wilkes (1994) workload classification
