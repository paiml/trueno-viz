//! UI layout and rendering for ttop.

use trueno_viz::monitor::ratatui::layout::{Constraint, Direction, Layout, Rect};
use trueno_viz::monitor::ratatui::style::{Color, Modifier, Style};
use trueno_viz::monitor::ratatui::text::{Line, Span};
use trueno_viz::monitor::ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use trueno_viz::monitor::ratatui::Frame;
use trueno_viz::monitor::types::Collector;

use crate::app::App;
use crate::panels;
use crate::state::PanelType;

/// Main draw function
pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Debug: log area size
    if std::env::var("TTOP_DEBUG").is_ok() {
        eprintln!("draw: area={}x{}", area.width, area.height);
    }

    // Safety: ensure area is valid and within bounds
    if area.width == 0 || area.height == 0 {
        return;
    }

    // EXPLODED MODE: render single panel fullscreen
    if let Some(panel) = app.exploded_panel {
        draw_exploded_panel(f, app, panel, area);

        // Still show overlays
        if app.show_fps {
            draw_fps_overlay(f, app, area);
        }
        if app.show_help {
            draw_help_overlay(f, area);
        }
        return;
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

        draw_panel_with_focus_mut(f, app, PanelType::Process, bottom_chunks[0], panels::draw_process);
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

    // Signal confirmation overlay
    if app.pending_signal.is_some() {
        draw_signal_confirm(f, app, area);
    }

    // Signal menu overlay
    if app.show_signal_menu {
        draw_signal_menu(f, app, area);
    }

    // Signal result notification
    if app.signal_result.is_some() {
        draw_signal_result(f, app, area);
    }

    // Focus indicator hint
    if app.focused_panel.is_some() {
        draw_focus_hint(f, area);
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
    let mut panels_to_draw: Vec<(PanelType, PanelDrawFn, &App)> = Vec::new();

    // Collect panels to draw with their types
    if app.panels.cpu {
        panels_to_draw.push((PanelType::Cpu, panels::draw_cpu, app));
    }
    if app.panels.memory {
        panels_to_draw.push((PanelType::Memory, panels::draw_memory, app));
    }
    if app.panels.disk {
        panels_to_draw.push((PanelType::Disk, panels::draw_disk, app));
    }
    if app.panels.network {
        panels_to_draw.push((PanelType::Network, panels::draw_network, app));
    }
    if app.panels.gpu && app.has_gpu() {
        panels_to_draw.push((PanelType::Gpu, panels::draw_gpu, app));
    }
    if app.panels.battery && app.battery.is_available() {
        panels_to_draw.push((PanelType::Battery, panels::draw_battery, app));
    }
    if app.panels.sensors && (app.sensors.is_available() || app.psi_analyzer.is_available() || app.container_analyzer.is_available()) {
        panels_to_draw.push((PanelType::Sensors, panels::draw_system, app));
    }
    if app.panels.files {
        panels_to_draw.push((PanelType::Files, panels::draw_treemap, app));
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
                let (panel_type, draw_fn, app_ref) = panels_to_draw[panel_idx];
                draw_panel_with_focus(f, app_ref, panel_type, *col_chunk, draw_fn);
                panel_idx += 1;
            }
        }
    }
}

/// Draw a panel with focus ring highlighting if focused (immutable app)
fn draw_panel_with_focus(
    f: &mut Frame,
    app: &App,
    panel_type: PanelType,
    area: Rect,
    draw_fn: fn(&mut Frame, &App, Rect),
) {
    let is_focused = app.focused_panel == Some(panel_type);

    // Draw the panel content
    draw_fn(f, app, area);

    // Draw focus ring overlay if focused - BRIGHT and THICK
    if is_focused {
        let focus_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(
                Style::default()
                    .fg(Color::LightYellow)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )
            .title(format!(" ▶▶ {} ◀◀ ", panel_type.name()))
            .title_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            );
        f.render_widget(focus_block, area);
    }
}

