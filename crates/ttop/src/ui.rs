//! UI layout and rendering for ttop.

use trueno_viz::monitor::ratatui::layout::{Constraint, Direction, Layout, Rect};
use trueno_viz::monitor::ratatui::style::{Color, Modifier, Style};
use trueno_viz::monitor::ratatui::text::{Line, Span};
use trueno_viz::monitor::ratatui::widgets::{Block, Borders, Clear, Paragraph};
use trueno_viz::monitor::ratatui::Frame;
use trueno_viz::monitor::types::Collector;

use crate::app::App;
use crate::panels;

/// Main draw function
pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Debug: log area size
    if std::env::var("TTOP_DEBUG").is_ok() {
        eprintln!("draw: area={}x{}", area.width, area.height);
    }

    // Calculate visible panel count for layout
    let top_panel_count = count_top_panels(app);
    let has_process = app.panels.process;

    if std::env::var("TTOP_DEBUG").is_ok() {
        eprintln!("draw: top_panels={}, has_process={}", top_panel_count, has_process);
    }

    // Layout based on visible panels
    if top_panel_count == 0 && !has_process {
        // Nothing to show
        if std::env::var("TTOP_DEBUG").is_ok() {
            eprintln!("draw: nothing to show!");
        }
        return;
    }

    let main_chunks = if top_panel_count > 0 && has_process {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(area)
    } else {
        // Either only top panels or only process panel - full height
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(100)])
            .split(area)
    };

    // Draw top panels in a grid layout
    if top_panel_count > 0 {
        draw_top_panels(f, app, main_chunks[0]);
    }

    // Draw bottom row: Processes | Network Connections | File Treemap
    if has_process {
        let bottom_area = if top_panel_count > 0 {
            main_chunks[1]
        } else {
            main_chunks[0]
        };

        // Split bottom into 3 columns
        let bottom_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40), // Processes
                Constraint::Percentage(30), // Network connections
                Constraint::Percentage(30), // File treemap
            ])
            .split(bottom_area);

        panels::draw_process(f, app, bottom_chunks[0]);
        panels::draw_connections(f, app, bottom_chunks[1]);
        panels::draw_treemap(f, app, bottom_chunks[2]);
    }

    // FPS overlay
    if app.show_fps {
        draw_fps_overlay(f, app, area);
    }

    // Help overlay
    if app.show_help {
        draw_help_overlay(f, area);
    }

    // Filter input overlay
    if app.show_filter_input {
        draw_filter_input(f, app, area);
    }
}

fn count_top_panels(app: &App) -> u32 {
    let mut count = 0;
    if app.panels.cpu {
        count += 1;
    }
    if app.panels.memory {
        count += 1;
    }
    if app.panels.disk {
        count += 1;
    }
    if app.panels.network {
        count += 1;
    }
    if app.panels.gpu && app.has_gpu() {
        count += 1;
    }
    if app.panels.battery && app.battery.is_available() {
        count += 1;
    }
    if app.panels.sensors && (app.sensors.is_available() || app.psi_analyzer.is_available() || app.container_analyzer.is_available()) {
        count += 1;
    }
    count
}

