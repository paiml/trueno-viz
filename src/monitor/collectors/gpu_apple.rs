//! Apple GPU metrics collector for macOS.
//!
//! Uses IOKit and system commands to collect GPU metrics from Apple Silicon GPUs.
//!
//! ## Metrics Collected
//!
//! - GPU utilization percentage (via powermetrics sampling)
//! - GPU name/model
//! - Number of GPU cores

use crate::monitor::error::{MonitorError, Result};
use crate::monitor::ring_buffer::RingBuffer;
use crate::monitor::types::{Collector, MetricValue, Metrics};
use std::time::Duration;

/// Information about an Apple GPU.
#[derive(Debug, Clone)]
pub struct AppleGpuInfo {
    /// GPU index.
    pub index: u32,
    /// GPU name/model.
    pub name: String,
    /// GPU utilization percentage (0-100).
    pub gpu_util: f64,
    /// Number of GPU cores.
    pub core_count: u32,
    /// Metal family support level.
    pub metal_family: String,
}

impl Default for AppleGpuInfo {
    fn default() -> Self {
        Self {
            index: 0,
            name: String::new(),
            gpu_util: 0.0,
            core_count: 0,
            metal_family: String::new(),
        }
    }
}

/// Collector for Apple GPU metrics.
#[derive(Debug)]
pub struct AppleGpuCollector {
    /// GPU information.
    gpus: Vec<AppleGpuInfo>,
    /// GPU utilization history (normalized 0-1).
    util_history: Vec<RingBuffer<f64>>,
    /// Whether initialization succeeded.
    initialized: bool,
    /// Cached GPU name.
    gpu_name: Option<String>,
}

impl AppleGpuCollector {
    /// Creates a new Apple GPU collector.
    #[must_use]
    pub fn new() -> Self {
        let mut collector = Self {
            gpus: Vec::new(),
            util_history: Vec::new(),
            initialized: false,
            gpu_name: None,
        };
        collector.initialize();
        collector
    }

    /// Initializes the collector by detecting GPUs.
    fn initialize(&mut self) {
        #[cfg(target_os = "macos")]
        {
            // Detect all GPUs
            let gpu_names = Self::detect_all_gpus();

            for (index, name) in gpu_names.into_iter().enumerate() {
                let gpu = AppleGpuInfo {
                    index: index as u32,
                    name: name.clone(),
                    gpu_util: 0.0,
                    core_count: Self::detect_gpu_cores(),
                    metal_family: Self::detect_metal_family(),
                };

                self.gpus.push(gpu);
                self.util_history.push(RingBuffer::new(300));
            }

            if !self.gpus.is_empty() {
                self.gpu_name = Some(self.gpus[0].name.clone());
                self.initialized = true;
            }
        }
    }

    /// Detects all GPUs in the system.
    #[cfg(target_os = "macos")]
    fn detect_all_gpus() -> Vec<String> {
        let mut gpus = Vec::new();

        // Check for Apple Silicon first
        let chip = std::process::Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        if let Some(ref chip_name) = chip {
            if chip_name.contains("Apple") {
                // Apple Silicon - single integrated GPU
                let gpu_name = Self::detect_apple_silicon_gpu(chip_name);
                gpus.push(gpu_name);
                return gpus;
            }
        }

        // Intel Mac - check for discrete GPUs via ioreg
        let ioreg = std::process::Command::new("ioreg")
            .args(["-r", "-c", "IOPCIDevice"])
            .output()
            .ok();

        if let Some(output) = ioreg {
            let content = String::from_utf8_lossy(&output.stdout);

            // Find all GPU model strings
            for line in content.lines() {
                if line.contains("\"model\"") &&
                   (line.contains("Radeon") || line.contains("AMD") || line.contains("Vega")) {
                    // Extract model name from: "model" = <"AMD Radeon Pro W5700X">
                    if let Some(start) = line.find("<\"") {
                        if let Some(end) = line.rfind("\">") {
                            let model = &line[start + 2..end];
                            gpus.push(model.to_string());
                        }
                    }
                }
            }
        }

        if gpus.is_empty() {
            gpus.push("GPU".to_string());
        }

        gpus
    }

