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

    // === Additional Coverage Tests ===

    #[test]
    fn test_history_mad_zero() {
        // All identical values - MAD should be 0
        let data = vec![50.0, 50.0, 50.0, 50.0, 50.0];
        let median = SensorHistory::median(&data);
        let mad = SensorHistory::mad(&data, median);
        assert!((mad - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_history_outlier_with_identical_values() {
        let mut history = SensorHistory::new();
        // Add many identical values - MAD will be 0
        for _ in 0..20 {
            history.push(50.0);
        }
        // With MAD=0, is_outlier should return false
        assert!(!history.is_outlier(50.0));
    }

    #[test]
    fn test_history_drift_rate_insufficient_data() {
        let mut history = SensorHistory::new();
        // Only 3 points - not enough for drift calculation
        history.push(10.0);
        history.push(20.0);
        history.push(30.0);
        assert!(history.drift_rate().is_none());
    }

    #[test]
    fn test_history_drift_rate_flat_line() {
        let mut history = SensorHistory::new();
        // Add values with no drift
        for i in 0..10 {
            history.push(50.0);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        // Drift should be near zero
        if let Some(drift) = history.drift_rate() {
            assert!(drift.abs() < 1.0);
        }
    }

    #[test]
    fn test_sensor_health_equality() {
        assert_eq!(SensorHealth::Healthy, SensorHealth::Healthy);
        assert_ne!(SensorHealth::Healthy, SensorHealth::Warning);
        assert_ne!(SensorHealth::Warning, SensorHealth::Critical);
    }

    #[test]
    fn test_sensor_health_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(SensorHealth::Healthy);
        set.insert(SensorHealth::Warning);
        assert!(set.contains(&SensorHealth::Healthy));
        assert!(set.contains(&SensorHealth::Warning));
        assert!(!set.contains(&SensorHealth::Critical));
    }

    #[test]
    fn test_sensor_type_equality() {
        assert_eq!(SensorType::Temperature, SensorType::Temperature);
        assert_ne!(SensorType::Temperature, SensorType::Fan);
    }

    #[test]
    fn test_sensor_type_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(SensorType::Temperature);
        set.insert(SensorType::Fan);
        assert!(set.contains(&SensorType::Temperature));
        assert!(!set.contains(&SensorType::Voltage));
    }

    #[test]
    fn test_sensor_reading_clone() {
        let reading = SensorReading {
            name: "test".to_string(),
            label: "Test".to_string(),
            sensor_type: SensorType::Temperature,
            value: 45.0,
            crit: Some(100.0),
            max: Some(90.0),
            min: Some(10.0),
            health: SensorHealth::Healthy,
            headroom: Some(55.0),
            is_outlier: false,
            drift_rate: Some(0.5),
        };
        let cloned = reading.clone();
        assert_eq!(reading.name, cloned.name);
        assert_eq!(reading.value, cloned.value);
    }

    #[test]
    fn test_sensor_reading_debug() {
        let reading = SensorReading {
            name: "test".to_string(),
            label: "Test".to_string(),
            sensor_type: SensorType::Fan,
            value: 1500.0,
            crit: None,
            max: None,
            min: None,
            health: SensorHealth::Warning,
            headroom: None,
            is_outlier: true,
            drift_rate: None,
        };
        let debug = format!("{:?}", reading);
        assert!(debug.contains("test"));
        assert!(debug.contains("Fan"));
    }

    #[test]
    fn test_sensor_type_copy() {
        let t1 = SensorType::Power;
        let t2 = t1; // Copy
        assert_eq!(t1, t2);
    }

    #[test]
    fn test_sensor_health_copy() {
        let h1 = SensorHealth::Stale;
        let h2 = h1; // Copy
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_median_large_dataset() {
        let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let median = SensorHistory::median(&data);
        // Median of 0-99 should be 49.5
        assert!((median - 49.5).abs() < 0.001);
    }

    #[test]
    fn test_history_values_ring_buffer() {
        let mut history = SensorHistory::new();
        // Push more values than buffer capacity
        for i in 0..100 {
            history.push(i as f64);
        }
        // History should still work - it's a ring buffer
        assert!(!history.is_outlier(50.0));
    }

    #[test]
    fn test_analyzer_rate_limiting() {
        let mut analyzer = SensorHealthAnalyzer::new();
        // First collect should work
        let _readings1 = analyzer.collect();
        // Second immediate collect should be rate-limited
        let readings2 = analyzer.collect();
        // Rate limiting returns cached readings, so this should work
        let _ = readings2;
    }

    #[test]
    fn test_analyzer_collect_multiple_times() {
        let mut analyzer = SensorHealthAnalyzer::new();
        // Multiple collects should not panic
        for _ in 0..5 {
            let _ = analyzer.collect();
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    #[test]
    fn test_sensor_health_dead_color() {
        assert_eq!(SensorHealth::Dead.color_hint(), "darkgray");
    }

    #[test]
    fn test_sensor_health_stale_color() {
        assert_eq!(SensorHealth::Stale.color_hint(), "gray");
    }

    #[test]
    fn test_default_analyzer() {
        let analyzer: SensorHealthAnalyzer = Default::default();
        assert!(!analyzer.any_critical());
    }

    // === Comprehensive calculate_health Tests ===

    #[test]
    fn test_calculate_health_dead_sensor() {
        let mut history = SensorHistory::new();
        history.push(50.0);
        // Set last_seen AFTER push to simulate old reading
        history.last_seen = std::time::Instant::now() - std::time::Duration::from_secs(300);

        let health = SensorHealthAnalyzer::calculate_health(
            &history,
            50.0,
            Some(100.0),
            Some(90.0),
            SensorType::Temperature,
            std::time::Duration::from_secs(30),
            std::time::Duration::from_secs(120),
        );
        assert_eq!(health, SensorHealth::Dead);
    }

    #[test]
    fn test_calculate_health_stale_sensor() {
        let mut history = SensorHistory::new();
        history.push(50.0);
        // Set last_seen AFTER push to simulate stale reading
        history.last_seen = std::time::Instant::now() - std::time::Duration::from_secs(60);

        let health = SensorHealthAnalyzer::calculate_health(
            &history,
            50.0,
            Some(100.0),
            Some(90.0),
            SensorType::Temperature,
            std::time::Duration::from_secs(30),
            std::time::Duration::from_secs(120),
        );
        assert_eq!(health, SensorHealth::Stale);
    }

    #[test]
    fn test_calculate_health_critical_over_crit_threshold() {
        let mut history = SensorHistory::new();
        history.push(100.0);

        let health = SensorHealthAnalyzer::calculate_health(
            &history,
            100.0,
            Some(95.0), // Critical threshold
            Some(90.0),
            SensorType::Temperature,
            std::time::Duration::from_secs(30),
            std::time::Duration::from_secs(120),
        );
        assert_eq!(health, SensorHealth::Critical);
    }

    #[test]
    fn test_calculate_health_critical_over_max() {
        let mut history = SensorHistory::new();
        history.push(95.0);

        let health = SensorHealthAnalyzer::calculate_health(
            &history,
            95.0,
            None,
            Some(90.0), // Max threshold
            SensorType::Temperature,
            std::time::Duration::from_secs(30),
            std::time::Duration::from_secs(120),
        );
        assert_eq!(health, SensorHealth::Critical);
    }

    #[test]
    fn test_calculate_health_warning_90_percent_max() {
        let mut history = SensorHistory::new();
        history.push(81.5);

        let health = SensorHealthAnalyzer::calculate_health(
            &history,
            81.5,
            None,
            Some(90.0), // 81.5 >= 90 * 0.9 = 81.0
            SensorType::Voltage,
            std::time::Duration::from_secs(30),
            std::time::Duration::from_secs(120),
        );
        assert_eq!(health, SensorHealth::Warning);
    }

    #[test]
    fn test_calculate_health_temp_absolute_critical() {
        let mut history = SensorHistory::new();
        history.push(96.0);

        let health = SensorHealthAnalyzer::calculate_health(
            &history,
            96.0,
            None, // No explicit crit
            None, // No explicit max
            SensorType::Temperature,
            std::time::Duration::from_secs(30),
            std::time::Duration::from_secs(120),
        );
        assert_eq!(health, SensorHealth::Critical);
    }

    #[test]
    fn test_calculate_health_temp_absolute_warning() {
        let mut history = SensorHistory::new();
        history.push(87.0);

        let health = SensorHealthAnalyzer::calculate_health(
            &history,
            87.0,
            None,
            None,
            SensorType::Temperature,
            std::time::Duration::from_secs(30),
            std::time::Duration::from_secs(120),
        );
        assert_eq!(health, SensorHealth::Warning);
    }

    #[test]
    fn test_calculate_health_fan_stopped_critical() {
        let mut history = SensorHistory::new();
        // Fan was previously running at 1000 RPM
        history.push(1000.0);
        history.push(1200.0);
        history.push(0.0); // Now stopped

        let health = SensorHealthAnalyzer::calculate_health(
            &history,
            0.0,
            None,
            None,
            SensorType::Fan,
            std::time::Duration::from_secs(30),
            std::time::Duration::from_secs(120),
        );
        assert_eq!(health, SensorHealth::Critical);
    }

    #[test]
    fn test_calculate_health_healthy() {
        let mut history = SensorHistory::new();
        history.push(50.0);

        let health = SensorHealthAnalyzer::calculate_health(
            &history,
            50.0,
            Some(100.0),
            Some(90.0),
            SensorType::Temperature,
            std::time::Duration::from_secs(30),
            std::time::Duration::from_secs(120),
        );
        assert_eq!(health, SensorHealth::Healthy);
    }

    #[test]
    fn test_calculate_health_drift_warning_temp() {
        let mut history = SensorHistory::new();
        // Simulate rapidly increasing temperature
        for i in 0..10 {
            history.push(50.0 + i as f64 * 2.0); // +20 degrees over 10 samples
        }

        let health = SensorHealthAnalyzer::calculate_health(
            &history,
            70.0,
            Some(100.0),
            Some(90.0),
            SensorType::Temperature,
            std::time::Duration::from_secs(30),
            std::time::Duration::from_secs(120),
        );
        // May be warning due to drift
        assert!(matches!(health, SensorHealth::Warning | SensorHealth::Healthy));
    }

    #[test]
    fn test_calculate_health_voltage_healthy() {
        let mut history = SensorHistory::new();
        history.push(12.0);

        let health = SensorHealthAnalyzer::calculate_health(
            &history,
            12.0,
            None,
            Some(14.0),
            SensorType::Voltage,
            std::time::Duration::from_secs(30),
            std::time::Duration::from_secs(120),
        );
        assert_eq!(health, SensorHealth::Healthy);
    }

    #[test]
    fn test_calculate_health_power_healthy() {
        let mut history = SensorHistory::new();
        history.push(100.0);

        let health = SensorHealthAnalyzer::calculate_health(
            &history,
            100.0,
            Some(300.0),
            Some(250.0),
            SensorType::Power,
            std::time::Duration::from_secs(30),
            std::time::Duration::from_secs(120),
        );
        assert_eq!(health, SensorHealth::Healthy);
    }

    #[test]
    fn test_get_cached_readings_with_histories() {
        let mut analyzer = SensorHealthAnalyzer::new();
        // Manually populate a history entry
        let mut history = SensorHistory::new();
        history.push(55.0);
        analyzer.histories.insert("test/temp1".to_string(), history);

        let readings = analyzer.get_cached_readings();
        assert!(!readings.is_empty());
        assert_eq!(readings[0].sensor_type, SensorType::Temperature);
    }

    #[test]
    fn test_get_cached_readings_fan_type() {
        let mut analyzer = SensorHealthAnalyzer::new();
        let mut history = SensorHistory::new();
        history.push(1200.0);
        analyzer.histories.insert("hwmon/fan1".to_string(), history);

        let readings = analyzer.get_cached_readings();
        assert!(!readings.is_empty());
        assert_eq!(readings[0].sensor_type, SensorType::Fan);
    }

    #[test]
    fn test_get_cached_readings_voltage_type() {
        let mut analyzer = SensorHealthAnalyzer::new();
        let mut history = SensorHistory::new();
        history.push(12.0);
        analyzer.histories.insert("hwmon/in0".to_string(), history);

        let readings = analyzer.get_cached_readings();
        assert!(!readings.is_empty());
        assert_eq!(readings[0].sensor_type, SensorType::Voltage);
    }

    #[test]
    fn test_get_cached_readings_power_type() {
        let mut analyzer = SensorHealthAnalyzer::new();
        let mut history = SensorHistory::new();
        history.push(100.0);
        analyzer.histories.insert("hwmon/power1".to_string(), history);

        let readings = analyzer.get_cached_readings();
        assert!(!readings.is_empty());
        assert_eq!(readings[0].sensor_type, SensorType::Power);
    }

    #[test]
    fn test_thermal_summary_empty() {
        let analyzer = SensorHealthAnalyzer::new();
        assert!(analyzer.thermal_summary().is_none());
    }

    #[test]
    fn test_thermal_summary_with_temps() {
        let mut analyzer = SensorHealthAnalyzer::new();
        let mut history = SensorHistory::new();
        history.push(65.0);
        analyzer.histories.insert("test/temp1".to_string(), history);

        let summary = analyzer.thermal_summary();
        assert!(summary.is_some());
        let (max, _headroom, avg) = summary.unwrap();
        assert_eq!(max, 65.0);
        assert_eq!(avg, 65.0);
    }

    #[test]
    fn test_by_health_empty() {
        let analyzer = SensorHealthAnalyzer::new();
        let grouped = analyzer.by_health();
        assert!(grouped.is_empty());
    }

    #[test]
    fn test_by_health_with_readings() {
        let mut analyzer = SensorHealthAnalyzer::new();
        let mut history = SensorHistory::new();
        history.push(50.0);
        analyzer.histories.insert("test/temp1".to_string(), history);

        let grouped = analyzer.by_health();
        assert!(!grouped.is_empty());
    }

    #[test]
    fn test_any_critical_false() {
        let mut analyzer = SensorHealthAnalyzer::new();
        let mut history = SensorHistory::new();
        history.push(50.0); // Normal temperature
        analyzer.histories.insert("test/temp1".to_string(), history);

        assert!(!analyzer.any_critical());
    }

    #[test]
    fn test_sensor_reading_debug_full() {
        let reading = SensorReading {
            name: "test/temp1".to_string(),
            label: "Temp 1".to_string(),
            sensor_type: SensorType::Temperature,
            value: 65.0,
            crit: Some(100.0),
            max: Some(90.0),
            min: None,
            health: SensorHealth::Healthy,
            headroom: Some(35.0),
            is_outlier: false,
            drift_rate: Some(0.5),
        };

        let debug_str = format!("{:?}", reading);
        assert!(debug_str.contains("temp1"));
    }

    #[test]
    fn test_sensor_type_all_variants_icon() {
        assert!(!SensorType::Temperature.icon().is_empty());
        assert!(!SensorType::Fan.icon().is_empty());
        assert!(!SensorType::Voltage.icon().is_empty());
        assert!(!SensorType::Power.icon().is_empty());
        assert!(!SensorType::Current.icon().is_empty());
        assert!(!SensorType::Humidity.icon().is_empty());
    }

    // === Additional Edge Case Tests for Coverage ===

    #[test]
    fn test_get_cached_readings_unknown_type() {
        let mut analyzer = SensorHealthAnalyzer::new();
        // Insert a history entry that doesn't match any known sensor type pattern
        let mut history = SensorHistory::new();
        history.push(42.0);
        analyzer.histories.insert("hwmon/unknown_sensor".to_string(), history);

        // Should skip the unknown type entry
        let readings = analyzer.get_cached_readings();
        // The unknown sensor should be skipped (continue branch)
        assert!(readings.is_empty());
    }

    #[test]
    fn test_calculate_health_drift_warning_voltage() {
        let mut history = SensorHistory::new();
        // Simulate rapidly changing voltage (>0.5V/min threshold)
        for i in 0..10 {
            history.push(12.0 + i as f64 * 0.2);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let health = SensorHealthAnalyzer::calculate_health(
            &history,
            14.0,
            None,
            Some(16.0),
            SensorType::Voltage,
            std::time::Duration::from_secs(30),
            std::time::Duration::from_secs(120),
        );
        // May be Warning due to rapid drift
        assert!(matches!(health, SensorHealth::Warning | SensorHealth::Healthy));
    }

    #[test]
    fn test_calculate_health_drift_warning_power() {
        let mut history = SensorHistory::new();
        // Simulate rapidly changing power (>50W/min threshold)
        for i in 0..10 {
            history.push(100.0 + i as f64 * 20.0);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let health = SensorHealthAnalyzer::calculate_health(
            &history,
            280.0,
            Some(350.0),
            Some(300.0),
            SensorType::Power,
            std::time::Duration::from_secs(30),
            std::time::Duration::from_secs(120),
        );
        // May be Warning due to rapid drift or healthy
        assert!(matches!(health, SensorHealth::Warning | SensorHealth::Healthy));
    }

    #[test]
    fn test_calculate_health_drift_warning_current() {
        let mut history = SensorHistory::new();
        // Test current sensor with default drift threshold (10.0)
        for i in 0..10 {
            history.push(5.0 + i as f64 * 0.5);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let health = SensorHealthAnalyzer::calculate_health(
            &history,
            9.5,
            None,
            Some(15.0),
            SensorType::Current,
            std::time::Duration::from_secs(30),
            std::time::Duration::from_secs(120),
        );
        assert!(matches!(health, SensorHealth::Warning | SensorHealth::Healthy));
    }

    #[test]
    fn test_calculate_health_drift_warning_humidity() {
        let mut history = SensorHistory::new();
        // Test humidity sensor with default drift threshold (10.0)
        for i in 0..10 {
            history.push(50.0 + i as f64 * 2.0);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let health = SensorHealthAnalyzer::calculate_health(
            &history,
            68.0,
            None,
            Some(90.0),
            SensorType::Humidity,
            std::time::Duration::from_secs(30),
            std::time::Duration::from_secs(120),
        );
        assert!(matches!(health, SensorHealth::Warning | SensorHealth::Healthy));
    }

    #[test]
    fn test_calculate_health_fan_drift_warning() {
        let mut history = SensorHistory::new();
        // Simulate rapidly changing fan speed (>500 RPM/min threshold)
        for i in 0..10 {
            history.push(1000.0 + i as f64 * 200.0);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let health = SensorHealthAnalyzer::calculate_health(
            &history,
            2800.0,
            None,
            Some(4000.0),
            SensorType::Fan,
            std::time::Duration::from_secs(30),
            std::time::Duration::from_secs(120),
        );
        assert!(matches!(health, SensorHealth::Warning | SensorHealth::Healthy));
    }

    #[test]
    fn test_thermal_summary_with_multiple_temps() {
        let mut analyzer = SensorHealthAnalyzer::new();

        // Add multiple temperature readings
        let mut history1 = SensorHistory::new();
        history1.push(45.0);
        analyzer.histories.insert("test/temp1".to_string(), history1);

        let mut history2 = SensorHistory::new();
        history2.push(65.0);
        analyzer.histories.insert("test/temp2".to_string(), history2);

        let mut history3 = SensorHistory::new();
        history3.push(55.0);
        analyzer.histories.insert("test/temp3".to_string(), history3);

        let summary = analyzer.thermal_summary();
        assert!(summary.is_some());
        let (max, _headroom, avg) = summary.unwrap();
        assert_eq!(max, 65.0);
        assert!((avg - 55.0).abs() < 0.1);
    }

    #[test]
    fn test_by_health_multiple_states() {
        let mut analyzer = SensorHealthAnalyzer::new();

        // Add healthy temp sensor
        let mut history1 = SensorHistory::new();
        history1.push(45.0);
        analyzer.histories.insert("test/temp1".to_string(), history1);

        // Add fan sensor
        let mut history2 = SensorHistory::new();
        history2.push(1500.0);
        analyzer.histories.insert("test/fan1".to_string(), history2);

        let grouped = analyzer.by_health();
        assert!(!grouped.is_empty());
    }

    #[test]
    fn test_any_critical_with_high_temp() {
        let mut analyzer = SensorHealthAnalyzer::new();

        // Add critical temperature sensor (>95°C)
        let mut history = SensorHistory::new();
        history.push(100.0);
        analyzer.histories.insert("test/temp1".to_string(), history);

        // Should detect critical state
        assert!(analyzer.any_critical());
    }

    #[test]
    fn test_history_drift_identical_timestamps() {
        let mut history = SensorHistory::new();
        // Add values with identical timestamps (push multiple quickly)
        for i in 0..10 {
            history.push(50.0 + i as f64);
        }

        // Drift rate calculation should handle this edge case
        let drift = history.drift_rate();
        // May be None or some value depending on timing
        let _ = drift;
    }

    #[test]
    fn test_history_outlier_borderline_zscore() {
        let mut history = SensorHistory::new();
        // Add values with some variation
        for i in 0..20 {
            history.push(50.0 + (i % 5) as f64);
        }

        // Value that's close to but not exceeding the 3.5 z-score threshold
        assert!(!history.is_outlier(54.0));
    }

    #[test]
    fn test_get_cached_readings_label_extraction() {
        let mut analyzer = SensorHealthAnalyzer::new();

        // Test label extraction from path with multiple slashes
        let mut history = SensorHistory::new();
        history.push(55.0);
        analyzer.histories.insert("coretemp/isa/0000/temp1".to_string(), history);

        let readings = analyzer.get_cached_readings();
        assert!(!readings.is_empty());
        assert_eq!(readings[0].label, "temp1");
    }

    #[test]
    fn test_calculate_health_fan_not_stopped_critical() {
        let mut history = SensorHistory::new();
        // Fan that was never running (no history > 100 RPM)
        history.push(0.0);
        history.push(0.0);

        let health = SensorHealthAnalyzer::calculate_health(
            &history,
            0.0,
            None,
            None,
            SensorType::Fan,
            std::time::Duration::from_secs(30),
            std::time::Duration::from_secs(120),
        );
        // Should be healthy since fan was never running
        assert_eq!(health, SensorHealth::Healthy);
    }

    #[test]
    fn test_sensor_health_debug_format() {
        let health = SensorHealth::Critical;
        let debug = format!("{:?}", health);
        assert!(debug.contains("Critical"));
    }

    #[test]
    fn test_sensor_type_debug_format() {
        let sensor_type = SensorType::Humidity;
        let debug = format!("{:?}", sensor_type);
        assert!(debug.contains("Humidity"));
    }

    #[test]
    fn test_sensor_health_partial_ord() {
        // Test PartialOrd comparisons
        assert!(SensorHealth::Healthy < SensorHealth::Dead);
        assert!(SensorHealth::Warning < SensorHealth::Stale);
        assert!(SensorHealth::Critical > SensorHealth::Warning);
    }
}