fn draw_top_panels(f: &mut Frame, app: &App, area: Rect) {
    let panel_count = count_top_panels(app);
    if panel_count == 0 {
        return;
    }

    // Determine grid layout: 2 rows with adaptive columns
    let cols = panel_count.div_ceil(2).max(1);
    let rows = if panel_count > cols { 2 } else { 1 };

    let row_constraints: Vec<Constraint> = (0..rows).map(|_| Constraint::Ratio(1, rows)).collect();

    let row_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(area);

    type PanelDrawFn = fn(&mut Frame, &App, Rect);
    let mut panel_idx = 0;
    let mut panels_to_draw: Vec<(PanelDrawFn, &App)> = Vec::new();

    // Collect panels to draw
    if app.panels.cpu {
        panels_to_draw.push((panels::draw_cpu, app));
    }
    if app.panels.memory {
        panels_to_draw.push((panels::draw_memory, app));
    }
    if app.panels.disk {
        panels_to_draw.push((panels::draw_disk, app));
    }
    if app.panels.network {
        panels_to_draw.push((panels::draw_network, app));
    }
    if app.panels.gpu && app.has_gpu() {
        panels_to_draw.push((panels::draw_gpu, app));
    }
    if app.panels.battery && app.battery.is_available() {
        panels_to_draw.push((panels::draw_battery, app));
    }
    if app.panels.sensors && (app.sensors.is_available() || app.psi_analyzer.is_available() || app.container_analyzer.is_available()) {
        panels_to_draw.push((panels::draw_system, app));
    }

    // Draw panels in grid
    for (row_idx, row_chunk) in row_chunks.iter().enumerate() {
        let first_row_count = (panel_count as usize).div_ceil(2);
        let panels_in_row = if row_idx == 0 {
            first_row_count
        } else {
            panel_count as usize - first_row_count
        };

        if panels_in_row == 0 {
            continue;
        }

        let col_constraints: Vec<Constraint> = (0..panels_in_row)
            .map(|_| Constraint::Ratio(1, panels_in_row as u32))
            .collect();

        let col_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(col_constraints)
            .split(*row_chunk);

        for col_chunk in col_chunks.iter() {
            if panel_idx < panels_to_draw.len() {
                let (draw_fn, app_ref) = panels_to_draw[panel_idx];
                draw_fn(f, app_ref, *col_chunk);
                panel_idx += 1;
            }
        }
    }
}

fn draw_fps_overlay(f: &mut Frame, app: &App, area: Rect) {
    let fps_str = format!(
        " Frame: {:4}μs avg │ {:4}μs max │ ID: {} ",
        app.avg_frame_time_us, app.max_frame_time_us, app.frame_id
    );

    let fps_para =
        Paragraph::new(fps_str).style(Style::default().fg(Color::Green).bg(Color::Black));

    let fps_area = Rect {
        x: area.width.saturating_sub(50),
        y: 0,
        width: 50.min(area.width),
        height: 1,
    };

    f.render_widget(fps_para, fps_area);
}

fn draw_help_overlay(f: &mut Frame, area: Rect) {
    let popup_width = 65;
    let popup_height = 28;

    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width.min(area.width),
        height: popup_height.min(area.height),
    };

    f.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  ttop - Terminal Top (10X Better Than btop)",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  Pure Rust System Monitor",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Navigation:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("    j/k, ↑/↓          Move up/down"),
        Line::from("    PgUp/PgDn         Page up/down"),
        Line::from("    g/G               Go to top/bottom"),
        Line::from(""),
        Line::from(Span::styled(
            "  Sorting & Filtering:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("    s, Tab            Cycle sort column"),
        Line::from("    r                 Reverse sort order"),
        Line::from("    f, /              Filter processes"),
        Line::from("    Del               Clear filter"),
        Line::from("    t                 Toggle tree view"),
        Line::from(""),
        Line::from(Span::styled(
            "  Panels:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("    1                 Toggle CPU panel"),
        Line::from("    2                 Toggle Memory panel"),
        Line::from("    3                 Toggle Disk panel"),
        Line::from("    4                 Toggle Network panel"),
        Line::from("    5                 Toggle Process panel"),
        Line::from("    6                 Toggle GPU panel"),
        Line::from("    7                 Toggle Battery panel"),
        Line::from("    8                 Toggle Sensors panel"),
        Line::from("    0                 Reset all panels"),
        Line::from(""),
        Line::from(Span::styled(
            "  General:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("    q, Esc            Quit"),
        Line::from("    ?, F1             Toggle help"),
        Line::from(""),
    ];

    let help = Paragraph::new(help_text).block(
        Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    f.render_widget(help, popup_area);
}

fn draw_filter_input(f: &mut Frame, app: &App, area: Rect) {
    let input_width = 50;
    let input_area = Rect {
        x: (area.width.saturating_sub(input_width)) / 2,
        y: area.height / 2,
        width: input_width.min(area.width),
        height: 3,
    };

    f.render_widget(Clear, input_area);

    let input = Paragraph::new(app.filter.as_str())
        .block(
            Block::default()
                .title(" Filter (Enter to confirm, Esc to cancel) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(input, input_area);
}
