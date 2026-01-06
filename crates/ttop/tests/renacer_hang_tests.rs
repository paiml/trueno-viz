//! Renacer-based hang detection and performance playbook tests.
//!
//! Uses renacer syscall tracing to:
//! - Detect collector hangs via anomaly detection
//! - Profile I/O bottlenecks in collectors
//! - Validate performance baselines for regression detection
//! - Generate distributed traces for debugging

use std::process::Command;
use std::time::{Duration, Instant};

use trueno_viz::monitor::collectors::{
    CpuCollector, DiskCollector, MemoryCollector, NetworkCollector, ProcessCollector,
    SensorCollector,
};
use trueno_viz::monitor::subprocess::{run_with_timeout, SubprocessResult};
use trueno_viz::monitor::types::Collector;

// ============================================================================
// Subprocess Timeout Verification Tests
// ============================================================================

mod timeout_verification {
    use super::*;

    /// Verify that run_with_timeout actually kills slow processes.
    /// This is the critical foundation for hang prevention.
    #[test]
    fn test_timeout_kills_slow_process() {
        let start = Instant::now();
        let result = run_with_timeout("sleep", &["60"], Duration::from_millis(100));
        let elapsed = start.elapsed();

        assert!(
            result.is_timeout(),
            "Expected timeout, got {:?}",
            result
        );
        assert!(
            elapsed < Duration::from_secs(1),
            "Timeout should occur in <1s, took {:?}",
            elapsed
        );
    }

    /// Verify that successful commands complete quickly.
    #[test]
    fn test_fast_command_succeeds() {
        let start = Instant::now();
        let result = run_with_timeout("echo", &["test"], Duration::from_secs(5));
        let elapsed = start.elapsed();

        assert!(result.is_success(), "Echo should succeed");
        assert!(
            elapsed < Duration::from_millis(500),
            "Echo should be fast, took {:?}",
            elapsed
        );
    }

    /// Stress test: many rapid timeouts should not leak resources.
    #[test]
    fn test_rapid_timeout_stress() {
        for i in 0..20 {
            let start = Instant::now();
            let result = run_with_timeout("sleep", &["10"], Duration::from_millis(10));
            let elapsed = start.elapsed();

            assert!(result.is_timeout(), "Iteration {} should timeout", i);
            assert!(
                elapsed < Duration::from_millis(500),
                "Iteration {} took too long: {:?}",
                i,
                elapsed
            );
        }
    }
}

// ============================================================================
// Collector Hang Detection Tests
// ============================================================================

mod collector_hang_detection {
    use super::*;

    const MAX_COLLECTOR_TIME: Duration = Duration::from_secs(10);

    /// Test CPU collector does not hang.
    #[test]
    fn test_cpu_collector_no_hang() {
        let mut collector = CpuCollector::new();
        let start = Instant::now();
        let result = collector.collect();
        let elapsed = start.elapsed();

        assert!(
            elapsed < MAX_COLLECTOR_TIME,
            "CPU collector hung for {:?}",
            elapsed
        );
        assert!(result.is_ok(), "CPU collection failed: {:?}", result.err());
    }

    /// Test memory collector does not hang.
    #[test]
    fn test_memory_collector_no_hang() {
        let mut collector = MemoryCollector::new();
        let start = Instant::now();
        let result = collector.collect();
        let elapsed = start.elapsed();

        assert!(
            elapsed < MAX_COLLECTOR_TIME,
            "Memory collector hung for {:?}",
            elapsed
        );
        assert!(result.is_ok(), "Memory collection failed: {:?}", result.err());
    }

    /// Test network collector does not hang.
    #[test]
    fn test_network_collector_no_hang() {
        let mut collector = NetworkCollector::new();
        let start = Instant::now();
        let result = collector.collect();
        let elapsed = start.elapsed();

        assert!(
            elapsed < MAX_COLLECTOR_TIME,
            "Network collector hung for {:?}",
            elapsed
        );
        assert!(result.is_ok(), "Network collection failed: {:?}", result.err());
    }

