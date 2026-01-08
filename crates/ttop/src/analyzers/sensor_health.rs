//! Sensor Health Analyzer
//!
//! Monitors hardware sensors (temperature, fan, voltage) for anomalies.
//! Implements:
//! - MAD-based outlier detection (Iglewicz & Hoaglin, 1993)
//! - Linear regression drift tracking
//! - Staleness detection for unresponsive sensors
//! - Thermal headroom calculation

use crate::ring_buffer::RingBuffer;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

/// Sensor type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SensorType {
    Temperature,
    Fan,
    Voltage,
    Power,
    Current,
    Humidity,
}

impl SensorType {
    pub fn unit(&self) -> &'static str {
        match self {
            Self::Temperature => "°C",
            Self::Fan => "RPM",
            Self::Voltage => "V",
            Self::Power => "W",
            Self::Current => "A",
            Self::Humidity => "%",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Temperature => "🌡",
            Self::Fan => "🌀",
            Self::Voltage => "⚡",
            Self::Power => "🔌",
            Self::Current => "〰",
            Self::Humidity => "💧",
        }
    }
}

/// Health status for a sensor
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SensorHealth {
    /// Sensor is healthy
    Healthy,
    /// Sensor reading is unusual but not alarming
    Warning,
    /// Sensor reading is outside normal bounds
    Critical,
    /// Sensor hasn't updated recently
    Stale,
    /// Sensor is not responding
    Dead,
}

impl SensorHealth {
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Healthy => "●",
            Self::Warning => "◐",
            Self::Critical => "○",
            Self::Stale => "◌",
            Self::Dead => "✗",
        }
    }

    pub fn color_hint(&self) -> &'static str {
        match self {
            Self::Healthy => "green",
            Self::Warning => "yellow",
            Self::Critical => "red",
            Self::Stale => "gray",
            Self::Dead => "darkgray",
        }
    }
}

/// Individual sensor reading with metadata
#[derive(Debug, Clone)]
pub struct SensorReading {
    /// Sensor name (e.g., "coretemp-isa-0000/temp1")
    pub name: String,
    /// Human-readable label (e.g., "Package id 0")
    pub label: String,
    /// Sensor type
    pub sensor_type: SensorType,
    /// Current value
    pub value: f64,
    /// Critical threshold (if known)
    pub crit: Option<f64>,
    /// Max threshold (if known)
    pub max: Option<f64>,
    /// Min threshold (if known)
    pub min: Option<f64>,
    /// Health status
    pub health: SensorHealth,
    /// Thermal headroom (for temperature sensors)
    pub headroom: Option<f64>,
    /// Is this an outlier based on MAD?
    pub is_outlier: bool,
    /// Drift rate (units per minute)
    pub drift_rate: Option<f64>,
}

/// Sensor history for tracking over time
struct SensorHistory {
    values: RingBuffer<f64>,
    timestamps: RingBuffer<Instant>,
    last_seen: Instant,
    last_value: f64,
}

impl SensorHistory {
    fn new() -> Self {
        Self {
            values: RingBuffer::new(60), // 60 samples
            timestamps: RingBuffer::new(60),
            last_seen: Instant::now(),
            last_value: 0.0,
        }
    }

    fn push(&mut self, value: f64) {
        self.values.push(value);
        self.timestamps.push(Instant::now());
        self.last_seen = Instant::now();
        self.last_value = value;
    }

    /// Detect outliers using MAD (Median Absolute Deviation)
    /// Modified Z-score > 3.5 indicates outlier (Iglewicz & Hoaglin, 1993)
    fn is_outlier(&self, value: f64) -> bool {
        let values: Vec<f64> = self.values.iter().copied().collect();
        if values.len() < 10 {
            return false; // Need enough data
        }

        let median = Self::median(&values);
        let mad = Self::mad(&values, median);

        if mad < 0.001 {
            return false; // All values identical
        }

        // Modified Z-score
        let z_score = 0.6745 * (value - median) / mad;
        z_score.abs() > 3.5
    }

