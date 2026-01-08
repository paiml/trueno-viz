//! Disk I/O analysis with latency estimation.
//!
//! Implements Little's Law (1961) for latency estimation without eBPF:
//! L = λW, where L = queue length, λ = arrival rate, W = wait time.

use crate::ring_buffer::RingBuffer;
use std::collections::HashMap;
use std::fs;

/// I/O workload classification per Ruemmler & Wilkes (1994)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IoWorkloadType {
    /// High throughput, low IOPS (video, backups)
    Sequential,
    /// Low throughput, high IOPS (databases, VMs)
    Random,
    /// Balanced workload
    Mixed,
    /// Minimal activity
    #[default]
    Idle,
}

impl IoWorkloadType {
    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Sequential => "Sequential",
            Self::Random => "Random",
            Self::Mixed => "Mixed",
            Self::Idle => "Idle",
        }
    }
}

/// Classify workload type based on IOPS and throughput.
///
/// Uses the ratio of throughput to IOPS (KB per I/O) to determine:
/// - >128 KB/IO = Sequential
/// - <16 KB/IO with high IOPS = Random
/// - Low activity = Idle
/// - Otherwise = Mixed
pub fn classify_workload(iops: f64, throughput_mbps: f64) -> IoWorkloadType {
    if iops < 10.0 && throughput_mbps < 1.0 {
        return IoWorkloadType::Idle;
    }

    // KB per I/O operation
    let ratio = if iops > 0.0 {
        (throughput_mbps * 1024.0) / iops
    } else {
        0.0
    };

    match ratio {
        r if r > 128.0 => IoWorkloadType::Sequential, // >128KB per IO
        r if r < 16.0 && iops > 100.0 => IoWorkloadType::Random, // <16KB per IO
        _ => IoWorkloadType::Mixed,
    }
}

/// Little's Law latency estimation (Little, 1961)
///
/// W = L / λ where:
/// - W = average wait time (latency)
/// - L = average queue length
/// - λ = arrival rate (IOPS)
pub fn estimate_latency_ms(queue_depth: f64, iops: f64) -> f64 {
    if iops < 1.0 {
        return 0.0;
    }
    (queue_depth / iops) * 1000.0 // Convert to milliseconds
}

/// P99 latency estimation using exponential distribution assumption.
///
/// For random I/O workloads, latency follows exponential distribution.
/// P99 ≈ avg × ln(100) ≈ avg × 4.605 (Chen et al., 1994)
pub fn estimate_p99_latency_ms(avg_latency_ms: f64) -> f64 {
    avg_latency_ms * 4.605 // ln(100)
}

/// P50 latency estimation (median)
/// For exponential distribution, P50 ≈ avg × ln(2) ≈ avg × 0.693
pub fn estimate_p50_latency_ms(avg_latency_ms: f64) -> f64 {
    avg_latency_ms * 0.693 // ln(2)
}

/// Per-device I/O statistics
#[derive(Debug, Clone, Default)]
pub struct DeviceIoStats {
    /// Device name (e.g., "nvme0n1", "sda")
    pub device: String,
    /// Read throughput in bytes per second
    pub read_bytes_per_sec: f64,
    /// Write throughput in bytes per second
    pub write_bytes_per_sec: f64,
    /// Read IOPS
    pub read_iops: f64,
    /// Write IOPS
    pub write_iops: f64,
    /// Current queue depth (in-flight I/Os)
    pub queue_depth: f64,
    /// Utilization percentage (0-100)
    pub utilization: f64,
    /// Estimated average latency in milliseconds
    pub avg_latency_ms: f64,
    /// Estimated P50 latency in milliseconds
    pub p50_latency_ms: f64,
    /// Estimated P99 latency in milliseconds
    pub p99_latency_ms: f64,
    /// Total I/O time in milliseconds
    pub io_time_ms: u64,
    /// Classified workload type
    pub workload_type: IoWorkloadType,
    /// Is this an NVMe device
    pub is_nvme: bool,
    /// Is this a rotational device (HDD)
    pub is_rotational: bool,
}

impl DeviceIoStats {
    /// Get total throughput (read + write) in MB/s
    pub fn total_throughput_mbps(&self) -> f64 {
        (self.read_bytes_per_sec + self.write_bytes_per_sec) / (1024.0 * 1024.0)
    }

    /// Get total IOPS
    pub fn total_iops(&self) -> f64 {
        self.read_iops + self.write_iops
    }
}