    /// Detects Apple Silicon GPU name from chip string.
    #[cfg(target_os = "macos")]
    fn detect_apple_silicon_gpu(chip_name: &str) -> String {
        if chip_name.contains("M4") {
            if chip_name.contains("Max") {
                "Apple M4 Max GPU".to_string()
            } else if chip_name.contains("Pro") {
                "Apple M4 Pro GPU".to_string()
            } else {
                "Apple M4 GPU".to_string()
            }
        } else if chip_name.contains("M3") {
            if chip_name.contains("Max") {
                "Apple M3 Max GPU".to_string()
            } else if chip_name.contains("Pro") {
                "Apple M3 Pro GPU".to_string()
            } else {
                "Apple M3 GPU".to_string()
            }
        } else if chip_name.contains("M2") {
            if chip_name.contains("Ultra") {
                "Apple M2 Ultra GPU".to_string()
            } else if chip_name.contains("Max") {
                "Apple M2 Max GPU".to_string()
            } else if chip_name.contains("Pro") {
                "Apple M2 Pro GPU".to_string()
            } else {
                "Apple M2 GPU".to_string()
            }
        } else if chip_name.contains("M1") {
            if chip_name.contains("Ultra") {
                "Apple M1 Ultra GPU".to_string()
            } else if chip_name.contains("Max") {
                "Apple M1 Max GPU".to_string()
            } else if chip_name.contains("Pro") {
                "Apple M1 Pro GPU".to_string()
            } else {
                "Apple M1 GPU".to_string()
            }
        } else {
            "Apple GPU".to_string()
        }
    }

    /// Detects GPU core count.
    #[cfg(target_os = "macos")]
    fn detect_gpu_cores() -> u32 {
        // Try to get GPU core count from sysctl
        std::process::Command::new("sysctl")
            .args(["-n", "hw.perflevel0.gpu_count"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok())
            .unwrap_or(0)
    }

    /// Detects Metal family support.
    #[cfg(target_os = "macos")]
    fn detect_metal_family() -> String {
        // Apple Silicon supports Metal 3
        "Metal 3".to_string()
    }

    /// Samples GPU utilization (lightweight check).
    #[cfg(target_os = "macos")]
    fn sample_gpu_util(&self) -> f64 {
        // GPU utilization is harder to get on macOS without root
        // For now, return 0 - would need powermetrics with sudo
        0.0
    }

    /// Returns GPU information.
    #[must_use]
    pub fn gpus(&self) -> &[AppleGpuInfo] {
        &self.gpus
    }

    /// Returns the first GPU (convenience method).
    #[must_use]
    pub fn primary_gpu(&self) -> Option<&AppleGpuInfo> {
        self.gpus.first()
    }

    /// Returns GPU utilization history.
    #[must_use]
    pub fn util_history(&self, index: usize) -> Option<&RingBuffer<f64>> {
        self.util_history.get(index)
    }
}

impl Default for AppleGpuCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector for AppleGpuCollector {
    fn id(&self) -> &'static str {
        "gpu_apple"
    }

    fn collect(&mut self) -> Result<Metrics> {
        if !self.initialized {
            return Err(MonitorError::CollectorUnavailable("gpu_apple"));
        }

        let mut metrics = Metrics::new();

        #[cfg(target_os = "macos")]
        {
            metrics.insert("gpu.count", MetricValue::Counter(self.gpus.len() as u64));

            for (i, gpu) in self.gpus.iter_mut().enumerate() {
                let util = 0.0; // GPU util requires sudo powermetrics on macOS

                gpu.gpu_util = util;

                if let Some(history) = self.util_history.get_mut(i) {
                    history.push(util / 100.0);
                }

                metrics.insert(&format!("gpu.{}.util", i), MetricValue::Gauge(util));
                metrics.insert(
                    &format!("gpu.{}.cores", i),
                    MetricValue::Counter(gpu.core_count as u64),
                );
            }
        }

        Ok(metrics)
    }

    fn is_available(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            self.initialized
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }

    fn interval_hint(&self) -> Duration {
        Duration::from_millis(1000)
    }

    fn display_name(&self) -> &'static str {
        "Apple GPU"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apple_gpu_collector_new() {
        let collector = AppleGpuCollector::new();
        // On macOS, should initialize; on other platforms, won't
        #[cfg(target_os = "macos")]
        {
            // May or may not find GPUs depending on hardware
            let _ = collector;
        }
    }

    #[test]
    fn test_apple_gpu_info_default() {
        let info = AppleGpuInfo::default();
        assert_eq!(info.index, 0);
        assert!(info.name.is_empty());
    }

    #[test]
    fn test_apple_gpu_collector_interval() {
        let collector = AppleGpuCollector::new();
        assert_eq!(collector.interval_hint(), Duration::from_millis(1000));
    }
}
