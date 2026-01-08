//! Storage analysis with large file anomaly detection.
//!
//! Implements Modified Z-Score outlier detection (Iglewicz & Hoaglin, 1993)
//! for identifying anomalously large files in real-time.

use crate::ring_buffer::RingBuffer;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;

/// A detected file size anomaly
#[derive(Debug, Clone)]
pub struct Anomaly {
    /// Path to the anomalous file
    pub path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// Modified Z-score (higher = more anomalous)
    pub z_score: f64,
    /// When the anomaly was detected
    pub timestamp: Instant,
}

impl Anomaly {
    /// Get a human-readable description of the anomaly
    pub fn description(&self) -> String {
        let size_str = format_bytes(self.size);
        let age = self.timestamp.elapsed();
        let age_str = if age.as_secs() < 60 {
            format!("{}s ago", age.as_secs())
        } else if age.as_secs() < 3600 {
            format!("{}m ago", age.as_secs() / 60)
        } else {
            format!("{}h ago", age.as_secs() / 3600)
        };

        format!(
            "{} {} (z={:.1}) {}",
            self.path.display(),
            size_str,
            self.z_score,
            age_str
        )
    }
}

/// Large file anomaly detector using Modified Z-Score.
///
/// The Modified Z-Score uses median and MAD (Median Absolute Deviation)
/// instead of mean and standard deviation, making it more robust to
/// outliers (Iglewicz & Hoaglin, 1993).
///
/// Modified Z-Score = 0.6745 × (x - median) / MAD
/// Values with |Z| > 3.5 are considered outliers.
#[derive(Debug)]
pub struct LargeFileDetector {
    /// History of recent file sizes
    size_history: RingBuffer<u64>,
    /// Cached median value
    median: u64,
    /// Cached MAD (Median Absolute Deviation)
    mad: u64,
    /// Detected anomalies (limited to last N)
    anomalies: VecDeque<Anomaly>,
    /// Maximum anomalies to keep
    max_anomalies: usize,
    /// Z-score threshold for anomaly detection
    z_threshold: f64,
}

impl LargeFileDetector {
    /// Create a new detector with default settings.
    /// Keeps 1000 file sizes for statistics and 100 recent anomalies.
    pub fn new() -> Self {
        Self {
            size_history: RingBuffer::new(1000),
            median: 0,
            mad: 0,
            anomalies: VecDeque::with_capacity(100),
            max_anomalies: 100,
            z_threshold: 3.5,
        }
    }

    /// Create a detector with custom capacity and threshold.
    pub fn with_capacity(history_size: usize, max_anomalies: usize, z_threshold: f64) -> Self {
        Self {
            size_history: RingBuffer::new(history_size),
            median: 0,
            mad: 0,
            anomalies: VecDeque::with_capacity(max_anomalies),
            max_anomalies,
            z_threshold,
        }
    }

    /// Calculate Modified Z-Score for a file size.
    ///
    /// Returns the absolute Z-score value (always positive).
    pub fn calculate_z_score(&self, file_size: u64) -> f64 {
        if self.mad == 0 {
            // If MAD is 0, use a simple heuristic
            if file_size > self.median * 10 {
                return 10.0; // Clearly anomalous
            }
            return 0.0;
        }

        // Modified Z-Score = 0.6745 × (x - median) / MAD
        0.6745 * (file_size as f64 - self.median as f64).abs() / self.mad as f64
    }

    /// Check if a file size is anomalous.
    pub fn is_anomaly(&self, file_size: u64) -> bool {
        self.calculate_z_score(file_size) > self.z_threshold
    }

    /// Process a new file creation event.
    /// Returns Some(Anomaly) if the file is anomalously large.
    pub fn on_file_created(&mut self, path: PathBuf, size: u64) -> Option<Anomaly> {
        // Add to history
        self.size_history.push(size);
        self.update_statistics();

        // Check for anomaly
        if self.is_anomaly(size) {
            let anomaly = Anomaly {
                path,
                size,
                z_score: self.calculate_z_score(size),
                timestamp: Instant::now(),
            };

            // Add to anomaly list
            if self.anomalies.len() >= self.max_anomalies {
                self.anomalies.pop_front();
            }
            self.anomalies.push_back(anomaly.clone());

            Some(anomaly)
        } else {
            None
        }
    }