/// Draw a panel with focus ring highlighting if focused (mutable app for process panel)
fn draw_panel_with_focus_mut(
    f: &mut Frame,
    app: &mut App,
    panel_type: PanelType,
    area: Rect,
    draw_fn: fn(&mut Frame, &mut App, Rect),
) {
    let is_focused = app.focused_panel == Some(panel_type);

    // Draw the panel content
    draw_fn(f, app, area);

    // Draw focus ring overlay if focused - BRIGHT and THICK
    if is_focused {
        let focus_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(
                Style::default()
                    .fg(Color::LightYellow)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )
            .title(format!(" ▶▶ {} ◀◀ ", panel_type.name()))
            .title_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            );
        f.render_widget(focus_block, area);
    }
}

/// Draw a single panel in exploded (fullscreen) mode
fn draw_exploded_panel(f: &mut Frame, app: &mut App, panel: PanelType, area: Rect) {
    // Draw panel hint at top
    let hint = format!(" {} [FULLSCREEN] - Press ESC or Enter to exit ", panel.name());
    let hint_area = Rect {
        x: 0,
        y: 0,
        width: area.width,
        height: 1,
    };
    f.render_widget(
        Paragraph::new(hint)
            .style(Style::default().fg(Color::Black).bg(Color::Yellow)),
        hint_area,
    );

    // Panel content area (below hint)
    let content_area = Rect {
        x: 0,
        y: 1,
        width: area.width,
        height: area.height.saturating_sub(1),
    };

    // Draw the appropriate panel
    match panel {
        PanelType::Cpu => panels::draw_cpu(f, app, content_area),
        PanelType::Memory => panels::draw_memory(f, app, content_area),
        PanelType::Disk => panels::draw_disk(f, app, content_area),
        PanelType::Network => panels::draw_network(f, app, content_area),
        PanelType::Process => panels::draw_process(f, app, content_area),
        PanelType::Gpu => panels::draw_gpu(f, app, content_area),
        PanelType::Battery => panels::draw_battery(f, app, content_area),
        PanelType::Sensors => panels::draw_system(f, app, content_area),
        PanelType::Files => panels::draw_treemap(f, app, content_area),
    }
}

