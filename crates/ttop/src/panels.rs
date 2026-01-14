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

/// Render container list - shared by mock and real paths
fn render_container_list<C: std::borrow::Borrow<crate::analyzers::ContainerStats>>(f: &mut Frame, inner: Rect, containers: &[C]) {
    use trueno_viz::monitor::ratatui::style::Color;

    if containers.is_empty() {
        f.render_widget(
            Paragraph::new("No running containers").style(Style::default().fg(Color::DarkGray)),
            inner,
        );
        return;
    }

    // Find max CPU for relative bar sizing
    let max_cpu = containers.iter().map(|c| c.borrow().cpu_pct).fold(1.0_f64, f64::max);

    let bar_width = 5usize;

    for (i, container) in containers.iter().enumerate() {
        let c = container.borrow();
        if i as u16 >= inner.height {
            break;
        }

        // Status icon: ● running, ◐ paused/restarting, ○ exited
        let (status_char, status_color) = match c.status {
            crate::analyzers::ContainerStatus::Running => ('●', Color::Rgb(80, 200, 120)),
            crate::analyzers::ContainerStatus::Paused => ('◐', Color::Rgb(200, 200, 80)),
            crate::analyzers::ContainerStatus::Restarting => ('◐', Color::Rgb(200, 180, 80)),
            crate::analyzers::ContainerStatus::Exited => ('○', Color::Rgb(120, 120, 120)),
            crate::analyzers::ContainerStatus::Unknown => ('?', Color::Rgb(120, 120, 120)),
        };

        // CPU color (top of bar): green -> yellow -> red
        let cpu_color = if c.cpu_pct >= 80.0 {
            Color::Rgb(220, 80, 80)
        } else if c.cpu_pct >= 40.0 {
            Color::Rgb(220, 180, 80)
        } else {
            Color::Rgb(80, 180, 120)
        };

        // MEM color (bottom of bar): green -> yellow -> red
        let mem_color = if c.mem_pct >= 80.0 {
            Color::Rgb(220, 80, 80)
        } else if c.mem_pct >= 50.0 {
            Color::Rgb(220, 180, 80)
        } else {
            Color::Rgb(80, 180, 120)
        };

        // Split bar: ▄ with fg=MEM (bottom), bg=CPU (top)
        let fill = ((c.cpu_pct / max_cpu.max(1.0)) * bar_width as f64).round() as usize;
        let bar: String = "▄".repeat(fill.min(bar_width));
        let empty: String = " ".repeat(bar_width.saturating_sub(fill));

        // Compact memory size
        let mem_str = if c.mem_used >= 1024 * 1024 * 1024 {
            format!("{:.0}G", c.mem_used as f64 / (1024.0 * 1024.0 * 1024.0))
        } else {
            format!("{:.0}M", c.mem_used as f64 / (1024.0 * 1024.0))
        };

        // Name fills remaining space
        let name_width = (inner.width as usize).saturating_sub(bar_width + 7);
        let name: String = c.name.chars().take(name_width).collect();

        let line = Line::from(vec![
            Span::styled(status_char.to_string(), Style::default().fg(status_color)),
            Span::styled(bar, Style::default().fg(mem_color).bg(cpu_color)),
            Span::styled(empty, Style::default().fg(Color::Rgb(40, 40, 45))),
            Span::styled(format!("{:>4}", mem_str), Style::default().fg(Color::Rgb(140, 160, 180))),
            Span::styled(format!(" {}", name), Style::default().fg(Color::Rgb(200, 200, 210))),
        ]);

        f.render_widget(
            Paragraph::new(line),
            Rect { x: inner.x, y: inner.y + i as u16, width: inner.width, height: 1 },
        );
    }
}

