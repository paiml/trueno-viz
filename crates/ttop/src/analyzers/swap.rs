//! Swap analysis with thrashing detection.
//!
//! Implements Denning's Working Set Model (1968) for thrashing detection
//! using multiple signals: PSI, swap I/O rate, and page fault rate.

use crate::ring_buffer::RingBuffer;
use std::fs;
use std::path::Path;

/// Thrashing severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThrashingSeverity {
    /// No thrashing detected
    None,
    /// Mild thrashing - system slightly impacted
    Mild,
    /// Moderate thrashing - noticeable performance impact
    Moderate,
    /// Severe thrashing - system heavily impacted
    Severe,
}

impl ThrashingSeverity {
    /// Get a human-readable status string
    pub fn status(&self) -> &'static str {
        match self {
            Self::None => "OK",
            Self::Mild => "Mild",
            Self::Moderate => "Warning",
            Self::Severe => "Critical",
        }
    }

    /// Get a color hint for UI rendering
    pub fn color_hint(&self) -> &'static str {
        match self {
            Self::None => "green",
            Self::Mild => "yellow",
            Self::Moderate => "orange",
            Self::Severe => "red",
        }
    }
}

/// ZRAM compression statistics
#[derive(Debug, Clone, Default)]
pub struct ZramStats {
    /// Original (uncompressed) data size in bytes
    pub orig_data_size: u64,
    /// Compressed data size in bytes
    pub compr_data_size: u64,
    /// Total memory used including metadata
    pub mem_used_total: u64,
    /// Memory limit (0 = no limit)
    pub mem_limit: u64,
    /// Maximum pages used
    pub max_used_pages: u64,
    /// Pages with identical content (zero pages)
    pub same_pages: u64,
    /// Pages that were compacted
    pub pages_compacted: u64,
    /// Huge (incompressible) pages
    pub huge_pages: u64,
    /// Compression algorithm name
    pub comp_algorithm: String,
    /// Device name (e.g., "zram0")
    pub device: String,
}

impl ZramStats {
    /// Calculate compression ratio (uncompressed / compressed)
    /// Higher is better. Returns 1.0 if no data.
    pub fn compression_ratio(&self) -> f64 {
        if self.compr_data_size == 0 {
            return 1.0;
        }
        self.orig_data_size as f64 / self.compr_data_size as f64
    }

    /// Calculate space savings as a percentage (0-100)
    pub fn space_savings_percent(&self) -> f64 {
        if self.orig_data_size == 0 {
            return 0.0;
        }
        (1.0 - (self.compr_data_size as f64 / self.orig_data_size as f64)) * 100.0
    }

    /// Check if ZRAM is active (has stored data)
    pub fn is_active(&self) -> bool {
        self.orig_data_size > 0
    }
}

/// Pressure Stall Information (PSI) metrics
#[derive(Debug, Clone, Default)]
pub struct PsiMetrics {
    /// "some" percentage (avg10)
    pub some_avg10: f64,
    /// "some" percentage (avg60)
    pub some_avg60: f64,
    /// "some" percentage (avg300)
    pub some_avg300: f64,
    /// "full" percentage (avg10)
    pub full_avg10: f64,
    /// "full" percentage (avg60)
    pub full_avg60: f64,
    /// "full" percentage (avg300)
    pub full_avg300: f64,
    /// Total stall time in microseconds
    pub some_total_us: u64,
    /// Total full stall time in microseconds
    pub full_total_us: u64,
}

/// Swap analyzer with thrashing detection.
///
/// Implements multi-signal thrashing detection per Denning's Working Set Model.
pub struct SwapAnalyzer {
    /// History of pages swapped in (per second)
    pswpin_history: RingBuffer<u64>,
    /// History of pages swapped out (per second)
    pswpout_history: RingBuffer<u64>,
    /// History of major page faults (per second)
    pgmajfault_history: RingBuffer<u64>,
    /// History of minor page faults (per second)
    pgfault_history: RingBuffer<u64>,
    /// Latest PSI metrics
    psi: PsiMetrics,
    /// Latest ZRAM stats (if available)
    zram_stats: Vec<ZramStats>,
    /// Previous vmstat values for delta calculation
    prev_pswpin: u64,
    prev_pswpout: u64,
    prev_pgmajfault: u64,
    prev_pgfault: u64,
    /// Sample interval in seconds
    sample_interval_secs: f64,
}

