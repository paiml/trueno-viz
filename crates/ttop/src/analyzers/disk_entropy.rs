//! Disk entropy analyzer for estimating data diversity/duplication.
//!
//! Uses Shannon entropy on sampled file content to estimate:
//! - How compressible the disk content is
//! - Potential for deduplication
//! - Data diversity score (0.0 = all identical, 1.0 = maximum entropy)
//!
//! Higher entropy = more unique/random data (less compressible)
//! Lower entropy = more repetitive/duplicate data (more compressible)

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::time::{Duration, Instant};

/// Entropy metrics for a single mount point
#[derive(Debug, Clone, Default)]
pub struct MountEntropy {
    /// Shannon entropy normalized to 0.0-1.0
    pub entropy: f64,
    /// Number of files sampled
    pub files_sampled: usize,
    /// Bytes sampled
    pub bytes_sampled: u64,
    /// Estimated deduplication potential (0.0-1.0)
    pub dedup_potential: f64,
    /// Last update time
    pub last_update: Option<Instant>,
}

impl MountEntropy {
    /// Get entropy as a 5-level gauge string
    pub fn gauge(&self) -> &'static str {
        if self.entropy >= 0.9 {
            "●●●●●" // Very high entropy (random/encrypted)
        } else if self.entropy >= 0.75 {
            "●●●●○" // High entropy (compressed/unique)
        } else if self.entropy >= 0.5 {
            "●●●○○" // Medium entropy (mixed)
        } else if self.entropy >= 0.25 {
            "●●○○○" // Low entropy (some duplication)
        } else {
            "●○○○○" // Very low entropy (high duplication)
        }
    }

    /// Get a compact indicator character
    pub fn indicator(&self) -> char {
        if self.entropy >= 0.8 {
            '●' // High entropy - unique
        } else if self.entropy >= 0.5 {
            '◐' // Medium entropy
        } else {
            '○' // Low entropy - duplicates
        }
    }
}

/// Disk entropy analyzer
#[derive(Debug, Default)]
pub struct DiskEntropyAnalyzer {
    /// Entropy per mount point
    pub mount_entropy: HashMap<String, MountEntropy>,
    /// Overall system entropy (weighted average)
    pub system_entropy: f64,
    /// Sample interval
    sample_interval: Duration,
    /// Last full scan time
    last_scan: Option<Instant>,
    /// Bytes per sample (read from each file)
    sample_size: usize,
    /// Max files to sample per mount
    max_files_per_mount: usize,
}

impl DiskEntropyAnalyzer {
    pub fn new() -> Self {
        Self {
            mount_entropy: HashMap::new(),
            system_entropy: 0.5, // Default to medium
            sample_interval: Duration::from_secs(30), // Sample every 30s
            last_scan: None,
            sample_size: 4096, // 4KB per file
            max_files_per_mount: 50, // Sample up to 50 files
        }
    }

    /// Collect entropy metrics (rate-limited)
    pub fn collect(&mut self, mounts: &[String]) {
        // Rate limit: only scan every sample_interval
        if let Some(last) = self.last_scan {
            if last.elapsed() < self.sample_interval {
                return;
            }
        }

        for mount in mounts {
            let entropy = self.analyze_mount(mount);
            self.mount_entropy.insert(mount.clone(), entropy);
        }

        // Calculate weighted system entropy
        self.calculate_system_entropy();
        self.last_scan = Some(Instant::now());
    }

    /// Analyze a single mount point
    fn analyze_mount(&self, mount_path: &str) -> MountEntropy {
        let path = Path::new(mount_path);
        if !path.exists() {
            return MountEntropy::default();
        }

        let mut byte_counts = [0u64; 256];
        let mut total_bytes = 0u64;
        let mut files_sampled = 0usize;

        // Walk directory and sample files
        if let Ok(entries) = self.walk_dir_limited(path, self.max_files_per_mount) {
            for entry_path in entries {
                if let Ok(sample) = self.sample_file(&entry_path) {
                    for byte in sample {
                        byte_counts[byte as usize] += 1;
                        total_bytes += 1;
                    }
                    files_sampled += 1;
                }
            }
        }

        if total_bytes == 0 {
            return MountEntropy {
                entropy: 0.5, // Unknown, assume medium
                files_sampled: 0,
                bytes_sampled: 0,
                dedup_potential: 0.0,
                last_update: Some(Instant::now()),
            };
        }

        // Calculate Shannon entropy
        let entropy = self.calculate_entropy(&byte_counts, total_bytes);

        // Normalize to 0-1 (max entropy for bytes is 8 bits)
        let normalized_entropy = entropy / 8.0;

        // Estimate dedup potential (inverse of entropy)
        // High entropy = low dedup potential
        let dedup_potential = 1.0 - normalized_entropy;

        MountEntropy {
            entropy: normalized_entropy,
            files_sampled,
            bytes_sampled: total_bytes,
            dedup_potential,
            last_update: Some(Instant::now()),
        }
    }

