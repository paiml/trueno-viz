//! Property-based testing for macOS collectors.
//!
//! These tests verify that the macOS-specific collection code handles
//! all edge cases correctly using proptest strategies.
#![allow(clippy::unwrap_used)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

#[cfg(target_os = "macos")]
mod macos_property_tests {
    use proptest::prelude::*;
    use trueno_viz::monitor::collectors::CpuCollector;
    use trueno_viz::monitor::types::Collector;

    proptest! {
        /// Test that CPU collector handles any valid frame count
        #[test]
        fn test_cpu_collector_multiple_frames(frame_count in 1usize..100) {
            let mut collector = CpuCollector::new();

            for _ in 0..frame_count {
                // Should never panic regardless of how many collections
                let result = collector.collect();
                // Collection should succeed on macOS
                prop_assert!(result.is_ok(), "CPU collection should succeed");
            }

            // Core count should be consistent
            prop_assert!(collector.core_count() >= 1, "Should have at least 1 core");
        }

        /// Test that CPU percentage values are always valid
        #[test]
        fn test_cpu_percentage_bounds(frames in 2usize..20) {
            let mut collector = CpuCollector::new();

            // Need at least 2 frames for delta calculation
            for _ in 0..frames {
                if let Ok(metrics) = collector.collect() {
                    // Check total CPU percentage
                    if let Some(total) = metrics.get_gauge("cpu.total") {
                        prop_assert!(
                            total >= 0.0 && total <= 100.0,
                            "Total CPU should be 0-100%, got {}",
                            total
                        );
                    }

                    // Check per-core percentages
                    for i in 0..collector.core_count() {
                        if let Some(core_pct) = metrics.get_gauge(&format!("cpu.core.{}", i)) {
                            prop_assert!(
                                core_pct >= 0.0 && core_pct <= 100.0,
                                "Core {} CPU should be 0-100%, got {}",
                                i,
                                core_pct
                            );
                        }
                    }
                }
            }
        }

        /// Test that core count is consistent across collections
        #[test]
        fn test_core_count_consistent(frames in 1usize..50) {
            let mut collector = CpuCollector::new();
            let initial_cores = collector.core_count();

            for _ in 0..frames {
                let _ = collector.collect();
                prop_assert_eq!(
                    collector.core_count(),
                    initial_cores,
                    "Core count should remain consistent"
                );
            }
        }

        /// Test load average is within reasonable bounds
        #[test]
        fn test_load_average_bounds(_frame in 0usize..10) {
            let mut collector = CpuCollector::new();
            let _ = collector.collect();

            let load = collector.load_average();
            // Load average can be > num cores under extreme load, but should be reasonable
            let max_reasonable = (collector.core_count() * 100) as f64;

            prop_assert!(
                load.one >= 0.0 && load.one < max_reasonable,
                "1-min load should be reasonable, got {}",
                load.one
            );
            prop_assert!(
                load.five >= 0.0 && load.five < max_reasonable,
                "5-min load should be reasonable, got {}",
                load.five
            );
            prop_assert!(
                load.fifteen >= 0.0 && load.fifteen < max_reasonable,
                "15-min load should be reasonable, got {}",
                load.fifteen
            );
        }

        /// Test uptime is always positive
        #[test]
        fn test_uptime_positive(_frame in 0usize..5) {
            let mut collector = CpuCollector::new();
            let _ = collector.collect();

            let uptime = collector.uptime_secs();
            prop_assert!(
                uptime >= 0.0,
                "Uptime should be non-negative, got {}",
                uptime
            );
            // System should have been up for at least a few seconds if tests are running
            prop_assert!(
                uptime > 1.0,
                "System should have been up for at least 1 second, got {}",
                uptime
            );
        }
    }
}

#[cfg(target_os = "macos")]
mod macos_memory_tests {
    use proptest::prelude::*;
    use trueno_viz::monitor::collectors::MemoryCollector;
    use trueno_viz::monitor::types::Collector;

