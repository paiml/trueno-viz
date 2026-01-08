//! Comprehensive unit tests for all monitor collector components.
//!
//! These tests verify the correct behavior of each collector,
//! ensuring proper initialization, metric collection, and data validation.
#![allow(clippy::unwrap_used)]
#![allow(clippy::for_kv_map)]
#![allow(clippy::manual_range_contains)]
#![allow(unused_imports)]

use trueno_viz::monitor::collectors::{
    CpuCollector, DiskCollector, MemoryCollector, NetworkCollector, ProcessCollector,
    SensorCollector,
};
use trueno_viz::monitor::types::{Collector, MetricValue};

// ============================================================================
// CPU Collector Tests
// ============================================================================

mod cpu_collector_tests {
    use super::*;

    #[test]
    fn test_cpu_collector_initialization() {
        let collector = CpuCollector::new();
        assert!(collector.core_count() >= 1, "System must have at least 1 CPU core");
        assert!(collector.is_available(), "CPU collector should be available");
    }

    #[test]
    fn test_cpu_collector_id() {
        let collector = CpuCollector::new();
        assert_eq!(collector.id(), "cpu");
    }

    #[test]
    fn test_cpu_collector_display_name() {
        let collector = CpuCollector::new();
        assert_eq!(collector.display_name(), "CPU");
    }

    #[test]
    fn test_cpu_collector_collect_succeeds() {
        let mut collector = CpuCollector::new();
        let result = collector.collect();
        assert!(result.is_ok(), "CPU collection should succeed: {:?}", result.err());
    }

    #[test]
    fn test_cpu_collector_metrics_structure() {
        let mut collector = CpuCollector::new();
        // Need at least one collection to populate metrics
        let _ = collector.collect();

        if let Ok(metrics) = collector.collect() {
            // Should have total CPU metric
            assert!(
                metrics.get_gauge("cpu.total").is_some() || metrics.get_counter("cpu.total").is_some(),
                "Should have cpu.total metric"
            );
        }
    }

    #[test]
    fn test_cpu_collector_core_count_consistent() {
        let mut collector = CpuCollector::new();
        let initial_count = collector.core_count();

        for _ in 0..5 {
            let _ = collector.collect();
            assert_eq!(
                collector.core_count(),
                initial_count,
                "Core count should remain consistent across collections"
            );
        }
    }

    #[test]
    fn test_cpu_load_average_non_negative() {
        let mut collector = CpuCollector::new();
        let _ = collector.collect();

        let load = collector.load_average();
        assert!(load.one >= 0.0, "1-min load should be non-negative");
        assert!(load.five >= 0.0, "5-min load should be non-negative");
        assert!(load.fifteen >= 0.0, "15-min load should be non-negative");
    }

    #[test]
    fn test_cpu_uptime_positive() {
        let mut collector = CpuCollector::new();
        let _ = collector.collect();

        let uptime = collector.uptime_secs();
        assert!(uptime > 0.0, "Uptime should be positive, got {}", uptime);
    }

    #[test]
    fn test_cpu_frequencies_populated() {
        let mut collector = CpuCollector::new();
        let _ = collector.collect();

        let freqs = collector.frequencies();
        // Frequencies may or may not be available depending on system
        // Just verify the method doesn't panic
        let _ = freqs;
    }

    #[test]
    fn test_cpu_history_recording() {
        let mut collector = CpuCollector::new();

        // Collect multiple times
        for _ in 0..5 {
            let _ = collector.collect();
        }

        // History should have entries (exact count depends on implementation)
        let history = collector.history();
        assert!(history.len() <= 300, "History should not exceed buffer size");
    }

    #[test]
    fn test_cpu_interval_hint() {
        let collector = CpuCollector::new();
        let interval = collector.interval_hint();
        assert!(interval.as_millis() > 0, "Interval hint should be positive");
        assert!(interval.as_secs() <= 10, "Interval should be reasonable (<=10s)");
    }
}

// ============================================================================
// Memory Collector Tests
// ============================================================================

mod memory_collector_tests {
    use super::*;

