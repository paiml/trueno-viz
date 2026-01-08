//! Analysis modules for advanced system metrics.
//!
//! Implements algorithms from ttop-improve.md specification:
//! - Swap thrashing detection (Denning, 1968)
//! - Disk I/O latency estimation (Little's Law, 1961)
//! - Large file anomaly detection (Iglewicz & Hoaglin, 1993)
//! - Network connection tracking (Little Snitch style)
//! - Squarified treemap (Bruls, Huizing, van Wijk 2000)
//! - PSI pressure stall monitoring (Linux 4.20+)
//! - Container/Docker monitoring

pub mod connections;
pub mod containers;
pub mod disk_io;
pub mod gpu_procs;
pub mod psi;
pub mod storage;
pub mod swap;
pub mod treemap;

pub use connections::{ConnState, Connection, ConnectionAnalyzer, Protocol};
pub use containers::{ContainerAnalyzer, ContainerStats, ContainerStatus};
pub use disk_io::{DiskIoAnalyzer, IoWorkloadType};
pub use gpu_procs::{GpuProcess, GpuProcessAnalyzer, GpuProcType};
pub use psi::{PressureLevel, PsiAnalyzer, PsiMetrics};
pub use storage::{Anomaly, LargeFileDetector, StorageAnalyzer};
pub use swap::{SwapAnalyzer, ThrashingSeverity, ZramStats};
pub use treemap::{TreemapAnalyzer, TreeRect};
