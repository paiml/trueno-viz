//! Application state and logic for ttop.

use crossterm::event::{KeyCode, KeyModifiers};
use std::time::{Duration, Instant};

use trueno_viz::monitor::collectors::{
    BatteryCollector, CpuCollector, DiskCollector, MemoryCollector, NetworkCollector,
    ProcessCollector, SensorCollector,
};
use trueno_viz::monitor::types::Collector;

#[cfg(feature = "nvidia")]
use trueno_viz::monitor::collectors::NvidiaGpuCollector;

#[cfg(target_os = "linux")]
use trueno_viz::monitor::collectors::AmdGpuCollector;

#[cfg(target_os = "macos")]
use trueno_viz::monitor::collectors::AppleGpuCollector;

use crate::analyzers::{ContainerAnalyzer, DiskEntropyAnalyzer, DiskIoAnalyzer, GpuProcessAnalyzer, NetworkStatsAnalyzer, PsiAnalyzer, SensorHealthAnalyzer, StorageAnalyzer, SwapAnalyzer, ThrashingSeverity};
use crate::state::{PanelType, ProcessSortColumn, SignalType};

/// Allocation-free case-insensitive substring search.
///
/// Checks if `haystack` contains `needle` (which must already be lowercase).
/// This avoids the O(n) allocation of `haystack.to_lowercase()` on every call.
#[inline]
fn contains_ignore_case(haystack: &str, needle_lower: &str) -> bool {
    if needle_lower.is_empty() {
        return true;
    }
    if haystack.len() < needle_lower.len() {
        return false;
    }

    // Fast path for ASCII-only strings (common for process names)
    let needle_bytes = needle_lower.as_bytes();
    let haystack_bytes = haystack.as_bytes();

    'outer: for i in 0..=(haystack_bytes.len() - needle_bytes.len()) {
        for (j, &nb) in needle_bytes.iter().enumerate() {
            let hb = haystack_bytes[i + j];
            // Compare lowercase ASCII bytes
            let hb_lower = if hb.is_ascii_uppercase() { hb + 32 } else { hb };
            if hb_lower != nb {
                continue 'outer;
            }
        }
        return true;
    }
    false
}

/// Mock GPU data for testing panel rendering
#[derive(Debug, Clone)]
pub struct MockGpuData {
    pub name: String,
    pub gpu_util: f64,
    pub vram_used: u64,
    pub vram_total: u64,
    pub temperature: f64,
    pub power_watts: u32,
    pub power_limit_watts: u32,
    pub clock_mhz: u32,
    pub history: Vec<f64>,
}

/// Mock battery data for testing panel rendering
#[derive(Debug, Clone)]
pub struct MockBatteryData {
    pub percent: f64,
    pub charging: bool,
    pub time_remaining_mins: Option<u32>,
    pub power_watts: f64,
    pub health_percent: f64,
    pub cycle_count: u32,
}

/// Mock sensor data for testing panel rendering
#[derive(Debug, Clone)]
pub struct MockSensorData {
    pub name: String,
    pub label: String,
    pub value: f64,
    pub max: Option<f64>,
    pub crit: Option<f64>,
    pub sensor_type: MockSensorType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MockSensorType {
    Temperature,
    Fan,
    Voltage,
    Power,
}

/// Mock container data for testing panel rendering
#[derive(Debug, Clone)]
pub struct MockContainerData {
    pub name: String,
    pub status: String,
    pub cpu_percent: f64,
    pub mem_used: u64,
    pub mem_limit: u64,
}

/// Panel visibility state
#[derive(Debug, Clone, Copy)]
pub struct PanelVisibility {
    pub cpu: bool,
    pub memory: bool,
    pub disk: bool,
    pub network: bool,
    pub process: bool,
    pub gpu: bool,
    pub battery: bool,
    pub sensors: bool,
    pub files: bool,
}

impl Default for PanelVisibility {
    fn default() -> Self {
        Self {
            cpu: true,
            memory: true,
            disk: true,
            network: true,
            process: true,
            gpu: true,
            battery: true,
            sensors: true,
            files: false, // Off by default, toggle with '9'
        }
    }
}

/// CPU core state breakdown (user/system/iowait/idle percentages)
#[derive(Debug, Clone, Default)]
pub struct CpuCoreState {
    pub user: f64,
    pub system: f64,
    pub iowait: f64,
    pub idle: f64,
}

impl CpuCoreState {
    /// Total busy percentage (user + system + iowait)
    pub fn total_busy(&self) -> f64 {
        self.user + self.system + self.iowait
    }
}

/// Top process info for a core
#[derive(Debug, Clone, Default)]
pub struct TopProcessForCore {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f64,
}

/// Memory breakdown categories
#[derive(Debug, Clone, Default)]
pub struct MemoryBreakdown {
    pub used_bytes: u64,
    pub cached_bytes: u64,
    pub buffers_bytes: u64,
    pub free_bytes: u64,
}

/// Swap usage trend indicator
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SwapTrend {
    Rising,
    #[default]
    Stable,
    Falling,
}

impl SwapTrend {
    /// Symbol for the trend
    pub fn symbol(&self) -> &'static str {
        match self {
            SwapTrend::Rising => "↑",
            SwapTrend::Stable => "→",
            SwapTrend::Falling => "↓",
        }
    }
}

/// Top memory consumer info
#[derive(Debug, Clone, Default)]
pub struct TopMemConsumer {
    pub pid: u32,
    pub name: String,
    pub mem_bytes: u64,
    pub mem_percent: f64,
}

/// Disk health status
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DiskHealth {
    #[default]
    Good,
    Warning,
    Critical,
    Unknown,
}

impl DiskHealth {
    /// Symbol for disk health
    pub fn symbol(&self) -> &'static str {
        match self {
            DiskHealth::Good => "✓",
            DiskHealth::Warning => "⚠",
            DiskHealth::Critical => "✗",
            DiskHealth::Unknown => "?",
        }
    }
}

/// Disk health status for a device
#[derive(Debug, Clone, Default)]
pub struct DiskHealthStatus {
    pub device: String,
    pub status: DiskHealth,
    pub temperature: Option<f64>,
    pub reallocated_sectors: u64,
}

/// Main application state
pub struct App {
    // Collectors
    pub cpu: CpuCollector,
    pub memory: MemoryCollector,
    pub disk: DiskCollector,
    pub network: NetworkCollector,
    pub process: ProcessCollector,
    pub sensors: SensorCollector,
    pub battery: BatteryCollector,

    #[cfg(feature = "nvidia")]
    pub nvidia_gpu: NvidiaGpuCollector,

    #[cfg(target_os = "linux")]
    pub amd_gpu: AmdGpuCollector,

    #[cfg(target_os = "macos")]
    pub apple_gpu: AppleGpuCollector,