    /// Test disk collector does not hang.
    /// This is particularly important as disk collectors can hang on NFS/network mounts.
    #[test]
    fn test_disk_collector_no_hang() {
        let mut collector = DiskCollector::new();
        let start = Instant::now();
        let result = collector.collect();
        let elapsed = start.elapsed();

        assert!(
            elapsed < MAX_COLLECTOR_TIME,
            "Disk collector hung for {:?}. Check for NFS/network mounts!",
            elapsed
        );
        assert!(result.is_ok(), "Disk collection failed: {:?}", result.err());
    }

    /// Test process collector does not hang.
    #[test]
    fn test_process_collector_no_hang() {
        let mut collector = ProcessCollector::new();
        let start = Instant::now();
        let result = collector.collect();
        let elapsed = start.elapsed();

        assert!(
            elapsed < MAX_COLLECTOR_TIME,
            "Process collector hung for {:?}",
            elapsed
        );
        assert!(result.is_ok(), "Process collection failed: {:?}", result.err());
    }

    /// Test sensor collector does not hang.
    #[test]
    fn test_sensor_collector_no_hang() {
        let mut collector = SensorCollector::new();
        let start = Instant::now();
        let result = collector.collect();
        let elapsed = start.elapsed();

        assert!(
            elapsed < MAX_COLLECTOR_TIME,
            "Sensor collector hung for {:?}",
            elapsed
        );
        assert!(result.is_ok(), "Sensor collection failed: {:?}", result.err());
    }
}

// ============================================================================
// Full Cycle Performance Tests
// ============================================================================

mod full_cycle_tests {
    use super::*;

    /// Test that a full collection cycle completes within reasonable time.
    /// This simulates ttop's main loop collect_metrics() call.
    #[test]
    fn test_full_collection_cycle() {
        let mut cpu = CpuCollector::new();
        let mut memory = MemoryCollector::new();
        let mut network = NetworkCollector::new();
        let mut disk = DiskCollector::new();
        let mut process = ProcessCollector::new();
        let mut sensor = SensorCollector::new();

        let start = Instant::now();

        // Simulate ttop's collect_metrics sequence
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
            "Full collection cycle took {:?}. This will cause UI freezes!",
            elapsed
        );

        // Ideal target: <1 second for smooth 1Hz refresh
        if elapsed > Duration::from_secs(1) {
            eprintln!(
                "WARNING: Collection cycle took {:?}. Target is <1s for smooth refresh.",
                elapsed
            );
        }
    }

    /// Test that repeated collection cycles don't accumulate delay.
    /// This catches resource leaks or thread accumulation.
    #[test]
    fn test_repeated_cycles_no_delay_accumulation() {
        let mut cpu = CpuCollector::new();
        let mut memory = MemoryCollector::new();
        let mut network = NetworkCollector::new();
        let mut process = ProcessCollector::new();

        let mut timings = Vec::new();

        for i in 0..5 {
            let start = Instant::now();

            let _ = cpu.collect();
            let _ = memory.collect();
            let _ = network.collect();
            let _ = process.collect();

            let elapsed = start.elapsed();
            timings.push(elapsed);

            assert!(
                elapsed < Duration::from_secs(10),
                "Cycle {} took {:?}",
                i,
                elapsed
            );
        }

        // Check that later cycles aren't significantly slower
        if timings.len() >= 3 {
            let first_avg = timings[..2].iter().map(|d| d.as_millis()).sum::<u128>() / 2;
            let last_avg = timings[timings.len() - 2..].iter().map(|d| d.as_millis()).sum::<u128>() / 2;

            // Allow 3x variance (system load can vary) but catch exponential growth
            assert!(
                last_avg < first_avg * 3 + 500,
                "Collection time growing: first avg {}ms, last avg {}ms",
                first_avg,
                last_avg
            );
        }
    }
}

// ============================================================================
// Renacer Integration Tests (when renacer CLI is available)
// ============================================================================

#[cfg(target_os = "linux")]
mod renacer_integration {
    use super::*;

