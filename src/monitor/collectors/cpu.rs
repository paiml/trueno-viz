//! CPU metrics collector.
//!
//! Parses `/proc/stat` on Linux to collect CPU utilization metrics.
//!
//! ## Metrics Collected
//!
//! - Total and per-core CPU utilization
//! - Load average (1, 5, 15 minute)
//! - CPU frequency per core
//! - System uptime

use crate::monitor::error::{MonitorError, Result};
use crate::monitor::ring_buffer::RingBuffer;
use crate::monitor::types::{Collector, MetricValue, Metrics};
use std::time::Duration;

/// Load average values.
#[derive(Debug, Clone, Copy, Default)]
pub struct LoadAverage {
    /// 1-minute load average.
    pub one: f64,
    /// 5-minute load average.
    pub five: f64,
    /// 15-minute load average.
    pub fifteen: f64,
}

/// CPU frequency information.
#[derive(Debug, Clone, Copy, Default)]
pub struct CpuFrequency {
    /// Current frequency in MHz.
    pub current_mhz: u64,
    /// Minimum frequency in MHz.
    pub min_mhz: u64,
    /// Maximum frequency in MHz.
    pub max_mhz: u64,
}

/// CPU statistics from /proc/stat.
#[derive(Debug, Clone, Default)]
struct CpuStats {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
}

impl CpuStats {
    /// Total CPU time.
    fn total(&self) -> u64 {
        self.user
            + self.nice
            + self.system
            + self.idle
            + self.iowait
            + self.irq
            + self.softirq
            + self.steal
    }

    /// Idle time (idle + iowait).
    fn idle_time(&self) -> u64 {
        self.idle + self.iowait
    }
}

/// Collector for CPU metrics.
#[derive(Debug)]
pub struct CpuCollector {
    /// Previous stats for delta calculation.
    prev_total: Option<CpuStats>,
    /// Per-core previous stats.
    prev_cores: Vec<CpuStats>,
    /// History of total CPU usage.
    history: RingBuffer<f64>,
    /// Per-core usage history.
    core_history: Vec<RingBuffer<f64>>,
    /// Number of CPU cores.
    core_count: usize,
    /// Latest load average.
    load_average: LoadAverage,
    /// Per-core frequency.
    frequencies: Vec<CpuFrequency>,
    /// System uptime in seconds.
    uptime_secs: f64,
}

impl CpuCollector {
    /// Creates a new CPU collector.
    #[must_use]
    pub fn new() -> Self {
        let core_count = Self::detect_core_count();
        let mut core_history = Vec::with_capacity(core_count);
        for _ in 0..core_count {
            core_history.push(RingBuffer::new(300));
        }

        Self {
            prev_total: None,
            prev_cores: Vec::new(),
            history: RingBuffer::new(300),
            core_history,
            core_count,
            load_average: LoadAverage::default(),
            frequencies: vec![CpuFrequency::default(); core_count],
            uptime_secs: 0.0,
        }
    }

    /// Detects the number of CPU cores.
    fn detect_core_count() -> usize {
        #[cfg(target_os = "linux")]
        {
            std::fs::read_to_string("/proc/stat")
                .map(|content| {
                    content
                        .lines()
                        .filter(|line| line.starts_with("cpu") && !line.starts_with("cpu "))
                        .count()
                })
                .unwrap_or(1)
        }
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("sysctl")
                .args(["-n", "hw.ncpu"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok())
                .unwrap_or(1)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            1
        }
    }

    /// Parses /proc/stat and returns CPU stats.
    #[cfg(target_os = "linux")]
    fn parse_proc_stat() -> Result<(CpuStats, Vec<CpuStats>)> {
        let content =
            std::fs::read_to_string("/proc/stat").map_err(|e| MonitorError::CollectionFailed {
                collector: "cpu",
                message: format!("Failed to read /proc/stat: {}", e),
            })?;

        let mut total = CpuStats::default();
        let mut cores = Vec::new();

        for line in content.lines() {
            if line.starts_with("cpu ") {
                total = Self::parse_cpu_line(line)?;
            } else if line.starts_with("cpu") {
                cores.push(Self::parse_cpu_line(line)?);
            }
        }

        Ok((total, cores))
    }

