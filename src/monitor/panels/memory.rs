//! Memory panel component.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crate::monitor::collectors::MemoryCollector;

/// Memory monitoring panel.
#[derive(Debug)]
pub struct MemoryPanel {
    /// Memory collector.
    pub collector: MemoryCollector,
}

impl MemoryPanel {
    /// Creates a new memory panel.
    #[must_use]
    pub fn new() -> Self {
        Self {
            collector: MemoryCollector::new(),
        }
    }
}

impl Default for MemoryPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for &MemoryPanel {
    fn render(self, _area: Rect, _buf: &mut Buffer) {
        // TODO: Implement full panel rendering
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_panel_new() {
        let _panel = MemoryPanel::new();
    }
}