impl SwapAnalyzer {
    /// Create a new swap analyzer with a 60-second history window.
    pub fn new() -> Self {
        Self {
            pswpin_history: RingBuffer::new(60),
            pswpout_history: RingBuffer::new(60),
            pgmajfault_history: RingBuffer::new(60),
            pgfault_history: RingBuffer::new(60),
            psi: PsiMetrics::default(),
            zram_stats: Vec::new(),
            prev_pswpin: 0,
            prev_pswpout: 0,
            prev_pgmajfault: 0,
            prev_pgfault: 0,
            sample_interval_secs: 1.0,
        }
    }

    /// Set the sample interval in seconds
    pub fn set_sample_interval(&mut self, secs: f64) {
        self.sample_interval_secs = secs;
    }

    /// Collect metrics from /proc/vmstat and /proc/pressure/memory
    pub fn collect(&mut self) {
        self.collect_vmstat();
        self.collect_psi();
        self.collect_zram();
    }

    /// Detect thrashing severity based on multiple signals.
    ///
    /// Uses thresholds derived from Denning's Working Set Model:
    /// - PSI some >50% or swap rate >1000/s with faults >100/s = Severe
    /// - PSI some >25% or swap rate >500/s with faults >50/s = Moderate
    /// - PSI some >10% or swap rate >100/s with faults >10/s = Mild
    pub fn detect_thrashing(&self) -> ThrashingSeverity {
        let swap_rate = self.swap_rate_per_sec();
        let fault_rate = self.major_fault_rate_per_sec();
        let psi_pressure = self.psi.some_avg10;

        // Multi-signal detection per specification
        if psi_pressure > 50.0 || (swap_rate > 1000.0 && fault_rate > 100.0) {
            ThrashingSeverity::Severe
        } else if psi_pressure > 25.0 || (swap_rate > 500.0 && fault_rate > 50.0) {
            ThrashingSeverity::Moderate
        } else if psi_pressure > 10.0 || (swap_rate > 100.0 && fault_rate > 10.0) {
            ThrashingSeverity::Mild
        } else {
            ThrashingSeverity::None
        }
    }

    /// Get the current swap I/O rate (pages in + pages out per second)
    pub fn swap_rate_per_sec(&self) -> f64 {
        self.pswpin_history.rate_per_sec(self.sample_interval_secs)
            + self.pswpout_history.rate_per_sec(self.sample_interval_secs)
    }

    /// Get the pages-in rate per second
    pub fn pages_in_rate(&self) -> f64 {
        self.pswpin_history.rate_per_sec(self.sample_interval_secs)
    }

    /// Get the pages-out rate per second
    pub fn pages_out_rate(&self) -> f64 {
        self.pswpout_history.rate_per_sec(self.sample_interval_secs)
    }

    /// Get the major page fault rate per second
    pub fn major_fault_rate_per_sec(&self) -> f64 {
        self.pgmajfault_history
            .rate_per_sec(self.sample_interval_secs)
    }

    /// Get the minor page fault rate per second
    pub fn minor_fault_rate_per_sec(&self) -> f64 {
        self.pgfault_history.rate_per_sec(self.sample_interval_secs)
    }

    /// Get the latest PSI metrics
    pub fn psi(&self) -> &PsiMetrics {
        &self.psi
    }

    /// Get all ZRAM device stats
    pub fn zram_stats(&self) -> &[ZramStats] {
        &self.zram_stats
    }

    /// Get combined ZRAM compression ratio across all devices
    pub fn zram_compression_ratio(&self) -> f64 {
        let total_orig: u64 = self.zram_stats.iter().map(|z| z.orig_data_size).sum();
        let total_compr: u64 = self.zram_stats.iter().map(|z| z.compr_data_size).sum();
        if total_compr == 0 {
            1.0
        } else {
            total_orig as f64 / total_compr as f64
        }
    }

    /// Check if ZRAM is available and active
    pub fn has_zram(&self) -> bool {
        self.zram_stats.iter().any(|z| z.is_active())
    }

    /// Get the page fault history for visualization
    pub fn fault_history(&self) -> &RingBuffer<u64> {
        &self.pgmajfault_history
    }

    /// Get the swap I/O history (combined in+out) for visualization
    pub fn swap_io_history(&self) -> Vec<f64> {
        // Combine in and out histories
        let in_iter = self.pswpin_history.iter();
        let out_iter = self.pswpout_history.iter();

        in_iter
            .zip(out_iter)
            .map(|(i, o)| (*i + *o) as f64)
            .collect()
    }

    // Private methods for data collection

