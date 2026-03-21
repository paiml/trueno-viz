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
