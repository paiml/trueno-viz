//! Extended process information - cgroups, FDs, CPU history.
//!
//! Provides additional process metadata beyond basic ProcessInfo.

use std::collections::HashMap;

#[cfg(target_os = "linux")]
use std::fs;

/// Extended process information
#[derive(Debug, Clone, Default)]
pub struct ProcessExtra {
    /// Container/cgroup name (Docker, Podman, systemd slice)
    pub container: Option<String>,
    /// Open file descriptor count
    pub fd_count: u32,
    /// File descriptor limit (ulimit)
    pub fd_limit: u32,
    /// CPU usage history (last N samples)
    pub cpu_history: Vec<f64>,
    /// Parent process chain (for ancestry)
    pub ancestors: Vec<(u32, String)>, // (pid, name)
}

impl ProcessExtra {
    /// Get container badge string
    pub fn container_badge(&self) -> Option<String> {
        self.container.as_ref().map(|c| {
            if c.len() > 12 {
                format!("[{}…]", &c[..11])
            } else {
                format!("[{}]", c)
            }
        })
    }

    /// Check if FD count is near limit (>= 80%)
    pub fn fd_warning(&self) -> bool {
        self.fd_limit > 0 && self.fd_count as f64 / self.fd_limit as f64 >= 0.8
    }

    /// Get FD usage percentage
    pub fn fd_percent(&self) -> f64 {
        if self.fd_limit > 0 {
            (self.fd_count as f64 / self.fd_limit as f64) * 100.0
        } else {
            0.0
        }
    }
}

/// Analyzer for extended process information
pub struct ProcessExtraAnalyzer {
    /// Extended info per PID
    extras: HashMap<u32, ProcessExtra>,
    /// CPU history length
    history_len: usize,
}

impl ProcessExtraAnalyzer {
    pub fn new() -> Self {
        Self {
            extras: HashMap::new(),
            history_len: 60, // 60 samples = 1 minute at 1Hz
        }
    }