    // Advanced analyzers (ttop-improve.md spec)
    pub swap_analyzer: SwapAnalyzer,
    pub disk_io_analyzer: DiskIoAnalyzer,
    pub storage_analyzer: StorageAnalyzer,
    pub connection_analyzer: crate::analyzers::ConnectionAnalyzer,
    pub treemap_analyzer: crate::analyzers::TreemapAnalyzer,
    pub gpu_process_analyzer: GpuProcessAnalyzer,
    pub psi_analyzer: PsiAnalyzer,
    pub container_analyzer: ContainerAnalyzer,
    pub network_stats: NetworkStatsAnalyzer,
    pub disk_entropy: DiskEntropyAnalyzer,
    pub process_extra: crate::analyzers::ProcessExtraAnalyzer,
    pub file_analyzer: crate::analyzers::FileAnalyzer,
    pub sensor_health: SensorHealthAnalyzer,

    // History buffers (normalized 0-1)
    pub cpu_history: Vec<f64>,
    pub mem_history: Vec<f64>,
    pub mem_available_history: Vec<f64>,
    pub mem_cached_history: Vec<f64>,
    pub mem_free_history: Vec<f64>,
    pub swap_history: Vec<f64>,
    pub net_rx_history: Vec<f64>,
    pub net_tx_history: Vec<f64>,
    pub per_core_percent: Vec<f64>,

    // CPU Exploded Mode Features (SPEC-TDD)
    /// Per-core CPU history (60 samples per core for sparklines)
    pub per_core_history: Vec<Vec<f64>>,
    /// Per-core state breakdown (user/system/iowait/idle)
    pub per_core_state: Vec<CpuCoreState>,
    /// Frequency history (average MHz over time)
    pub freq_history: Vec<f64>,
    /// Top process consuming each core
    pub top_process_per_core: Vec<TopProcessForCore>,
    /// Thermal throttling active (None if unknown)
    pub thermal_throttle_active: Option<bool>,

    // Memory Exploded Mode Features (SPEC-TDD)
    /// Memory pressure history (PSI avg10 values, 60 samples)
    pub mem_pressure_history: Vec<f64>,
    /// Memory reclaim rate (pages/sec)
    pub mem_reclaim_rate: f64,
    /// Top memory consumers
    pub top_mem_consumers: Vec<TopMemConsumer>,
    /// Swap usage trend
    pub swap_trend: SwapTrend,

    // Disk Exploded Mode Features (SPEC-TDD)
    /// Disk latency history (60 samples)
    pub disk_latency_history: Vec<f64>,
    /// Read IOPS
    pub disk_read_iops: f64,
    /// Write IOPS
    pub disk_write_iops: f64,
    /// Queue depth
    pub disk_queue_depth: f64,
    /// Disk health status
    pub disk_health: Vec<DiskHealthStatus>,

    // Latest memory values (bytes)
    pub mem_total: u64,
    pub mem_used: u64,
    pub mem_available: u64,
    pub mem_cached: u64,
    pub mem_free: u64,
    pub swap_total: u64,
    pub swap_used: u64,

    // Network totals (bytes)
    pub net_rx_total: u64,
    pub net_tx_total: u64,
    pub net_interface_ip: String,

    // Network peak tracking
    pub net_rx_peak: f64,
    pub net_tx_peak: f64,
    pub net_rx_peak_time: Instant,
    pub net_tx_peak_time: Instant,

    // Network Exploded Mode Features (SPEC-TDD)
    /// Network errors count
    pub net_errors: u64,
    /// Network drops count
    pub net_drops: u64,
    /// Established connection count
    pub net_established: u64,
    /// Listening port count
    pub net_listening: u64,

    // UI state
    pub panels: PanelVisibility,
    pub process_selected: usize,
    pub process_scroll_offset: usize,
    pub sort_column: ProcessSortColumn,
    pub sort_descending: bool,
    pub filter: String,
    pub show_filter_input: bool,
    pub show_help: bool,
    pub show_tree: bool,

    // Signal/kill mode
    pub show_signal_menu: bool,
    pub pending_signal: Option<(u32, String, SignalType)>, // (pid, name, signal)
    pub signal_result: Option<(bool, String, Instant)>,     // (success, message, timestamp)

    // Panel focus/explode state
    pub focused_panel: Option<PanelType>,
    pub exploded_panel: Option<PanelType>,

    // Files panel view mode (SIZE/ENTROPY/IO)
    pub files_view_mode: crate::state::FilesViewMode,

    // Frame timing
    pub frame_id: u64,
    pub last_collect: Instant,
    pub avg_frame_time_us: u64,
    pub max_frame_time_us: u64,
    pub show_fps: bool,

    // Mode flags
    pub deterministic: bool,

    // Mock data for testing (dependency injection)
    pub mock_gpus: Vec<MockGpuData>,
    pub mock_battery: Option<MockBatteryData>,
    pub mock_sensors: Vec<MockSensorData>,
    pub mock_containers: Vec<MockContainerData>,
}

impl App {
    /// Get current swap thrashing severity
    pub fn thrashing_severity(&self) -> ThrashingSeverity {
        self.swap_analyzer.detect_thrashing()
    }

    /// Check if system has ZRAM configured
    pub fn has_zram(&self) -> bool {
        self.swap_analyzer.has_zram()
    }

    /// Get ZRAM compression ratio (combined across all devices)
    pub fn zram_ratio(&self) -> f64 {
        self.swap_analyzer.zram_compression_ratio()
    }
}