    fn collect_vmstat(&mut self) {
        let content = match fs::read_to_string("/proc/vmstat") {
            Ok(c) => c,
            Err(_) => return,
        };

        let mut pswpin = 0u64;
        let mut pswpout = 0u64;
        let mut pgmajfault = 0u64;
        let mut pgfault = 0u64;

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }
            let value: u64 = parts[1].parse().unwrap_or(0);
            match parts[0] {
                "pswpin" => pswpin = value,
                "pswpout" => pswpout = value,
                "pgmajfault" => pgmajfault = value,
                "pgfault" => pgfault = value,
                _ => {}
            }
        }

        // Calculate deltas and push to history
        if self.prev_pswpin > 0 {
            let delta = crate::ring_buffer::handle_counter_wrap(self.prev_pswpin, pswpin);
            self.pswpin_history.push(delta);
        }
        if self.prev_pswpout > 0 {
            let delta = crate::ring_buffer::handle_counter_wrap(self.prev_pswpout, pswpout);
            self.pswpout_history.push(delta);
        }
        if self.prev_pgmajfault > 0 {
            let delta = crate::ring_buffer::handle_counter_wrap(self.prev_pgmajfault, pgmajfault);
            self.pgmajfault_history.push(delta);
        }
        if self.prev_pgfault > 0 {
            let delta = crate::ring_buffer::handle_counter_wrap(self.prev_pgfault, pgfault);
            self.pgfault_history.push(delta);
        }

        self.prev_pswpin = pswpin;
        self.prev_pswpout = pswpout;
        self.prev_pgmajfault = pgmajfault;
        self.prev_pgfault = pgfault;
    }

    fn collect_psi(&mut self) {
        let content = match fs::read_to_string("/proc/pressure/memory") {
            Ok(c) => c,
            Err(_) => return,
        };

        for line in content.lines() {
            // Format: "some avg10=0.00 avg60=0.00 avg300=0.00 total=123456"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let is_some = parts[0] == "some";
            let is_full = parts[0] == "full";

            if !is_some && !is_full {
                continue;
            }

            for part in &parts[1..] {
                if let Some((key, value)) = part.split_once('=') {
                    let val: f64 = value.parse().unwrap_or(0.0);
                    match (is_some, key) {
                        (true, "avg10") => self.psi.some_avg10 = val,
                        (true, "avg60") => self.psi.some_avg60 = val,
                        (true, "avg300") => self.psi.some_avg300 = val,
                        (true, "total") => self.psi.some_total_us = val as u64,
                        (false, "avg10") => self.psi.full_avg10 = val,
                        (false, "avg60") => self.psi.full_avg60 = val,
                        (false, "avg300") => self.psi.full_avg300 = val,
                        (false, "total") => self.psi.full_total_us = val as u64,
                        _ => {}
                    }
                }
            }
        }
    }

    fn collect_zram(&mut self) {
        self.zram_stats.clear();

        // Scan for ZRAM devices
        for i in 0..16 {
            let device = format!("zram{}", i);
            let base_path = format!("/sys/block/{}", device);

            if !Path::new(&base_path).exists() {
                continue;
            }

            let mut stats = ZramStats {
                device: device.clone(),
                ..Default::default()
            };

            // Read mm_stat (space-separated values)
            // Format: orig_data_size compr_data_size mem_used_total mem_limit max_used_pages same_pages pages_compacted huge_pages
            if let Ok(mm_stat) = fs::read_to_string(format!("{}/mm_stat", base_path)) {
                let parts: Vec<u64> = mm_stat
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();

                if parts.len() >= 8 {
                    stats.orig_data_size = parts[0];
                    stats.compr_data_size = parts[1];
                    stats.mem_used_total = parts[2];
                    stats.mem_limit = parts[3];
                    stats.max_used_pages = parts[4];
                    stats.same_pages = parts[5];
                    stats.pages_compacted = parts[6];
                    stats.huge_pages = parts[7];
                }
            }

            // Read compression algorithm
            if let Ok(algo) = fs::read_to_string(format!("{}/comp_algorithm", base_path)) {
                // Format: "lzo lzo-rle [lz4] zstd" with brackets around active
                for part in algo.split_whitespace() {
                    if part.starts_with('[') && part.ends_with(']') {
                        stats.comp_algorithm = part[1..part.len() - 1].to_string();
                        break;
                    }
                }
                // Fallback to first algorithm if no brackets found
                if stats.comp_algorithm.is_empty() {
                    if let Some(first) = algo.split_whitespace().next() {
                        stats.comp_algorithm = first.to_string();
                    }
                }
            }

            self.zram_stats.push(stats);
        }
    }
}