    #[cfg(target_os = "macos")]
    fn parse_proc_stat() -> Result<(CpuStats, Vec<CpuStats>)> {
        // Use top -l 1 to get CPU stats on macOS
        let output = std::process::Command::new("top")
            .args(["-l", "1", "-n", "0", "-s", "0"])
            .output()
            .map_err(|e| MonitorError::CollectionFailed {
                collector: "cpu",
                message: format!("Failed to run top: {}", e),
            })?;

        let content = String::from_utf8_lossy(&output.stdout);
        let mut user: u64 = 0;
        let mut sys: u64 = 0;
        let mut idle: u64 = 0;

        // Parse "CPU usage: X.X% user, Y.Y% sys, Z.Z% idle"
        for line in content.lines() {
            if line.starts_with("CPU usage:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                for (i, part) in parts.iter().enumerate() {
                    if *part == "user," || *part == "user" {
                        if let Some(val) = parts.get(i - 1) {
                            user = val.trim_end_matches('%').parse::<f64>().unwrap_or(0.0) as u64;
                        }
                    } else if *part == "sys," || *part == "sys" {
                        if let Some(val) = parts.get(i - 1) {
                            sys = val.trim_end_matches('%').parse::<f64>().unwrap_or(0.0) as u64;
                        }
                    } else if *part == "idle" {
                        if let Some(val) = parts.get(i - 1) {
                            idle = val.trim_end_matches('%').parse::<f64>().unwrap_or(0.0) as u64;
                        }
                    }
                }
                break;
            }
        }

        // Convert percentages to counts (scale by 100 for consistency with Linux jiffies)
        let total = CpuStats {
            user: user * 100,
            nice: 0,
            system: sys * 100,
            idle: idle * 100,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
        };

        // On macOS, we don't easily get per-core stats from top, so duplicate total for each core
        let core_count = Self::detect_core_count();
        let cores = vec![total.clone(); core_count];

        Ok((total, cores))
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    fn parse_proc_stat() -> Result<(CpuStats, Vec<CpuStats>)> {
        // Return dummy data on other systems
        Ok((CpuStats::default(), vec![CpuStats::default()]))
    }

    /// Parses a single CPU line from /proc/stat.
    fn parse_cpu_line(line: &str) -> Result<CpuStats> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 8 {
            return Err(MonitorError::CollectionFailed {
                collector: "cpu",
                message: "Invalid /proc/stat format".to_string(),
            });
        }