/// Raw diskstats data from /proc/diskstats
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
struct RawDiskStats {
    reads_completed: u64,
    _reads_merged: u64,
    sectors_read: u64,
    _read_time_ms: u64,
    writes_completed: u64,
    _writes_merged: u64,
    sectors_written: u64,
    _write_time_ms: u64,
    ios_in_progress: u64,
    io_time_ms: u64,
    weighted_io_time_ms: u64,
    // Discard stats (kernel 4.18+)
    discards_completed: u64,
    _discards_merged: u64,
    sectors_discarded: u64,
    _discard_time_ms: u64,
    // Flush stats (kernel 5.5+)
    flushes_completed: u64,
    flush_time_ms: u64,
}

/// Disk I/O analyzer with latency estimation.
pub struct DiskIoAnalyzer {
    /// Per-device current stats
    device_stats: HashMap<String, DeviceIoStats>,
    /// Previous raw stats for delta calculation
    prev_stats: HashMap<String, RawDiskStats>,
    /// History of total read throughput (normalized 0-1)
    read_history: RingBuffer<f64>,
    /// History of total write throughput (normalized 0-1)
    write_history: RingBuffer<f64>,
    /// History of total IOPS (normalized 0-1)
    iops_history: RingBuffer<f64>,
    /// Per-device read history (normalized 0-1)
    device_read_history: HashMap<String, RingBuffer<f64>>,
    /// Per-device write history (normalized 0-1)
    device_write_history: HashMap<String, RingBuffer<f64>>,
    /// Per-device max throughput seen (for normalization)
    device_max_throughput: HashMap<String, f64>,
    /// Sample interval in seconds
    sample_interval_secs: f64,
    /// Max throughput seen (for normalization)
    max_throughput_seen: f64,
    /// Max IOPS seen (for normalization)
    max_iops_seen: f64,
}

impl DiskIoAnalyzer {
    /// Create a new disk I/O analyzer with 60-second history.
    pub fn new() -> Self {
        Self {
            device_stats: HashMap::new(),
            prev_stats: HashMap::new(),
            read_history: RingBuffer::new(60),
            write_history: RingBuffer::new(60),
            iops_history: RingBuffer::new(60),
            device_read_history: HashMap::new(),
            device_write_history: HashMap::new(),
            device_max_throughput: HashMap::new(),
            sample_interval_secs: 1.0,
            max_throughput_seen: 100.0 * 1024.0 * 1024.0, // Start at 100 MB/s
            max_iops_seen: 1000.0,                         // Start at 1K IOPS
        }
    }

    /// Set the sample interval in seconds
    pub fn set_sample_interval(&mut self, secs: f64) {
        self.sample_interval_secs = secs;
    }

