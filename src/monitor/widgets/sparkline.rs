//! Inline sparkline widget for compact trend display.
//!
//! Uses 8-level Unicode block characters (▁▂▃▄▅▆▇█) to show trends
//! in a compact format suitable for table cells.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

/// A compact inline sparkline.
#[derive(Debug, Clone)]
pub struct MonitorSparkline<'a> {
    /// Data points to display.
    data: &'a [f64],
    /// Color for the sparkline.
    color: Color,
    /// Whether to show a trend indicator suffix (↑↓→).
    show_trend: bool,
}

impl<'a> MonitorSparkline<'a> {
    /// Creates a new sparkline with the given data.
    #[must_use]
    pub fn new(data: &'a [f64]) -> Self {
        Self {
            data,
            color: Color::Cyan,
            show_trend: true,
        }
    }

    /// Sets the color.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Sets whether to show trend indicator.
    #[must_use]
    pub fn show_trend(mut self, show: bool) -> Self {
        self.show_trend = show;
        self
    }

    /// Calculates the trend based on recent values.
    fn trend(&self) -> char {
        if self.data.len() < 2 {
            return '→';
        }

        let recent: Vec<_> = self.data.iter().rev().take(5).collect();
        if recent.len() < 2 {
            return '→';
        }

        // Safe: we've verified recent.len() >= 2 above
        let (Some(first), Some(last)) = (recent.last(), recent.first()) else {
            return '→';
        };
        let first = **first;
        let last = **last;
        let diff = last - first;
        let threshold = 0.05; // 5% threshold

        if diff > threshold {
            '↑'
        } else if diff < -threshold {
            '↓'
        } else {
            '→'
        }
    }
}

impl Widget for MonitorSparkline<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || self.data.is_empty() {
            return;
        }

        // 8-level block characters
        let blocks = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

        // Find min/max for scaling
        let min = self.data.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = self.data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max - min;

        // Reserve space for trend indicator
        let chart_width = if self.show_trend {
            area.width.saturating_sub(1)
        } else {
            area.width
        } as usize;

        // Sample data to fit width
        for i in 0..chart_width.min(self.data.len()) {
            let data_idx = if chart_width >= self.data.len() {
                i
            } else {
                (i * self.data.len()) / chart_width
            };

            let value = self.data.get(data_idx).copied().unwrap_or(0.0);
            let normalized = if range > 0.0 {
                ((value - min) / range).clamp(0.0, 1.0)
            } else {
                0.5
            };

            let block_idx = ((normalized * 7.0) as usize).min(7);
            let block = blocks[block_idx];

            buf.set_string(
                area.x + i as u16,
                area.y,
                block.to_string(),
                Style::default().fg(self.color),
            );
        }

        // Add trend indicator
        if self.show_trend && area.width > 1 {
            let trend = self.trend();
            buf.set_string(
                area.x + area.width - 1,
                area.y,
                trend.to_string(),
                Style::default().fg(self.color),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn test_sparkline_new() {
        let data = vec![0.5; 10];
        let sparkline = MonitorSparkline::new(&data);

        assert_eq!(sparkline.color, Color::Cyan);
        assert!(sparkline.show_trend);
    }

    #[test]
    fn test_sparkline_trend_up() {
        let data = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let sparkline = MonitorSparkline::new(&data);

        assert_eq!(sparkline.trend(), '↑');
    }

    #[test]
    fn test_sparkline_trend_down() {
        let data = vec![0.5, 0.4, 0.3, 0.2, 0.1];
        let sparkline = MonitorSparkline::new(&data);

        assert_eq!(sparkline.trend(), '↓');
    }

    #[test]
    fn test_sparkline_trend_stable() {
        let data = vec![0.5, 0.5, 0.5, 0.5, 0.5];
        let sparkline = MonitorSparkline::new(&data);

        assert_eq!(sparkline.trend(), '→');
    }

    #[test]
    fn test_sparkline_renders() {
        let backend = TestBackend::new(15, 1);
        let mut terminal = Terminal::new(backend).unwrap();

        let data = vec![0.1, 0.3, 0.5, 0.7, 0.9];

        terminal
            .draw(|frame| {
                let sparkline = MonitorSparkline::new(&data);
                frame.render_widget(sparkline, frame.area());
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let content: String = buffer
            .content()
            .iter()
            .map(|c| c.symbol().chars().next().unwrap_or(' '))
            .collect();

        // Should contain block characters
        assert!(
            content.chars().any(|c| "▁▂▃▄▅▆▇█".contains(c)),
            "Should contain block characters"
        );
    }
}