    /// Get recent anomalies.
    pub fn recent_anomalies(&self) -> impl Iterator<Item = &Anomaly> {
        self.anomalies.iter()
    }

    /// Get the number of samples in the history.
    pub fn sample_count(&self) -> usize {
        self.size_history.len()
    }

    /// Get the current median file size.
    pub fn median(&self) -> u64 {
        self.median
    }

    /// Get the current MAD value.
    pub fn mad(&self) -> u64 {
        self.mad
    }

    /// Clear all history and anomalies.
    pub fn clear(&mut self) {
        self.size_history.clear();
        self.anomalies.clear();
        self.median = 0;
        self.mad = 0;
    }

    /// Update median and MAD from current history.
    fn update_statistics(&mut self) {
        if self.size_history.is_empty() {
            self.median = 0;
            self.mad = 0;
            return;
        }

        // Get sorted values for median calculation
        let mut values: Vec<u64> = self.size_history.iter().copied().collect();
        values.sort_unstable();

        // Calculate median
        let len = values.len();
        self.median = if len % 2 == 0 {
            (values[len / 2 - 1] + values[len / 2]) / 2
        } else {
            values[len / 2]
        };

        // Calculate MAD (Median Absolute Deviation)
        let mut deviations: Vec<u64> = values
            .iter()
            .map(|&x| x.abs_diff(self.median))
            .collect();
        deviations.sort_unstable();

        self.mad = if len % 2 == 0 {
            (deviations[len / 2 - 1] + deviations[len / 2]) / 2
        } else {
            deviations[len / 2]
        };
    }
}

impl Default for LargeFileDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Mount point storage information
#[derive(Debug, Clone)]
pub struct MountInfo {
    /// Mount point path
    pub mount_point: String,
    /// Device path
    pub device: String,
    /// Filesystem type
    pub fs_type: String,
    /// Total space in bytes
    pub total_bytes: u64,
    /// Used space in bytes
    pub used_bytes: u64,
    /// Free space in bytes
    pub free_bytes: u64,
    /// Available space in bytes (may be less than free due to reserved)
    pub available_bytes: u64,
    /// Total inodes
    pub inodes_total: u64,
    /// Used inodes
    pub inodes_used: u64,
    /// Free inodes
    pub inodes_free: u64,
}