    fn renacer_available() -> bool {
        Command::new("renacer")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Use renacer to trace collector execution and detect anomalies.
    #[test]
    #[ignore = "requires renacer CLI installed"]
    fn test_renacer_anomaly_detection() {
        if !renacer_available() {
            eprintln!("Skipping: renacer CLI not available");
            return;
        }

        // Build and run a simple collector test with renacer tracing
        let output = Command::new("renacer")
            .args([
                "--anomaly-realtime",
                "-T",
                "-c",
                "--",
                "cargo",
                "test",
                "--test",
                "renacer_hang_tests",
                "collector_hang_detection::test_disk_collector_no_hang",
                "--",
                "--exact",
                "--nocapture",
            ])
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);

                // Check for high-severity anomalies
                if stderr.contains("ANOMALY") && stderr.contains("High") {
                    eprintln!("RENACER DETECTED HIGH SEVERITY ANOMALY:");
                    eprintln!("{}", stderr);
                }

                // Test should pass
                assert!(
                    out.status.success() || stdout.contains("1 passed"),
                    "Test failed under renacer tracing"
                );
            }
            Err(e) => {
                eprintln!("Failed to run renacer: {}", e);
            }
        }
    }

    /// Use renacer to generate performance baseline for regression detection.
    #[test]
    #[ignore = "requires renacer CLI installed"]
    fn test_renacer_generate_baseline() {
        if !renacer_available() {
            eprintln!("Skipping: renacer CLI not available");
            return;
        }

        let baseline_dir = std::env::temp_dir().join("ttop_baseline");

        // Generate baseline
        let output = Command::new("renacer")
            .args([
                "validate",
                "--generate",
                baseline_dir.to_str().unwrap(),
                "--",
                "cargo",
                "test",
                "--test",
                "renacer_hang_tests",
                "full_cycle_tests::test_full_collection_cycle",
                "--",
                "--exact",
            ])
            .output();

        match output {
            Ok(out) => {
                assert!(
                    out.status.success(),
                    "Failed to generate baseline: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
                eprintln!("Baseline generated at {:?}", baseline_dir);
            }
            Err(e) => {
                eprintln!("Failed to run renacer validate: {}", e);
            }
        }
    }

    /// Use renacer function profiler to identify I/O bottlenecks.
    #[test]
    #[ignore = "requires renacer CLI installed"]
    fn test_renacer_function_profiling() {
        if !renacer_available() {
            eprintln!("Skipping: renacer CLI not available");
            return;
        }

        let output = Command::new("renacer")
            .args([
                "--function-time",
                "-T",
                "--",
                "cargo",
                "test",
                "--test",
                "renacer_hang_tests",
                "collector_hang_detection::test_disk_collector_no_hang",
                "--",
                "--exact",
            ])
            .output();

        match output {
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);

                // Look for I/O bottleneck warnings
                if stderr.contains("SLOW I/O") || stderr.contains("bottleneck") {
                    eprintln!("RENACER DETECTED I/O BOTTLENECK:");
                    eprintln!("{}", stderr);
                }
            }
            Err(e) => {
                eprintln!("Failed to run renacer: {}", e);
            }
        }
    }
}

// ============================================================================
// Probador Playbook Tests - Deterministic Collector Behavior
// ============================================================================

mod probador_playbook {
    use super::*;

    /// Playbook: CPU collector returns valid metrics after N collections.
    #[test]
    fn playbook_cpu_valid_metrics_after_warmup() {
        let mut collector = CpuCollector::new();

        // Warmup: need 2 samples for delta calculation
        let _ = collector.collect();
        let _ = collector.collect();

        // Now should have valid data
        let result = collector.collect();
        assert!(result.is_ok());

        let metrics = result.unwrap();
        let total = metrics.get_gauge("cpu.total");
        assert!(total.is_some(), "Should have cpu.total after warmup");
        assert!(
            total.unwrap() >= 0.0 && total.unwrap() <= 100.0,
            "CPU total should be 0-100%"
        );
    }