    /// Collect extended info for all processes
    #[cfg(target_os = "linux")]
    pub fn collect(&mut self, pids: &[u32], cpu_percents: &HashMap<u32, f64>) {
        // Clean up dead processes
        self.extras.retain(|pid, _| pids.contains(pid));

        for &pid in pids {
            let extra = self.extras.entry(pid).or_default();

            // Update container info
            extra.container = Self::get_container_name(pid);

            // Update FD count
            let (fd_count, fd_limit) = Self::get_fd_info(pid);
            extra.fd_count = fd_count;
            extra.fd_limit = fd_limit;

            // Update CPU history
            if let Some(&cpu) = cpu_percents.get(&pid) {
                extra.cpu_history.push(cpu);
                if extra.cpu_history.len() > self.history_len {
                    extra.cpu_history.remove(0);
                }
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub fn collect(&mut self, pids: &[u32], cpu_percents: &HashMap<u32, f64>) {
        // Clean up dead processes
        self.extras.retain(|pid, _| pids.contains(pid));

        for &pid in pids {
            let extra = self.extras.entry(pid).or_default();

            // Update CPU history only on non-Linux
            if let Some(&cpu) = cpu_percents.get(&pid) {
                extra.cpu_history.push(cpu);
                if extra.cpu_history.len() > self.history_len {
                    extra.cpu_history.remove(0);
                }
            }
        }
    }

    /// Get extended info for a process
    pub fn get(&self, pid: u32) -> Option<&ProcessExtra> {
        self.extras.get(&pid)
    }

    /// Get container/cgroup name from /proc/PID/cgroup
    #[cfg(target_os = "linux")]
    fn get_container_name(pid: u32) -> Option<String> {
        let cgroup_path = format!("/proc/{}/cgroup", pid);
        let content = fs::read_to_string(&cgroup_path).ok()?;

        for line in content.lines() {
            // Format: hierarchy-ID:controller-list:cgroup-path
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() < 3 {
                continue;
            }

            let cgroup_path = parts[2];

            // Docker container
            if cgroup_path.contains("/docker/") {
                if let Some(id) = cgroup_path.split("/docker/").nth(1) {
                    let short_id = &id[..id.len().min(12)];
                    return Some(format!("docker:{}", short_id));
                }
            }

            // Podman container
            if cgroup_path.contains("/libpod-") {
                if let Some(id) = cgroup_path.split("/libpod-").nth(1) {
                    let clean_id = id.split('.').next().unwrap_or(id);
                    let short_id = &clean_id[..clean_id.len().min(12)];
                    return Some(format!("podman:{}", short_id));
                }
            }

            // Kubernetes pod
            if cgroup_path.contains("/kubepods/") {
                // Try to extract pod name
                if let Some(pod_part) = cgroup_path.split("/kubepods/").nth(1) {
                    let parts: Vec<&str> = pod_part.split('/').collect();
                    if parts.len() >= 2 {
                        return Some(format!("k8s:{}", parts[1].chars().take(12).collect::<String>()));
                    }
                }
            }

            // systemd user slice
            if cgroup_path.contains(".slice") {
                let slice_parts: Vec<&str> = cgroup_path
                    .split('/')
                    .filter(|s| s.ends_with(".slice") || s.ends_with(".service"))
                    .collect();
                let slice_name = slice_parts.last().copied();
                if let Some(name) = slice_name {
                    let clean_name = name
                        .trim_end_matches(".slice")
                        .trim_end_matches(".service")
                        .trim_start_matches("user@")
                        .trim_start_matches("user-");
                    if !clean_name.is_empty() && clean_name != "user" && clean_name != "system" {
                        return Some(clean_name.chars().take(15).collect());
                    }
                }
            }
        }

        None
    }

    /// Get FD count and limit from /proc/PID/fd and /proc/PID/limits
    #[cfg(target_os = "linux")]
    fn get_fd_info(pid: u32) -> (u32, u32) {
        let fd_path = format!("/proc/{}/fd", pid);
        let fd_count = fs::read_dir(&fd_path)
            .map(|d| d.count() as u32)
            .unwrap_or(0);

        let limits_path = format!("/proc/{}/limits", pid);
        let fd_limit = fs::read_to_string(&limits_path)
            .ok()
            .and_then(|content| {
                for line in content.lines() {
                    if line.starts_with("Max open files") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        // Format: "Max open files            1024                 1048576              files"
                        if parts.len() >= 5 {
                            return parts[3].parse().ok();
                        }
                    }
                }
                None
            })
            .unwrap_or(0);

        (fd_count, fd_limit)
    }

    /// Build process ancestry chain
    #[cfg(target_os = "linux")]
    pub fn build_ancestry(&mut self, pid: u32, processes: &HashMap<u32, (u32, String)>) {
        if let Some(extra) = self.extras.get_mut(&pid) {
            extra.ancestors.clear();

            let mut current_pid = pid;
            let mut depth = 0;
            const MAX_DEPTH: usize = 10;

            while depth < MAX_DEPTH {
                if let Some((ppid, name)) = processes.get(&current_pid) {
                    if *ppid == 0 || *ppid == current_pid {
                        break;
                    }
                    extra.ancestors.push((*ppid, name.clone()));
                    current_pid = *ppid;
                    depth += 1;
                } else {
                    break;
                }
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub fn build_ancestry(&mut self, _pid: u32, _processes: &HashMap<u32, (u32, String)>) {
        // Not implemented on non-Linux
    }
}

impl Default for ProcessExtraAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_extra_default() {
        let extra = ProcessExtra::default();
        assert!(extra.container.is_none());
        assert_eq!(extra.fd_count, 0);
        assert!(extra.cpu_history.is_empty());
    }

    #[test]
    fn test_container_badge() {
        let mut extra = ProcessExtra::default();
        extra.container = Some("nginx".to_string());
        assert_eq!(extra.container_badge(), Some("[nginx]".to_string()));

        extra.container = Some("very-long-container-name".to_string());
        assert_eq!(extra.container_badge(), Some("[very-long-c…]".to_string()));
    }

    #[test]
    fn test_fd_warning() {
        let mut extra = ProcessExtra::default();
        extra.fd_count = 800;
        extra.fd_limit = 1000;
        assert!(extra.fd_warning()); // 80% = warning

        extra.fd_count = 500;
        assert!(!extra.fd_warning()); // 50% = ok
    }

    #[test]
    fn test_fd_percent() {
        let mut extra = ProcessExtra::default();
        extra.fd_count = 250;
        extra.fd_limit = 1000;
        assert!((extra.fd_percent() - 25.0).abs() < 0.01);
    }

    #[test]
    fn test_analyzer_creation() {
        let analyzer = ProcessExtraAnalyzer::new();
        assert!(analyzer.get(1).is_none());
    }

    #[test]
    fn test_cpu_history_tracking() {
        let mut analyzer = ProcessExtraAnalyzer::new();
        let pids = vec![1234];
        let mut cpu_percents = HashMap::new();
        cpu_percents.insert(1234, 50.0);

        analyzer.collect(&pids, &cpu_percents);

        let extra = analyzer.get(1234).unwrap();
        assert_eq!(extra.cpu_history.len(), 1);
        assert!((extra.cpu_history[0] - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_container_badge_none() {
        let extra = ProcessExtra::default();
        assert_eq!(extra.container_badge(), None);
    }

    #[test]
    fn test_container_badge_exact_12() {
        let mut extra = ProcessExtra::default();
        extra.container = Some("exactly12chr".to_string());
        assert_eq!(extra.container_badge(), Some("[exactly12chr]".to_string()));
    }

    #[test]
    fn test_fd_warning_zero_limit() {
        let mut extra = ProcessExtra::default();
        extra.fd_count = 100;
        extra.fd_limit = 0;
        assert!(!extra.fd_warning()); // 0 limit = no warning
    }

    #[test]
    fn test_fd_percent_zero_limit() {
        let mut extra = ProcessExtra::default();
        extra.fd_count = 100;
        extra.fd_limit = 0;
        assert_eq!(extra.fd_percent(), 0.0);
    }

    #[test]
    fn test_fd_percent_100() {
        let mut extra = ProcessExtra::default();
        extra.fd_count = 1000;
        extra.fd_limit = 1000;
        assert!((extra.fd_percent() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_fd_warning_at_boundary() {
        let mut extra = ProcessExtra::default();
        extra.fd_limit = 1000;

        // Exactly 80% = warning
        extra.fd_count = 800;
        assert!(extra.fd_warning());

        // Just under 80% = no warning
        extra.fd_count = 799;
        assert!(!extra.fd_warning());
    }

    #[test]
    fn test_analyzer_dead_process_cleanup() {
        let mut analyzer = ProcessExtraAnalyzer::new();

        // Add process 1234
        let pids = vec![1234];
        let mut cpu_percents = HashMap::new();
        cpu_percents.insert(1234, 50.0);
        analyzer.collect(&pids, &cpu_percents);
        assert!(analyzer.get(1234).is_some());

        // Collect with empty list - process should be cleaned up
        analyzer.collect(&[], &HashMap::new());
        assert!(analyzer.get(1234).is_none());
    }

    #[test]
    fn test_analyzer_multiple_processes() {
        let mut analyzer = ProcessExtraAnalyzer::new();
        let pids = vec![100, 200, 300];
        let mut cpu_percents = HashMap::new();
        cpu_percents.insert(100, 10.0);
        cpu_percents.insert(200, 20.0);
        cpu_percents.insert(300, 30.0);

        analyzer.collect(&pids, &cpu_percents);

        assert!(analyzer.get(100).is_some());
        assert!(analyzer.get(200).is_some());
        assert!(analyzer.get(300).is_some());
        assert!(analyzer.get(400).is_none());
    }

    #[test]
    fn test_process_extra_ancestors() {
        let mut extra = ProcessExtra::default();
        extra.ancestors = vec![
            (1, "init".to_string()),
            (100, "systemd".to_string()),
        ];
        assert_eq!(extra.ancestors.len(), 2);
        assert_eq!(extra.ancestors[0].0, 1);
        assert_eq!(extra.ancestors[0].1, "init");
    }

    #[test]
    fn test_cpu_history_no_percent() {
        let mut analyzer = ProcessExtraAnalyzer::new();
        let pids = vec![1234];
        let cpu_percents: HashMap<u32, f64> = HashMap::new(); // No CPU percent for this PID

        analyzer.collect(&pids, &cpu_percents);

        let extra = analyzer.get(1234).unwrap();
        assert!(extra.cpu_history.is_empty()); // No history added
    }

    #[test]
    fn test_analyzer_default() {
        let analyzer = ProcessExtraAnalyzer::default();
        assert!(analyzer.get(1).is_none());
    }

    #[test]
    fn test_cpu_history_limit() {
        let mut analyzer = ProcessExtraAnalyzer::new();
        let pids = vec![1234];

        // Add 100 samples (more than the 60 limit)
        for i in 0..100 {
            let mut cpu_percents = HashMap::new();
            cpu_percents.insert(1234, i as f64);
            analyzer.collect(&pids, &cpu_percents);
        }

        let extra = analyzer.get(1234).unwrap();
        // History should be limited to history_len (60)
        assert!(extra.cpu_history.len() <= 60);
    }

    #[test]
    fn test_process_extra_all_fields() {
        let extra = ProcessExtra {
            container: Some("docker-abc123".to_string()),
            fd_count: 500,
            fd_limit: 1024,
            cpu_history: vec![10.0, 20.0, 30.0],
            ancestors: vec![(1, "init".to_string())],
        };

        assert_eq!(extra.container, Some("docker-abc123".to_string()));
        assert_eq!(extra.fd_count, 500);
        assert_eq!(extra.fd_limit, 1024);
        assert_eq!(extra.cpu_history.len(), 3);
        assert_eq!(extra.ancestors.len(), 1);
    }

    #[test]
    fn test_container_badge_short() {
        let mut extra = ProcessExtra::default();
        extra.container = Some("short".to_string());
        assert_eq!(extra.container_badge(), Some("[short]".to_string()));
    }

    #[test]
    fn test_container_badge_long() {
        let mut extra = ProcessExtra::default();
        extra.container = Some("verylongcontainername123".to_string());
        let badge = extra.container_badge().unwrap();
        assert!(badge.contains("…")); // Should be truncated
        // Format: [11chars…] = 14 chars max
    }

    #[test]
    fn test_fd_percent_50() {
        let mut extra = ProcessExtra::default();
        extra.fd_count = 500;
        extra.fd_limit = 1000;
        assert!((extra.fd_percent() - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_fd_warning_under_threshold() {
        let mut extra = ProcessExtra::default();
        extra.fd_count = 700;
        extra.fd_limit = 1000;
        assert!(!extra.fd_warning()); // 70% < 80%
    }

    #[test]
    fn test_process_extra_clone() {
        let extra = ProcessExtra {
            container: Some("test".to_string()),
            fd_count: 100,
            fd_limit: 1000,
            cpu_history: vec![1.0, 2.0],
            ancestors: vec![(1, "init".to_string())],
        };

        let cloned = extra.clone();
        assert_eq!(cloned.container, extra.container);
        assert_eq!(cloned.fd_count, extra.fd_count);
        assert_eq!(cloned.cpu_history.len(), extra.cpu_history.len());
    }

    #[test]
    fn test_analyzer_get_nonexistent() {
        let analyzer = ProcessExtraAnalyzer::new();
        assert!(analyzer.get(99999).is_none());
    }
}