        Ok(CpuStats {
            user: parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
            nice: parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0),
            system: parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0),
            idle: parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0),
            iowait: parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(0),
            irq: parts.get(6).and_then(|s| s.parse().ok()).unwrap_or(0),
            softirq: parts.get(7).and_then(|s| s.parse().ok()).unwrap_or(0),
            steal: parts.get(8).and_then(|s| s.parse().ok()).unwrap_or(0),
        })
    }

    /// Calculates CPU percentage from delta.
    fn calculate_percentage(prev: &CpuStats, curr: &CpuStats) -> f64 {
        let total_delta = curr.total().saturating_sub(prev.total());
        let idle_delta = curr.idle_time().saturating_sub(prev.idle_time());

        if total_delta == 0 {
            return 0.0;
        }

        let used_delta = total_delta.saturating_sub(idle_delta);
        (used_delta as f64 / total_delta as f64) * 100.0
    }

    /// Returns the CPU usage history.
    #[must_use]
    pub fn history(&self) -> &RingBuffer<f64> {
        &self.history
    }

    /// Returns the per-core usage history.
    #[must_use]
    pub fn core_history(&self, core: usize) -> Option<&RingBuffer<f64>> {
        self.core_history.get(core)
    }

    /// Returns the number of CPU cores.
    #[must_use]
    pub fn core_count(&self) -> usize {
        self.core_count
    }

    /// Returns the latest load average.
    #[must_use]
    pub fn load_average(&self) -> LoadAverage {
        self.load_average
    }

    /// Returns the per-core frequency information.
    #[must_use]
    pub fn frequencies(&self) -> &[CpuFrequency] {
        &self.frequencies
    }

    /// Returns the system uptime in seconds.
    #[must_use]
    pub fn uptime_secs(&self) -> f64 {
        self.uptime_secs
    }

    /// Reads load average from /proc/loadavg.
    #[cfg(target_os = "linux")]
    fn read_load_average() -> LoadAverage {
        std::fs::read_to_string("/proc/loadavg")
            .ok()
            .and_then(|content| {
                let parts: Vec<&str> = content.split_whitespace().collect();
                if parts.len() >= 3 {
                    Some(LoadAverage {
                        one: parts[0].parse().unwrap_or(0.0),
                        five: parts[1].parse().unwrap_or(0.0),
                        fifteen: parts[2].parse().unwrap_or(0.0),
                    })
                } else {
                    None
                }
            })
            .unwrap_or_default()
    }

    #[cfg(target_os = "macos")]
    fn read_load_average() -> LoadAverage {
        std::process::Command::new("sysctl")
            .args(["-n", "vm.loadavg"])
            .output()
            .ok()
            .and_then(|o| {
                let content = String::from_utf8_lossy(&o.stdout);
                // Format: "{ 1.23 4.56 7.89 }"
                let trimmed = content.trim().trim_start_matches('{').trim_end_matches('}');
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 3 {
                    Some(LoadAverage {
                        one: parts[0].parse().unwrap_or(0.0),
                        five: parts[1].parse().unwrap_or(0.0),
                        fifteen: parts[2].parse().unwrap_or(0.0),
                    })
                } else {
                    None
                }
            })
            .unwrap_or_default()
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    fn read_load_average() -> LoadAverage {
        LoadAverage::default()
    }

    /// Reads CPU frequency for a specific core.
    #[cfg(target_os = "linux")]
    fn read_frequency(core: usize) -> CpuFrequency {
        let base = format!("/sys/devices/system/cpu/cpu{}/cpufreq", core);

        let current = std::fs::read_to_string(format!("{}/scaling_cur_freq", base))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0)
            / 1000; // Convert from kHz to MHz

        let min = std::fs::read_to_string(format!("{}/scaling_min_freq", base))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0)
            / 1000;

        let max = std::fs::read_to_string(format!("{}/scaling_max_freq", base))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0)
            / 1000;

        CpuFrequency {
            current_mhz: current,
            min_mhz: min,
            max_mhz: max,
        }
    }

    #[cfg(target_os = "macos")]
    fn read_frequency(_core: usize) -> CpuFrequency {
        // macOS doesn't expose per-core frequency easily, use sysctl for base frequency
        let current = std::process::Command::new("sysctl")
            .args(["-n", "hw.cpufrequency"])
            .output()
            .ok()
            .and_then(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .trim()
                    .parse::<u64>()
                    .ok()
            })
            .unwrap_or(0)
            / 1_000_000; // Convert Hz to MHz

        CpuFrequency {
            current_mhz: current,
            min_mhz: current,
            max_mhz: current,
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    fn read_frequency(_core: usize) -> CpuFrequency {
        CpuFrequency::default()
    }

    /// Reads system uptime from /proc/uptime.
    #[cfg(target_os = "linux")]
    fn read_uptime() -> f64 {
        std::fs::read_to_string("/proc/uptime")
            .ok()
            .and_then(|content| {
                content
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())
            })
            .unwrap_or(0.0)
    }

    #[cfg(target_os = "macos")]
    fn read_uptime() -> f64 {
        std::process::Command::new("sysctl")
            .args(["-n", "kern.boottime"])
            .output()
            .ok()
            .and_then(|o| {
                let content = String::from_utf8_lossy(&o.stdout);
                // Format: "{ sec = 1234567890, usec = 123456 } Mon Jan 1 00:00:00 2024"
                content
                    .split("sec = ")
                    .nth(1)
                    .and_then(|s| s.split(',').next())
                    .and_then(|s| s.trim().parse::<u64>().ok())
                    .map(|boot_time| {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        (now - boot_time) as f64
                    })
            })
            .unwrap_or(0.0)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    fn read_uptime() -> f64 {
        0.0
    }
}