    /// Calculate drift rate using simple linear regression (units per minute)
    fn drift_rate(&self) -> Option<f64> {
        let values: Vec<f64> = self.values.iter().copied().collect();
        let timestamps: Vec<Instant> = self.timestamps.iter().copied().collect();

        if values.len() < 5 {
            return None;
        }

        let n = values.len() as f64;
        let base_time = timestamps[0];

        // Convert timestamps to seconds from start
        let times: Vec<f64> = timestamps
            .iter()
            .map(|t| t.duration_since(base_time).as_secs_f64())
            .collect();

        // Linear regression: y = mx + b
        let sum_x: f64 = times.iter().sum();
        let sum_y: f64 = values.iter().sum();
        let sum_xy: f64 = times.iter().zip(values.iter()).map(|(x, y)| x * y).sum();
        let sum_x2: f64 = times.iter().map(|x| x * x).sum();

        let denom = n * sum_x2 - sum_x * sum_x;
        if denom.abs() < 0.001 {
            return None;
        }

        let slope = (n * sum_xy - sum_x * sum_y) / denom;

        // Convert from per-second to per-minute
        Some(slope * 60.0)
    }

    fn median(data: &[f64]) -> f64 {
        let mut sorted = data.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            (sorted[mid - 1] + sorted[mid]) / 2.0
        } else {
            sorted[mid]
        }
    }

    fn mad(data: &[f64], median: f64) -> f64 {
        let deviations: Vec<f64> = data.iter().map(|x| (x - median).abs()).collect();
        Self::median(&deviations)
    }
}

/// Sensor Health Analyzer
pub struct SensorHealthAnalyzer {
    /// Sensor histories keyed by sensor name
    histories: HashMap<String, SensorHistory>,
    /// Last collection timestamp
    last_collect: Instant,
    /// Staleness threshold
    stale_threshold: Duration,
    /// Dead threshold
    dead_threshold: Duration,
}

impl SensorHealthAnalyzer {
    pub fn new() -> Self {
        Self {
            histories: HashMap::new(),
            last_collect: Instant::now() - Duration::from_secs(10),
            stale_threshold: Duration::from_secs(30),
            dead_threshold: Duration::from_secs(120),
        }
    }