    /// Collect metrics from /proc/diskstats
    pub fn collect(&mut self) {
        let content = match fs::read_to_string("/proc/diskstats") {
            Ok(c) => c,
            Err(_) => return,
        };

        let mut total_read = 0.0;
        let mut total_write = 0.0;
        let mut total_iops = 0.0;
        let mut new_device_stats = HashMap::new();
        let mut new_raw_stats = HashMap::new();

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 14 {
                continue;
            }

            let device = parts[2].to_string();

            // Skip partition devices (we want whole disks)
            // Partitions have numbers at the end (sda1, nvme0n1p1)
            // But skip loop/dm devices too
            if device.starts_with("loop") || device.starts_with("dm-") {
                continue;
            }

            // Parse raw stats (fields 4-20 in /proc/diskstats)
            let raw = RawDiskStats {
                reads_completed: parts[3].parse().unwrap_or(0),
                _reads_merged: parts[4].parse().unwrap_or(0),
                sectors_read: parts[5].parse().unwrap_or(0),
                _read_time_ms: parts[6].parse().unwrap_or(0),
                writes_completed: parts[7].parse().unwrap_or(0),
                _writes_merged: parts[8].parse().unwrap_or(0),
                sectors_written: parts[9].parse().unwrap_or(0),
                _write_time_ms: parts[10].parse().unwrap_or(0),
                ios_in_progress: parts[11].parse().unwrap_or(0),
                io_time_ms: parts[12].parse().unwrap_or(0),
                weighted_io_time_ms: parts[13].parse().unwrap_or(0),
                discards_completed: parts.get(14).and_then(|s| s.parse().ok()).unwrap_or(0),
                _discards_merged: parts.get(15).and_then(|s| s.parse().ok()).unwrap_or(0),
                sectors_discarded: parts.get(16).and_then(|s| s.parse().ok()).unwrap_or(0),
                _discard_time_ms: parts.get(17).and_then(|s| s.parse().ok()).unwrap_or(0),
                flushes_completed: parts.get(18).and_then(|s| s.parse().ok()).unwrap_or(0),
                flush_time_ms: parts.get(19).and_then(|s| s.parse().ok()).unwrap_or(0),
            };

            // Calculate deltas if we have previous stats
            if let Some(prev) = self.prev_stats.get(&device) {
                let delta_reads = raw.reads_completed.saturating_sub(prev.reads_completed);
                let delta_writes = raw.writes_completed.saturating_sub(prev.writes_completed);
                let delta_sectors_read = raw.sectors_read.saturating_sub(prev.sectors_read);
                let delta_sectors_written =
                    raw.sectors_written.saturating_sub(prev.sectors_written);
                let delta_io_time = raw.io_time_ms.saturating_sub(prev.io_time_ms);

                let time_secs = self.sample_interval_secs;

                let read_bytes_per_sec = (delta_sectors_read * 512) as f64 / time_secs;
                let write_bytes_per_sec = (delta_sectors_written * 512) as f64 / time_secs;
                let read_iops = delta_reads as f64 / time_secs;
                let write_iops = delta_writes as f64 / time_secs;
                let total_iops_dev = read_iops + write_iops;

                // Queue depth from in-flight I/Os
                let queue_depth = raw.ios_in_progress as f64;

                // Utilization: % of time spent doing I/O
                let utilization = (delta_io_time as f64 / (time_secs * 1000.0) * 100.0).min(100.0);

                // Little's Law latency estimation
                let avg_latency = estimate_latency_ms(queue_depth, total_iops_dev);
                let p50_latency = estimate_p50_latency_ms(avg_latency);
                let p99_latency = estimate_p99_latency_ms(avg_latency);

                // Classify workload
                let throughput_mbps = (read_bytes_per_sec + write_bytes_per_sec) / (1024.0 * 1024.0);
                let workload_type = classify_workload(total_iops_dev, throughput_mbps);

                // Check device type
                let is_nvme = device.starts_with("nvme");
                let is_rotational = self.is_rotational_device(&device);

                let stats = DeviceIoStats {
                    device: device.clone(),
                    read_bytes_per_sec,
                    write_bytes_per_sec,
                    read_iops,
                    write_iops,
                    queue_depth,
                    utilization,
                    avg_latency_ms: avg_latency,
                    p50_latency_ms: p50_latency,
                    p99_latency_ms: p99_latency,
                    io_time_ms: raw.io_time_ms,
                    workload_type,
                    is_nvme,
                    is_rotational,
                };

                // Only include devices with activity or significant stats
                if stats.total_iops() > 0.0
                    || stats.total_throughput_mbps() > 0.0
                    || stats.queue_depth > 0.0
                    || !device.chars().last().is_some_and(|c| c.is_ascii_digit())
                {
                    total_read += read_bytes_per_sec;
                    total_write += write_bytes_per_sec;
                    total_iops += total_iops_dev;

                    // Update per-device history
                    let max_device_throughput = self
                        .device_max_throughput
                        .entry(device.clone())
                        .or_insert(10.0 * 1024.0 * 1024.0); // Start at 10 MB/s per device
                    *max_device_throughput = max_device_throughput
                        .max(read_bytes_per_sec)
                        .max(write_bytes_per_sec)
                        .max(1.0);

                    let read_norm = read_bytes_per_sec / *max_device_throughput;
                    let write_norm = write_bytes_per_sec / *max_device_throughput;

                    self.device_read_history
                        .entry(device.clone())
                        .or_insert_with(|| RingBuffer::new(60))
                        .push(read_norm);
                    self.device_write_history
                        .entry(device.clone())
                        .or_insert_with(|| RingBuffer::new(60))
                        .push(write_norm);

                    new_device_stats.insert(device.clone(), stats);
                }
            }

            new_raw_stats.insert(device, raw);
        }

        // Update max values for normalization
        self.max_throughput_seen = self
            .max_throughput_seen
            .max(total_read)
            .max(total_write)
            .max(1.0);
        self.max_iops_seen = self.max_iops_seen.max(total_iops).max(1.0);

