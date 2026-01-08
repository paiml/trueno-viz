//! Container/Docker Analyzer
//!
//! Monitors running Docker containers and their resource usage.

use std::process::Command;
use std::time::{Duration, Instant};

/// Container statistics
#[derive(Debug, Clone)]
pub struct ContainerStats {
    /// Container name
    pub name: String,
    /// CPU usage percentage
    pub cpu_pct: f64,
    /// Memory usage in bytes
    pub mem_used: u64,
    /// Memory limit in bytes
    pub mem_limit: u64,
    /// Memory percentage
    pub mem_pct: f64,
    /// Container status (running, paused, etc.)
    pub status: ContainerStatus,
}

/// Container status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerStatus {
    Running,
    Paused,
    Restarting,
    Exited,
    Unknown,
}

impl ContainerStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            ContainerStatus::Running => "▶",
            ContainerStatus::Paused => "⏸",
            ContainerStatus::Restarting => "↻",
            ContainerStatus::Exited => "□",
            ContainerStatus::Unknown => "?",
        }
    }
}

/// Container analyzer
pub struct ContainerAnalyzer {
    containers: Vec<ContainerStats>,
    last_collect: Instant,
    available: bool,
    total_count: usize,
    running_count: usize,
}

impl ContainerAnalyzer {
    pub fn new() -> Self {
        // Check if docker is available
        let available = Command::new("docker")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        Self {
            containers: Vec::new(),
            last_collect: Instant::now() - Duration::from_secs(10),
            available,
            total_count: 0,
            running_count: 0,
        }
    }

    /// Collect container stats
    pub fn collect(&mut self) {
        if !self.available {
            return;
        }

        // Collect at most once every 2 seconds (docker stats is slow)
        if self.last_collect.elapsed() < Duration::from_secs(2) {
            return;
        }
        self.last_collect = Instant::now();

        // Get container count first (fast)
        if let Ok(output) = Command::new("docker")
            .args(["ps", "-a", "--format", "{{.Status}}"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = stdout.lines().collect();
                self.total_count = lines.len();
                self.running_count = lines.iter().filter(|l| l.starts_with("Up")).count();
            }
        }

        // Get stats for running containers
        let output = match Command::new("docker")
            .args([
                "stats",
                "--no-stream",
                "--format",
                "{{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.MemPerc}}",
            ])
            .output()
        {
            Ok(o) => o,
            Err(_) => return,
        };

        if !output.status.success() {
            return;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        self.containers = parse_docker_stats(&stdout);

        // Sort by CPU usage descending
        self.containers.sort_by(|a, b| {
            b.cpu_pct
                .partial_cmp(&a.cpu_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Check if Docker is available
    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Get container stats
    pub fn containers(&self) -> &[ContainerStats] {
        &self.containers
    }

    /// Get top N containers by CPU usage
    pub fn top_containers(&self, n: usize) -> Vec<&ContainerStats> {
        self.containers.iter().take(n).collect()
    }

    /// Get total container count
    pub fn total_count(&self) -> usize {
        self.total_count
    }

    /// Get running container count
    pub fn running_count(&self) -> usize {
        self.running_count
    }
}

impl Default for ContainerAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse docker stats output
fn parse_docker_stats(output: &str) -> Vec<ContainerStats> {
    let mut containers = Vec::new();

    for line in output.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 4 {
            continue;
        }

        let name = parts[0].to_string();

        // Parse CPU percentage (e.g., "0.07%")
        let cpu_pct = parts[1]
            .trim_end_matches('%')
            .parse::<f64>()
            .unwrap_or(0.0);

        // Parse memory usage (e.g., "2.352MiB / 125.3GiB")
        let (mem_used, mem_limit) = parse_mem_usage(parts[2]);

        // Parse memory percentage
        let mem_pct = parts[3]
            .trim_end_matches('%')
            .parse::<f64>()
            .unwrap_or(0.0);

        containers.push(ContainerStats {
            name,
            cpu_pct,
            mem_used,
            mem_limit,
            mem_pct,
            status: ContainerStatus::Running,
        });
    }

    containers
}

/// Parse memory usage string like "2.352MiB / 125.3GiB"
fn parse_mem_usage(s: &str) -> (u64, u64) {
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() != 2 {
        return (0, 0);
    }

    let used = parse_mem_value(parts[0].trim());
    let limit = parse_mem_value(parts[1].trim());
    (used, limit)
}

/// Parse memory value like "2.352MiB" or "125.3GiB"
fn parse_mem_value(s: &str) -> u64 {
    let s = s.trim();

    // Find where the number ends
    let num_end = s.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(s.len());
    let (num_str, unit) = s.split_at(num_end);

    let value: f64 = num_str.parse().unwrap_or(0.0);
    let unit = unit.trim();

    let multiplier: f64 = match unit {
        "B" => 1.0,
        "KiB" | "kB" => 1024.0,
        "MiB" | "MB" => 1024.0 * 1024.0,
        "GiB" | "GB" => 1024.0 * 1024.0 * 1024.0,
        "TiB" | "TB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => 1.0,
    };

    (value * multiplier) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mem_value() {
        assert_eq!(parse_mem_value("100B"), 100);
        assert_eq!(parse_mem_value("1KiB"), 1024);
        assert_eq!(parse_mem_value("1MiB"), 1024 * 1024);
        assert_eq!(parse_mem_value("1GiB"), 1024 * 1024 * 1024);
        assert_eq!(parse_mem_value("2.5MiB"), (2.5 * 1024.0 * 1024.0) as u64);
    }

    #[test]
    fn test_parse_mem_usage() {
        let (used, limit) = parse_mem_usage("2.352MiB / 125.3GiB");
        assert!(used > 2 * 1024 * 1024); // > 2 MiB
        assert!(limit > 100 * 1024 * 1024 * 1024); // > 100 GiB
    }

    #[test]
    fn test_parse_docker_stats() {
        let output = "duende-test\t0.07%\t2.352MiB / 125.3GiB\t0.00%\n";
        let containers = parse_docker_stats(output);

        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0].name, "duende-test");
        assert!((containers[0].cpu_pct - 0.07).abs() < 0.01);
    }

    #[test]
    fn test_container_status_symbols() {
        assert_eq!(ContainerStatus::Running.symbol(), "▶");
        assert_eq!(ContainerStatus::Paused.symbol(), "⏸");
        assert_eq!(ContainerStatus::Exited.symbol(), "□");
    }

    #[test]
    fn test_analyzer_creation() {
        let analyzer = ContainerAnalyzer::new();
        // Should not panic
        let _ = analyzer.is_available();
    }

    #[test]
    fn test_empty_output() {
        let containers = parse_docker_stats("");
        assert!(containers.is_empty());
    }

    #[test]
    fn test_multiple_containers() {
        let output = "web\t5.20%\t512MiB / 8GiB\t6.25%\n\
                      db\t2.10%\t1GiB / 4GiB\t25.00%\n\
                      cache\t0.50%\t256MiB / 1GiB\t25.00%\n";
        let containers = parse_docker_stats(output);

        assert_eq!(containers.len(), 3);
        assert_eq!(containers[0].name, "web");
        assert_eq!(containers[1].name, "db");
        assert_eq!(containers[2].name, "cache");
    }
}