    /// Collect all sensor readings
    pub fn collect(&mut self) -> Vec<SensorReading> {
        // Rate limit collection
        if self.last_collect.elapsed() < Duration::from_millis(500) {
            return self.get_cached_readings();
        }
        self.last_collect = Instant::now();

        let mut readings = Vec::new();

        // Scan hwmon devices
        if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                let hwmon_path = entry.path();
                let chip_name = self.read_hwmon_name(&hwmon_path);

                // Read temperature sensors
                readings.extend(self.read_temp_sensors(&hwmon_path, &chip_name));

                // Read fan sensors
                readings.extend(self.read_fan_sensors(&hwmon_path, &chip_name));

                // Read voltage sensors
                readings.extend(self.read_voltage_sensors(&hwmon_path, &chip_name));

                // Read power sensors
                readings.extend(self.read_power_sensors(&hwmon_path, &chip_name));
            }
        }

        readings
    }

    /// Get thermal headroom summary
    pub fn thermal_summary(&self) -> Option<(f64, f64, f64)> {
        let readings = self.get_cached_readings();
        let temps: Vec<&SensorReading> = readings
            .iter()
            .filter(|r| r.sensor_type == SensorType::Temperature)
            .collect();

        if temps.is_empty() {
            return None;
        }

        let max_temp = temps.iter().map(|t| t.value).fold(f64::NEG_INFINITY, f64::max);
        let min_headroom = temps
            .iter()
            .filter_map(|t| t.headroom)
            .fold(f64::INFINITY, f64::min);
        let avg_temp = temps.iter().map(|t| t.value).sum::<f64>() / temps.len() as f64;

        Some((max_temp, min_headroom, avg_temp))
    }

    /// Check if any sensor is in critical state
    pub fn any_critical(&self) -> bool {
        self.get_cached_readings()
            .iter()
            .any(|r| r.health == SensorHealth::Critical)
    }

    /// Get sensors grouped by health status
    pub fn by_health(&self) -> HashMap<SensorHealth, Vec<SensorReading>> {
        let readings = self.get_cached_readings();
        let mut grouped: HashMap<SensorHealth, Vec<SensorReading>> = HashMap::new();

        for reading in readings {
            grouped
                .entry(reading.health)
                .or_default()
                .push(reading);
        }

        grouped
    }

    pub fn get_cached_readings(&self) -> Vec<SensorReading> {
        // Re-collect if needed
        let mut readings = Vec::new();

        for (name, history) in &self.histories {
            let parts: Vec<&str> = name.split('/').collect();
            let sensor_type = if name.contains("temp") {
                SensorType::Temperature
            } else if name.contains("fan") {
                SensorType::Fan
            } else if name.contains("in") {
                SensorType::Voltage
            } else if name.contains("power") {
                SensorType::Power
            } else {
                continue;
            };

            let health = Self::calculate_health(history, history.last_value, None, None, sensor_type, self.stale_threshold, self.dead_threshold);

            readings.push(SensorReading {
                name: name.clone(),
                label: parts.last().unwrap_or(&"unknown").to_string(),
                sensor_type,
                value: history.last_value,
                crit: None,
                max: None,
                min: None,
                health,
                headroom: None,
                is_outlier: history.is_outlier(history.last_value),
                drift_rate: history.drift_rate(),
            });
        }

        readings
    }

    fn read_hwmon_name(&self, path: &Path) -> String {
        fs::read_to_string(path.join("name"))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    }

    fn read_temp_sensors(&mut self, hwmon_path: &Path, chip_name: &str) -> Vec<SensorReading> {
        let mut readings = Vec::new();
        let stale_threshold = self.stale_threshold;
        let dead_threshold = self.dead_threshold;

        for i in 1..=16 {
            let input_path = hwmon_path.join(format!("temp{}_input", i));
            if !input_path.exists() {
                continue;
            }

            let value = match Self::read_millidegree(&input_path) {
                Some(v) => v,
                None => continue,
            };

            let name = format!("{}/temp{}", chip_name, i);
            let label = Self::read_label(hwmon_path, &format!("temp{}_label", i))
                .unwrap_or_else(|| format!("Temp {}", i));
            let crit = Self::read_millidegree(&hwmon_path.join(format!("temp{}_crit", i)));
            let max = Self::read_millidegree(&hwmon_path.join(format!("temp{}_max", i)));

            // Update history
            let history = self.histories.entry(name.clone()).or_insert_with(SensorHistory::new);
            history.push(value);

            let headroom = crit.or(max).map(|limit| limit - value);
            let is_outlier = history.is_outlier(value);
            let drift_rate = history.drift_rate();
            let health = Self::calculate_health(history, value, crit, max, SensorType::Temperature, stale_threshold, dead_threshold);

            readings.push(SensorReading {
                name,
                label,
                sensor_type: SensorType::Temperature,
                value,
                crit,
                max,
                min: None,
                health,
                headroom,
                is_outlier,
                drift_rate,
            });
        }

        readings
    }

    fn read_fan_sensors(&mut self, hwmon_path: &Path, chip_name: &str) -> Vec<SensorReading> {
        let mut readings = Vec::new();
        let stale_threshold = self.stale_threshold;
        let dead_threshold = self.dead_threshold;

        for i in 1..=8 {
            let input_path = hwmon_path.join(format!("fan{}_input", i));
            if !input_path.exists() {
                continue;
            }

            let value = match Self::read_raw(&input_path) {
                Some(v) => v,
                None => continue,
            };

            let name = format!("{}/fan{}", chip_name, i);
            let label = Self::read_label(hwmon_path, &format!("fan{}_label", i))
                .unwrap_or_else(|| format!("Fan {}", i));
            let min = Self::read_raw(&hwmon_path.join(format!("fan{}_min", i)));
            let max = Self::read_raw(&hwmon_path.join(format!("fan{}_max", i)));

            let history = self.histories.entry(name.clone()).or_insert_with(SensorHistory::new);
            history.push(value);

            let is_outlier = history.is_outlier(value);
            let drift_rate = history.drift_rate();
            let health = Self::calculate_health(history, value, None, max, SensorType::Fan, stale_threshold, dead_threshold);

            readings.push(SensorReading {
                name,
                label,
                sensor_type: SensorType::Fan,
                value,
                crit: None,
                max,
                min,
                health,
                headroom: None,
                is_outlier,
                drift_rate,
            });
        }

        readings
    }

    fn read_voltage_sensors(&mut self, hwmon_path: &Path, chip_name: &str) -> Vec<SensorReading> {
        let mut readings = Vec::new();
        let stale_threshold = self.stale_threshold;
        let dead_threshold = self.dead_threshold;

        for i in 0..=16 {
            let input_path = hwmon_path.join(format!("in{}_input", i));
            if !input_path.exists() {
                continue;
            }

            // Voltage is in millivolts
            let value = match Self::read_milliunit(&input_path) {
                Some(v) => v,
                None => continue,
            };

            let name = format!("{}/in{}", chip_name, i);
            let label = Self::read_label(hwmon_path, &format!("in{}_label", i))
                .unwrap_or_else(|| format!("Voltage {}", i));
            let min = Self::read_milliunit(&hwmon_path.join(format!("in{}_min", i)));
            let max = Self::read_milliunit(&hwmon_path.join(format!("in{}_max", i)));

            let history = self.histories.entry(name.clone()).or_insert_with(SensorHistory::new);
            history.push(value);

            let is_outlier = history.is_outlier(value);
            let drift_rate = history.drift_rate();
            let health = Self::calculate_health(history, value, None, max, SensorType::Voltage, stale_threshold, dead_threshold);

            readings.push(SensorReading {
                name,
                label,
                sensor_type: SensorType::Voltage,
                value,
                crit: None,
                max,
                min,
                health,
                headroom: None,
                is_outlier,
                drift_rate,
            });
        }

        readings
    }

    fn read_power_sensors(&mut self, hwmon_path: &Path, chip_name: &str) -> Vec<SensorReading> {
        let mut readings = Vec::new();
        let stale_threshold = self.stale_threshold;
        let dead_threshold = self.dead_threshold;

        for i in 1..=8 {
            let input_path = hwmon_path.join(format!("power{}_input", i));
            if !input_path.exists() {
                continue;
            }

            // Power is in microwatts
            let value = match Self::read_microunit(&input_path) {
                Some(v) => v,
                None => continue,
            };

            let name = format!("{}/power{}", chip_name, i);
            let label = Self::read_label(hwmon_path, &format!("power{}_label", i))
                .unwrap_or_else(|| format!("Power {}", i));
            let max = Self::read_microunit(&hwmon_path.join(format!("power{}_max", i)));
            let crit = Self::read_microunit(&hwmon_path.join(format!("power{}_crit", i)));

            let history = self.histories.entry(name.clone()).or_insert_with(SensorHistory::new);
            history.push(value);

            let is_outlier = history.is_outlier(value);
            let drift_rate = history.drift_rate();
            let health = Self::calculate_health(history, value, crit, max, SensorType::Power, stale_threshold, dead_threshold);

            readings.push(SensorReading {
                name,
                label,
                sensor_type: SensorType::Power,
                value,
                crit,
                max,
                min: None,
                health,
                headroom: crit.or(max).map(|limit| limit - value),
                is_outlier,
                drift_rate,
            });
        }

        readings
    }

    fn calculate_health(
        history: &SensorHistory,
        value: f64,
        crit: Option<f64>,
        max: Option<f64>,
        sensor_type: SensorType,
        stale_threshold: Duration,
        dead_threshold: Duration,
    ) -> SensorHealth {
        // Check staleness first
        let age = history.last_seen.elapsed();
        if age > dead_threshold {
            return SensorHealth::Dead;
        }
        if age > stale_threshold {
            return SensorHealth::Stale;
        }

        // Check against thresholds
        if let Some(c) = crit {
            if value >= c {
                return SensorHealth::Critical;
            }
        }

        if let Some(m) = max {
            // Warning at 90% of max
            if value >= m {
                return SensorHealth::Critical;
            }
            if value >= m * 0.9 {
                return SensorHealth::Warning;
            }
        }

        // Type-specific checks
        match sensor_type {
            SensorType::Temperature => {
                // Absolute temperature thresholds
                if value >= 95.0 {
                    return SensorHealth::Critical;
                }
                if value >= 85.0 {
                    return SensorHealth::Warning;
                }
            }
            SensorType::Fan => {
                // Fan stopped might be critical (if it should be running)
                if value < 1.0 && history.values.iter().any(|&v| v > 100.0) {
                    return SensorHealth::Critical;
                }
            }
            _ => {}
        }

        // Check for rapid drift (>5 units/min for temp, >100 for fan)
        if let Some(drift) = history.drift_rate() {
            let drift_threshold = match sensor_type {
                SensorType::Temperature => 5.0,
                SensorType::Fan => 500.0,
                SensorType::Voltage => 0.5,
                SensorType::Power => 50.0,
                _ => 10.0,
            };
            if drift.abs() > drift_threshold {
                return SensorHealth::Warning;
            }
        }

        SensorHealth::Healthy
    }

    fn read_millidegree(path: &Path) -> Option<f64> {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| s.trim().parse::<i64>().ok())
            .map(|v| v as f64 / 1000.0)
    }

    fn read_milliunit(path: &Path) -> Option<f64> {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| s.trim().parse::<i64>().ok())
            .map(|v| v as f64 / 1000.0)
    }

    fn read_microunit(path: &Path) -> Option<f64> {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| s.trim().parse::<i64>().ok())
            .map(|v| v as f64 / 1_000_000.0)
    }

    fn read_raw(path: &Path) -> Option<f64> {
        fs::read_to_string(path)
            .ok()
            .and_then(|s| s.trim().parse::<f64>().ok())
    }

    fn read_label(hwmon_path: &Path, label_file: &str) -> Option<String> {
        fs::read_to_string(hwmon_path.join(label_file))
            .ok()
            .map(|s| s.trim().to_string())
    }
}