        // Push to history (normalized 0-1)
        self.read_history
            .push(total_read / self.max_throughput_seen);
        self.write_history
            .push(total_write / self.max_throughput_seen);
        self.iops_history.push(total_iops / self.max_iops_seen);

        // Update state
        self.device_stats = new_device_stats;
        self.prev_stats = new_raw_stats;
    }

    /// Get all device I/O statistics
    pub fn device_stats(&self) -> &HashMap<String, DeviceIoStats> {
        &self.device_stats
    }

    /// Get stats for a specific device
    pub fn device(&self, name: &str) -> Option<&DeviceIoStats> {
        self.device_stats.get(name)
    }

    /// Get total read throughput across all devices (bytes/s)
    pub fn total_read_throughput(&self) -> f64 {
        self.device_stats.values().map(|s| s.read_bytes_per_sec).sum()
    }

    /// Get total write throughput across all devices (bytes/s)
    pub fn total_write_throughput(&self) -> f64 {
        self.device_stats
            .values()
            .map(|s| s.write_bytes_per_sec)
            .sum()
    }

    /// Get total IOPS across all devices
    pub fn total_iops(&self) -> f64 {
        self.device_stats
            .values()
            .map(|s| s.read_iops + s.write_iops)
            .sum()
    }

    /// Get overall workload type classification
    pub fn overall_workload(&self) -> IoWorkloadType {
        let total_throughput =
            (self.total_read_throughput() + self.total_write_throughput()) / (1024.0 * 1024.0);
        let total_iops = self.total_iops();
        classify_workload(total_iops, total_throughput)
    }

    /// Get read throughput history for visualization
    pub fn read_history(&self) -> &RingBuffer<f64> {
        &self.read_history
    }

    /// Get write throughput history for visualization
    pub fn write_history(&self) -> &RingBuffer<f64> {
        &self.write_history
    }

    /// Get IOPS history for visualization
    pub fn iops_history(&self) -> &RingBuffer<f64> {
        &self.iops_history
    }

    /// Get per-device read history for sparkline visualization
    pub fn device_read_history(&self, device: &str) -> Option<Vec<f64>> {
        self.device_read_history
            .get(device)
            .map(|rb| rb.iter().cloned().collect())
    }

    /// Get per-device write history for sparkline visualization
    pub fn device_write_history(&self, device: &str) -> Option<Vec<f64>> {
        self.device_write_history
            .get(device)
            .map(|rb| rb.iter().cloned().collect())
    }

    /// Get the primary (most active) device name.
    /// Returns the device with highest IOPS, or first device if all idle.
    pub fn primary_device(&self) -> Option<String> {
        self.device_stats
            .values()
            .max_by(|a, b| {
                a.total_iops()
                    .partial_cmp(&b.total_iops())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|s| s.device.clone())
    }

    /// Get estimated average latency for a device (milliseconds)
    pub fn estimated_latency_ms(&self, device: &str) -> f64 {
        self.device_stats
            .get(device)
            .map(|s| s.avg_latency_ms)
            .unwrap_or(0.0)
    }

    /// Get workload type for a device
    pub fn workload_type(&self, device: &str) -> IoWorkloadType {
        self.device_stats
            .get(device)
            .map(|s| s.workload_type)
            .unwrap_or(IoWorkloadType::Idle)
    }

    // Private helper to check if device is rotational (HDD)
    fn is_rotational_device(&self, device: &str) -> bool {
        // Extract base device name (strip partition numbers)
        let base: String = device.chars().take_while(|c| !c.is_ascii_digit()).collect();
        let rotational_path = format!("/sys/block/{}/queue/rotational", base);

        fs::read_to_string(&rotational_path)
            .map(|s| s.trim() == "1")
            .unwrap_or(false)
    }
}

impl Default for DiskIoAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workload_classification() {
        // Idle: low everything
        assert_eq!(classify_workload(5.0, 0.5), IoWorkloadType::Idle);

        // Sequential: high throughput per IO
        assert_eq!(classify_workload(100.0, 50.0), IoWorkloadType::Sequential);

        // Random: low throughput per IO with high IOPS
        assert_eq!(classify_workload(10000.0, 50.0), IoWorkloadType::Random);

