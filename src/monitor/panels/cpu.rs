//! CPU panel component.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crate::monitor::collectors::CpuCollector;

/// CPU monitoring panel.
#[derive(Debug)]
pub struct CpuPanel {
    /// CPU collector.
    pub collector: CpuCollector,
}

impl CpuPanel {
    /// Creates a new CPU panel.
    #[must_use]
    pub fn new() -> Self {
        Self {
            collector: CpuCollector::new(),
        }
    }
}

impl Default for CpuPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for &CpuPanel {
    fn render(self, _area: Rect, _buf: &mut Buffer) {
        // TODO: Implement full panel rendering
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_panel_new() {
        let panel = CpuPanel::new();
        assert!(panel.collector.core_count() >= 1);
    }
}