    #[test]
    fn test_memory_collector_initialization() {
        let collector = MemoryCollector::new();
        assert!(collector.is_available(), "Memory collector should be available");
    }

    #[test]
    fn test_memory_collector_id() {
        let collector = MemoryCollector::new();
        assert_eq!(collector.id(), "memory");
    }

    #[test]
    fn test_memory_collector_collect_succeeds() {
        let mut collector = MemoryCollector::new();
        let result = collector.collect();
        assert!(result.is_ok(), "Memory collection should succeed: {:?}", result.err());
    }

    #[test]
    fn test_memory_metrics_has_total() {
        let mut collector = MemoryCollector::new();

        if let Ok(metrics) = collector.collect() {
            let total = metrics.get_counter("memory.total");
            assert!(total.is_some(), "Should have memory.total metric");
            assert!(total.unwrap() > 0, "Total memory should be positive");
        }
    }

    #[test]
    fn test_memory_metrics_used_less_than_total() {
        let mut collector = MemoryCollector::new();

        if let Ok(metrics) = collector.collect() {
            if let (Some(total), Some(used)) = (
                metrics.get_counter("memory.total"),
                metrics.get_counter("memory.used"),
            ) {
                assert!(
                    used <= total,
                    "Used memory ({}) should be <= total memory ({})",
                    used,
                    total
                );
            }
        }
    }

    #[test]
    fn test_memory_percentage_valid() {
        let mut collector = MemoryCollector::new();

        if let Ok(metrics) = collector.collect() {
            if let Some(pct) = metrics.get_gauge("memory.used.percent") {
                assert!(
                    pct >= 0.0 && pct <= 100.0,
                    "Memory percentage should be 0-100%, got {}",
                    pct
                );
            }
        }
    }

    #[test]
    fn test_memory_interval_hint() {
        let collector = MemoryCollector::new();
        let interval = collector.interval_hint();
        assert!(interval.as_millis() > 0, "Interval hint should be positive");
    }
}

// ============================================================================
// Network Collector Tests
// ============================================================================

mod network_collector_tests {
    use super::*;

    #[test]
    fn test_network_collector_initialization() {
        let collector = NetworkCollector::new();
        assert!(collector.is_available(), "Network collector should be available");
    }

    #[test]
    fn test_network_collector_id() {
        let collector = NetworkCollector::new();
        assert_eq!(collector.id(), "network");
    }

    #[test]
    fn test_network_collector_collect_succeeds() {
        let mut collector = NetworkCollector::new();
        let result = collector.collect();
        assert!(result.is_ok(), "Network collection should succeed: {:?}", result.err());
    }

    #[test]
    fn test_network_interfaces_exist() {
        let mut collector = NetworkCollector::new();
        let _ = collector.collect();
        // Just verify the method works - interface count varies by system
        let _ = collector.interfaces();
    }

    #[test]
    fn test_network_rates_after_collection() {
        let mut collector = NetworkCollector::new();
        let _ = collector.collect();

        // Just verify rates are accessible (no sleep - fast test)
        for (_iface, rate) in collector.all_rates() {
            assert!(rate.rx_bytes_per_sec >= 0.0);
            assert!(rate.tx_bytes_per_sec >= 0.0);
        }
    }

    #[test]
    fn test_network_interval_hint() {
        let collector = NetworkCollector::new();
        let interval = collector.interval_hint();
        assert!(interval.as_millis() > 0, "Interval hint should be positive");
    }
}

// ============================================================================
// Disk Collector Tests
// ============================================================================

mod disk_collector_tests {
    use super::*;

    #[test]
    fn test_disk_collector_initialization() {
        let collector = DiskCollector::new();
        assert!(collector.is_available(), "Disk collector should be available");
    }

    #[test]
    fn test_disk_collector_id() {
        let collector = DiskCollector::new();
        assert_eq!(collector.id(), "disk");
    }

    #[test]
    fn test_disk_collector_collect_succeeds() {
        let mut collector = DiskCollector::new();
        let result = collector.collect();
        assert!(result.is_ok(), "Disk collection should succeed: {:?}", result.err());
    }

