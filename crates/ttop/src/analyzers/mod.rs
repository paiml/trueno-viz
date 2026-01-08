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
//! - Network protocol/error/latency analysis

pub mod connections;
pub mod containers;
pub mod disk_entropy;
pub mod disk_io;
pub mod file_analyzer;
pub mod geoip;
pub mod gpu_procs;
pub mod network_stats;
pub mod process_extra;
pub mod psi;
pub mod sensor_health;
pub mod storage;
pub mod swap;
pub mod treemap;

pub use connections::{ConnState, Connection, ConnectionAnalyzer, Protocol, port_to_service, port_to_icon};
pub use containers::{ContainerAnalyzer, ContainerStats, ContainerStatus};
pub use disk_entropy::{DiskEntropyAnalyzer, MountEntropy};
pub use disk_io::{DiskIoAnalyzer, IoWorkloadType};
pub use file_analyzer::{FileAnalyzer, FileEntry, FileType, DuplicateGroup, WatchedFile, IoActivity, EntropyLevel, FileActivityMetrics};
pub use gpu_procs::{GpuProcess, GpuProcessAnalyzer, GpuProcType};
pub use network_stats::{NetworkStatsAnalyzer, ProtocolStats, TcpPerformance, QueueStats};
pub use process_extra::{ProcessExtra, ProcessExtraAnalyzer};
pub use psi::{PressureLevel, PsiAnalyzer, PsiMetrics};
pub use storage::{Anomaly, LargeFileDetector, StorageAnalyzer};
pub use sensor_health::{SensorHealth, SensorHealthAnalyzer, SensorReading, SensorType};
pub use swap::{SwapAnalyzer, ThrashingSeverity, ZramStats};
pub use treemap::{TreemapAnalyzer, TreeRect, FileCategory};