    /// Walk directory with file limit
    fn walk_dir_limited(&self, path: &Path, max_files: usize) -> std::io::Result<Vec<std::path::PathBuf>> {
        let mut files = Vec::new();
        self.walk_dir_recursive(path, &mut files, max_files, 0)?;
        Ok(files)
    }

    fn walk_dir_recursive(
        &self,
        path: &Path,
        files: &mut Vec<std::path::PathBuf>,
        max_files: usize,
        depth: usize,
    ) -> std::io::Result<()> {
        // Limit depth to avoid deep traversal
        if depth > 5 || files.len() >= max_files {
            return Ok(());
        }

        // Skip special directories
        let path_str = path.to_string_lossy();
        if path_str.contains("/proc")
            || path_str.contains("/sys")
            || path_str.contains("/dev")
            || path_str.contains("/run")
            || path_str.contains("/tmp")
        {
            return Ok(());
        }

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if files.len() >= max_files {
                    break;
                }

                let entry_path = entry.path();
                if entry_path.is_file() {
                    // Skip very small files and special files
                    if let Ok(meta) = entry.metadata() {
                        if meta.len() >= 1024 && meta.len() <= 100_000_000 {
                            files.push(entry_path);
                        }
                    }
                } else if entry_path.is_dir() {
                    let _ = self.walk_dir_recursive(&entry_path, files, max_files, depth + 1);
                }
            }
        }

        Ok(())
    }

    /// Sample bytes from a file (beginning, middle, end)
    fn sample_file(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        let mut file = File::open(path)?;
        let meta = file.metadata()?;
        let file_size = meta.len();

        if file_size == 0 {
            return Ok(Vec::new());
        }

        let mut buffer = Vec::with_capacity(self.sample_size * 3);
        let chunk_size = self.sample_size.min(file_size as usize);

        // Read from beginning
        let mut chunk = vec![0u8; chunk_size];
        file.read_exact(&mut chunk)?;
        buffer.extend_from_slice(&chunk);

        // Read from middle (if file is large enough)
        if file_size > (chunk_size * 2) as u64 {
            let mid_pos = file_size / 2;
            file.seek(SeekFrom::Start(mid_pos))?;
            let mut mid_chunk = vec![0u8; chunk_size.min((file_size - mid_pos) as usize)];
            if file.read_exact(&mut mid_chunk).is_ok() {
                buffer.extend_from_slice(&mid_chunk);
            }
        }

        // Read from end (if file is large enough)
        if file_size > (chunk_size * 3) as u64 {
            let end_pos = file_size.saturating_sub(chunk_size as u64);
            file.seek(SeekFrom::Start(end_pos))?;
            let mut end_chunk = vec![0u8; chunk_size];
            if file.read_exact(&mut end_chunk).is_ok() {
                buffer.extend_from_slice(&end_chunk);
            }
        }

        Ok(buffer)
    }

    /// Calculate Shannon entropy from byte frequency counts
    fn calculate_entropy(&self, counts: &[u64; 256], total: u64) -> f64 {
        if total == 0 {
            return 0.0;
        }

        let mut entropy = 0.0;
        let total_f64 = total as f64;

        for &count in counts {
            if count > 0 {
                let probability = count as f64 / total_f64;
                entropy -= probability * probability.log2();
            }
        }

        entropy
    }

    /// Calculate weighted system entropy
    fn calculate_system_entropy(&mut self) {
        if self.mount_entropy.is_empty() {
            self.system_entropy = 0.5;
            return;
        }

        let total_bytes: u64 = self.mount_entropy.values().map(|e| e.bytes_sampled).sum();
        if total_bytes == 0 {
            self.system_entropy = 0.5;
            return;
        }

        let weighted_sum: f64 = self
            .mount_entropy
            .values()
            .map(|e| e.entropy * e.bytes_sampled as f64)
            .sum();

        self.system_entropy = weighted_sum / total_bytes as f64;
    }

    /// Get entropy for a specific mount
    pub fn get_mount_entropy(&self, mount: &str) -> Option<&MountEntropy> {
        self.mount_entropy.get(mount)
    }

    /// Get system-wide entropy gauge
    pub fn system_gauge(&self) -> &'static str {
        if self.system_entropy >= 0.9 {
            "●●●●●"
        } else if self.system_entropy >= 0.75 {
            "●●●●○"
        } else if self.system_entropy >= 0.5 {
            "●●●○○"
        } else if self.system_entropy >= 0.25 {
            "●●○○○"
        } else {
            "●○○○○"
        }
    }

    /// Format entropy as percentage
    pub fn format_entropy_pct(&self, entropy: f64) -> String {
        format!("{:.0}%", entropy * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_gauge_thresholds() {
        let mut me = MountEntropy::default();

        me.entropy = 0.95;
        assert_eq!(me.gauge(), "●●●●●");

        me.entropy = 0.8;
        assert_eq!(me.gauge(), "●●●●○");

        me.entropy = 0.6;
        assert_eq!(me.gauge(), "●●●○○");

        me.entropy = 0.3;
        assert_eq!(me.gauge(), "●●○○○");

        me.entropy = 0.1;
        assert_eq!(me.gauge(), "●○○○○");
    }

    #[test]
    fn test_entropy_indicator() {
        let mut me = MountEntropy::default();

        me.entropy = 0.9;
        assert_eq!(me.indicator(), '●');

        me.entropy = 0.6;
        assert_eq!(me.indicator(), '◐');

        me.entropy = 0.3;
        assert_eq!(me.indicator(), '○');
    }

    #[test]
    fn test_analyzer_new() {
        let analyzer = DiskEntropyAnalyzer::new();
        assert!(analyzer.mount_entropy.is_empty());
        assert_eq!(analyzer.system_entropy, 0.5);
    }

    #[test]
    fn test_calculate_entropy_uniform() {
        let analyzer = DiskEntropyAnalyzer::new();

        // Uniform distribution (maximum entropy)
        let counts = [1u64; 256];
        let total = 256u64;
        let entropy = analyzer.calculate_entropy(&counts, total);

        // Should be close to 8 bits (log2(256))
        assert!((entropy - 8.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_entropy_single_value() {
        let analyzer = DiskEntropyAnalyzer::new();

        // All same byte (minimum entropy)
        let mut counts = [0u64; 256];
        counts[0] = 1000;
        let entropy = analyzer.calculate_entropy(&counts, 1000);

        // Should be 0 (no diversity)
        assert_eq!(entropy, 0.0);
    }

    #[test]
    fn test_system_gauge() {
        let mut analyzer = DiskEntropyAnalyzer::new();

        analyzer.system_entropy = 0.95;
        assert_eq!(analyzer.system_gauge(), "●●●●●");

        analyzer.system_entropy = 0.5;
        assert_eq!(analyzer.system_gauge(), "●●●○○");

        analyzer.system_entropy = 0.1;
        assert_eq!(analyzer.system_gauge(), "●○○○○");
    }

    #[test]
    fn test_system_gauge_all_thresholds() {
        let mut analyzer = DiskEntropyAnalyzer::new();

        analyzer.system_entropy = 0.9;
        assert_eq!(analyzer.system_gauge(), "●●●●●");

        analyzer.system_entropy = 0.75;
        assert_eq!(analyzer.system_gauge(), "●●●●○");

        analyzer.system_entropy = 0.25;
        assert_eq!(analyzer.system_gauge(), "●●○○○");

        analyzer.system_entropy = 0.24;
        assert_eq!(analyzer.system_gauge(), "●○○○○");
    }

    #[test]
    fn test_format_entropy_pct() {
        let analyzer = DiskEntropyAnalyzer::new();
        assert_eq!(analyzer.format_entropy_pct(0.0), "0%");
        assert_eq!(analyzer.format_entropy_pct(0.5), "50%");
        assert_eq!(analyzer.format_entropy_pct(1.0), "100%");
        assert_eq!(analyzer.format_entropy_pct(0.333), "33%");
    }

    #[test]
    fn test_get_mount_entropy_none() {
        let analyzer = DiskEntropyAnalyzer::new();
        assert!(analyzer.get_mount_entropy("/nonexistent").is_none());
    }

    #[test]
    fn test_get_mount_entropy_some() {
        let mut analyzer = DiskEntropyAnalyzer::new();
        analyzer.mount_entropy.insert(
            "/home".to_string(),
            MountEntropy {
                entropy: 0.75,
                files_sampled: 10,
                bytes_sampled: 1024,
                dedup_potential: 0.25,
                last_update: Some(Instant::now()),
            },
        );
        let result = analyzer.get_mount_entropy("/home");
        assert!(result.is_some());
        assert!((result.unwrap().entropy - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_mount_entropy_default() {
        let me = MountEntropy::default();
        assert_eq!(me.entropy, 0.0);
        assert_eq!(me.files_sampled, 0);
        assert_eq!(me.bytes_sampled, 0);
        assert_eq!(me.dedup_potential, 0.0);
        assert!(me.last_update.is_none());
    }

    #[test]
    fn test_mount_entropy_gauge_boundaries() {
        let mut me = MountEntropy::default();

        me.entropy = 0.0;
        assert_eq!(me.gauge(), "●○○○○");

        me.entropy = 0.249;
        assert_eq!(me.gauge(), "●○○○○");

        me.entropy = 0.25;
        assert_eq!(me.gauge(), "●●○○○");

        me.entropy = 0.499;
        assert_eq!(me.gauge(), "●●○○○");

        me.entropy = 0.5;
        assert_eq!(me.gauge(), "●●●○○");

        me.entropy = 0.749;
        assert_eq!(me.gauge(), "●●●○○");

        me.entropy = 0.75;
        assert_eq!(me.gauge(), "●●●●○");

        me.entropy = 0.899;
        assert_eq!(me.gauge(), "●●●●○");

        me.entropy = 0.9;
        assert_eq!(me.gauge(), "●●●●●");

        me.entropy = 1.0;
        assert_eq!(me.gauge(), "●●●●●");
    }

    #[test]
    fn test_mount_entropy_indicator_boundaries() {
        let mut me = MountEntropy::default();

        me.entropy = 0.0;
        assert_eq!(me.indicator(), '○');

        me.entropy = 0.499;
        assert_eq!(me.indicator(), '○');

        me.entropy = 0.5;
        assert_eq!(me.indicator(), '◐');

        me.entropy = 0.799;
        assert_eq!(me.indicator(), '◐');

        me.entropy = 0.8;
        assert_eq!(me.indicator(), '●');

        me.entropy = 1.0;
        assert_eq!(me.indicator(), '●');
    }

    #[test]
    fn test_calculate_entropy_zero_total() {
        let analyzer = DiskEntropyAnalyzer::new();
        let counts = [0u64; 256];
        let entropy = analyzer.calculate_entropy(&counts, 0);
        assert_eq!(entropy, 0.0);
    }

    #[test]
    fn test_calculate_entropy_two_values() {
        let analyzer = DiskEntropyAnalyzer::new();
        let mut counts = [0u64; 256];
        counts[0] = 500;
        counts[255] = 500;
        let entropy = analyzer.calculate_entropy(&counts, 1000);
        // Should be close to 1 bit (log2(2))
        assert!((entropy - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_entropy_four_values() {
        let analyzer = DiskEntropyAnalyzer::new();
        let mut counts = [0u64; 256];
        counts[0] = 250;
        counts[1] = 250;
        counts[2] = 250;
        counts[3] = 250;
        let entropy = analyzer.calculate_entropy(&counts, 1000);
        // Should be close to 2 bits (log2(4))
        assert!((entropy - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_analyzer_default() {
        let analyzer = DiskEntropyAnalyzer::default();
        assert!(analyzer.mount_entropy.is_empty());
        assert_eq!(analyzer.system_entropy, 0.0);
    }

    #[test]
    fn test_calculate_system_entropy_empty() {
        let mut analyzer = DiskEntropyAnalyzer::new();
        analyzer.calculate_system_entropy();
        assert_eq!(analyzer.system_entropy, 0.5);
    }

    #[test]
    fn test_calculate_system_entropy_zero_bytes() {
        let mut analyzer = DiskEntropyAnalyzer::new();
        analyzer.mount_entropy.insert(
            "/home".to_string(),
            MountEntropy {
                entropy: 0.8,
                files_sampled: 0,
                bytes_sampled: 0,
                dedup_potential: 0.2,
                last_update: None,
            },
        );
        analyzer.calculate_system_entropy();
        assert_eq!(analyzer.system_entropy, 0.5);
    }

    #[test]
    fn test_calculate_system_entropy_weighted() {
        let mut analyzer = DiskEntropyAnalyzer::new();
        analyzer.mount_entropy.insert(
            "/home".to_string(),
            MountEntropy {
                entropy: 0.8,
                files_sampled: 10,
                bytes_sampled: 800,
                dedup_potential: 0.2,
                last_update: None,
            },
        );
        analyzer.mount_entropy.insert(
            "/var".to_string(),
            MountEntropy {
                entropy: 0.4,
                files_sampled: 5,
                bytes_sampled: 200,
                dedup_potential: 0.6,
                last_update: None,
            },
        );
        analyzer.calculate_system_entropy();
        // Weighted: (0.8*800 + 0.4*200) / 1000 = (640+80)/1000 = 0.72
        assert!((analyzer.system_entropy - 0.72).abs() < 0.01);
    }
}