impl MountInfo {
    /// Get usage percentage
    pub fn usage_percent(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.used_bytes as f64 / self.total_bytes as f64) * 100.0
    }

    /// Get inode usage percentage
    pub fn inode_usage_percent(&self) -> f64 {
        if self.inodes_total == 0 {
            return 0.0;
        }
        (self.inodes_used as f64 / self.inodes_total as f64) * 100.0
    }

    /// Get total space in GB
    pub fn total_gb(&self) -> f64 {
        self.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    /// Get used space in GB
    pub fn used_gb(&self) -> f64 {
        self.used_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    /// Get free space in GB
    pub fn free_gb(&self) -> f64 {
        self.free_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }
}

/// Storage analyzer combining mount monitoring and anomaly detection.
pub struct StorageAnalyzer {
    /// Mount point information
    mounts: Vec<MountInfo>,
    /// Large file anomaly detector
    detector: LargeFileDetector,
    /// Storage usage history per mount (keyed by mount point)
    usage_history: std::collections::HashMap<String, RingBuffer<f64>>,
}

impl StorageAnalyzer {
    /// Create a new storage analyzer.
    pub fn new() -> Self {
        Self {
            mounts: Vec::new(),
            detector: LargeFileDetector::new(),
            usage_history: std::collections::HashMap::new(),
        }
    }

    /// Collect storage information from the system.
    pub fn collect(&mut self) {
        self.collect_mounts();
        self.update_history();
    }

    /// Get all mount information.
    pub fn mounts(&self) -> &[MountInfo] {
        &self.mounts
    }

    /// Get the anomaly detector for processing file events.
    pub fn detector(&self) -> &LargeFileDetector {
        &self.detector
    }

    /// Get mutable anomaly detector for processing file events.
    pub fn detector_mut(&mut self) -> &mut LargeFileDetector {
        &mut self.detector
    }

    /// Get recent anomalies.
    pub fn recent_anomalies(&self) -> impl Iterator<Item = &Anomaly> {
        self.detector.recent_anomalies()
    }

    /// Get usage history for a mount point.
    pub fn usage_history(&self, mount_point: &str) -> Option<&RingBuffer<f64>> {
        self.usage_history.get(mount_point)
    }

    /// Get total storage across all mounts.
    pub fn total_storage_bytes(&self) -> u64 {
        self.mounts.iter().map(|m| m.total_bytes).sum()
    }

    /// Get total used storage across all mounts.
    pub fn total_used_bytes(&self) -> u64 {
        self.mounts.iter().map(|m| m.used_bytes).sum()
    }

    /// Get overall usage percentage.
    pub fn overall_usage_percent(&self) -> f64 {
        let total = self.total_storage_bytes();
        if total == 0 {
            return 0.0;
        }
        (self.total_used_bytes() as f64 / total as f64) * 100.0
    }

    fn collect_mounts(&mut self) {
        use std::fs;

        self.mounts.clear();

        // Read /proc/mounts for mount points
        let mounts_content = match fs::read_to_string("/proc/mounts") {
            Ok(c) => c,
            Err(_) => return,
        };

        for line in mounts_content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 4 {
                continue;
            }

            let device = parts[0];
            let mount_point = parts[1];
            let fs_type = parts[2];

            // Skip virtual/system filesystems
            if fs_type == "proc"
                || fs_type == "sysfs"
                || fs_type == "devpts"
                || fs_type == "tmpfs"
                || fs_type == "devtmpfs"
                || fs_type == "cgroup"
                || fs_type == "cgroup2"
                || fs_type == "securityfs"
                || fs_type == "debugfs"
                || fs_type == "tracefs"
                || fs_type == "configfs"
                || fs_type == "fusectl"
                || fs_type == "hugetlbfs"
                || fs_type == "mqueue"
                || fs_type == "binfmt_misc"
                || fs_type == "autofs"
                || fs_type == "pstore"
                || fs_type == "efivarfs"
            {
                continue;
            }

            // Skip if mount point is not accessible
            if !std::path::Path::new(mount_point).exists() {
                continue;
            }

            // Get statvfs info
            #[cfg_attr(not(target_os = "linux"), allow(unused_mut))]
            let mut mount_info = MountInfo {
                mount_point: mount_point.to_string(),
                device: device.to_string(),
                fs_type: fs_type.to_string(),
                total_bytes: 0,
                used_bytes: 0,
                free_bytes: 0,
                available_bytes: 0,
                inodes_total: 0,
                inodes_used: 0,
                inodes_free: 0,
            };

            #[cfg(target_os = "linux")]
            {
                use std::ffi::CString;
                use std::mem::MaybeUninit;

                if let Ok(path_cstr) = CString::new(mount_point) {
                    let mut stat = MaybeUninit::<libc::statvfs>::uninit();
                    // SAFETY: statvfs is a POSIX syscall that initializes the stat buffer
                    #[allow(unsafe_code)]
                    unsafe {
                        if libc::statvfs(path_cstr.as_ptr(), stat.as_mut_ptr()) == 0 {
                            let stat = stat.assume_init();
                            let block_size = stat.f_frsize;
                            mount_info.total_bytes = stat.f_blocks * block_size;
                            mount_info.free_bytes = stat.f_bfree * block_size;
                            mount_info.available_bytes = stat.f_bavail * block_size;
                            mount_info.used_bytes =
                                mount_info.total_bytes - mount_info.free_bytes;
                            mount_info.inodes_total = stat.f_files;
                            mount_info.inodes_free = stat.f_ffree;
                            mount_info.inodes_used =
                                mount_info.inodes_total - mount_info.inodes_free;
                        }
                    }
                }
            }

            // Only include mounts with actual storage
            if mount_info.total_bytes > 0 {
                self.mounts.push(mount_info);
            }
        }

        // Sort by mount point for consistent ordering
        self.mounts
            .sort_by(|a, b| a.mount_point.cmp(&b.mount_point));
    }

    fn update_history(&mut self) {
        for mount in &self.mounts {
            let usage = mount.usage_percent() / 100.0; // Normalize to 0-1
            self.usage_history
                .entry(mount.mount_point.clone())
                .or_insert_with(|| RingBuffer::new(60))
                .push(usage);
        }
    }
}