    proptest! {
        /// Test memory collector handles multiple frames
        #[test]
        fn test_memory_collector_frames(frame_count in 1usize..50) {
            let mut collector = MemoryCollector::new();

            for _ in 0..frame_count {
                let result = collector.collect();
                prop_assert!(result.is_ok(), "Memory collection should succeed");
            }
        }

        /// Test memory values are consistent
        #[test]
        fn test_memory_values_valid(_frames in 1usize..10) {
            let mut collector = MemoryCollector::new();

            if let Ok(metrics) = collector.collect() {
                // Total memory should be positive
                if let Some(total) = metrics.get_counter("memory.total") {
                    prop_assert!(total > 0, "Total memory should be positive");

                    // Used + free should roughly equal total
                    if let (Some(used), Some(free)) = (
                        metrics.get_counter("memory.used"),
                        metrics.get_counter("memory.free"),
                    ) {
                        // Allow some buffer overhead
                        prop_assert!(
                            used + free <= total * 2,
                            "Used + free should be less than 2x total"
                        );
                    }
                }

                // Percentage should be 0-100
                if let Some(pct) = metrics.get_gauge("memory.used.percent") {
                    prop_assert!(
                        pct >= 0.0 && pct <= 100.0,
                        "Memory percentage should be 0-100%, got {}",
                        pct
                    );
                }
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod macos_network_tests {
    use proptest::prelude::*;
    use trueno_viz::monitor::collectors::NetworkCollector;
    use trueno_viz::monitor::types::Collector;

    proptest! {
        /// Test network collector handles multiple frames
        #[test]
        fn test_network_collector_frames(frame_count in 1usize..30) {
            let mut collector = NetworkCollector::new();

            for _ in 0..frame_count {
                let result = collector.collect();
                prop_assert!(result.is_ok(), "Network collection should succeed");
            }
        }

        /// Test network rates are non-negative
        #[test]
        fn test_network_rates_valid(frames in 2usize..10) {
            let mut collector = NetworkCollector::new();

            for _ in 0..frames {
                let _ = collector.collect();
            }

            // Check all interface rates
            for (iface, rate) in collector.all_rates() {
                prop_assert!(
                    rate.rx_bytes_per_sec >= 0.0,
                    "RX rate for {} should be >= 0, got {}",
                    iface,
                    rate.rx_bytes_per_sec
                );
                prop_assert!(
                    rate.tx_bytes_per_sec >= 0.0,
                    "TX rate for {} should be >= 0, got {}",
                    iface,
                    rate.tx_bytes_per_sec
                );
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod macos_disk_tests {
    use proptest::prelude::*;
    use trueno_viz::monitor::collectors::DiskCollector;
    use trueno_viz::monitor::types::Collector;

    proptest! {
        /// Test disk collector handles multiple frames
        #[test]
        fn test_disk_collector_frames(frame_count in 1usize..20) {
            let mut collector = DiskCollector::new();

            for _ in 0..frame_count {
                let result = collector.collect();
                prop_assert!(result.is_ok(), "Disk collection should succeed");
            }
        }

        /// Test disk mount values are valid
        #[test]
        fn test_disk_mounts_valid(_frames in 1usize..5) {
            let mut collector = DiskCollector::new();
            let _ = collector.collect();

            for mount in collector.mounts() {
                // Total should be positive for valid mounts
                if mount.total_bytes > 0 {
                    prop_assert!(
                        mount.used_bytes <= mount.total_bytes,
                        "Used ({}) should be <= total ({}) for {}",
                        mount.used_bytes,
                        mount.total_bytes,
                        mount.mount_point
                    );

                    let usage = mount.usage_percent();
                    prop_assert!(
                        usage >= 0.0 && usage <= 100.0,
                        "Usage should be 0-100%, got {} for {}",
                        usage,
                        mount.mount_point
                    );
                }
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod macos_apple_gpu_tests {
    use proptest::prelude::*;
    use trueno_viz::monitor::collectors::AppleGpuCollector;
    use trueno_viz::monitor::types::Collector;

    proptest! {
        /// Test Apple GPU collector handles multiple frames
        #[test]
        fn test_apple_gpu_collector_frames(frame_count in 1usize..30) {
            let mut collector = AppleGpuCollector::new();

            // Skip if GPU not available
            if !collector.is_available() {
                return Ok(());
            }

            for _ in 0..frame_count {
                let result = collector.collect();
                prop_assert!(result.is_ok(), "Apple GPU collection should succeed");
            }
        }

        /// Test GPU utilization is in valid range
        #[test]
        fn test_apple_gpu_util_valid(_frames in 1usize..10) {
            let mut collector = AppleGpuCollector::new();

            if !collector.is_available() {
                return Ok(());
            }

            let _ = collector.collect();

            for gpu in collector.gpus() {
                prop_assert!(
                    gpu.gpu_util >= 0.0 && gpu.gpu_util <= 100.0,
                    "GPU util should be 0-100%, got {}",
                    gpu.gpu_util
                );
                // Core count may be 0 if IOKit doesn't report it
                // Just verify the name is not empty as a sanity check
                prop_assert!(
                    !gpu.name.is_empty(),
                    "GPU should have a name"
                );
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod macos_sensor_tests {
    use proptest::prelude::*;
    use trueno_viz::monitor::collectors::SensorCollector;
    use trueno_viz::monitor::types::Collector;

    proptest! {
        /// Test sensor collector handles multiple frames
        #[test]
        fn test_sensor_collector_frames(frame_count in 1usize..20) {
            let mut collector = SensorCollector::new();

            for _ in 0..frame_count {
                let result = collector.collect();
                prop_assert!(result.is_ok(), "Sensor collection should succeed");
            }
        }

        /// Test temperature values are in reasonable range
        #[test]
        fn test_sensor_temps_reasonable(_frames in 1usize..5) {
            let mut collector = SensorCollector::new();
            let _ = collector.collect();

            for reading in collector.readings() {
                // Temperature should be between -40°C and 150°C (reasonable hardware range)
                prop_assert!(
                    reading.current >= -40.0 && reading.current <= 150.0,
                    "Temperature should be reasonable, got {}°C for {}",
                    reading.current,
                    reading.label
                );
            }

            if let Some(max) = collector.max_temp() {
                prop_assert!(
                    max >= -40.0 && max <= 150.0,
                    "Max temp should be reasonable, got {}°C",
                    max
                );
            }
        }
    }
}

// Cross-platform property tests
#[cfg(test)]
mod cross_platform_tests {
    use proptest::prelude::*;
    use ttop::theme::{format_bytes, format_bytes_rate, format_uptime, percent_color, temp_color};

    proptest! {
        /// Test format_bytes never panics for any u64
        #[test]
        fn test_format_bytes_any_value(bytes in 0u64..u64::MAX) {
            let result = format_bytes(bytes);
            prop_assert!(!result.is_empty(), "format_bytes should return non-empty string");
        }

        /// Test format_bytes_rate never panics for any f64
        #[test]
        fn test_format_bytes_rate_any_value(rate in 0.0f64..1e18) {
            let result = format_bytes_rate(rate);
            prop_assert!(!result.is_empty(), "format_bytes_rate should return non-empty string");
        }

        /// Test format_uptime never panics for any f64
        #[test]
        fn test_format_uptime_any_value(secs in 0.0f64..1e10) {
            let result = format_uptime(secs);
            prop_assert!(!result.is_empty(), "format_uptime should return non-empty string");
        }

        /// Test percent_color never panics for any f64
        #[test]
        fn test_percent_color_any_value(percent in -1000.0f64..1000.0) {
            let _ = percent_color(percent);
            // If we get here without panic, test passes
        }

        /// Test temp_color never panics for any f64
        #[test]
        fn test_temp_color_any_value(temp in -273.0f64..1000.0) {
            let _ = temp_color(temp);
            // If we get here without panic, test passes
        }

        /// Test format_bytes returns consistent unit progression
        #[test]
        fn test_format_bytes_unit_progression(power in 0u32..5) {
            let bytes: u64 = 1024u64.pow(power);
            let result = format_bytes(bytes);

            let expected_unit = match power {
                0 => "B",
                1 => "K",
                2 => "M",
                3 => "G",
                4 => "T",
                _ => "",
            };

            prop_assert!(
                result.contains(expected_unit),
                "1024^{} bytes should have {} unit, got {}",
                power,
                expected_unit,
                result
            );
        }
    }
}
