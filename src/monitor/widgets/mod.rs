//! TUI widgets for the monitoring system.
//!
//! This module provides reusable widgets for building monitoring panels:
//!
//! - [`Graph`]: Time-series visualization with braille/block/TTY modes
//! - [`Meter`]: Percentage bar with gradient coloring
//! - [`Gauge`]: Arc/circular gauge for compact display
//! - [`Table`]: Sortable, scrollable data table
//! - [`Tree`]: Collapsible hierarchy view
//! - [`Sparkline`]: Inline mini-graph
//! - [`Heatmap`]: Grid heatmap for temperature/load visualization
//!
//! All widgets implement the ratatui `Widget` trait for rendering.

pub mod gauge;
pub mod graph;
pub mod heatmap;
pub mod meter;
pub mod sparkline;
pub mod table;
pub mod tree;

pub use gauge::{Gauge, GaugeMode};
pub use graph::{Graph, GraphMode};
pub use heatmap::{Heatmap, HeatmapCell, HeatmapPalette};
pub use meter::Meter;
pub use sparkline::MonitorSparkline;
pub use table::{MonitorTable, SortDirection};
pub use tree::Tree;