impl App {
    /// Create a new application instance
    pub fn new(deterministic: bool, show_fps: bool) -> Self {
        use trueno_viz::monitor::debug::{self, Level, TimingGuard};

        debug::log(Level::Debug, "app", "Initializing CPU collector...");
        let _t = TimingGuard::new("app", "CpuCollector::new");
        let cpu = CpuCollector::new();
        drop(_t);
        debug::log(Level::Info, "app", &format!("CPU: {} cores", cpu.core_count()));

        debug::log(Level::Debug, "app", "Initializing Memory collector...");
        let memory = MemoryCollector::new();

        debug::log(Level::Debug, "app", "Initializing Disk collector...");
        let disk = DiskCollector::new();

        debug::log(Level::Debug, "app", "Initializing Network collector...");
        let network = NetworkCollector::new();

        debug::log(Level::Debug, "app", "Initializing Process collector...");
        let process = ProcessCollector::new();

        debug::log(Level::Debug, "app", "Initializing Sensors collector...");
        let sensors = SensorCollector::new();

        debug::log(Level::Debug, "app", "Initializing Battery collector...");
        let battery = BatteryCollector::new();

        #[cfg(feature = "nvidia")]
        let nvidia_gpu = {
            debug::log(Level::Debug, "app", "Initializing NVIDIA GPU collector...");
            NvidiaGpuCollector::new()
        };

        #[cfg(target_os = "linux")]
        let amd_gpu = {
            debug::log(Level::Debug, "app", "Initializing AMD GPU collector...");
            AmdGpuCollector::new()
        };

        #[cfg(target_os = "macos")]
        let apple_gpu = {
            debug::log(Level::Debug, "app", "Initializing Apple GPU collector...");
            let _t = TimingGuard::new("app", "AppleGpuCollector::new");
            let g = AppleGpuCollector::new();
            drop(_t);
            debug::log(Level::Info, "app", &format!("Apple GPU: {} devices", g.gpus().len()));
            g
        };

        debug::log(Level::Debug, "app", "All collectors initialized");

        let mut app = Self {
            cpu,
            memory,
            disk,
            network,
            process,
            sensors,
            battery,

            #[cfg(feature = "nvidia")]
            nvidia_gpu,

            #[cfg(target_os = "linux")]
            amd_gpu,

            #[cfg(target_os = "macos")]
            apple_gpu,

            // Initialize advanced analyzers
            swap_analyzer: SwapAnalyzer::default(),
            disk_io_analyzer: DiskIoAnalyzer::default(),
            storage_analyzer: StorageAnalyzer::default(),
            connection_analyzer: crate::analyzers::ConnectionAnalyzer::default(),
            treemap_analyzer: crate::analyzers::TreemapAnalyzer::new("/"),
            gpu_process_analyzer: GpuProcessAnalyzer::default(),
            psi_analyzer: PsiAnalyzer::default(),
            container_analyzer: ContainerAnalyzer::default(),
            network_stats: NetworkStatsAnalyzer::default(),
            disk_entropy: DiskEntropyAnalyzer::new(),
            process_extra: crate::analyzers::ProcessExtraAnalyzer::new(),
            file_analyzer: crate::analyzers::FileAnalyzer::new(),
            sensor_health: SensorHealthAnalyzer::default(),

            cpu_history: Vec::with_capacity(300),
            mem_history: Vec::with_capacity(300),
            mem_available_history: Vec::with_capacity(300),
            mem_cached_history: Vec::with_capacity(300),
            mem_free_history: Vec::with_capacity(300),
            swap_history: Vec::with_capacity(300),
            net_rx_history: Vec::with_capacity(300),
            net_tx_history: Vec::with_capacity(300),
            per_core_percent: Vec::new(),

            // CPU Exploded Mode Features
            per_core_history: Vec::new(),
            per_core_state: Vec::new(),
            freq_history: Vec::with_capacity(60),
            top_process_per_core: Vec::new(),
            thermal_throttle_active: None,

            // Memory Exploded Mode Features
            mem_pressure_history: Vec::with_capacity(60),
            mem_reclaim_rate: 0.0,
            top_mem_consumers: Vec::new(),
            swap_trend: SwapTrend::Stable,

            // Disk Exploded Mode Features
            disk_latency_history: Vec::with_capacity(60),
            disk_read_iops: 0.0,
            disk_write_iops: 0.0,
            disk_queue_depth: 0.0,
            disk_health: Vec::new(),

            mem_total: 0,
            mem_used: 0,
            mem_available: 0,
            mem_cached: 0,
            mem_free: 0,
            swap_total: 0,
            swap_used: 0,

            net_rx_total: 0,
            net_tx_total: 0,
            net_interface_ip: String::new(),

            net_rx_peak: 0.0,
            net_tx_peak: 0.0,
            net_rx_peak_time: Instant::now(),
            net_tx_peak_time: Instant::now(),

            // Network Exploded Mode Features
            net_errors: 0,
            net_drops: 0,
            net_established: 0,
            net_listening: 0,

            panels: PanelVisibility::default(),
            process_selected: 0,
            process_scroll_offset: 0,
            sort_column: ProcessSortColumn::Cpu,
            sort_descending: true,
            filter: String::new(),
            show_filter_input: false,
            show_help: false,
            show_tree: false,

            show_signal_menu: false,
            pending_signal: None,
            signal_result: None,

            focused_panel: None,
            exploded_panel: None,

            files_view_mode: crate::state::FilesViewMode::default(),

            frame_id: 0,
            last_collect: Instant::now(),
            avg_frame_time_us: 0,
            max_frame_time_us: 0,
            show_fps,

            deterministic,

            // No mock data in production mode
            mock_gpus: Vec::new(),
            mock_battery: None,
            mock_sensors: Vec::new(),
            mock_containers: Vec::new(),
        };

        // Initial collection (need 2 for deltas)
        debug::log(Level::Debug, "app", "Initial metrics collection (1/2)...");
        app.collect_metrics();
        debug::log(Level::Debug, "app", "Initial metrics collection (2/2)...");
        app.collect_metrics();
        debug::log(Level::Info, "app", "App initialization complete");

        app
    }