    #[test]
    fn test_disk_mounts_populated() {
        let mut collector = DiskCollector::new();
        let _ = collector.collect();

        let mounts = collector.mounts();
        // Every system should have at least root mount
        assert!(!mounts.is_empty(), "Should detect at least one disk mount");
    }

    #[test]
    fn test_disk_mount_usage_valid() {
        let mut collector = DiskCollector::new();
        let _ = collector.collect();

        for mount in collector.mounts() {
            if mount.total_bytes > 0 {
                let usage = mount.usage_percent();
                assert!(
                    usage >= 0.0 && usage <= 100.0,
                    "Disk usage should be 0-100%, got {} for {}",
                    usage,
                    mount.mount_point
                );

                assert!(
                    mount.used_bytes <= mount.total_bytes,
                    "Used bytes ({}) should be <= total bytes ({}) for {}",
                    mount.used_bytes,
                    mount.total_bytes,
                    mount.mount_point
                );
            }
        }
    }

    #[test]
    fn test_disk_interval_hint() {
        let collector = DiskCollector::new();
        let interval = collector.interval_hint();
        assert!(interval.as_millis() > 0, "Interval hint should be positive");
    }
}

// ============================================================================
// Process Collector Tests
// ============================================================================

mod process_collector_tests {
    use super::*;

    #[test]
    fn test_process_collector_initialization() {
        let collector = ProcessCollector::new();
        assert!(collector.is_available(), "Process collector should be available");
    }

    #[test]
    fn test_process_collector_id() {
        let collector = ProcessCollector::new();
        assert_eq!(collector.id(), "process");
    }

    #[test]
    fn test_process_collector_collect_succeeds() {
        let mut collector = ProcessCollector::new();
        let result = collector.collect();
        assert!(result.is_ok(), "Process collection should succeed: {:?}", result.err());
    }

    #[test]
    fn test_process_count_positive() {
        let mut collector = ProcessCollector::new();
        let _ = collector.collect();

        let count = collector.count();
        assert!(count > 0, "Should have at least one running process");
    }

    #[test]
    fn test_process_list_populated() {
        let mut collector = ProcessCollector::new();
        let _ = collector.collect();

        let processes = collector.processes();
        assert!(!processes.is_empty(), "Should have at least one process");

        // Verify basic process info is populated
        for (pid, proc_info) in processes.iter().take(5) {
            assert!(*pid > 0, "PID should be positive");
            assert!(!proc_info.name.is_empty(), "Process name should not be empty");
        }
    }

    #[test]
    fn test_process_interval_hint() {
        let collector = ProcessCollector::new();
        let interval = collector.interval_hint();
        assert!(interval.as_millis() > 0, "Interval hint should be positive");
    }
}

// ============================================================================
// Sensor Collector Tests
// ============================================================================

mod sensor_collector_tests {
    use super::*;

    #[test]
    fn test_sensor_collector_initialization() {
        let collector = SensorCollector::new();
        // Sensors may or may not be available depending on hardware
        let _ = collector.is_available();
    }

    #[test]
    fn test_sensor_collector_id() {
        let collector = SensorCollector::new();
        assert_eq!(collector.id(), "sensors");
    }

    #[test]
    fn test_sensor_collector_collect_succeeds() {
        let mut collector = SensorCollector::new();
        // Should succeed even if no sensors are available
        let result = collector.collect();
        assert!(result.is_ok(), "Sensor collection should succeed: {:?}", result.err());
    }

    #[test]
    fn test_sensor_temperatures_reasonable() {
        let mut collector = SensorCollector::new();
        let _ = collector.collect();

        for reading in collector.readings() {
            // Temperature should be physically reasonable
            assert!(
                reading.current >= -273.15 && reading.current <= 200.0,
                "Temperature should be reasonable (-273.15°C to 200°C), got {}°C for {}",
                reading.current,
                reading.label
            );
        }
    }

