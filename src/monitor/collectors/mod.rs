//! Metric collectors for system and ML workload monitoring.
//!
//! This module provides collectors for gathering metrics from various sources:
//!
//! - **System**: CPU, memory, disk, network, processes, sensors, battery
//! - **GPU**: NVIDIA (via NVML), AMD (via ROCm SMI)
//! - **Stack**: realizar, entrenar, trueno-zram, repartir

// Core system collectors
pub mod battery;
pub mod cpu;
pub mod disk;
pub mod memory;
pub mod network;
pub mod process;
pub mod sensors;

pub use battery::BatteryCollector;
pub use cpu::{CpuCollector, CpuFrequency, LoadAverage};
pub use disk::DiskCollector;
pub use memory::MemoryCollector;
pub use network::NetworkCollector;
pub use process::ProcessCollector;
pub use sensors::SensorCollector;

// GPU collectors (feature-gated)
#[cfg(feature = "monitor-nvidia")]
#[cfg_attr(docsrs, doc(cfg(feature = "monitor-nvidia")))]
pub mod gpu_nvidia;

#[cfg(feature = "monitor-nvidia")]
pub use gpu_nvidia::{GpuInfo, NvidiaGpuCollector};

// AMD GPU (always compiled, dynamically loads librocm_smi64.so at runtime)
#[cfg(target_os = "linux")]
pub mod gpu_amd;

#[cfg(target_os = "linux")]
pub use gpu_amd::{AmdGpuCollector, AmdGpuInfo};

// Stack collectors (feature-gated)
#[cfg(feature = "monitor-stack")]
pub mod stack;