    /// Create a mock application instance for testing
    /// This creates an App with default collectors and populated test data
    /// without making real system calls.
    /// Available for integration tests and benchmarks.
    pub fn new_mock() -> Self {
        Self {
            cpu: CpuCollector::default(),
            memory: MemoryCollector::default(),
            disk: DiskCollector::default(),
            network: NetworkCollector::default(),
            process: ProcessCollector::default(),
            sensors: SensorCollector::default(),
            battery: BatteryCollector::default(),

            #[cfg(feature = "nvidia")]
            nvidia_gpu: NvidiaGpuCollector::default(),

            #[cfg(target_os = "linux")]
            amd_gpu: AmdGpuCollector::default(),

            #[cfg(target_os = "macos")]
            apple_gpu: AppleGpuCollector::default(),

            swap_analyzer: SwapAnalyzer::new(),
            disk_io_analyzer: DiskIoAnalyzer::new(),
            storage_analyzer: StorageAnalyzer::new(),
            connection_analyzer: crate::analyzers::ConnectionAnalyzer::new(),
            treemap_analyzer: crate::analyzers::TreemapAnalyzer::new("/tmp"),
            gpu_process_analyzer: GpuProcessAnalyzer::new(),
            psi_analyzer: PsiAnalyzer::new(),
            container_analyzer: ContainerAnalyzer::new(),
            network_stats: NetworkStatsAnalyzer::new(),
            disk_entropy: DiskEntropyAnalyzer::new(),
            process_extra: crate::analyzers::ProcessExtraAnalyzer::new(),
            file_analyzer: crate::analyzers::FileAnalyzer::new(),
            sensor_health: SensorHealthAnalyzer::new(),

            // Populate with test data
            cpu_history: vec![0.25, 0.30, 0.35, 0.40, 0.45, 0.50, 0.45, 0.40],
            mem_history: vec![0.60, 0.61, 0.62, 0.63, 0.64, 0.65, 0.64, 0.63],
            mem_available_history: vec![0.40, 0.39, 0.38, 0.37, 0.36, 0.35, 0.36, 0.37],
            mem_cached_history: vec![0.20, 0.21, 0.22, 0.23, 0.22, 0.21, 0.20, 0.21],
            mem_free_history: vec![0.10, 0.09, 0.08, 0.07, 0.08, 0.09, 0.10, 0.09],
            swap_history: vec![0.05, 0.06, 0.07, 0.08, 0.07, 0.06, 0.05, 0.06],
            net_rx_history: vec![0.01, 0.02, 0.03, 0.04, 0.03, 0.02, 0.01, 0.02],
            net_tx_history: vec![0.005, 0.01, 0.015, 0.02, 0.015, 0.01, 0.005, 0.01],
            per_core_percent: vec![25.0, 30.0, 35.0, 40.0, 45.0, 50.0, 55.0, 60.0],

            // CPU Exploded Mode Features (mock data with proper capacity)
            per_core_history: {
                let mut histories = Vec::with_capacity(8);
                for i in 0..8 {
                    let mut h = Vec::with_capacity(60);
                    let base = 20.0 + (i as f64 * 5.0);
                    for j in 0..6 {
                        h.push(base + (j as f64 * 0.5));
                    }
                    histories.push(h);
                }
                histories
            },
            per_core_state: vec![
                CpuCoreState { user: 20.0, system: 3.0, iowait: 2.0, idle: 75.0 },
                CpuCoreState { user: 25.0, system: 3.0, iowait: 2.0, idle: 70.0 },
                CpuCoreState { user: 28.0, system: 4.0, iowait: 3.0, idle: 65.0 },
                CpuCoreState { user: 32.0, system: 5.0, iowait: 3.0, idle: 60.0 },
                CpuCoreState { user: 35.0, system: 6.0, iowait: 4.0, idle: 55.0 },
                CpuCoreState { user: 38.0, system: 7.0, iowait: 5.0, idle: 50.0 },
                CpuCoreState { user: 42.0, system: 8.0, iowait: 5.0, idle: 45.0 },
                CpuCoreState { user: 45.0, system: 9.0, iowait: 6.0, idle: 40.0 },
            ],
            freq_history: {
                let mut h = Vec::with_capacity(60);
                h.extend_from_slice(&[3200.0, 3400.0, 3600.0, 3800.0, 4000.0, 4200.0]);
                h
            },
            top_process_per_core: vec![
                TopProcessForCore { pid: 1234, name: "firefox".to_string(), cpu_percent: 15.0 },
                TopProcessForCore { pid: 5678, name: "chrome".to_string(), cpu_percent: 12.0 },
                TopProcessForCore { pid: 9012, name: "code".to_string(), cpu_percent: 10.0 },
                TopProcessForCore { pid: 3456, name: "rustc".to_string(), cpu_percent: 25.0 },
                TopProcessForCore { pid: 7890, name: "cargo".to_string(), cpu_percent: 20.0 },
                TopProcessForCore { pid: 1111, name: "node".to_string(), cpu_percent: 8.0 },
                TopProcessForCore { pid: 2222, name: "python".to_string(), cpu_percent: 6.0 },
                TopProcessForCore { pid: 3333, name: "java".to_string(), cpu_percent: 5.0 },
            ],
            thermal_throttle_active: Some(false),

            // Memory Exploded Mode Features (mock data)
            mem_pressure_history: {
                let mut h = Vec::with_capacity(60);
                h.extend_from_slice(&[5.0, 8.0, 12.0, 15.0, 10.0, 7.0]);
                h
            },
            mem_reclaim_rate: 1250.0,  // 1250 pages/sec
            top_mem_consumers: vec![
                TopMemConsumer { pid: 1234, name: "firefox".to_string(), mem_bytes: 2 * 1024 * 1024 * 1024, mem_percent: 12.5 },
                TopMemConsumer { pid: 5678, name: "chrome".to_string(), mem_bytes: 1500 * 1024 * 1024, mem_percent: 9.4 },
                TopMemConsumer { pid: 9012, name: "code".to_string(), mem_bytes: 800 * 1024 * 1024, mem_percent: 5.0 },
                TopMemConsumer { pid: 3456, name: "slack".to_string(), mem_bytes: 600 * 1024 * 1024, mem_percent: 3.8 },
            ],
            swap_trend: SwapTrend::Stable,

            // Disk Exploded Mode Features (mock data)
            disk_latency_history: {
                let mut h = Vec::with_capacity(60);
                h.extend_from_slice(&[2.5, 3.0, 4.5, 8.0, 5.5, 3.2]);
                h
            },
            disk_read_iops: 1250.0,
            disk_write_iops: 850.0,
            disk_queue_depth: 2.5,
            disk_health: vec![
                DiskHealthStatus {
                    device: "nvme0n1".to_string(),
                    status: DiskHealth::Good,
                    temperature: Some(42.0),
                    reallocated_sectors: 0,
                },
            ],

            mem_total: 16 * 1024 * 1024 * 1024,  // 16 GB
            mem_used: 10 * 1024 * 1024 * 1024,   // 10 GB
            mem_available: 6 * 1024 * 1024 * 1024, // 6 GB
            mem_cached: 3 * 1024 * 1024 * 1024,  // 3 GB
            mem_free: 2 * 1024 * 1024 * 1024,    // 2 GB
            swap_total: 4 * 1024 * 1024 * 1024,  // 4 GB
            swap_used: 500 * 1024 * 1024,        // 500 MB

            net_rx_total: 1024 * 1024 * 1024,    // 1 GB
            net_tx_total: 512 * 1024 * 1024,     // 512 MB
            net_interface_ip: "192.168.1.100".to_string(),

            net_rx_peak: 100_000_000.0,  // 100 MB/s
            net_tx_peak: 50_000_000.0,   // 50 MB/s
            net_rx_peak_time: Instant::now(),
            net_tx_peak_time: Instant::now(),

            // Network Exploded Mode Features (mock data)
            net_errors: 5,
            net_drops: 2,
            net_established: 42,
            net_listening: 15,

            panels: PanelVisibility::default(),
            process_selected: 0,
            process_scroll_offset: 0,
            sort_column: ProcessSortColumn::Cpu,
            sort_descending: true,
            filter: String::new(),
            show_filter_input: false,
            show_help: false,
            show_tree: false,

            show_signal_menu: false,
            pending_signal: None,
            signal_result: None,

            focused_panel: None,
            exploded_panel: None,

            files_view_mode: crate::state::FilesViewMode::default(),

            frame_id: 100,
            last_collect: Instant::now(),
            avg_frame_time_us: 1000,
            max_frame_time_us: 2000,
            show_fps: false,

            deterministic: true,

            // Populate mock data for testing hardware-dependent panel rendering
            mock_gpus: vec![
                MockGpuData {
                    name: "NVIDIA RTX 4090".to_string(),
                    gpu_util: 75.0,
                    vram_used: 20 * 1024 * 1024 * 1024, // 20 GB
                    vram_total: 24 * 1024 * 1024 * 1024, // 24 GB
                    temperature: 72.0,
                    power_watts: 350,
                    power_limit_watts: 450,
                    clock_mhz: 2520,
                    history: vec![0.65, 0.70, 0.75, 0.80, 0.75, 0.70, 0.75, 0.80],
                },
                MockGpuData {
                    name: "NVIDIA RTX 3080".to_string(),
                    gpu_util: 45.0,
                    vram_used: 6 * 1024 * 1024 * 1024, // 6 GB
                    vram_total: 10 * 1024 * 1024 * 1024, // 10 GB
                    temperature: 65.0,
                    power_watts: 220,
                    power_limit_watts: 320,
                    clock_mhz: 1950,
                    history: vec![0.40, 0.45, 0.50, 0.45, 0.40, 0.45, 0.50, 0.45],
                },
            ],
            mock_battery: Some(MockBatteryData {
                percent: 72.5,
                charging: false,
                time_remaining_mins: Some(185),
                power_watts: 15.2,
                health_percent: 94.0,
                cycle_count: 342,
            }),
            mock_sensors: vec![
                MockSensorData {
                    name: "coretemp/temp1".to_string(),
                    label: "Package".to_string(),
                    value: 65.0,
                    max: Some(100.0),
                    crit: Some(105.0),
                    sensor_type: MockSensorType::Temperature,
                },
                MockSensorData {
                    name: "coretemp/temp2".to_string(),
                    label: "Core 0".to_string(),
                    value: 62.0,
                    max: Some(100.0),
                    crit: Some(105.0),
                    sensor_type: MockSensorType::Temperature,
                },
                MockSensorData {
                    name: "coretemp/temp3".to_string(),
                    label: "Core 1".to_string(),
                    value: 64.0,
                    max: Some(100.0),
                    crit: Some(105.0),
                    sensor_type: MockSensorType::Temperature,
                },
                MockSensorData {
                    name: "nct6798/fan1".to_string(),
                    label: "CPU Fan".to_string(),
                    value: 1200.0,
                    max: Some(3000.0),
                    crit: None,
                    sensor_type: MockSensorType::Fan,
                },
                MockSensorData {
                    name: "nct6798/fan2".to_string(),
                    label: "Chassis Fan".to_string(),
                    value: 800.0,
                    max: Some(2000.0),
                    crit: None,
                    sensor_type: MockSensorType::Fan,
                },
                MockSensorData {
                    name: "nct6798/in0".to_string(),
                    label: "Vcore".to_string(),
                    value: 1.25,
                    max: Some(1.50),
                    crit: None,
                    sensor_type: MockSensorType::Voltage,
                },
            ],
            mock_containers: vec![
                MockContainerData {
                    name: "nginx-proxy".to_string(),
                    status: "running".to_string(),
                    cpu_percent: 2.5,
                    mem_used: 128 * 1024 * 1024, // 128 MB
                    mem_limit: 512 * 1024 * 1024, // 512 MB
                },
                MockContainerData {
                    name: "postgres-db".to_string(),
                    status: "running".to_string(),
                    cpu_percent: 8.2,
                    mem_used: 512 * 1024 * 1024, // 512 MB
                    mem_limit: 2 * 1024 * 1024 * 1024, // 2 GB
                },
                MockContainerData {
                    name: "redis-cache".to_string(),
                    status: "running".to_string(),
                    cpu_percent: 1.1,
                    mem_used: 64 * 1024 * 1024, // 64 MB
                    mem_limit: 256 * 1024 * 1024, // 256 MB
                },
            ],
        }
    }