impl Default for CpuCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector for CpuCollector {
    fn id(&self) -> &'static str {
        "cpu"
    }

    fn collect(&mut self) -> Result<Metrics> {
        let (curr_total, curr_cores) = Self::parse_proc_stat()?;

        let mut metrics = Metrics::new();

        // Calculate total CPU percentage
        if let Some(prev) = &self.prev_total {
            let percent = Self::calculate_percentage(prev, &curr_total);
            metrics.insert("cpu.total", percent);
            self.history.push(percent / 100.0); // Normalized for graphs
        }

        // Calculate per-core percentages
        for (i, curr) in curr_cores.iter().enumerate() {
            if let Some(prev) = self.prev_cores.get(i) {
                let percent = Self::calculate_percentage(prev, curr);
                metrics.insert(format!("cpu.core.{}", i), percent);

                // Update per-core history
                if let Some(history) = self.core_history.get_mut(i) {
                    history.push(percent / 100.0);
                }
            }
        }

        // Update previous values
        self.prev_total = Some(curr_total);
        self.prev_cores = curr_cores;

        // Add core count
        metrics.insert("cpu.cores", MetricValue::Counter(self.core_count as u64));

        // Load average
        self.load_average = Self::read_load_average();
        metrics.insert("cpu.load.1", self.load_average.one);
        metrics.insert("cpu.load.5", self.load_average.five);
        metrics.insert("cpu.load.15", self.load_average.fifteen);

        // CPU frequency per core
        for i in 0..self.core_count {
            let freq = Self::read_frequency(i);
            if let Some(f) = self.frequencies.get_mut(i) {
                *f = freq;
            }
            metrics.insert(
                format!("cpu.freq.{}", i),
                MetricValue::Counter(freq.current_mhz),
            );
        }

        // Average frequency across all cores
        let avg_freq: u64 = self
            .frequencies
            .iter()
            .map(|f| f.current_mhz)
            .sum::<u64>()
            .checked_div(self.core_count as u64)
            .unwrap_or(0);
        metrics.insert("cpu.freq.avg", MetricValue::Counter(avg_freq));

        // Uptime
        self.uptime_secs = Self::read_uptime();
        metrics.insert("cpu.uptime", self.uptime_secs);

        Ok(metrics)
    }

    fn is_available(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            std::path::Path::new("/proc/stat").exists()
        }
        #[cfg(target_os = "macos")]
        {
            true // macOS uses sysctl and top, which are always available
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            false
        }
    }

    fn interval_hint(&self) -> Duration {
        Duration::from_millis(1000)
    }

    fn display_name(&self) -> &'static str {
        "CPU"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_collector_new() {
        let collector = CpuCollector::new();
        assert!(collector.core_count >= 1);
    }

    #[test]
    fn test_cpu_stats_total() {
        let stats = CpuStats {
            user: 100,
            nice: 10,
            system: 50,
            idle: 800,
            iowait: 20,
            irq: 5,
            softirq: 5,
            steal: 10,
        };

        assert_eq!(stats.total(), 1000);
        assert_eq!(stats.idle_time(), 820);
    }

    #[test]
    fn test_calculate_percentage() {
        let prev = CpuStats {
            user: 100,
            nice: 0,
            system: 0,
            idle: 900,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
        };

        let curr = CpuStats {
            user: 200,
            nice: 0,
            system: 0,
            idle: 1800,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
        };

        let percent = CpuCollector::calculate_percentage(&prev, &curr);
        // 100 user delta, 1000 total delta = 10%
        assert!((percent - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_parse_cpu_line() {
        let line = "cpu0 100 10 50 800 20 5 5 10 0 0";
        let stats = CpuCollector::parse_cpu_line(line).unwrap();

        assert_eq!(stats.user, 100);
        assert_eq!(stats.nice, 10);
        assert_eq!(stats.system, 50);
        assert_eq!(stats.idle, 800);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_cpu_collector_is_available() {
        let collector = CpuCollector::new();
        assert!(collector.is_available());
    }

    #[test]
    fn test_cpu_collector_interval() {
        let collector = CpuCollector::new();
        assert_eq!(collector.interval_hint(), Duration::from_millis(1000));
    }
}
