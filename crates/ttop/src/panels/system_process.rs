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
