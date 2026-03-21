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
