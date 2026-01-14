//! ttop library - Terminal Top system monitor
//!
//! This module exposes the core components for testing and embedding.
#![cfg_attr(test, allow(clippy::unwrap_used))]
#![cfg_attr(test, allow(clippy::field_reassign_with_default))]
//!
//! ## Architecture
//!
//! The library is organized into several key modules:
//!
//! - **app**: Main application state and logic
//! - **analyzers**: Advanced analysis algorithms (swap thrashing, disk I/O latency, anomaly detection)
//! - **panels**: TUI panel rendering
//! - **ring_buffer**: SIMD-optimized time-series data structure
//! - **state**: UI state management
//! - **theme**: Color schemes and styling
//! - **ui**: Main rendering logic
//!
//! ## Key Features
//!
//! - Swap thrashing detection using Denning's Working Set Model (1968)
//! - Disk I/O latency estimation using Little's Law (1961)
//! - Large file anomaly detection using Modified Z-Score (Iglewicz & Hoaglin, 1993)
//! - ZRAM compression monitoring

pub mod analyzers;
pub mod app;
pub mod display_rules;
pub mod panels;
pub mod ring_buffer;
pub mod state;
pub mod theme;
pub mod ui;

// Re-export key types for convenience
pub use analyzers::{
    DiskIoAnalyzer, IoWorkloadType, LargeFileDetector, StorageAnalyzer, SwapAnalyzer,
    ThrashingSeverity, ZramStats,
};
pub use ring_buffer::{handle_counter_wrap, RingBuffer};
