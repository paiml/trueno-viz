//! Process panel component.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crate::monitor::collectors::ProcessCollector;

/// Process monitoring panel.
#[derive(Debug)]
pub struct ProcessPanel {
    /// Process collector.
    pub collector: ProcessCollector,
}

impl ProcessPanel {
    /// Creates a new process panel.
    #[must_use]
    pub fn new() -> Self {
        Self {
            collector: ProcessCollector::new(),
        }
    }
}

impl Default for ProcessPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for &ProcessPanel {
    fn render(self, _area: Rect, _buf: &mut Buffer) {
        // TODO: Implement full panel rendering
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_panel_new() {
        let _panel = ProcessPanel::new();
    }
}