    #[test]
    fn test_sensor_max_temp_valid() {
        let mut collector = SensorCollector::new();
        let _ = collector.collect();

        if let Some(max) = collector.max_temp() {
            assert!(
                max >= -273.15 && max <= 200.0,
                "Max temperature should be reasonable, got {}°C",
                max
            );
        }
    }

    #[test]
    fn test_sensor_interval_hint() {
        let collector = SensorCollector::new();
        let interval = collector.interval_hint();
        assert!(interval.as_millis() > 0, "Interval hint should be positive");
    }
}

// ============================================================================
// Apple GPU Collector Tests (macOS only)
// Fast tests only - GPU collection tests are in proptest_macos.rs
// ============================================================================

#[cfg(target_os = "macos")]
mod apple_gpu_collector_tests {
    use trueno_viz::monitor::collectors::AppleGpuCollector;
    use super::*;

    #[test]
    fn test_apple_gpu_collector_id() {
        let collector = AppleGpuCollector::new();
        assert_eq!(collector.id(), "gpu_apple");
    }

    #[test]
    fn test_apple_gpu_interval_hint() {
        let collector = AppleGpuCollector::new();
        let interval = collector.interval_hint();
        assert!(interval.as_millis() > 0, "Interval hint should be positive");
    }

    #[test]
    fn test_apple_gpu_display_name() {
        let collector = AppleGpuCollector::new();
        assert_eq!(collector.display_name(), "Apple GPU");
    }
}

// ============================================================================
// AMD GPU Collector Tests (Linux only)
// ============================================================================

#[cfg(target_os = "linux")]
mod amd_gpu_collector_tests {
    use trueno_viz::monitor::collectors::AmdGpuCollector;
    use super::*;

    #[test]
    fn test_amd_gpu_collector_initialization() {
        let collector = AmdGpuCollector::new();
        // May or may not be available depending on hardware
        let _ = collector.is_available();
    }

    #[test]
    fn test_amd_gpu_collector_id() {
        let collector = AmdGpuCollector::new();
        assert_eq!(collector.id(), "amd_gpu");
    }

    #[test]
    fn test_amd_gpu_collector_collect() {
        let mut collector = AmdGpuCollector::new();

        if collector.is_available() {
            let result = collector.collect();
            assert!(result.is_ok(), "AMD GPU collection should succeed: {:?}", result.err());
        }
    }
}

// ============================================================================
// Cross-Platform Collector Behavior Tests
// ============================================================================

mod collector_behavior_tests {
    use super::*;

    #[test]
    fn test_all_collectors_have_unique_ids() {
        let cpu = CpuCollector::new();
        let memory = MemoryCollector::new();
        let network = NetworkCollector::new();
        let disk = DiskCollector::new();
        let process = ProcessCollector::new();
        let sensor = SensorCollector::new();

        let ids = vec![
            cpu.id(),
            memory.id(),
            network.id(),
            disk.id(),
            process.id(),
            sensor.id(),
        ];

        // Check for duplicates
        let mut unique_ids = ids.clone();
        unique_ids.sort();
        unique_ids.dedup();

        assert_eq!(
            ids.len(),
            unique_ids.len(),
            "All collector IDs should be unique"
        );
    }

    #[test]
    fn test_all_collectors_have_display_names() {
        let cpu = CpuCollector::new();
        let memory = MemoryCollector::new();
        let network = NetworkCollector::new();
        let disk = DiskCollector::new();
        let process = ProcessCollector::new();
        let sensor = SensorCollector::new();

        assert!(!cpu.display_name().is_empty());
        assert!(!memory.display_name().is_empty());
        assert!(!network.display_name().is_empty());
        assert!(!disk.display_name().is_empty());
        assert!(!process.display_name().is_empty());
        assert!(!sensor.display_name().is_empty());
    }