    /// Collect metrics from all collectors
    pub fn collect_metrics(&mut self) {
        use trueno_viz::monitor::debug::{self, Level};

        self.frame_id += 1;

        // Skip real collection in deterministic/mock mode - data is pre-populated
        if self.deterministic {
            return;
        }

        let is_first = self.frame_id <= 2;

        // CPU
        if is_first { debug::log(Level::Trace, "collect", "cpu..."); }
        if self.cpu.is_available() {
            if let Ok(metrics) = self.cpu.collect() {
                if let Some(total) = metrics.get_gauge("cpu.total") {
                    Self::push_to_history(&mut self.cpu_history, total / 100.0);
                }

                // Per-core percentages
                self.per_core_percent.clear();
                for i in 0..self.cpu.core_count() {
                    if let Some(percent) = metrics.get_gauge(&format!("cpu.core.{i}")) {
                        self.per_core_percent.push(percent);
                    }
                }
            }
        }

        // Memory
        if is_first { debug::log(Level::Trace, "collect", "memory..."); }
        if self.memory.is_available() {
            if let Ok(metrics) = self.memory.collect() {
                // Cache raw values first
                if let Some(total) = metrics.get_counter("memory.total") {
                    self.mem_total = total;
                }
                if let Some(used) = metrics.get_counter("memory.used") {
                    self.mem_used = used;
                }
                if let Some(available) = metrics.get_counter("memory.available") {
                    self.mem_available = available;
                }
                if let Some(cached) = metrics.get_counter("memory.cached") {
                    self.mem_cached = cached;
                }
                if let Some(free) = metrics.get_counter("memory.free") {
                    self.mem_free = free;
                }
                if let Some(swap_total) = metrics.get_counter("memory.swap.total") {
                    self.swap_total = swap_total;
                }
                if let Some(swap_used) = metrics.get_counter("memory.swap.used") {
                    self.swap_used = swap_used;
                }

                // Track history for all memory metrics (normalized 0-1 relative to total)
                if self.mem_total > 0 {
                    let total = self.mem_total as f64;

                    // Used percentage history
                    if let Some(percent) = metrics.get_gauge("memory.used.percent") {
                        Self::push_to_history(&mut self.mem_history, percent / 100.0);
                    }

                    // Available percentage history
                    let avail_pct = self.mem_available as f64 / total;
                    Self::push_to_history(&mut self.mem_available_history, avail_pct);

                    // Cached percentage history
                    let cached_pct = self.mem_cached as f64 / total;
                    Self::push_to_history(&mut self.mem_cached_history, cached_pct);

                    // Free percentage history
                    let free_pct = self.mem_free as f64 / total;
                    Self::push_to_history(&mut self.mem_free_history, free_pct);
                }

                // Swap percentage history
                if let Some(swap_percent) = metrics.get_gauge("memory.swap.percent") {
                    Self::push_to_history(&mut self.swap_history, swap_percent / 100.0);
                }
            }
        }

        // Network
        if is_first { debug::log(Level::Trace, "collect", "network..."); }
        if self.network.is_available() {
            let _ = self.network.collect();
            if let Some(iface) = self.network.current_interface() {
                if let Some(rates) = self.network.all_rates().get(iface) {
                    // Normalize to 0-1 range (assume max 1 GB/s for scaling)
                    let rx_norm = (rates.rx_bytes_per_sec / 1_000_000_000.0).min(1.0);
                    let tx_norm = (rates.tx_bytes_per_sec / 1_000_000_000.0).min(1.0);
                    Self::push_to_history(&mut self.net_rx_history, rx_norm);
                    Self::push_to_history(&mut self.net_tx_history, tx_norm);

                    // Accumulate total bytes (approximate from rates)
                    // This is reset on app start, but gives session totals
                    self.net_rx_total += rates.rx_bytes_per_sec as u64;
                    self.net_tx_total += rates.tx_bytes_per_sec as u64;

                    // Track peak rates
                    if rates.rx_bytes_per_sec > self.net_rx_peak {
                        self.net_rx_peak = rates.rx_bytes_per_sec;
                        self.net_rx_peak_time = Instant::now();
                    }
                    if rates.tx_bytes_per_sec > self.net_tx_peak {
                        self.net_tx_peak = rates.tx_bytes_per_sec;
                        self.net_tx_peak_time = Instant::now();
                    }
                }
            }
        }

        // Disk
        if is_first { debug::log(Level::Trace, "collect", "disk..."); }
        if self.disk.is_available() {
            let _ = self.disk.collect();
        }

        // Process
        if is_first { debug::log(Level::Trace, "collect", "process..."); }
        if self.process.is_available() {
            let _ = self.process.collect();
        }

        // Sensors
        if is_first { debug::log(Level::Trace, "collect", "sensors..."); }
        if self.sensors.is_available() {
            let _ = self.sensors.collect();
        }

        // Battery
        if is_first { debug::log(Level::Trace, "collect", "battery..."); }
        if self.battery.is_available() {
            let _ = self.battery.collect();
        }

        // GPU
        if is_first { debug::log(Level::Trace, "collect", "gpu..."); }
        #[cfg(feature = "nvidia")]
        if self.nvidia_gpu.is_available() {
            let _ = self.nvidia_gpu.collect();
        }

        #[cfg(target_os = "linux")]
        if self.amd_gpu.is_available() {
            let _ = self.amd_gpu.collect();
        }

        #[cfg(target_os = "macos")]
        if self.apple_gpu.is_available() {
            let _ = self.apple_gpu.collect();
        }

        // Advanced analyzers (ttop-improve.md spec)
        if is_first { debug::log(Level::Trace, "collect", "swap_analyzer..."); }
        self.swap_analyzer.collect();

        if is_first { debug::log(Level::Trace, "collect", "disk_io_analyzer..."); }
        self.disk_io_analyzer.collect();

        // Disk entropy analysis (rate-limited internally)
        if is_first { debug::log(Level::Trace, "collect", "disk_entropy..."); }
        let mount_paths: Vec<String> = self.disk.mounts().iter().map(|m| m.mount_point.clone()).collect();
        self.disk_entropy.collect(&mount_paths);

        if is_first { debug::log(Level::Trace, "collect", "storage_analyzer..."); }
        self.storage_analyzer.collect();

        if is_first { debug::log(Level::Trace, "collect", "connection_analyzer..."); }
        self.connection_analyzer.collect();

        if is_first { debug::log(Level::Trace, "collect", "treemap_analyzer..."); }
        self.treemap_analyzer.collect();

        if is_first { debug::log(Level::Trace, "collect", "gpu_process_analyzer..."); }
        self.gpu_process_analyzer.collect();

        if is_first { debug::log(Level::Trace, "collect", "psi_analyzer..."); }
        self.psi_analyzer.collect();

        if is_first { debug::log(Level::Trace, "collect", "container_analyzer..."); }
        self.container_analyzer.collect();

        // Linux network stats (protocol counts, errors, latency)
        #[cfg(target_os = "linux")]
        {
            if is_first { debug::log(Level::Trace, "collect", "network_stats..."); }
            self.network_stats.collect();
        }

        // Extended process info (cgroups, FDs, CPU history)
        if is_first { debug::log(Level::Trace, "collect", "process_extra..."); }
        let pids: Vec<u32> = self.process.processes().keys().copied().collect();
        let cpu_percents: std::collections::HashMap<u32, f64> = self.process.processes()
            .iter()
            .map(|(&pid, p)| (pid, p.cpu_percent))
            .collect();
        self.process_extra.collect(&pids, &cpu_percents);

        // File analyzer for treemap enhancements (rate-limited internally)
        if is_first { debug::log(Level::Trace, "collect", "file_analyzer..."); }
        self.file_analyzer.collect("/");

        // Sensor health analysis (outliers, drift, staleness)
        if is_first { debug::log(Level::Trace, "collect", "sensor_health..."); }
        let _ = self.sensor_health.collect();

        self.last_collect = Instant::now();
    }