impl Default for SwapAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thrashing_severity_ordering() {
        assert!(ThrashingSeverity::None < ThrashingSeverity::Mild);
        assert!(ThrashingSeverity::Mild < ThrashingSeverity::Moderate);
        assert!(ThrashingSeverity::Moderate < ThrashingSeverity::Severe);
    }

    #[test]
    fn test_thrashing_severity_status() {
        assert_eq!(ThrashingSeverity::None.status(), "OK");
        assert_eq!(ThrashingSeverity::Mild.status(), "Mild");
        assert_eq!(ThrashingSeverity::Moderate.status(), "Warning");
        assert_eq!(ThrashingSeverity::Severe.status(), "Critical");
    }

    #[test]
    fn test_thrashing_severity_color_hint() {
        assert_eq!(ThrashingSeverity::None.color_hint(), "green");
        assert_eq!(ThrashingSeverity::Mild.color_hint(), "yellow");
        assert_eq!(ThrashingSeverity::Moderate.color_hint(), "orange");
        assert_eq!(ThrashingSeverity::Severe.color_hint(), "red");
    }

    #[test]
    fn test_zram_stats_compression_ratio() {
        let stats = ZramStats {
            orig_data_size: 1000,
            compr_data_size: 250,
            ..Default::default()
        };
        assert!((stats.compression_ratio() - 4.0).abs() < 0.001);
        assert!((stats.space_savings_percent() - 75.0).abs() < 0.001);
    }

    #[test]
    fn test_zram_stats_no_data() {
        let stats = ZramStats::default();
        assert!((stats.compression_ratio() - 1.0).abs() < 0.001);
        assert!((stats.space_savings_percent() - 0.0).abs() < 0.001);
        assert!(!stats.is_active());
    }

    #[test]
    fn test_zram_stats_active() {
        let stats = ZramStats {
            orig_data_size: 100,
            compr_data_size: 50,
            ..Default::default()
        };
        assert!(stats.is_active());
    }

    #[test]
    fn test_swap_analyzer_creation() {
        let analyzer = SwapAnalyzer::new();
        assert_eq!(analyzer.detect_thrashing(), ThrashingSeverity::None);
    }

    #[test]
    fn test_swap_analyzer_default() {
        let analyzer = SwapAnalyzer::default();
        assert_eq!(analyzer.detect_thrashing(), ThrashingSeverity::None);
        assert!(!analyzer.has_zram());
        assert!((analyzer.zram_compression_ratio() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_swap_analyzer_rates() {
        let analyzer = SwapAnalyzer::new();
        // Initial rates should be 0 (no data yet)
        assert!(analyzer.swap_rate_per_sec() < 0.001);
        assert!(analyzer.pages_in_rate() < 0.001);
        assert!(analyzer.pages_out_rate() < 0.001);
        assert!(analyzer.major_fault_rate_per_sec() < 0.001);
        assert!(analyzer.minor_fault_rate_per_sec() < 0.001);
    }

    #[test]
    fn test_swap_analyzer_psi_metrics() {
        let analyzer = SwapAnalyzer::new();
        let psi = analyzer.psi();
        // Default PSI should be zeros
        assert!(psi.some_avg10 < 0.001);
        assert!(psi.some_avg60 < 0.001);
        assert!(psi.some_avg300 < 0.001);
        assert!(psi.full_avg10 < 0.001);
        assert!(psi.full_avg60 < 0.001);
        assert!(psi.full_avg300 < 0.001);
    }

    #[test]
    fn test_swap_analyzer_zram_stats() {
        let analyzer = SwapAnalyzer::new();
        assert!(analyzer.zram_stats().is_empty());
    }

    #[test]
    fn test_swap_analyzer_sample_interval() {
        let mut analyzer = SwapAnalyzer::new();
        analyzer.set_sample_interval(2.0);
        // This doesn't change behavior directly, but exercises the method
        assert_eq!(analyzer.detect_thrashing(), ThrashingSeverity::None);
    }

    #[test]
    fn test_swap_analyzer_collect_safe() {
        // Collect should be safe even if system files don't exist
        let mut analyzer = SwapAnalyzer::new();
        analyzer.collect();
        // Should still have default values
        assert_eq!(analyzer.detect_thrashing(), ThrashingSeverity::None);
    }

    #[test]
    fn test_swap_analyzer_fault_history() {
        let analyzer = SwapAnalyzer::new();
        let history = analyzer.fault_history();
        // History should be empty initially
        assert!(history.iter().count() == 0);
    }

    #[test]
    fn test_swap_analyzer_swap_io_history() {
        let analyzer = SwapAnalyzer::new();
        let history = analyzer.swap_io_history();
        // History should be empty initially
        assert!(history.is_empty());
    }

    #[test]
    fn test_psi_metrics_default() {
        let psi = PsiMetrics::default();
        assert_eq!(psi.some_total_us, 0);
        assert_eq!(psi.full_total_us, 0);
    }
}