impl Default for SensorHealthAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_sensor_type_units() {
        assert_eq!(SensorType::Temperature.unit(), "°C");
        assert_eq!(SensorType::Fan.unit(), "RPM");
        assert_eq!(SensorType::Voltage.unit(), "V");
        assert_eq!(SensorType::Power.unit(), "W");
        assert_eq!(SensorType::Current.unit(), "A");
        assert_eq!(SensorType::Humidity.unit(), "%");
    }

    #[test]
    fn test_sensor_type_icons() {
        assert_eq!(SensorType::Temperature.icon(), "🌡");
        assert_eq!(SensorType::Fan.icon(), "🌀");
        assert_eq!(SensorType::Voltage.icon(), "⚡");
        assert_eq!(SensorType::Power.icon(), "🔌");
    }

    #[test]
    fn test_sensor_health_ordering() {
        assert!(SensorHealth::Healthy < SensorHealth::Warning);
        assert!(SensorHealth::Warning < SensorHealth::Critical);
        assert!(SensorHealth::Critical < SensorHealth::Stale);
        assert!(SensorHealth::Stale < SensorHealth::Dead);
    }

    #[test]
    fn test_sensor_health_symbols() {
        assert_eq!(SensorHealth::Healthy.symbol(), "●");
        assert_eq!(SensorHealth::Warning.symbol(), "◐");
        assert_eq!(SensorHealth::Critical.symbol(), "○");
        assert_eq!(SensorHealth::Stale.symbol(), "◌");
        assert_eq!(SensorHealth::Dead.symbol(), "✗");
    }

    #[test]
    fn test_sensor_health_colors() {
        assert_eq!(SensorHealth::Healthy.color_hint(), "green");
        assert_eq!(SensorHealth::Warning.color_hint(), "yellow");
        assert_eq!(SensorHealth::Critical.color_hint(), "red");
    }

    #[test]
    fn test_history_median() {
        let data = vec![1.0, 3.0, 5.0, 7.0, 9.0];
        assert!((SensorHistory::median(&data) - 5.0).abs() < 0.001);

        let data_even = vec![1.0, 3.0, 5.0, 7.0];
        assert!((SensorHistory::median(&data_even) - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_history_mad() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let median = SensorHistory::median(&data);
        let mad = SensorHistory::mad(&data, median);
        assert!((mad - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_history_outlier_detection() {
        let mut history = SensorHistory::new();

        // Add normal values
        for i in 0..20 {
            history.push(50.0 + (i as f64 % 3.0)); // 50, 51, 52, 50, 51, 52...
        }

        // Normal value should not be outlier
        assert!(!history.is_outlier(51.0));

        // Extreme value should be outlier
        assert!(history.is_outlier(100.0));
        assert!(history.is_outlier(10.0));
    }

    #[test]
    fn test_history_drift_rate() {
        let mut history = SensorHistory::new();

        // Not enough data
        history.push(50.0);
        assert!(history.drift_rate().is_none());

        // Add more data with clear upward trend
        // (Note: timestamps are real-time, so drift calculation depends on actual time)
        for i in 0..10 {
            history.push(50.0 + i as f64);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let drift = history.drift_rate();
        assert!(drift.is_some());
        // Drift should be positive (increasing)
        assert!(drift.unwrap() > 0.0);
    }

    #[test]
    fn test_analyzer_creation() {
        let analyzer = SensorHealthAnalyzer::new();
        assert!(!analyzer.any_critical());
    }

    #[test]
    fn test_analyzer_default() {
        let analyzer = SensorHealthAnalyzer::default();
        assert!(analyzer.thermal_summary().is_none());
    }

    #[test]
    fn test_analyzer_collect_safe() {
        let mut analyzer = SensorHealthAnalyzer::new();
        // Should not panic even if hwmon doesn't exist
        let readings = analyzer.collect();
        // May or may not have readings depending on system
        let _ = readings;
    }

    #[test]
    fn test_analyzer_by_health() {
        let analyzer = SensorHealthAnalyzer::new();
        let grouped = analyzer.by_health();
        // Should return a valid HashMap (may be empty)
        assert!(grouped.len() <= 5); // At most 5 health states
    }

    #[test]
    fn test_sensor_reading_structure() {
        let reading = SensorReading {
            name: "test/temp1".to_string(),
            label: "CPU".to_string(),
            sensor_type: SensorType::Temperature,
            value: 45.0,
            crit: Some(100.0),
            max: Some(90.0),
            min: None,
            health: SensorHealth::Healthy,
            headroom: Some(55.0),
            is_outlier: false,
            drift_rate: Some(0.5),
        };

        assert_eq!(reading.name, "test/temp1");
        assert_eq!(reading.value, 45.0);
        assert_eq!(reading.headroom, Some(55.0));
    }

    #[test]
    fn test_sensor_reading_full_struct() {
        let reading = SensorReading {
            name: "hwmon0/fan1".to_string(),
            label: "CPU Fan".to_string(),
            sensor_type: SensorType::Fan,
            value: 1500.0,
            crit: None,
            max: Some(3000.0),
            min: Some(500.0),
            health: SensorHealth::Healthy,
            headroom: Some(1500.0),
            is_outlier: false,
            drift_rate: None,
        };

        assert_eq!(reading.sensor_type, SensorType::Fan);
        assert_eq!(reading.min, Some(500.0));
        assert!(!reading.is_outlier);
    }

    #[test]
    fn test_sensor_type_all_units() {
        // Test all variants
        let types = [
            SensorType::Temperature,
            SensorType::Fan,
            SensorType::Voltage,
            SensorType::Power,
            SensorType::Current,
            SensorType::Humidity,
        ];

        for t in types {
            let unit = t.unit();
            assert!(!unit.is_empty());
            let icon = t.icon();
            assert!(!icon.is_empty());
        }
    }

    #[test]
    fn test_sensor_health_all_symbols() {
        let healths = [
            SensorHealth::Healthy,
            SensorHealth::Warning,
            SensorHealth::Critical,
            SensorHealth::Stale,
            SensorHealth::Dead,
        ];

        for h in healths {
            let symbol = h.symbol();
            assert!(!symbol.is_empty());
            let color = h.color_hint();
            assert!(!color.is_empty());
        }
    }

    #[test]
    fn test_history_two_elements_median() {
        let two = vec![10.0, 20.0];
        let median = SensorHistory::median(&two);
        assert!((median - 15.0).abs() < 0.001);
    }

    #[test]
    fn test_history_single_median() {
        let single = vec![42.0];
        let median = SensorHistory::median(&single);
        assert!((median - 42.0).abs() < 0.001);
    }

    #[test]
    fn test_history_outlier_empty() {
        let history = SensorHistory::new();
        // Empty history should not consider anything an outlier
        assert!(!history.is_outlier(50.0));
    }

    #[test]
    fn test_history_outlier_insufficient_data() {
        let mut history = SensorHistory::new();
        history.push(50.0);
        history.push(51.0);
        // With only 2 points, not enough data for outlier detection
        assert!(!history.is_outlier(100.0));
    }

    #[test]
    fn test_analyzer_get_cached_readings() {
        let analyzer = SensorHealthAnalyzer::new();
        let cached = analyzer.get_cached_readings();
        // Initially empty
        assert!(cached.is_empty());
    }

    #[test]
    fn test_analyzer_thermal_summary_empty() {
        let analyzer = SensorHealthAnalyzer::new();
        let summary = analyzer.thermal_summary();
        assert!(summary.is_none());
    }

    #[test]
    fn test_analyzer_any_critical_empty() {
        let analyzer = SensorHealthAnalyzer::new();
        assert!(!analyzer.any_critical());
    }

    #[test]
    fn test_sensor_type_current_unit() {
        assert_eq!(SensorType::Current.unit(), "A");
        assert_eq!(SensorType::Current.icon(), "〰"); // Wavy line
    }

    #[test]
    fn test_sensor_type_humidity_unit() {
        assert_eq!(SensorType::Humidity.unit(), "%");
        assert_eq!(SensorType::Humidity.icon(), "💧");
    }
}