    fn push_to_history(history: &mut Vec<f64>, value: f64) {
        history.push(value);
        if history.len() > 300 {
            history.remove(0);
        }
    }

    /// Update frame timing statistics
    pub fn update_frame_stats(&mut self, frame_times: &[Duration]) {
        if frame_times.is_empty() {
            return;
        }

        let total: u128 = frame_times.iter().map(|d| d.as_micros()).sum();
        self.avg_frame_time_us = (total / frame_times.len() as u128) as u64;
        self.max_frame_time_us = frame_times
            .iter()
            .map(|d| d.as_micros() as u64)
            .max()
            .unwrap_or(0);
    }

    /// Handle keyboard input. Returns true if app should quit.
    pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        // Signal confirmation mode (Y/n prompt)
        if self.pending_signal.is_some() {
            match code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    self.confirm_signal();
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.cancel_signal();
                }
                _ => {}
            }
            return false;
        }

        // Signal menu mode (pick signal type)
        if self.show_signal_menu {
            match code {
                KeyCode::Esc => {
                    self.show_signal_menu = false;
                }
                KeyCode::Char('x') => {
                    self.show_signal_menu = false;
                    self.request_signal(SignalType::Term);
                }
                KeyCode::Char('K') => {
                    self.show_signal_menu = false;
                    self.request_signal(SignalType::Kill);
                }
                KeyCode::Char('H') => {
                    self.show_signal_menu = false;
                    self.request_signal(SignalType::Hup);
                }
                KeyCode::Char('i') => {
                    self.show_signal_menu = false;
                    self.request_signal(SignalType::Int);
                }
                KeyCode::Char('p') => {
                    self.show_signal_menu = false;
                    self.request_signal(SignalType::Stop);
                }
                KeyCode::Char('c') => {
                    self.show_signal_menu = false;
                    self.request_signal(SignalType::Cont);
                }
                _ => {}
            }
            return false;
        }

        // Filter input mode
        if self.show_filter_input {
            match code {
                KeyCode::Esc => {
                    self.show_filter_input = false;
                    self.filter.clear();
                }
                KeyCode::Enter => {
                    self.show_filter_input = false;
                }
                KeyCode::Backspace => {
                    self.filter.pop();
                }
                KeyCode::Char(c) => {
                    self.filter.push(c);
                }
                _ => {}
            }
            return false;
        }

        // ESC handling: exit explode -> clear focus -> quit
        if code == KeyCode::Esc {
            if self.exploded_panel.is_some() {
                self.exploded_panel = None;
                return false;
            }
            if self.focused_panel.is_some() {
                self.focused_panel = None;
                return false;
            }
            return true; // Quit
        }

        // Ctrl+C always quits
        if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
            return true;
        }

        // EXPLODED MODE: pass most keys through to panel controls
        // Only handle exit keys (Enter/z/ESC already handled above for ESC)
        if self.exploded_panel.is_some() {
            match code {
                // Exit explode with Enter or z
                KeyCode::Enter | KeyCode::Char('z') => {
                    self.exploded_panel = None;
                    return false;
                }
                // All other keys fall through to normal handling (process nav, sort, etc.)
                _ => {}
            }
        }
        // FOCUSED MODE (not exploded): arrow/hjkl navigate between panels
        else if self.focused_panel.is_some() {
            match code {
                // Explode with Enter or z
                KeyCode::Enter | KeyCode::Char('z') => {
                    if let Some(panel) = self.focused_panel {
                        self.exploded_panel = Some(panel);
                    }
                    return false;
                }
                // Arrow navigation between panels
                KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                    self.navigate_panel_focus(code);
                    return false;
                }
                // hjkl navigation between panels
                KeyCode::Char('h') => {
                    self.navigate_panel_focus(KeyCode::Left);
                    return false;
                }
                KeyCode::Char('l') => {
                    self.navigate_panel_focus(KeyCode::Right);
                    return false;
                }
                KeyCode::Char('j') => {
                    self.navigate_panel_focus(KeyCode::Down);
                    return false;
                }
                KeyCode::Char('k') => {
                    self.navigate_panel_focus(KeyCode::Up);
                    return false;
                }
                _ => {}
            }
        }
        // NOT FOCUSED: h/l start panel focus, j/k navigate process list
        else {
            match code {
                KeyCode::Char('h') => {
                    self.focused_panel = Some(self.first_visible_panel());
                    return false;
                }
                KeyCode::Char('l') => {
                    self.focused_panel = Some(self.first_visible_panel());
                    return false;
                }
                _ => {}
            }
        }

        match code {
            // Quit
            KeyCode::Char('q') => return true,

            // Help
            KeyCode::Char('?') | KeyCode::F(1) => self.show_help = !self.show_help,

            // Panel toggles (1-8)
            KeyCode::Char('1') => self.panels.cpu = !self.panels.cpu,
            KeyCode::Char('2') => self.panels.memory = !self.panels.memory,
            KeyCode::Char('3') => self.panels.disk = !self.panels.disk,
            KeyCode::Char('4') => self.panels.network = !self.panels.network,
            KeyCode::Char('5') => self.panels.process = !self.panels.process,
            KeyCode::Char('6') => self.panels.gpu = !self.panels.gpu,
            KeyCode::Char('7') => self.panels.battery = !self.panels.battery,
            KeyCode::Char('8') => self.panels.sensors = !self.panels.sensors,
            KeyCode::Char('9') => self.panels.files = !self.panels.files,

            // Files view mode cycle (v = view mode: SIZE -> ENTROPY -> I/O)
            KeyCode::Char('v') => self.files_view_mode = self.files_view_mode.next(),

            // Process navigation (when no panel focused, or when exploded)
            KeyCode::Down if self.focused_panel.is_none() || self.exploded_panel.is_some() => {
                self.navigate_process(1)
            }
            KeyCode::Up if self.focused_panel.is_none() || self.exploded_panel.is_some() => {
                self.navigate_process(-1)
            }
            // j/k for process navigation when exploded
            KeyCode::Char('j') if self.exploded_panel.is_some() => self.navigate_process(1),
            KeyCode::Char('k') if self.exploded_panel.is_some() => self.navigate_process(-1),
            KeyCode::PageDown => self.navigate_process(10),
            KeyCode::PageUp => self.navigate_process(-10),
            KeyCode::Home | KeyCode::Char('g') => self.process_selected = 0,
            KeyCode::Char('G') => {
                let count = self.process_count();
                if count > 0 {
                    self.process_selected = count - 1;
                }
            }

            // Sorting
            KeyCode::Tab | KeyCode::Char('s') => {
                self.sort_column = self.sort_column.next();
            }
            KeyCode::Char('r') => self.sort_descending = !self.sort_descending,

            // Tree view
            KeyCode::Char('t') => self.show_tree = !self.show_tree,

            // Signal menu (kill process) - 'k' key opens signal menu
            // Quick kill shortcuts (no menu): x=TERM, X=KILL
            KeyCode::Char('X') if self.focused_panel.is_none() || self.exploded_panel == Some(PanelType::Process) => {
                // Quick SIGKILL (uppercase X)
                self.request_signal(SignalType::Kill);
            }
            KeyCode::Char('x') if self.focused_panel.is_none() => {
                // Quick SIGTERM (lowercase x)
                self.request_signal(SignalType::Term);
            }

            // z key starts focus when nothing is focused/exploded
            KeyCode::Char('z') if self.focused_panel.is_none() && self.exploded_panel.is_none() => {
                self.focused_panel = Some(self.first_visible_panel());
            }

            // Filter
            KeyCode::Char('f') | KeyCode::Char('/') => {
                self.show_filter_input = true;
            }
            KeyCode::Delete => self.filter.clear(),

            // Reset view
            KeyCode::Char('0') => {
                self.panels = PanelVisibility::default();
                self.focused_panel = None;
                self.exploded_panel = None;
            }

            _ => {}
        }

        false
    }

    /// Navigate panel focus with arrow keys
    fn navigate_panel_focus(&mut self, direction: KeyCode) {
        let visible = self.visible_panels();
        if visible.is_empty() {
            return;
        }

        let current = self.focused_panel.unwrap_or_else(|| self.first_visible_panel());
        let current_idx = visible.iter().position(|&p| p == current).unwrap_or(0);

        let new_idx = match direction {
            KeyCode::Left | KeyCode::Up => {
                if current_idx == 0 {
                    visible.len() - 1
                } else {
                    current_idx - 1
                }
            }
            KeyCode::Right | KeyCode::Down => {
                if current_idx >= visible.len() - 1 {
                    0
                } else {
                    current_idx + 1
                }
            }
            _ => current_idx,
        };

        self.focused_panel = Some(visible[new_idx]);
    }

    /// Get list of currently visible panels
    pub fn visible_panels(&self) -> Vec<PanelType> {
        let mut visible = Vec::new();
        if self.panels.cpu {
            visible.push(PanelType::Cpu);
        }
        if self.panels.memory {
            visible.push(PanelType::Memory);
        }
        if self.panels.disk {
            visible.push(PanelType::Disk);
        }
        if self.panels.network {
            visible.push(PanelType::Network);
        }
        if self.panels.process {
            visible.push(PanelType::Process);
        }
        if self.panels.gpu && self.has_gpu() {
            visible.push(PanelType::Gpu);
        }
        if self.panels.battery && self.battery.is_available() {
            visible.push(PanelType::Battery);
        }
        if self.panels.sensors && self.sensors.is_available() {
            visible.push(PanelType::Sensors);
        }
        if self.panels.files {
            visible.push(PanelType::Files);
        }
        visible
    }

    /// Get first visible panel (for default focus)
    fn first_visible_panel(&self) -> PanelType {
        self.visible_panels().first().copied().unwrap_or(PanelType::Cpu)
    }

    /// Check if a specific panel is visible
    pub fn is_panel_visible(&self, panel: PanelType) -> bool {
        match panel {
            PanelType::Cpu => self.panels.cpu,
            PanelType::Memory => self.panels.memory,
            PanelType::Disk => self.panels.disk,
            PanelType::Network => self.panels.network,
            PanelType::Process => self.panels.process,
            PanelType::Gpu => self.panels.gpu && self.has_gpu(),
            PanelType::Battery => self.panels.battery && self.battery.is_available(),
            PanelType::Sensors => self.panels.sensors && self.sensors.is_available(),
            PanelType::Files => self.panels.files,
        }
    }

    fn navigate_process(&mut self, delta: isize) {
        let count = self.process_count();
        if count == 0 {
            return;
        }

        let current = self.process_selected;
        let new = if delta > 0 {
            (current + delta as usize).min(count - 1)
        } else {
            current.saturating_sub((-delta) as usize)
        };
        self.process_selected = new;
    }

    fn process_count(&self) -> usize {
        let filter_lower = self.filter.to_lowercase();
        self.process
            .processes()
            .values()
            .filter(|p| {
                if filter_lower.is_empty() {
                    true
                } else {
                    // Use allocation-free case-insensitive contains
                    contains_ignore_case(&p.name, &filter_lower)
                        || contains_ignore_case(&p.cmdline, &filter_lower)
                }
            })
            .count()
    }

    /// Get sorted and filtered processes
    pub fn sorted_processes(&self) -> Vec<&trueno_viz::monitor::collectors::process::ProcessInfo> {
        // Cache lowercase filter once per call
        let filter_lower = self.filter.to_lowercase();

        let mut procs: Vec<_> = self
            .process
            .processes()
            .values()
            .filter(|p| {
                if filter_lower.is_empty() {
                    true
                } else {
                    // Use allocation-free case-insensitive contains
                    contains_ignore_case(&p.name, &filter_lower)
                        || contains_ignore_case(&p.cmdline, &filter_lower)
                }
            })
            .collect();

        procs.sort_by(|a, b| {
            let cmp = match self.sort_column {
                ProcessSortColumn::Pid => a.pid.cmp(&b.pid),
                ProcessSortColumn::Name => a.name.cmp(&b.name),
                ProcessSortColumn::Cpu => a
                    .cpu_percent
                    .partial_cmp(&b.cpu_percent)
                    .unwrap_or(std::cmp::Ordering::Equal),
                ProcessSortColumn::Mem => a
                    .mem_percent
                    .partial_cmp(&b.mem_percent)
                    .unwrap_or(std::cmp::Ordering::Equal),
                ProcessSortColumn::State => a.state.as_char().cmp(&b.state.as_char()),
                ProcessSortColumn::User => a.user.cmp(&b.user),
                ProcessSortColumn::Threads => a.threads.cmp(&b.threads),
            };
            if self.sort_descending {
                cmp.reverse()
            } else {
                cmp
            }
        });

        procs
    }

    /// Check if any GPU is available
    pub fn has_gpu(&self) -> bool {
        #[cfg(feature = "nvidia")]
        if self.nvidia_gpu.is_available() {
            return true;
        }

        #[cfg(target_os = "linux")]
        if self.amd_gpu.is_available() {
            return true;
        }

        #[cfg(target_os = "macos")]
        if self.apple_gpu.is_available() {
            return true;
        }

        false
    }

    /// Send a signal to a process
    #[cfg(unix)]
    pub fn send_signal(&mut self, pid: u32, signal: SignalType) -> Result<(), String> {
        use std::process::Command;

        let signal_num = signal.number();
        let result = Command::new("kill")
            .arg(format!("-{}", signal_num))
            .arg(pid.to_string())
            .output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    self.signal_result = Some((
                        true,
                        format!("Sent {} to PID {}", signal.name(), pid),
                        Instant::now(),
                    ));
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let msg = format!("Failed to send {} to {}: {}", signal.name(), pid, stderr.trim());
                    self.signal_result = Some((false, msg.clone(), Instant::now()));
                    Err(msg)
                }
            }
            Err(e) => {
                let msg = format!("Failed to execute kill: {}", e);
                self.signal_result = Some((false, msg.clone(), Instant::now()));
                Err(msg)
            }
        }
    }

    #[cfg(not(unix))]
    pub fn send_signal(&mut self, _pid: u32, _signal: SignalType) -> Result<(), String> {
        Err("Signal sending not supported on this platform".to_string())
    }

    /// Get the currently selected process info (pid, name)
    pub fn selected_process(&self) -> Option<(u32, String)> {
        let procs = self.sorted_processes();
        procs.get(self.process_selected).map(|p| (p.pid, p.name.clone()))
    }

    /// Request to send a signal to the selected process (shows confirmation)
    pub fn request_signal(&mut self, signal: SignalType) {
        if let Some((pid, name)) = self.selected_process() {
            self.pending_signal = Some((pid, name, signal));
        }
    }

    /// Confirm and send the pending signal
    pub fn confirm_signal(&mut self) {
        if let Some((pid, _name, signal)) = self.pending_signal.take() {
            let _ = self.send_signal(pid, signal);
        }
    }

    /// Cancel the pending signal
    pub fn cancel_signal(&mut self) {
        self.pending_signal = None;
    }

    /// Clear old signal results (after 3 seconds)
    pub fn clear_old_signal_result(&mut self) {
        if let Some((_, _, timestamp)) = &self.signal_result {
            if timestamp.elapsed() > Duration::from_secs(3) {
                self.signal_result = None;
            }
        }
    }
}


#[cfg(test)]
#[path = "tests.rs"]
mod tests;
