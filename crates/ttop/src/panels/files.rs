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
#[path = "files_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "files_tests_b.rs"]
mod tests_b;

#[cfg(test)]
#[path = "files_tests_extended.rs"]
mod tests_extended;