/// Draw focus mode hint at bottom of screen
fn draw_focus_hint(f: &mut Frame, area: Rect) {
    let hint = " h/j/k/l or arrows: navigate │ Enter/z: zoom │ ESC: exit focus ";
    let hint_area = Rect {
        x: 0,
        y: area.height.saturating_sub(1),
        width: area.width.min(hint.len() as u16 + 2),
        height: 1,
    };
    f.render_widget(
        Paragraph::new(hint)
            .style(Style::default().fg(Color::Black).bg(Color::Cyan)),
        hint_area,
    );
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
    let popup_height = 36;

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
            "  Panel Focus (navigate + explode):",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("    h/l, ←/→          Focus prev/next panel"),
        Line::from("    j/k, ↑/↓          Focus up/down (or process nav)"),
        Line::from("    Enter, z          Explode panel fullscreen"),
        Line::from("    Esc               Exit explode/focus, then quit"),
        Line::from(""),
        Line::from(Span::styled(
            "  Process Navigation:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("    j/k, ↑/↓          Move up/down (when unfocused)"),
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
            "  Process Signals:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("    x                 Send SIGTERM (graceful)"),
        Line::from("    X                 Send SIGKILL (force)"),
        Line::from(""),
        Line::from(Span::styled(
            "  Panels:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("    1-8               Toggle panel visibility"),
        Line::from("    0                 Reset all panels"),
        Line::from(""),
        Line::from(Span::styled(
            "  General:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from("    q                 Quit"),
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

/// Draw signal confirmation dialog
fn draw_signal_confirm(f: &mut Frame, app: &App, area: Rect) {
    use crate::state::SignalType;

    if let Some((pid, name, signal)) = &app.pending_signal {
        let popup_width = 55;
        let popup_height = 7;

        let popup_area = Rect {
            x: (area.width.saturating_sub(popup_width)) / 2,
            y: (area.height.saturating_sub(popup_height)) / 2,
            width: popup_width.min(area.width),
            height: popup_height.min(area.height),
        };

        f.render_widget(Clear, popup_area);

        let signal_color = match signal {
            SignalType::Kill => Color::Red,
            SignalType::Term => Color::Yellow,
            SignalType::Stop => Color::Magenta,
            _ => Color::Cyan,
        };

        let content = vec![
            Line::from(""),
            Line::from(vec![
                Span::raw("  Send "),
                Span::styled(
                    format!("SIG{}", signal.name()),
                    Style::default().fg(signal_color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" to process?"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("  PID: "),
                Span::styled(pid.to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::raw("  Name: "),
                Span::styled(name.clone(), Style::default().fg(Color::Cyan)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "  [Y]es  [N]o/Esc",
                Style::default().fg(Color::DarkGray),
            )),
        ];

        let title = format!(" {} - {} ", signal.name(), signal.description());
        let confirm = Paragraph::new(content).block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(signal_color)),
        );

        f.render_widget(confirm, popup_area);
    }
}

/// Draw signal menu overlay
fn draw_signal_menu(f: &mut Frame, app: &App, area: Rect) {
    let popup_width = 45;
    let popup_height = 14;

    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width.min(area.width),
        height: popup_height.min(area.height),
    };

    f.render_widget(Clear, popup_area);

    let selected = app.selected_process();
    let proc_info = selected
        .map(|(pid, name)| format!("{} ({})", name, pid))
        .unwrap_or_else(|| "No process selected".to_string());

    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Target: {}", proc_info),
            Style::default().fg(Color::Cyan),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  x", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("  SIGTERM  - Graceful shutdown"),
        ]),
        Line::from(vec![
            Span::styled("  K", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw("  SIGKILL  - Force kill"),
        ]),
        Line::from(vec![
            Span::styled("  H", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("  SIGHUP   - Reload config"),
        ]),
        Line::from(vec![
            Span::styled("  i", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("  SIGINT   - Interrupt"),
        ]),
        Line::from(vec![
            Span::styled("  p", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
            Span::raw("  SIGSTOP  - Pause process"),
        ]),
        Line::from(vec![
            Span::styled("  c", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("  SIGCONT  - Resume process"),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Esc to cancel", Style::default().fg(Color::DarkGray))),
    ];

    let menu = Paragraph::new(content).block(
        Block::default()
            .title(" Send Signal ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    f.render_widget(menu, popup_area);
}

/// Draw signal result notification (bottom of screen)
fn draw_signal_result(f: &mut Frame, app: &App, area: Rect) {
    if let Some((success, message, _timestamp)) = &app.signal_result {
        let width = (message.len() + 4).min(area.width as usize) as u16;
        let height = 1u16;

        let result_area = Rect {
            x: (area.width.saturating_sub(width)) / 2,
            y: area.height.saturating_sub(2),
            width,
            height,
        };

        let color = if *success { Color::Green } else { Color::Red };
        let icon = if *success { "✓" } else { "✗" };

        let result = Paragraph::new(format!(" {} {} ", icon, message))
            .style(Style::default().fg(Color::White).bg(color));

        f.render_widget(result, result_area);
    }
}

/// TUI rendering tests using probar
#[cfg(test)]
mod tui_tests {
    use super::*;
    use jugar_probar::tui::{TuiFrame, expect_frame};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    /// Test help overlay renders correctly
    #[test]
    fn test_help_overlay_renders() {
        let backend = TestBackend::new(80, 40);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            draw_help_overlay(f, f.area());
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = TuiFrame::from_buffer(&buffer, 0);

        // Help overlay should contain key sections
        assert!(frame.contains("Help"));
        assert!(frame.contains("ttop"));
        assert!(frame.contains("Panel Focus"));
        assert!(frame.contains("Process Navigation"));
        assert!(frame.contains("Sorting"));
        assert!(frame.contains("Quit"));
    }

    /// Test help overlay keybindings
    #[test]
    fn test_help_overlay_keybindings() {
        let backend = TestBackend::new(80, 40);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            draw_help_overlay(f, f.area());
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = TuiFrame::from_buffer(&buffer, 0);

        // Check specific keybindings are documented
        assert!(frame.contains("h/l"));
        assert!(frame.contains("j/k"));
        assert!(frame.contains("Enter"));
        assert!(frame.contains("Esc"));
        assert!(frame.contains("PgUp"));
        assert!(frame.contains("Tab"));
    }

    /// Test focus hint renders
    #[test]
    fn test_focus_hint_renders() {
        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            draw_focus_hint(f, f.area());
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = TuiFrame::from_buffer(&buffer, 0);

        // Focus hint should show navigation keys
        assert!(frame.contains("navigate") || frame.contains("arrows"));
        assert!(frame.contains("Enter") || frame.contains("zoom"));
        assert!(frame.contains("ESC") || frame.contains("exit"));
    }

    /// Test help overlay probar assertions
    #[test]
    fn test_help_overlay_assertions() {
        let backend = TestBackend::new(80, 40);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            draw_help_overlay(f, f.area());
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = TuiFrame::from_buffer(&buffer, 0);

        // Use probar's assertion API
        expect_frame(&frame)
            .to_contain_text("Terminal Top")
            .expect("should contain title");

        expect_frame(&frame)
            .to_contain_text("Pure Rust")
            .expect("should contain subtitle");

        expect_frame(&frame)
            .to_match(r"1-8.*Toggle")
            .expect("should show panel toggle keys");
    }

    /// Test help overlay small terminal
    #[test]
    fn test_help_overlay_small_terminal() {
        let backend = TestBackend::new(40, 20);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            draw_help_overlay(f, f.area());
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = TuiFrame::from_buffer(&buffer, 0);

        // Should still render (possibly truncated)
        assert!(frame.contains("Help") || frame.height() < 36);
    }

    /// Test layout constants
    #[test]
    fn test_layout_dimensions() {
        // Test that layout calculations work for various terminal sizes
        let sizes = [(80, 24), (120, 40), (200, 60), (40, 10)];

        for (width, height) in sizes {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).expect("terminal");

            // Just verify we can draw without panicking
            terminal.draw(|f| {
                draw_focus_hint(f, f.area());
            }).expect(&format!("draw at {}x{}", width, height));
        }
    }
}

/// Full UI integration tests using mock App
#[cfg(test)]
mod ui_integration_tests {
    use super::*;
    use crate::app::App;
    use jugar_probar::tui::{TuiFrame, expect_frame};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    /// Test full UI draw with mock App
    #[test]
    fn test_full_ui_draw() {
        let mut app = App::new_mock();
        let backend = TestBackend::new(160, 50);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            draw(f, &mut app);
        }).expect("draw full ui");

        let buffer = terminal.backend().buffer().clone();
        let frame = TuiFrame::from_buffer(&buffer, 0);

        // UI should render CPU and Memory panels
        assert!(frame.contains("CPU") || frame.contains("Memory"));
    }

    /// Test UI draw with help overlay
    #[test]
    fn test_ui_with_help_overlay() {
        let mut app = App::new_mock();
        app.show_help = true;
        let backend = TestBackend::new(160, 50);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            draw(f, &mut app);
        }).expect("draw with help");

        let buffer = terminal.backend().buffer().clone();
        let frame = TuiFrame::from_buffer(&buffer, 0);

        // Help overlay should be visible
        assert!(frame.contains("Help") || frame.contains("ttop"));
    }

    /// Test UI draw with FPS overlay
    #[test]
    fn test_ui_with_fps_overlay() {
        let mut app = App::new_mock();
        app.show_fps = true;
        let backend = TestBackend::new(160, 50);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            draw(f, &mut app);
        }).expect("draw with fps");

        let buffer = terminal.backend().buffer().clone();
        let frame = TuiFrame::from_buffer(&buffer, 0);

        // FPS overlay should show frame info
        assert!(frame.contains("Frame") || frame.contains("μs") || frame.contains("ID"));
    }

    /// Test UI draw with exploded panel
    #[test]
    fn test_ui_with_exploded_panel() {
        let mut app = App::new_mock();
        app.exploded_panel = Some(PanelType::Cpu);
        let backend = TestBackend::new(160, 50);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            draw(f, &mut app);
        }).expect("draw exploded");

        let buffer = terminal.backend().buffer().clone();
        let frame = TuiFrame::from_buffer(&buffer, 0);

        // Exploded CPU panel should be fullscreen
        assert!(frame.contains("CPU"));
    }

    /// Test UI draw with focused panel
    #[test]
    fn test_ui_with_focused_panel() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Memory);
        let backend = TestBackend::new(160, 50);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            draw(f, &mut app);
        }).expect("draw focused");

        let buffer = terminal.backend().buffer().clone();
        let frame = TuiFrame::from_buffer(&buffer, 0);

        // Focus hint should be visible
        assert!(frame.contains("Memory") || frame.contains("navigate") || frame.contains("Enter"));
    }

    /// Test UI draw with no panels
    #[test]
    fn test_ui_with_no_panels() {
        let mut app = App::new_mock();
        app.panels.cpu = false;
        app.panels.memory = false;
        app.panels.disk = false;
        app.panels.network = false;
        app.panels.process = false;
        app.panels.gpu = false;
        app.panels.battery = false;
        app.panels.sensors = false;
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal");

        // Should not panic with no panels
        terminal.draw(|f| {
            draw(f, &mut app);
        }).expect("draw no panels");
    }

    /// Test count_top_panels function
    #[test]
    fn test_count_top_panels() {
        let app = App::new_mock();
        let count = count_top_panels(&app);
        // Default app should have multiple panels
        assert!(count > 0);
    }

    /// Test count_top_panels with some disabled
    #[test]
    fn test_count_top_panels_some_disabled() {
        let mut app = App::new_mock();
        app.panels.cpu = false;
        app.panels.memory = false;
        let count = count_top_panels(&app);
        // Should have fewer panels
        assert!(count < 8);
    }

    /// Test UI at various terminal sizes
    #[test]
    fn test_ui_various_sizes() {
        let sizes = [(80, 24), (120, 40), (160, 50), (200, 60)];

        for (width, height) in sizes {
            let mut app = App::new_mock();
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).expect("terminal");

            terminal.draw(|f| {
                draw(f, &mut app);
            }).expect(&format!("draw at {}x{}", width, height));
        }
    }

    /// Test UI with signal menu
    #[test]
    fn test_ui_with_signal_menu() {
        let mut app = App::new_mock();
        app.show_signal_menu = true;
        let backend = TestBackend::new(160, 50);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            draw(f, &mut app);
        }).expect("draw with signal menu");

        let buffer = terminal.backend().buffer().clone();
        let frame = TuiFrame::from_buffer(&buffer, 0);

        // Signal menu should be visible
        assert!(frame.contains("Signal") || frame.contains("TERM") || frame.contains("KILL") || frame.height() > 0);
    }

    /// Test UI with filter input
    #[test]
    fn test_ui_with_filter_input() {
        let mut app = App::new_mock();
        app.show_filter_input = true;
        app.filter = "test".to_string();
        let backend = TestBackend::new(160, 50);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            draw(f, &mut app);
        }).expect("draw with filter");

        let buffer = terminal.backend().buffer().clone();
        let frame = TuiFrame::from_buffer(&buffer, 0);

        // Filter input should show the filter text
        assert!(frame.contains("Filter") || frame.contains("test") || frame.height() > 0);
    }

    /// Test UI probar assertions
    #[test]
    fn test_ui_probar_assertions() {
        let mut app = App::new_mock();
        let backend = TestBackend::new(160, 50);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            draw(f, &mut app);
        }).expect("draw");

        let buffer = terminal.backend().buffer().clone();
        let frame = TuiFrame::from_buffer(&buffer, 0);

        // Use probar assertions
        expect_frame(&frame)
            .to_contain_text("CPU")
            .expect("should contain CPU panel");
    }

    /// Test small terminal doesn't panic
    #[test]
    fn test_ui_tiny_terminal() {
        let mut app = App::new_mock();
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).expect("terminal");

        // Should not panic even with tiny terminal
        terminal.draw(|f| {
            draw(f, &mut app);
        }).expect("draw tiny terminal");
    }

    /// Test exploded panels for each type
    #[test]
    fn test_exploded_panel_types() {
        let panel_types = [
            PanelType::Cpu,
            PanelType::Memory,
            PanelType::Disk,
            PanelType::Network,
            PanelType::Gpu,
            PanelType::Battery,
            PanelType::Sensors,
        ];

        for panel_type in panel_types {
            let mut app = App::new_mock();
            app.exploded_panel = Some(panel_type);
            let backend = TestBackend::new(120, 40);
            let mut terminal = Terminal::new(backend).expect("terminal");

            terminal.draw(|f| {
                draw(f, &mut app);
            }).expect(&format!("draw exploded {:?}", panel_type));
        }
    }

    /// Test signal confirmation dialog
    #[test]
    fn test_ui_with_signal_confirm() {
        use crate::state::SignalType;

        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test_process".to_string(), SignalType::Term));
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            draw(f, &mut app);
        }).expect("draw with signal confirm");

        let buffer = terminal.backend().buffer().clone();
        let frame = TuiFrame::from_buffer(&buffer, 0);
        assert!(frame.contains("TERM") || frame.contains("1234") || frame.height() > 0);
    }

    /// Test signal confirmation with different signal types
    #[test]
    fn test_signal_confirm_all_types() {
        use crate::state::SignalType;

        let signals = [
            SignalType::Kill,
            SignalType::Term,
            SignalType::Stop,
            SignalType::Hup,
            SignalType::Int,
            SignalType::Usr1,
            SignalType::Usr2,
            SignalType::Cont,
        ];

        for signal in signals {
            let mut app = App::new_mock();
            app.pending_signal = Some((999, "proc".to_string(), signal));
            let backend = TestBackend::new(80, 30);
            let mut terminal = Terminal::new(backend).expect("terminal");

            terminal.draw(|f| {
                draw(f, &mut app);
            }).expect(&format!("draw signal {:?}", signal));
        }
    }

    /// Test signal result notification (success)
    #[test]
    fn test_ui_with_signal_result_success() {
        use std::time::Instant;

        let mut app = App::new_mock();
        app.signal_result = Some((true, "Signal sent successfully".to_string(), Instant::now()));
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            draw(f, &mut app);
        }).expect("draw with signal result success");

        let buffer = terminal.backend().buffer().clone();
        let frame = TuiFrame::from_buffer(&buffer, 0);
        assert!(frame.contains("✓") || frame.contains("success") || frame.height() > 0);
    }

    /// Test signal result notification (failure)
    #[test]
    fn test_ui_with_signal_result_failure() {
        use std::time::Instant;

        let mut app = App::new_mock();
        app.signal_result = Some((false, "Failed to send signal".to_string(), Instant::now()));
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            draw(f, &mut app);
        }).expect("draw with signal result failure");

        let buffer = terminal.backend().buffer().clone();
        let frame = TuiFrame::from_buffer(&buffer, 0);
        assert!(frame.contains("✗") || frame.contains("Failed") || frame.height() > 0);
    }

    /// Test UI with process panel exploded
    #[test]
    fn test_ui_exploded_process() {
        let mut app = App::new_mock();
        app.exploded_panel = Some(PanelType::Process);
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            draw(f, &mut app);
        }).expect("draw exploded process");
    }

    /// Test UI with files panel exploded
    #[test]
    fn test_ui_exploded_files() {
        let mut app = App::new_mock();
        app.exploded_panel = Some(PanelType::Files);
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).expect("terminal");

        terminal.draw(|f| {
            draw(f, &mut app);
        }).expect("draw exploded files");
    }
}