    /// Playbook: Memory collector returns consistent total memory.
    #[test]
    fn playbook_memory_total_consistent() {
        let mut collector = MemoryCollector::new();

        let r1 = collector.collect().unwrap();
        let r2 = collector.collect().unwrap();

        let total1 = r1.get_counter("memory.total").unwrap_or(0);
        let total2 = r2.get_counter("memory.total").unwrap_or(0);

        assert_eq!(
            total1, total2,
            "Total memory should be consistent across collections"
        );
        assert!(total1 > 0, "Total memory should be positive");
    }

    /// Playbook: Process collector finds at least our test process.
    #[test]
    fn playbook_process_finds_self() {
        let mut collector = ProcessCollector::new();
        let _ = collector.collect();

        let count = collector.count();
        assert!(count > 0, "Should find at least one process");

        // Should include current process
        let our_pid = std::process::id();
        let found = collector.processes().contains_key(&our_pid);
        assert!(found, "Should find our own process (PID {})", our_pid);
    }

    /// Playbook: Network collector handles interface changes gracefully.
    #[test]
    fn playbook_network_stable_under_repeated_calls() {
        let mut collector = NetworkCollector::new();

        for _ in 0..10 {
            let result = collector.collect();
            assert!(result.is_ok(), "Network collection should always succeed");
        }

        // Should have detected some interfaces
        let interfaces = collector.interfaces();
        eprintln!("Detected {} network interfaces", interfaces.len());
    }

    /// Playbook: Disk collector handles mount point filtering correctly.
    #[test]
    fn playbook_disk_filters_virtual_fs() {
        let mut collector = DiskCollector::new();
        let _ = collector.collect();

        for mount in collector.mounts() {
            // Should not include /proc, /sys, /dev, /snap
            assert!(
                !mount.mount_point.starts_with("/proc"),
                "Should filter /proc mounts"
            );
            assert!(
                !mount.mount_point.starts_with("/sys"),
                "Should filter /sys mounts"
            );
            assert!(
                !mount.mount_point.starts_with("/dev/"),
                "Should filter /dev mounts"
            );
            assert!(
                !mount.mount_point.starts_with("/snap"),
                "Should filter /snap mounts"
            );
        }
    }

    /// Playbook: Sensor collector returns physically valid temperatures.
    #[test]
    fn playbook_sensor_valid_temperatures() {
        let mut collector = SensorCollector::new();
        let _ = collector.collect();

        for reading in collector.readings() {
            // Temperature should be physically reasonable
            assert!(
                reading.current >= -50.0 && reading.current <= 150.0,
                "Temperature {} should be reasonable for {:?}",
                reading.current,
                reading.label
            );
        }
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

mod edge_cases {
    use super::*;

    /// Test behavior with nonexistent command.
    #[test]
    fn test_nonexistent_command_timeout() {
        let result = run_with_timeout(
            "this_command_definitely_does_not_exist_12345",
            &[],
            Duration::from_secs(1),
        );

        assert!(
            matches!(result, SubprocessResult::SpawnError),
            "Nonexistent command should return SpawnError"
        );
    }

    /// Test behavior with command that exits with error.
    #[test]
    fn test_failing_command() {
        let result = run_with_timeout("false", &[], Duration::from_secs(1));

        assert!(
            matches!(result, SubprocessResult::Failed(_)),
            "Failed command should return Failed"
        );
    }

    /// Test empty output handling.
    #[test]
    fn test_empty_output_command() {
        let result = run_with_timeout("true", &[], Duration::from_secs(1));

        assert!(result.is_success());
        let stdout = result.stdout_string().unwrap_or_default();
        assert!(stdout.is_empty() || stdout.trim().is_empty());
    }

    /// Test large output handling.
    #[test]
    fn test_large_output_command() {
        // seq 1 10000 produces ~50KB output
        let result = run_with_timeout("seq", &["1", "10000"], Duration::from_secs(5));

        assert!(result.is_success(), "seq should succeed");
        let stdout = result.stdout_string().unwrap();
        assert!(stdout.contains("10000"), "Should include final number");
        assert!(stdout.lines().count() >= 10000, "Should have many lines");
    }
}
