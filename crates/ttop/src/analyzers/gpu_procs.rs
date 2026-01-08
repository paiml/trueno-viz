//! GPU Process Analyzer - tracks processes using GPU resources
//!
//! Uses nvidia-smi pmon to collect GPU process metrics on NVIDIA GPUs.

use std::process::Command;
use std::time::{Duration, Instant};

/// A process using GPU resources
#[derive(Debug, Clone)]
pub struct GpuProcess {
    /// GPU index
    pub gpu_idx: u32,
    /// Process ID
    pub pid: u32,
    /// Process type: Compute or Graphics
    pub proc_type: GpuProcType,
    /// SM (shader) utilization percentage
    pub sm_util: u8,
    /// Memory utilization percentage
    pub mem_util: u8,
    /// Command name
    pub command: String,
}

/// GPU process type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuProcType {
    Compute,
    Graphics,
}

impl std::fmt::Display for GpuProcType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuProcType::Compute => write!(f, "C"),
            GpuProcType::Graphics => write!(f, "G"),
        }
    }
}

/// Analyzer for GPU processes
pub struct GpuProcessAnalyzer {
    processes: Vec<GpuProcess>,
    last_collect: Instant,
    available: bool,
}

impl GpuProcessAnalyzer {
    pub fn new() -> Self {
        // Check if nvidia-smi is available
        let available = Command::new("nvidia-smi")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        Self {
            processes: Vec::new(),
            last_collect: Instant::now() - Duration::from_secs(10),
            available,
        }
    }

    /// Collect GPU process information
    pub fn collect(&mut self) {
        if !self.available {
            return;
        }

        // Collect at most once per second
        if self.last_collect.elapsed() < Duration::from_secs(1) {
            return;
        }
        self.last_collect = Instant::now();

        // Run nvidia-smi pmon with single sample
        let output = match Command::new("nvidia-smi")
            .args(["pmon", "-c", "1"])
            .output()
        {
            Ok(o) => o,
            Err(_) => return,
        };

        if !output.status.success() {
            return;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        self.processes = parse_pmon_output(&stdout);
    }

    /// Get current GPU processes, sorted by SM utilization
    pub fn processes(&self) -> &[GpuProcess] {
        &self.processes
    }

    /// Check if GPU process monitoring is available
    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Get top N processes by SM utilization
    pub fn top_processes(&self, n: usize) -> Vec<&GpuProcess> {
        let mut sorted: Vec<_> = self.processes.iter().collect();
        sorted.sort_by(|a, b| b.sm_util.cmp(&a.sm_util));
        sorted.truncate(n);
        sorted
    }
}

impl Default for GpuProcessAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse nvidia-smi pmon output
fn parse_pmon_output(output: &str) -> Vec<GpuProcess> {
    let mut processes = Vec::new();

    for line in output.lines() {
        // Skip header lines (start with #)
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 10 {
            continue;
        }

        // Parse fields: gpu pid type sm mem enc dec jpg ofa command
        let gpu_idx = match parts[0].parse::<u32>() {
            Ok(v) => v,
            Err(_) => continue,
        };

        let pid = match parts[1].parse::<u32>() {
            Ok(v) => v,
            Err(_) => continue,
        };

        let proc_type = match parts[2] {
            "C" => GpuProcType::Compute,
            "G" => GpuProcType::Graphics,
            _ => continue,
        };

        // SM and mem utilization (may be "-" if not available)
        let sm_util = parts[3].parse::<u8>().unwrap_or(0);
        let mem_util = parts[4].parse::<u8>().unwrap_or(0);

        // Command is the last field
        let command = parts[9].to_string();

        processes.push(GpuProcess {
            gpu_idx,
            pid,
            proc_type,
            sm_util,
            mem_util,
            command,
        });
    }

    // Sort by SM utilization descending
    processes.sort_by(|a, b| b.sm_util.cmp(&a.sm_util));
    processes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pmon_empty() {
        let output = "";
        let procs = parse_pmon_output(output);
        assert!(procs.is_empty());
    }

    #[test]
    fn test_parse_pmon_headers_only() {
        let output = r#"# gpu         pid   type     sm    mem    enc    dec    jpg    ofa    command
# Idx           #    C/G      %      %      %      %      %      %    name
"#;
        let procs = parse_pmon_output(output);
        assert!(procs.is_empty());
    }

    #[test]
    fn test_parse_pmon_with_processes() {
        let output = r#"# gpu         pid   type     sm    mem    enc    dec    jpg    ofa    command
# Idx           #    C/G      %      %      %      %      %      %    name
    0       2584     G     11      3      -      -      -      -    Xorg
    0       3056     G      8      2      -      -      -      -    gnome-shell
"#;
        let procs = parse_pmon_output(output);
        assert_eq!(procs.len(), 2);

        // Should be sorted by SM util descending
        assert_eq!(procs[0].command, "Xorg");
        assert_eq!(procs[0].sm_util, 11);
        assert_eq!(procs[0].proc_type, GpuProcType::Graphics);

        assert_eq!(procs[1].command, "gnome-shell");
        assert_eq!(procs[1].sm_util, 8);
    }

    #[test]
    fn test_parse_pmon_compute_process() {
        let output = r#"# header
    0       1234     C     50     30      -      -      -      -    python
"#;
        let procs = parse_pmon_output(output);
        assert_eq!(procs.len(), 1);
        assert_eq!(procs[0].proc_type, GpuProcType::Compute);
        assert_eq!(procs[0].command, "python");
        assert_eq!(procs[0].sm_util, 50);
    }

    #[test]
    fn test_gpu_proc_type_display() {
        assert_eq!(format!("{}", GpuProcType::Compute), "C");
        assert_eq!(format!("{}", GpuProcType::Graphics), "G");
    }

    #[test]
    fn test_analyzer_creation() {
        let analyzer = GpuProcessAnalyzer::new();
        // Should not panic even if nvidia-smi isn't available
        let _ = analyzer.is_available();
    }

    #[test]
    fn test_top_processes() {
        let mut analyzer = GpuProcessAnalyzer::new();
        analyzer.processes = vec![
            GpuProcess { gpu_idx: 0, pid: 1, proc_type: GpuProcType::Graphics, sm_util: 10, mem_util: 5, command: "a".into() },
            GpuProcess { gpu_idx: 0, pid: 2, proc_type: GpuProcType::Graphics, sm_util: 30, mem_util: 5, command: "b".into() },
            GpuProcess { gpu_idx: 0, pid: 3, proc_type: GpuProcType::Compute, sm_util: 20, mem_util: 5, command: "c".into() },
        ];

        let top2 = analyzer.top_processes(2);
        assert_eq!(top2.len(), 2);
        assert_eq!(top2[0].command, "b"); // 30%
        assert_eq!(top2[1].command, "c"); // 20%
    }
}