impl Default for StorageAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Format bytes into human-readable string
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_large_file_detector() {
        let mut detector = LargeFileDetector::new();

        // Add some normal file sizes (1KB - 10MB range)
        for i in 1..100 {
            let size = (i * 1024 * 10) as u64; // 10KB to 1MB
            detector.on_file_created(PathBuf::from(format!("/tmp/file{}", i)), size);
        }

        // A 10GB file should be anomalous
        let anomaly = detector.on_file_created(
            PathBuf::from("/tmp/huge_file"),
            10 * 1024 * 1024 * 1024, // 10GB
        );
        assert!(anomaly.is_some());
        assert!(anomaly.unwrap().z_score > 3.5);

        // A 500KB file should be normal
        let normal =
            detector.on_file_created(PathBuf::from("/tmp/normal_file"), 500 * 1024);
        assert!(normal.is_none());
    }

    #[test]
    fn test_z_score_calculation() {
        let mut detector = LargeFileDetector::with_capacity(10, 10, 3.5);

        // Add identical values - MAD will be 0
        for _ in 0..10 {
            detector.size_history.push(1000);
        }
        detector.update_statistics();

        // With MAD=0, very large values should still be detected
        assert!(detector.is_anomaly(100000)); // 100x larger
        assert!(!detector.is_anomaly(1000)); // Same as median
    }

    #[test]
    fn test_mount_info_calculations() {
        let mount = MountInfo {
            mount_point: "/".to_string(),
            device: "/dev/sda1".to_string(),
            fs_type: "ext4".to_string(),
            total_bytes: 1000 * 1024 * 1024 * 1024, // 1TB
            used_bytes: 500 * 1024 * 1024 * 1024,   // 500GB
            free_bytes: 500 * 1024 * 1024 * 1024,
            available_bytes: 450 * 1024 * 1024 * 1024,
            inodes_total: 1000000,
            inodes_used: 250000,
            inodes_free: 750000,
        };

        assert!((mount.usage_percent() - 50.0).abs() < 0.1);
        assert!((mount.total_gb() - 1000.0).abs() < 1.0);
        assert!((mount.inode_usage_percent() - 25.0).abs() < 0.1);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1500), "1.5 KB");
        assert_eq!(format_bytes(1500 * 1024), "1.5 MB");
        assert_eq!(format_bytes(1500 * 1024 * 1024), "1.5 GB");
        assert_eq!(format_bytes(1500u64 * 1024 * 1024 * 1024), "1.5 TB");
    }

    #[test]
    fn test_format_bytes_zero() {
        assert_eq!(format_bytes(0), "0 B");
    }

    #[test]
    fn test_format_bytes_exact_boundaries() {
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn test_mount_info_zero_values() {
        let mount = MountInfo {
            mount_point: "/".to_string(),
            device: "/dev/sda1".to_string(),
            fs_type: "ext4".to_string(),
            total_bytes: 0,
            used_bytes: 0,
            free_bytes: 0,
            available_bytes: 0,
            inodes_total: 0,
            inodes_used: 0,
            inodes_free: 0,
        };

        // Should not panic with zero values
        assert_eq!(mount.usage_percent(), 0.0);
        assert_eq!(mount.total_gb(), 0.0);
        assert_eq!(mount.inode_usage_percent(), 0.0);
    }

    #[test]
    fn test_mount_info_default_like() {
        let mount = MountInfo {
            mount_point: String::new(),
            device: String::new(),
            fs_type: String::new(),
            total_bytes: 0,
            used_bytes: 0,
            free_bytes: 0,
            available_bytes: 0,
            inodes_total: 0,
            inodes_used: 0,
            inodes_free: 0,
        };
        assert_eq!(mount.mount_point, "");
        assert_eq!(mount.device, "");
        assert_eq!(mount.fs_type, "");
        assert_eq!(mount.total_bytes, 0);
    }

    #[test]
    fn test_anomaly_struct() {
        let anomaly = Anomaly {
            path: PathBuf::from("/test/path"),
            size: 1000000,
            z_score: 5.0,
            timestamp: std::time::Instant::now(),
        };
        assert_eq!(anomaly.path, PathBuf::from("/test/path"));
        assert_eq!(anomaly.size, 1000000);
        assert!((anomaly.z_score - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_large_file_detector_with_capacity() {
        let detector = LargeFileDetector::with_capacity(100, 50, 4.0);
        // With no history, any file > 0 is considered anomalous
        // because median=0 and file_size > median*10
        assert!(detector.is_anomaly(1000)); // z_score = 10.0 > 4.0
        // Zero-size file is not anomalous
        assert!(!detector.is_anomaly(0));
    }

    #[test]
    fn test_large_file_detector_new() {
        let detector = LargeFileDetector::new();
        // Initially nothing is an anomaly
        assert!(!detector.is_anomaly(0));
    }

    #[test]
    fn test_large_file_detector_z_threshold() {
        let mut detector = LargeFileDetector::with_capacity(10, 5, 2.0); // Lower threshold

        // Add some normal values
        for _ in 0..10 {
            detector.size_history.push(1000);
        }
        detector.update_statistics();

        // With threshold of 2.0, a 10x value might be anomalous
        // But MAD will be 0, so we hit the special case
        assert!(detector.is_anomaly(11000)); // > 10x median
    }

    #[test]
    fn test_storage_analyzer_creation() {
        let analyzer = StorageAnalyzer::new();
        // Should have some mounts (at least root on Linux)
        let _ = analyzer.mounts(); // May be empty in test environment
    }

    #[test]
    fn test_storage_analyzer_default() {
        let analyzer = StorageAnalyzer::default();
        let _ = analyzer.mounts();
    }

    #[test]
    fn test_storage_analyzer_anomalies() {
        let analyzer = StorageAnalyzer::new();
        assert_eq!(analyzer.recent_anomalies().count(), 0);
    }

    #[test]
    fn test_storage_analyzer_collect_safe() {
        let mut analyzer = StorageAnalyzer::new();
        // Collect should not panic
        analyzer.collect();
    }

    #[test]
    fn test_large_file_detector_no_data() {
        let detector = LargeFileDetector::new();
        // With no history data (MAD=0, median=0):
        // - file_size > median*10 (1000 > 0) → returns 10.0
        assert_eq!(detector.calculate_z_score(1000), 10.0);
        // - file_size = 0 → not > median*10 → returns 0.0
        assert_eq!(detector.calculate_z_score(0), 0.0);
    }

    #[test]
    fn test_large_file_detector_varied_data() {
        let mut detector = LargeFileDetector::with_capacity(10, 5, 3.5);

        // Add varied file sizes
        detector.size_history.push(100);
        detector.size_history.push(200);
        detector.size_history.push(300);
        detector.size_history.push(400);
        detector.size_history.push(500);
        detector.update_statistics();

        // Median should be around 300
        // A very large file should be detected as anomaly
        assert!(detector.is_anomaly(10000)); // Much larger than median
    }

    #[test]
    fn test_mount_info_partial_data() {
        let mount = MountInfo {
            mount_point: "/data".to_string(),
            device: "/dev/nvme0n1p1".to_string(),
            fs_type: "xfs".to_string(),
            total_bytes: 100 * 1024 * 1024 * 1024, // 100GB
            used_bytes: 90 * 1024 * 1024 * 1024,   // 90GB used
            free_bytes: 10 * 1024 * 1024 * 1024,
            available_bytes: 5 * 1024 * 1024 * 1024,
            inodes_total: 100000,
            inodes_used: 100000, // All inodes used
            inodes_free: 0,
        };

        assert!((mount.usage_percent() - 90.0).abs() < 0.1);
        assert!((mount.inode_usage_percent() - 100.0).abs() < 0.1);
    }
}