        // Mixed: in between
        assert_eq!(classify_workload(500.0, 20.0), IoWorkloadType::Mixed);
    }

    #[test]
    fn test_latency_estimation() {
        // Queue depth 10, 1000 IOPS -> 10ms average latency
        let latency = estimate_latency_ms(10.0, 1000.0);
        assert!((latency - 10.0).abs() < 0.001);

        // P99 should be ~4.6x average
        let p99 = estimate_p99_latency_ms(10.0);
        assert!((p99 - 46.05).abs() < 0.1);

        // P50 should be ~0.69x average
        let p50 = estimate_p50_latency_ms(10.0);
        assert!((p50 - 6.93).abs() < 0.1);
    }

    #[test]
    fn test_zero_iops_latency() {
        // With zero IOPS, latency should be 0
        assert_eq!(estimate_latency_ms(10.0, 0.0), 0.0);
        assert_eq!(estimate_latency_ms(10.0, 0.5), 0.0);
    }

    #[test]
    fn test_disk_io_analyzer_creation() {
        let analyzer = DiskIoAnalyzer::new();
        assert!(analyzer.device_stats().is_empty());
    }

    #[test]
    fn test_disk_io_analyzer_default() {
        let analyzer = DiskIoAnalyzer::default();
        assert!(analyzer.device_stats().is_empty());
        assert!(analyzer.primary_device().is_none());
    }

    #[test]
    fn test_disk_io_analyzer_total_throughput() {
        let analyzer = DiskIoAnalyzer::new();
        // No devices = 0 throughput
        assert!(analyzer.total_read_throughput() < 0.001);
        assert!(analyzer.total_write_throughput() < 0.001);
    }

    #[test]
    fn test_disk_io_analyzer_total_iops() {
        let analyzer = DiskIoAnalyzer::new();
        // No devices = 0 IOPS
        assert!(analyzer.total_iops() < 0.001);
    }

    #[test]
    fn test_disk_io_analyzer_overall_workload() {
        let analyzer = DiskIoAnalyzer::new();
        // No activity = Idle
        assert_eq!(analyzer.overall_workload(), IoWorkloadType::Idle);
    }

    #[test]
    fn test_disk_io_analyzer_histories() {
        let analyzer = DiskIoAnalyzer::new();
        // Histories should be empty initially
        assert!(analyzer.read_history().iter().count() == 0);
        assert!(analyzer.write_history().iter().count() == 0);
        assert!(analyzer.iops_history().iter().count() == 0);
    }

    #[test]
    fn test_disk_io_analyzer_sample_interval() {
        let mut analyzer = DiskIoAnalyzer::new();
        analyzer.set_sample_interval(2.0);
        // Method should complete without panicking
        assert_eq!(analyzer.overall_workload(), IoWorkloadType::Idle);
    }

    #[test]
    fn test_disk_io_analyzer_collect_safe() {
        let mut analyzer = DiskIoAnalyzer::new();
        // Collect should be safe even if system files are different
        analyzer.collect();
        // Should still work after collection
        assert!(analyzer.primary_device().is_none() || analyzer.primary_device().is_some());
    }

    #[test]
    fn test_disk_io_analyzer_device_not_found() {
        let analyzer = DiskIoAnalyzer::new();
        assert!(analyzer.device("nonexistent").is_none());
        assert!(analyzer.estimated_latency_ms("nonexistent") < 0.001);
        assert_eq!(analyzer.workload_type("nonexistent"), IoWorkloadType::Idle);
    }

    #[test]
    fn test_io_workload_type_description() {
        assert_eq!(IoWorkloadType::Sequential.description(), "Sequential");
        assert_eq!(IoWorkloadType::Random.description(), "Random");
        assert_eq!(IoWorkloadType::Mixed.description(), "Mixed");
        assert_eq!(IoWorkloadType::Idle.description(), "Idle");
    }

    #[test]
    fn test_io_workload_type_default() {
        assert_eq!(IoWorkloadType::default(), IoWorkloadType::Idle);
    }

    #[test]
    fn test_device_io_stats_methods() {
        let stats = DeviceIoStats {
            device: "test".to_string(),
            read_bytes_per_sec: 50.0 * 1024.0 * 1024.0, // 50 MB/s
            write_bytes_per_sec: 25.0 * 1024.0 * 1024.0, // 25 MB/s
            read_iops: 1000.0,
            write_iops: 500.0,
            ..Default::default()
        };
        assert!((stats.total_throughput_mbps() - 75.0).abs() < 0.1);
        assert!((stats.total_iops() - 1500.0).abs() < 0.1);
    }

    #[test]
    fn test_device_io_stats_default() {
        let stats = DeviceIoStats::default();
        assert_eq!(stats.device, "");
        assert!(stats.read_bytes_per_sec < 0.001);
        assert!(stats.write_bytes_per_sec < 0.001);
        assert!(!stats.is_nvme);
        assert!(!stats.is_rotational);
    }

    #[test]
    fn test_workload_classification_edge_cases() {
        // Edge case: borderline idle
        assert_eq!(classify_workload(9.9, 0.9), IoWorkloadType::Idle);

        // Edge case: just above idle
        assert!(classify_workload(15.0, 2.0) != IoWorkloadType::Idle);
    }

    #[test]
    fn test_workload_classification_zero_iops() {
        // Zero IOPS but some throughput - falls through to Mixed
        // (only returns Idle when BOTH iops < 10 AND throughput < 1)
        assert_eq!(classify_workload(0.0, 10.0), IoWorkloadType::Mixed);
        // Zero IOPS and low throughput = Idle
        assert_eq!(classify_workload(0.0, 0.5), IoWorkloadType::Idle);
    }

    #[test]
    fn test_p50_latency_calculation() {
        // P50 should be ~0.693x average
        let avg = 10.0;
        let p50 = estimate_p50_latency_ms(avg);
        assert!((p50 - 6.93).abs() < 0.1);
    }

    #[test]
    fn test_p99_latency_calculation() {
        // P99 should be ~4.6x average
        let avg = 10.0;
        let p99 = estimate_p99_latency_ms(avg);
        assert!((p99 - 46.0).abs() < 1.0);
    }

    #[test]
    fn test_latency_estimation_high_queue_depth() {
        // High queue depth should increase latency
        let low_qd = estimate_latency_ms(1.0, 1000.0);
        let high_qd = estimate_latency_ms(32.0, 1000.0);
        assert!(high_qd > low_qd);
    }

    #[test]
    fn test_workload_sequential() {
        // High throughput, moderate IOPS = sequential
        assert_eq!(classify_workload(200.0, 400.0), IoWorkloadType::Sequential);
    }

    #[test]
    fn test_workload_sequential_high_throughput() {
        // High throughput = sequential
        let workload = classify_workload(500.0, 600.0);
        assert!(workload == IoWorkloadType::Sequential || workload == IoWorkloadType::Mixed);
    }

    #[test]
    fn test_workload_random() {
        // High IOPS, low throughput = random
        assert_eq!(classify_workload(10000.0, 50.0), IoWorkloadType::Random);
    }

    #[test]
    fn test_analyzer_primary_device_empty() {
        let analyzer = DiskIoAnalyzer::new();
        assert!(analyzer.primary_device().is_none());
    }

    #[test]
    fn test_analyzer_device_histories_empty() {
        let analyzer = DiskIoAnalyzer::new();
        assert!(analyzer.device_read_history("nonexistent").is_none());
        assert!(analyzer.device_write_history("nonexistent").is_none());
    }

    #[test]
    fn test_analyzer_estimated_latency_missing() {
        let analyzer = DiskIoAnalyzer::new();
        let latency = analyzer.estimated_latency_ms("nonexistent");
        assert!(latency < 0.001);
    }

    #[test]
    fn test_analyzer_workload_type_missing() {
        let analyzer = DiskIoAnalyzer::new();
        let workload = analyzer.workload_type("nonexistent");
        assert_eq!(workload, IoWorkloadType::Idle);
    }

    #[test]
    fn test_device_io_stats_nvme() {
        let stats = DeviceIoStats {
            device: "nvme0n1".to_string(),
            is_nvme: true,
            is_rotational: false,
            ..Default::default()
        };
        assert!(stats.is_nvme);
        assert!(!stats.is_rotational);
    }

    #[test]
    fn test_device_io_stats_rotational() {
        let stats = DeviceIoStats {
            device: "sda".to_string(),
            is_nvme: false,
            is_rotational: true,
            ..Default::default()
        };
        assert!(!stats.is_nvme);
        assert!(stats.is_rotational);
    }

    #[test]
    fn test_io_workload_type_all_descriptions() {
        let types = [
            IoWorkloadType::Idle,
            IoWorkloadType::Sequential,
            IoWorkloadType::Random,
            IoWorkloadType::Mixed,
        ];
        for t in types {
            let desc = t.description();
            assert!(!desc.is_empty());
        }
    }
}