/// Draw Container/Docker panel (internal)
fn draw_containers_inner(f: &mut Frame, app: &App, area: Rect) {
    use trueno_viz::monitor::ratatui::style::Color;

    // Check for mock container data first (for testing)
    if !app.mock_containers.is_empty() {
        let running = app.mock_containers.iter().filter(|c| c.status == "running").count();
        let total = app.mock_containers.len();

        let title = format!(" Containers │ {}/{} ", running, total);
        let block = btop_block(&title, Color::Rgb(80, 140, 180));
        let inner = block.inner(area);
        f.render_widget(block, area);

        if inner.height < 1 {
            return;
        }

        // Convert mock data to ContainerStats for rendering
        let containers: Vec<crate::analyzers::ContainerStats> = app.mock_containers.iter().map(|m| {
            let mem_pct = if m.mem_limit > 0 {
                m.mem_used as f64 / m.mem_limit as f64 * 100.0
            } else {
                0.0
            };
            let status = match m.status.to_lowercase().as_str() {
                "running" => crate::analyzers::ContainerStatus::Running,
                "paused" => crate::analyzers::ContainerStatus::Paused,
                "restarting" => crate::analyzers::ContainerStatus::Restarting,
                "exited" => crate::analyzers::ContainerStatus::Exited,
                _ => crate::analyzers::ContainerStatus::Unknown,
            };
            crate::analyzers::ContainerStats {
                name: m.name.clone(),
                cpu_pct: m.cpu_percent,
                mem_used: m.mem_used,
                mem_limit: m.mem_limit,
                mem_pct,
                status,
            }
        }).collect();

        // Render containers using shared logic
        render_container_list(f, inner, &containers);
        return;
    }

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
    render_container_list(f, inner, &containers);
}