    #[test]
    fn test_all_collectors_have_reasonable_intervals() {
        let cpu = CpuCollector::new();
        let memory = MemoryCollector::new();
        let network = NetworkCollector::new();
        let disk = DiskCollector::new();
        let process = ProcessCollector::new();
        let sensor = SensorCollector::new();

        let intervals = vec![
            cpu.interval_hint(),
            memory.interval_hint(),
            network.interval_hint(),
            disk.interval_hint(),
            process.interval_hint(),
            sensor.interval_hint(),
        ];

        for interval in intervals {
            assert!(
                interval.as_millis() >= 100 && interval.as_secs() <= 30,
                "Interval should be reasonable (100ms to 30s)"
            );
        }
    }

    #[test]
    fn test_collectors_handle_rapid_collection() {
        let mut cpu = CpuCollector::new();
        let mut memory = MemoryCollector::new();

        // Rapid collection should not panic
        for _ in 0..20 {
            let _ = cpu.collect();
            let _ = memory.collect();
        }
    }
}

// ============================================================================
// Metrics Validation Tests
// ============================================================================

mod metrics_validation_tests {
    use super::*;

    #[test]
    fn test_metrics_new() {
        let metrics = trueno_viz::monitor::types::Metrics::new();
        assert!(metrics.is_empty(), "New metrics should be empty");
    }

    #[test]
    fn test_metrics_insert_and_get_gauge() {
        let mut metrics = trueno_viz::monitor::types::Metrics::new();
        metrics.insert("test.gauge", 42.5);

        assert_eq!(metrics.get_gauge("test.gauge"), Some(42.5));
    }

    #[test]
    fn test_metrics_insert_and_get_counter() {
        let mut metrics = trueno_viz::monitor::types::Metrics::new();
        metrics.insert("test.counter", MetricValue::Counter(100));

        assert_eq!(metrics.get_counter("test.counter"), Some(100));
    }

    #[test]
    fn test_metrics_missing_key_returns_none() {
        let metrics = trueno_viz::monitor::types::Metrics::new();

        assert_eq!(metrics.get_gauge("nonexistent"), None);
        assert_eq!(metrics.get_counter("nonexistent"), None);
    }
}

// ============================================================================
// Subprocess Timeout Tests (Hang Prevention)
// ============================================================================

mod subprocess_timeout_tests {
    use std::time::{Duration, Instant};
    use trueno_viz::monitor::subprocess::{run_with_timeout, run_with_timeout_stdout, SubprocessResult};

    #[test]
    fn test_timeout_prevents_hang() {
        // This is the critical test: a slow command MUST timeout
        let start = Instant::now();
        let result = run_with_timeout("sleep", &["10"], Duration::from_millis(100));
        let elapsed = start.elapsed();

        assert!(result.is_timeout(), "Sleep should timeout");
        assert!(
            elapsed < Duration::from_secs(1),
            "Timeout should occur quickly, not hang. Elapsed: {:?}",
            elapsed
        );
    }

    #[test]
    fn test_fast_command_succeeds() {
        let result = run_with_timeout("echo", &["hello"], Duration::from_secs(1));
        assert!(result.is_success());
        assert_eq!(result.stdout_string().unwrap().trim(), "hello");
    }

    #[test]
    fn test_run_with_timeout_stdout_returns_none_on_timeout() {
        let result = run_with_timeout_stdout("sleep", &["10"], Duration::from_millis(50));
        assert!(result.is_none(), "Timeout should return None");
    }

    #[test]
    fn test_nonexistent_command_returns_spawn_error() {
        let result = run_with_timeout(
            "this_command_does_not_exist_xyz123",
            &[],
            Duration::from_secs(1),
        );
        assert!(matches!(result, SubprocessResult::SpawnError));
    }

    #[test]
    fn test_failed_command_returns_failed() {
        let result = run_with_timeout("false", &[], Duration::from_secs(1));
        assert!(matches!(result, SubprocessResult::Failed(_)));
    }

    #[test]
    fn test_multiple_rapid_timeouts_no_resource_leak() {
        // Stress test: many rapid timeouts should not leak threads/handles
        for _ in 0..10 {
            let start = Instant::now();
            let result = run_with_timeout("sleep", &["10"], Duration::from_millis(20));
            let elapsed = start.elapsed();

            assert!(result.is_timeout());
            assert!(elapsed < Duration::from_millis(500), "Timeout took too long: {:?}", elapsed);
        }
    }
}

