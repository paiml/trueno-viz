//! PSI (Pressure Stall Information) Analyzer
//!
//! Reads Linux PSI metrics from /proc/pressure/* to detect system resource pressure.
//! PSI provides insight into resource contention before it causes visible problems.

use std::fs;
use std::time::{Duration, Instant};

/// PSI metrics for a single resource (CPU, memory, or I/O)
#[derive(Debug, Clone, Copy, Default)]
pub struct PsiMetrics {
    /// Percentage of time some tasks were stalled (avg over 10s)
    pub some_avg10: f64,
    /// Percentage of time some tasks were stalled (avg over 60s)
    pub some_avg60: f64,
    /// Percentage of time ALL tasks were stalled (avg over 10s)
    pub full_avg10: f64,
    /// Percentage of time ALL tasks were stalled (avg over 60s)
    pub full_avg60: f64,
}

/// Pressure level classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressureLevel {
    /// No significant pressure (<5%)
    None,
    /// Low pressure (5-15%)
    Low,
    /// Medium pressure (15-40%)
    Medium,
    /// High pressure (40-70%)
    High,
    /// Critical pressure (>70%)
    Critical,
}

impl PressureLevel {
    pub fn from_pct(pct: f64) -> Self {
        if pct < 5.0 {
            PressureLevel::None
        } else if pct < 15.0 {
            PressureLevel::Low
        } else if pct < 40.0 {
            PressureLevel::Medium
        } else if pct < 70.0 {
            PressureLevel::High
        } else {
            PressureLevel::Critical
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            PressureLevel::None => "○",
            PressureLevel::Low => "◔",
            PressureLevel::Medium => "◑",
            PressureLevel::High => "◕",
            PressureLevel::Critical => "●",
        }
    }
}

/// PSI Analyzer - monitors system pressure
pub struct PsiAnalyzer {
    pub cpu: PsiMetrics,
    pub memory: PsiMetrics,
    pub io: PsiMetrics,
    last_collect: Instant,
    available: bool,
}

impl PsiAnalyzer {
    pub fn new() -> Self {
        let available = fs::metadata("/proc/pressure/cpu").is_ok();
        Self {
            cpu: PsiMetrics::default(),
            memory: PsiMetrics::default(),
            io: PsiMetrics::default(),
            last_collect: Instant::now() - Duration::from_secs(10),
            available,
        }
    }

    /// Collect PSI metrics
    pub fn collect(&mut self) {
        if !self.available {
            return;
        }

        // Collect at most once per second
        if self.last_collect.elapsed() < Duration::from_secs(1) {
            return;
        }
        self.last_collect = Instant::now();

        self.cpu = parse_psi_file("/proc/pressure/cpu");
        self.memory = parse_psi_file("/proc/pressure/memory");
        self.io = parse_psi_file("/proc/pressure/io");
    }

    /// Check if PSI is available
    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Get CPU pressure level (based on some_avg10)
    pub fn cpu_level(&self) -> PressureLevel {
        PressureLevel::from_pct(self.cpu.some_avg10)
    }

    /// Get memory pressure level (based on some_avg10)
    pub fn memory_level(&self) -> PressureLevel {
        PressureLevel::from_pct(self.memory.some_avg10)
    }

    /// Get I/O pressure level (based on some_avg10)
    pub fn io_level(&self) -> PressureLevel {
        PressureLevel::from_pct(self.io.some_avg10)
    }

    /// Get overall system pressure (max of all)
    pub fn overall_level(&self) -> PressureLevel {
        let max_pct = self.cpu.some_avg10
            .max(self.memory.some_avg10)
            .max(self.io.some_avg10);
        PressureLevel::from_pct(max_pct)
    }
}

impl Default for PsiAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a PSI file (/proc/pressure/*)
fn parse_psi_file(path: &str) -> PsiMetrics {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return PsiMetrics::default(),
    };

    let mut metrics = PsiMetrics::default();

    for line in content.lines() {
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
            if let Some((key, val)) = part.split_once('=') {
                let value: f64 = val.parse().unwrap_or(0.0);
                match (is_some, key) {
                    (true, "avg10") => metrics.some_avg10 = value,
                    (true, "avg60") => metrics.some_avg60 = value,
                    (false, "avg10") => metrics.full_avg10 = value,
                    (false, "avg60") => metrics.full_avg60 = value,
                    _ => {}
                }
            }
        }
    }

    metrics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pressure_level_from_pct() {
        assert_eq!(PressureLevel::from_pct(0.0), PressureLevel::None);
        assert_eq!(PressureLevel::from_pct(4.9), PressureLevel::None);
        assert_eq!(PressureLevel::from_pct(5.0), PressureLevel::Low);
        assert_eq!(PressureLevel::from_pct(14.9), PressureLevel::Low);
        assert_eq!(PressureLevel::from_pct(15.0), PressureLevel::Medium);
        assert_eq!(PressureLevel::from_pct(39.9), PressureLevel::Medium);
        assert_eq!(PressureLevel::from_pct(40.0), PressureLevel::High);
        assert_eq!(PressureLevel::from_pct(69.9), PressureLevel::High);
        assert_eq!(PressureLevel::from_pct(70.0), PressureLevel::Critical);
        assert_eq!(PressureLevel::from_pct(100.0), PressureLevel::Critical);
    }

    #[test]
    fn test_pressure_level_symbols() {
        assert_eq!(PressureLevel::None.symbol(), "○");
        assert_eq!(PressureLevel::Low.symbol(), "◔");
        assert_eq!(PressureLevel::Medium.symbol(), "◑");
        assert_eq!(PressureLevel::High.symbol(), "◕");
        assert_eq!(PressureLevel::Critical.symbol(), "●");
    }

    #[test]
    fn test_parse_psi_content() {
        // Test with sample PSI content
        let metrics = parse_psi_file("/proc/pressure/cpu");
        // Just verify it doesn't panic - actual values depend on system state
        let _ = metrics.some_avg10;
    }

    #[test]
    fn test_psi_analyzer_creation() {
        let analyzer = PsiAnalyzer::new();
        // Should not panic
        let _ = analyzer.is_available();
    }

    #[test]
    fn test_psi_levels() {
        let mut analyzer = PsiAnalyzer::new();
        analyzer.cpu.some_avg10 = 25.0;
        analyzer.memory.some_avg10 = 50.0;
        analyzer.io.some_avg10 = 5.0;

        assert_eq!(analyzer.cpu_level(), PressureLevel::Medium);
        assert_eq!(analyzer.memory_level(), PressureLevel::High);
        assert_eq!(analyzer.io_level(), PressureLevel::Low);
        assert_eq!(analyzer.overall_level(), PressureLevel::High);
    }
}