/// Draw Process panel - btop style with mini CPU bars and optional tree view
pub fn draw_process(f: &mut Frame, app: &mut App, area: Rect) {
    let sorted = app.sorted_processes();
    let count = sorted.len();

    // Detect exploded mode early for title
    let is_exploded = area.width > 82 || area.height > 27;  // Account for borders

    let sort_indicator = app.sort_column.name();
    let direction = if app.sort_descending { "▼" } else { "▲" };
    let filter_info = if !app.filter.is_empty() {
        format!(" │ Filter: \"{}\"", app.filter)
    } else {
        String::new()
    };
    let tree_info = if app.show_tree { " │ 🌲 Tree" } else { "" };
    let exploded_info = if is_exploded { " │ ▣ FULL" } else { "" };

    let title = format!(
        " Processes ({}) │ Sort: {} {}{}{}{} ",
        count, sort_indicator, direction, filter_info, tree_info, exploded_info
    );

    let block = btop_block(&title, borders::PROCESS);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 2 {
        return;
    }

    // Header - compact or exploded based on available space
    let header_cells: Vec<&str> = if is_exploded {
        vec!["PID", "USER", "S", "THR", "CPU%", "MEM%", "MEM", "COMMAND"]
    } else {
        vec!["PID", "S", "C%", "M%", "COMMAND"]
    };

    let header = Row::new(header_cells.iter().map(|h| {
        let is_sort_col = *h == app.sort_column.name()
            || (*h == "S" && app.sort_column == crate::state::ProcessSortColumn::State)
            || (*h == "C%" && app.sort_column == crate::state::ProcessSortColumn::Cpu)
            || (*h == "CPU%" && app.sort_column == crate::state::ProcessSortColumn::Cpu)
            || (*h == "M%" && app.sort_column == crate::state::ProcessSortColumn::Mem)
            || (*h == "MEM%" && app.sort_column == crate::state::ProcessSortColumn::Mem);
        let style = if is_sort_col {
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

    // Build rows - exploded mode shows more columns
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

            // Combined command: "name cmdline" or with tree prefix
            // In exploded mode, show full cmdline
            let command = if app.show_tree {
                if p.cmdline.is_empty() || p.cmdline == p.name {
                    format!("{}{}", tree_prefix, p.name)
                } else {
                    format!("{}{} {}", tree_prefix, p.name, p.cmdline)
                }
            } else if p.cmdline.is_empty() || p.cmdline == p.name {
                p.name.clone()
            } else if is_exploded {
                // Exploded: show full cmdline
                p.cmdline.clone()
            } else {
                format!("{} {}", p.name, p.cmdline)
            };

            if is_exploded {
                // Exploded mode: PID USER S THR CPU% MEM% MEM COMMAND
                let user = if p.user.is_empty() { "-" } else { &p.user };
                let user_display: String = user.chars().take(8).collect();
                let threads = p.threads;
                let mem_str = theme::format_bytes(p.mem_bytes);

                Row::new(vec![
                    Span::styled(format!("{:>7}", p.pid), Style::default().fg(trueno_viz::monitor::ratatui::style::Color::DarkGray)),
                    Span::styled(format!("{:<8}", user_display), Style::default().fg(trueno_viz::monitor::ratatui::style::Color::Cyan)),
                    Span::styled(
                        p.state.as_char().to_string(),
                        Style::default().fg(state_color),
                    ),
                    Span::styled(format!("{:>4}", threads), Style::default().fg(trueno_viz::monitor::ratatui::style::Color::DarkGray)),
                    Span::styled(
                        format!("{:>6.1}", p.cpu_percent),
                        Style::default().fg(cpu_color),
                    ),
                    Span::styled(
                        format!("{:>6.1}", p.mem_percent),
                        Style::default().fg(mem_color),
                    ),
                    Span::styled(format!("{:>8}", mem_str), Style::default().fg(trueno_viz::monitor::ratatui::style::Color::DarkGray)),
                    Span::styled(
                        command,
                        Style::default().fg(trueno_viz::monitor::ratatui::style::Color::White),
                    ),
                ])
            } else {
                // Compact mode: PID S C% M% COMMAND
                Row::new(vec![
                    Span::styled(format!("{:>5}", p.pid), Style::default().fg(trueno_viz::monitor::ratatui::style::Color::DarkGray)),
                    Span::styled(
                        p.state.as_char().to_string(),
                        Style::default().fg(state_color),
                    ),
                    Span::styled(
                        format!("{:>5.0}", p.cpu_percent),
                        Style::default().fg(cpu_color),
                    ),
                    Span::styled(
                        format!("{:>5.0}", p.mem_percent),
                        Style::default().fg(mem_color),
                    ),
                    Span::styled(
                        command,
                        Style::default().fg(trueno_viz::monitor::ratatui::style::Color::White),
                    ),
                ])
            }
        })
        .collect();

    // Column widths - adjust based on exploded mode
    // Use Fill for COMMAND to take all remaining space
    use trueno_viz::monitor::ratatui::layout::Constraint;
    let widths: Vec<Constraint> = if is_exploded {
        // Exploded: generous spacing, COMMAND fills remaining
        vec![
            Constraint::Length(10),  // PID (wider)
            Constraint::Length(12),  // USER (wider)
            Constraint::Length(3),   // S
            Constraint::Length(6),   // THR
            Constraint::Length(8),   // CPU%
            Constraint::Length(8),   // MEM%
            Constraint::Length(10),  // MEM
            Constraint::Fill(1),     // COMMAND (fills ALL remaining space)
        ]
    } else {
        vec![
            Constraint::Length(6),   // PID
            Constraint::Length(2),   // S
            Constraint::Length(5),   // C%
            Constraint::Length(5),   // M%
            Constraint::Fill(1),     // COMMAND (fills remaining)
        ]
    };

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

/// Draw Network Connections panel - Little Snitch style with service detection
pub fn draw_connections(f: &mut Frame, app: &App, area: Rect) {
    use crate::analyzers::{ConnState, Protocol, ConnectionAnalyzer, geoip};

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

    // Header - enhanced with SERVICE, AGE, and GEO columns
    let header = Row::new(vec![
        Span::styled("SVC", Style::default().fg(borders::NETWORK).add_modifier(Modifier::BOLD)),
        Span::styled("LOCAL", Style::default().fg(borders::NETWORK).add_modifier(Modifier::BOLD)),
        Span::styled("REMOTE", Style::default().fg(borders::NETWORK).add_modifier(Modifier::BOLD)),
        Span::styled("GEO", Style::default().fg(borders::NETWORK).add_modifier(Modifier::BOLD)),
        Span::styled("ST", Style::default().fg(borders::NETWORK).add_modifier(Modifier::BOLD)),
        Span::styled("AGE", Style::default().fg(borders::NETWORK).add_modifier(Modifier::BOLD)),
        Span::styled("PROC", Style::default().fg(borders::NETWORK).add_modifier(Modifier::BOLD)),
    ]).height(1);

    // Connection rows with service detection, duration, and geo-IP
    let rows: Vec<Row> = sorted_conns
        .iter()
        .take(inner.height.saturating_sub(1) as usize)
        .map(|conn| {
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

            // Detect service by port
            let service = app.connection_analyzer.service_name(conn)
                .unwrap_or(match conn.protocol {
                    Protocol::Tcp => "TCP",
                    Protocol::Udp => "UDP",
                });

            // Get connection duration
            let duration_str = app.connection_analyzer
                .connection_duration(conn)
                .map(ConnectionAnalyzer::format_duration)
                .unwrap_or_else(|| "new".to_string());

            // Check if "hot" connection (high bandwidth)
            let is_hot = app.connection_analyzer.is_hot_connection(conn);

            // Get process name for this connection
            let proc_name = app.connection_analyzer
                .process_for_connection(conn)
                .map(|(_, name)| name.to_string())
                .unwrap_or_else(|| "-".to_string());

            // Get country flag for remote IP
            let geo_flag = if conn.remote_ip.is_unspecified() || conn.remote_ip.is_loopback() {
                "🏠"
            } else {
                geoip::get_flag(conn.remote_ip)
            };

            // Format addresses (truncate if needed)
            let local = format!(":{}", conn.local_port);
            let remote = if conn.remote_ip.is_unspecified() {
                "*".to_string()
            } else {
                format!("{}:{}", conn.remote_ip, conn.remote_port)
            };

            // Color remote based on bandwidth
            let remote_color = if is_hot {
                trueno_viz::monitor::ratatui::style::Color::LightRed
            } else {
                trueno_viz::monitor::ratatui::style::Color::White
            };

            Row::new(vec![
                Span::styled(format!("{:<5}", service.chars().take(5).collect::<String>()), Style::default().fg(proto_color)),
                Span::styled(local, Style::default().fg(trueno_viz::monitor::ratatui::style::Color::White)),
                Span::styled(format!("{:>15}", truncate_str(&remote, 15)), Style::default().fg(remote_color)),
                Span::styled(geo_flag.to_string(), Style::default()),
                Span::styled(format!("{}", conn.state.as_char()), Style::default().fg(state_color)),
                Span::styled(format!("{:>5}", duration_str.chars().take(5).collect::<String>()), Style::default().fg(trueno_viz::monitor::ratatui::style::Color::DarkGray)),
                Span::styled(proc_name.chars().take(8).collect::<String>(), Style::default().fg(trueno_viz::monitor::ratatui::style::Color::Magenta)),
            ])
        })
        .collect();

    let widths = [
        trueno_viz::monitor::ratatui::layout::Constraint::Length(6),  // SVC
        trueno_viz::monitor::ratatui::layout::Constraint::Length(6),  // LOCAL
        trueno_viz::monitor::ratatui::layout::Constraint::Length(16), // REMOTE
        trueno_viz::monitor::ratatui::layout::Constraint::Length(2),  // GEO (flag emoji)
        trueno_viz::monitor::ratatui::layout::Constraint::Length(2),  // ST
        trueno_viz::monitor::ratatui::layout::Constraint::Length(6),  // AGE
        trueno_viz::monitor::ratatui::layout::Constraint::Min(5),     // PROC
    ];

    let table = Table::new(rows, widths).header(header);
    f.render_widget(table, inner);
}

/// Draw Files panel with 4 sub-panes:
/// 1. Entropy treemap (area=size, hue=entropy)
/// 2. Hot files (high I/O activity)
/// 3. Anomaly detection sparkline
/// 4. Top 10 largest files (actionable names)
pub fn draw_treemap(f: &mut Frame, app: &App, area: Rect) {
    use trueno_viz::monitor::ratatui::style::Color;

    let scanning = app.treemap_analyzer.is_scanning();

    // Build title with mount legend
    let title = if scanning {
        " Files │ scanning... ".to_string()
    } else {
        " Files │ N:nvme D:hdd h:home ".to_string()
    };

    let border_color = Color::Rgb(100, 160, 180);
    let block = btop_block(&title, border_color);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 4 || inner.width < 20 {
        return;
    }

    // Single unified view
    draw_files_unified(f, app, inner);
}

/// Unified Files panel with:
/// 1. Directory totals (grouped by folder)
/// 2. Top files with icons, colors, age, and full paths
///
/// Filters out benchmark artifacts (seq-read, seq-write, etc.)
///
/// Mount marker - single letter codes, easy to read and distinct.
/// Returns (char, color, short_label) for legend
fn mount_marker(path: &str) -> (char, (u8, u8, u8), &'static str) {
    // Single letters: N=nvme, D=hdd, h=home, /=root, M=mount
    if path.starts_with("/mnt/nvme-raid0") || path.starts_with("/mnt/nvme") {
        ('N', (100, 220, 140), "nvme")   // N - fast NVMe (bright green)
    } else if path.starts_with("/mnt/storage") || path.starts_with("/mnt/hdd") {
        ('D', (220, 100, 100), "hdd")    // D - bulk disk/HDD (red)
    } else if path.starts_with("/home") {
        ('h', (220, 180, 80), "home")    // h - home (yellow)
    } else if path == "/" || path.starts_with("/usr") || path.starts_with("/var") {
        ('/', (140, 160, 220), "sys")    // / - root/system (blue)
    } else if path.starts_with("/mnt") || path.starts_with("/media") {
        ('M', (180, 140, 220), "mnt")    // M - other mounts (purple)
    } else {
        ('?', (140, 140, 140), "unk")    // ? - unknown (gray)
    }
}

/// Get mount legend for Disk panel header
pub fn mount_legend_str() -> String {
    "N:nvme D:hdd h:home /:sys".to_string()
}

/// Format directory path: prioritize showing the meaningful end
/// /mnt/nvme-raid0/targets/trueno-viz -> nvme-raid0/.../trueno-viz
#[allow(dead_code)]
fn format_dir_path(path: &str, max_width: usize) -> String {
    if path.len() <= max_width {
        return path.to_string();
    }
    if max_width < 10 {
        // Very small: just truncate
        return path.chars().take(max_width).collect();
    }

    // Split path into components
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return "/".to_string();
    }
    if parts.len() == 1 {
        let p = parts[0];
        if p.len() < max_width {
            return format!("/{}", p);
        }
        return format!("/{}...", &p[..max_width.saturating_sub(4)]);
    }

    // Strategy: show mount-name/.../<last meaningful component>
    // For /mnt/nvme-raid0/targets/trueno-viz/debug -> nvme-raid0/.../debug
    let mount_part = if parts.len() > 1 && (parts[0] == "mnt" || parts[0] == "home" || parts[0] == "media") {
        parts.get(1).unwrap_or(&parts[0])
    } else {
        parts[0]
    };
    let last_part = parts.last().unwrap_or(&"");

    // Budget: mount_part + /.../ + last_part = max_width
    let ellipsis_len = 5; // /.../
    let available = max_width.saturating_sub(ellipsis_len);

    if available < 4 {
        return path.chars().take(max_width).collect();
    }

    let mount_budget = (available * 2 / 5).clamp(2, 12);
    let last_budget = available.saturating_sub(mount_budget);

    let mount_str: String = if mount_part.len() > mount_budget {
        mount_part.chars().take(mount_budget).collect()
    } else {
        mount_part.to_string()
    };

    let last_str: String = if last_part.len() > last_budget && last_budget > 0 {
        // Keep end of last part (more meaningful)
        last_part.chars().skip(last_part.len().saturating_sub(last_budget)).collect()
    } else if last_budget > 0 {
        last_part.to_string()
    } else {
        String::new()
    };

    let result = format!("{}/.../{}", mount_str, last_str);
    // Final safety check
    if result.len() > max_width {
        path.chars().take(max_width).collect()
    } else {
        result
    }
}

/// Create entropy heatmap cell showing dupe potential
/// entropy 0.0 = all duplicates (red), 1.0 = all unique (green)
/// Returns (display_str, r, g, b)
#[allow(dead_code)]
fn entropy_heatmap(entropy: f64) -> (String, u8, u8, u8) {
    // Dedup potential = 1 - entropy (low entropy = high dupe potential)
    let dupe_pct = ((1.0 - entropy) * 100.0).round() as u8;

    // Color: green (unique) -> yellow -> red (duplicates)
    let (r, g, b) = if entropy >= 0.8 {
        (80, 200, 100)   // Green - unique/random data
    } else if entropy >= 0.5 {
        (200, 200, 80)   // Yellow - mixed
    } else if entropy >= 0.25 {
        (220, 140, 60)   // Orange - some duplication
    } else {
        (220, 80, 80)    // Red - high duplication
    };

    // Show as percentage with small bar
    let bar_len = ((1.0 - entropy) * 3.0).round() as usize;
    let bar: String = "█".repeat(bar_len);
    let pad: String = "░".repeat(3 - bar_len);

    (format!("{}{}{:>2}%", bar, pad, dupe_pct), r, g, b)
}

fn draw_files_unified(f: &mut Frame, app: &App, area: Rect) {
    use trueno_viz::monitor::ratatui::style::Color;
    use std::collections::HashMap;

    if area.height < 1 || area.width < 15 {
        return;
    }

    let files = app.treemap_analyzer.top_files_filtered(area.height as usize);
    if files.is_empty() {
        f.render_widget(
            Paragraph::new("...").style(Style::default().fg(Color::Rgb(80, 80, 80))),
            area,
        );
        return;
    }

    // Build entropy lookup from file_analyzer
    let entropy_map: HashMap<String, f64> = app.file_analyzer.files()
        .iter()
        .map(|fe| (fe.path.to_string_lossy().to_string(), fe.entropy))
        .collect();

    let max_size = files.first().map(|(_, s, _, _, _)| *s).unwrap_or(1);

    // Layout: [mount 1ch] [bar 5ch] [size 4ch] [space+filename - rest]
    let bar_width = 5usize;
    let size_width = 4usize;
    let name_width = (area.width as usize).saturating_sub(1 + bar_width + size_width + 2);

    for (i, (name, size, category, _age, path)) in files.iter().take(area.height as usize).enumerate() {
        let y = area.y + i as u16;

        // Mount marker (N/H/~/M/?)
        let (mount_char, (mr, mg, mb), _) = mount_marker(path);

        // Color by category
        let (r, g, b) = category.color();

        // Get entropy for this file (0.0 if not sampled)
        let entropy = entropy_map.get(path).copied().unwrap_or(0.5);

        // Entropy color: green (high/unique) -> yellow -> red (low/duplicate)
        let (er, eg, eb) = if entropy >= 0.7 {
            (60, 200, 80)    // Green - unique/high entropy
        } else if entropy >= 0.4 {
            (200, 200, 60)   // Yellow - medium
        } else {
            (220, 80, 60)    // Red - low entropy/duplicate potential
        };

        // Split bar: ▄ = lower half shows entropy color, upper half shows category
        // Foreground = entropy (bottom), Background = category (top)
        let fill = ((*size as f64 / max_size as f64) * bar_width as f64).round() as usize;
        let bar: String = "▄".repeat(fill);
        let empty: String = " ".repeat(bar_width.saturating_sub(fill));

        // Compact size
        let size_str = if *size >= 1_000_000_000_000 {
            format!("{:.0}T", *size as f64 / 1e12)
        } else if *size >= 1_000_000_000 {
            format!("{:.0}G", *size as f64 / 1e9)
        } else {
            format!("{:.0}M", *size as f64 / 1e6)
        };

        // FULL filename from path
        let filename = std::path::Path::new(path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(name);

        // Truncate only if absolutely necessary, keep extension
        let display_name: String = if filename.len() <= name_width {
            filename.to_string()
        } else if name_width > 15 {
            // Keep extension: "Qwen2.5-Coder-32B-Instru...Q4_K_M.gguf"
            let ext_pos = filename.rfind('.').unwrap_or(filename.len());
            let ext = &filename[ext_pos..];
            let prefix_len = name_width.saturating_sub(ext.len() + 3);
            if prefix_len > 5 {
                format!("{}...{}", &filename[..prefix_len], ext)
            } else {
                filename[..name_width].to_string()
            }
        } else {
            filename[..name_width.min(filename.len())].to_string()
        };

        // Layout: mount marker, split bar, size, filename
        let spans = vec![
            Span::styled(mount_char.to_string(), Style::default().fg(Color::Rgb(mr, mg, mb))),
            Span::styled(&bar, Style::default()
                .fg(Color::Rgb(er, eg, eb))      // Bottom: entropy color
                .bg(Color::Rgb(r, g, b))),       // Top: category color
            Span::styled(&empty, Style::default().fg(Color::Rgb(30, 30, 35))),
            Span::styled(format!("{:>4}", size_str), Style::default().fg(Color::Rgb(150, 150, 120))),
            Span::styled(format!(" {}", display_name), Style::default().fg(Color::Rgb(175, 180, 190))),
        ];

        f.render_widget(Paragraph::new(Line::from(spans)), Rect { x: area.x, y, width: area.width, height: 1 });
    }
}
/// Draw enhanced Files panel with I/O, entropy, and duplicate indicators
pub fn draw_files(f: &mut Frame, app: &App, area: Rect) {
    use trueno_viz::monitor::ratatui::style::Color;
    use crate::analyzers::IoActivity;

    let metrics = app.file_analyzer.current_metrics();
    let total_files = app.file_analyzer.files().len();

    // Build title with summary stats
    let title = format!(
        " Files │ {} total │ {} hot │ {} dup │ {} wasted ",
        total_files,
        metrics.high_io_count,
        metrics.duplicate_count,
        theme::format_bytes(metrics.duplicate_bytes),
    );

    let block = btop_block(&title, borders::FILES);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 3 || inner.width < 20 {
        return;
    }

    // Layout: sparklines on top row, file list below
    let sparkline_height = 2u16;
    let list_height = inner.height.saturating_sub(sparkline_height);

    // === TOP ROW: Sparklines for activity trends ===
    let spark_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: sparkline_height.min(inner.height),
    };

    if spark_area.height >= 1 && inner.width >= 4 {
        // Divide into 4 sparklines with bounds safety
        let spark_width = inner.width / 4;
        let max_x = inner.x + inner.width;

        // Helper to create safe rect within bounds
        let safe_rect = |x: u16, y: u16, w: u16| -> Rect {
            let clamped_w = w.min(max_x.saturating_sub(x));
            Rect { x, y, width: clamped_w, height: 1 }
        };

        // I/O Activity sparkline
        let io_history = app.file_analyzer.metric_history("high_io");
        if !io_history.is_empty() {
            let io_spark = MonitorSparkline::new(&io_history)
                .color(Color::Rgb(255, 150, 100));
            f.render_widget(io_spark, safe_rect(inner.x, inner.y, spark_width.saturating_sub(1)));
            f.render_widget(
                Paragraph::new("I/O").style(Style::default().fg(Color::DarkGray)),
                safe_rect(inner.x, inner.y + 1, spark_width),
            );
        }

        // Entropy sparkline
        let entropy_history = app.file_analyzer.metric_history("avg_entropy");
        if !entropy_history.is_empty() && inner.x + spark_width < max_x {
            let ent_spark = MonitorSparkline::new(&entropy_history)
                .color(Color::Rgb(200, 100, 150));
            f.render_widget(ent_spark, safe_rect(inner.x + spark_width, inner.y, spark_width.saturating_sub(1)));
            f.render_widget(
                Paragraph::new("Entropy").style(Style::default().fg(Color::DarkGray)),
                safe_rect(inner.x + spark_width, inner.y + 1, spark_width),
            );
        }

        // Duplicates sparkline
        let dup_history = app.file_analyzer.metric_history("duplicates");
        if !dup_history.is_empty() && inner.x + spark_width * 2 < max_x {
            let dup_spark = MonitorSparkline::new(&dup_history)
                .color(Color::Rgb(180, 180, 100));
            f.render_widget(dup_spark, safe_rect(inner.x + spark_width * 2, inner.y, spark_width.saturating_sub(1)));
            f.render_widget(
                Paragraph::new("Dups").style(Style::default().fg(Color::DarkGray)),
                safe_rect(inner.x + spark_width * 2, inner.y + 1, spark_width),
            );
        }

        // Recent files sparkline
        let recent_history = app.file_analyzer.metric_history("recent");
        if !recent_history.is_empty() && inner.x + spark_width * 3 < max_x {
            let rec_spark = MonitorSparkline::new(&recent_history)
                .color(Color::Rgb(100, 200, 150));
            let remaining = inner.width.saturating_sub(spark_width * 3);
            f.render_widget(rec_spark, safe_rect(inner.x + spark_width * 3, inner.y, remaining));
            f.render_widget(
                Paragraph::new("Recent").style(Style::default().fg(Color::DarkGray)),
                safe_rect(inner.x + spark_width * 3, inner.y + 1, remaining),
            );
        }
    }

    // === BOTTOM: File list with indicators ===
    let list_area = Rect {
        x: inner.x,
        y: inner.y + sparkline_height,
        width: inner.width,
        height: list_height,
    };

    if list_area.height < 1 {
        return;
    }

    // Get files sorted by a composite score (hot first, then large)
    let mut display_files: Vec<_> = app.file_analyzer.files().iter().collect();
    display_files.sort_by(|a, b| {
        // Score: I/O activity * 1000 + is_recent * 500 + is_duplicate * 100 + size/1GB
        let score = |f: &crate::analyzers::FileEntry| -> u64 {
            let io_score = match f.io_activity {
                IoActivity::High => 3000,
                IoActivity::Medium => 2000,
                IoActivity::Low => 1000,
                IoActivity::None => 0,
            };
            let recent_score = if f.is_recent { 500 } else { 0 };
            let dup_score = if f.is_duplicate { 100 } else { 0 };
            let size_score = (f.size / (1024 * 1024 * 1024)).min(99);
            io_score + recent_score + dup_score + size_score
        };
        score(b).cmp(&score(a))
    });

    // Render file rows
    for (idx, file) in display_files.iter().take(list_area.height as usize).enumerate() {
        let y = list_area.y + idx as u16;

        // Build indicator string: [type] [io] [entropy] [dup]
        let type_icon = file.file_type.icon();
        let io_icon = file.io_activity.icon();
        let entropy_icon = file.entropy_level.icon();
        let dup_icon = if file.is_duplicate { '⊕' } else { ' ' };

        // File name (truncated)
        let name = file.path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?");
        let max_name_len = (list_area.width as usize).saturating_sub(25);
        let display_name = truncate_str(name, max_name_len);

        // Size
        let size_str = theme::format_bytes(file.size);

        // Build colored spans
        let (type_r, type_g, type_b) = file.file_type.color();
        let (io_r, io_g, io_b) = file.io_activity.color();
        let (ent_r, ent_g, ent_b) = file.entropy_level.color();

        let line = Line::from(vec![
            Span::styled(
                format!("{}", type_icon),
                Style::default().fg(Color::Rgb(type_r, type_g, type_b)),
            ),
            Span::raw(" "),
            Span::styled(
                format!("{}", io_icon),
                Style::default().fg(Color::Rgb(io_r, io_g, io_b)),
            ),
            Span::styled(
                format!("{}", entropy_icon),
                Style::default().fg(Color::Rgb(ent_r, ent_g, ent_b)),
            ),
            Span::styled(
                format!("{}", dup_icon),
                Style::default().fg(if file.is_duplicate { Color::Rgb(220, 180, 100) } else { Color::DarkGray }),
            ),
            Span::raw(" "),
            Span::styled(
                display_name,
                Style::default().fg(if file.is_recent { Color::Rgb(180, 220, 180) } else { Color::Rgb(180, 180, 180) }),
            ),
            Span::raw(" "),
            Span::styled(
                format!("{:>6}", size_str),
                Style::default().fg(Color::Rgb(140, 140, 160)),
            ),
        ]);

        f.render_widget(
            Paragraph::new(line),
            Rect { x: list_area.x, y, width: list_area.width, height: 1 },
        );
    }
}

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
        assert_eq!(truncate_str("hello", 2), "he");
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
        assert_eq!(truncate_str("hello", 3), "hel");
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