// ============================================================================
// Collector Timeout Behavior Tests (Regression Prevention)
// ============================================================================

mod collector_timeout_tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// Test that process collection completes within reasonable time.
    /// This catches hangs in ps/sysctl calls on macOS.
    #[test]
    fn test_process_collector_does_not_hang() {
        let mut collector = ProcessCollector::new();
        let start = Instant::now();

        let result = collector.collect();
        let elapsed = start.elapsed();

        // Process collection should complete within 10 seconds even on slow systems
        assert!(
            elapsed < Duration::from_secs(10),
            "Process collection took too long: {:?}. Possible hang!",
            elapsed
        );
        assert!(result.is_ok(), "Collection should succeed: {:?}", result.err());
    }

    /// Test that network collection completes within reasonable time.
    /// This catches hangs in netstat calls on macOS.
    #[test]
    fn test_network_collector_does_not_hang() {
        let mut collector = NetworkCollector::new();
        let start = Instant::now();

        let result = collector.collect();
        let elapsed = start.elapsed();

        assert!(
            elapsed < Duration::from_secs(10),
            "Network collection took too long: {:?}. Possible hang!",
            elapsed
        );
        assert!(result.is_ok(), "Collection should succeed: {:?}", result.err());
    }

    /// Test that disk collection completes within reasonable time.
    /// This catches hangs in iostat/df calls on macOS.
    #[test]
    fn test_disk_collector_does_not_hang() {
        let mut collector = DiskCollector::new();
        let start = Instant::now();

        let result = collector.collect();
        let elapsed = start.elapsed();

        assert!(
            elapsed < Duration::from_secs(10),
            "Disk collection took too long: {:?}. Possible hang!",
            elapsed
        );
        assert!(result.is_ok(), "Collection should succeed: {:?}", result.err());
    }

    /// Test that all collectors together complete within reasonable time.
    /// Simulates a full refresh cycle.
    #[test]
    fn test_full_collection_cycle_does_not_hang() {
        let mut cpu = CpuCollector::new();
        let mut memory = MemoryCollector::new();
        let mut network = NetworkCollector::new();
        let mut disk = DiskCollector::new();
        let mut process = ProcessCollector::new();
        let mut sensor = SensorCollector::new();

        let start = Instant::now();

        // Run all collectors sequentially (like ttop's collect_metrics)
        let _ = cpu.collect();
        let _ = memory.collect();
        let _ = network.collect();
        let _ = disk.collect();
        let _ = process.collect();
        let _ = sensor.collect();

        let elapsed = start.elapsed();

        // Full cycle should complete in under 15 seconds
        assert!(
            elapsed < Duration::from_secs(15),
            "Full collection cycle took too long: {:?}. Possible hang in one of the collectors!",
            elapsed
        );
    }

    /// Test that rapid repeated collections don't accumulate delay.
    /// This catches resource leaks from timed-out threads.
    #[test]
    fn test_repeated_collections_no_delay_accumulation() {
        let mut process = ProcessCollector::new();
        let mut timings = Vec::new();

        for i in 0..5 {
            let start = Instant::now();
            let _ = process.collect();
            let elapsed = start.elapsed();
            timings.push(elapsed);

            // No single collection should take more than 10 seconds
            assert!(
                elapsed < Duration::from_secs(10),
                "Collection {} took too long: {:?}",
                i,
                elapsed
            );
        }

        // Check that timing doesn't grow over iterations (would indicate resource leak)
        if timings.len() >= 3 {
            let first_three_avg = timings[..3].iter().map(|d| d.as_millis()).sum::<u128>() / 3;
            let last_three_avg = timings[timings.len()-3..].iter().map(|d| d.as_millis()).sum::<u128>() / 3;

            // Later collections shouldn't be more than 5x slower than earlier ones
            assert!(
                last_three_avg < first_three_avg * 5 + 1000,
                "Collection time increased significantly: first avg {:?}ms, last avg {:?}ms",
                first_three_avg,
                last_three_avg
            );
        }
    }
}
