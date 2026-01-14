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
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn test_panel_visibility_default() {
        let vis = PanelVisibility::default();
        assert!(vis.cpu);
        assert!(vis.memory);
        assert!(vis.disk);
        assert!(vis.network);
        assert!(vis.process);
        assert!(vis.gpu);
        assert!(vis.battery);
        assert!(vis.sensors);
        assert!(!vis.files); // Off by default
    }

    #[test]
    fn test_mock_app_creation() {
        let app = App::new_mock();
        assert!(app.deterministic);
        assert_eq!(app.frame_id, 100);
        assert_eq!(app.avg_frame_time_us, 1000);
        assert_eq!(app.max_frame_time_us, 2000);
        assert!(!app.show_fps);
    }

    #[test]
    fn test_mock_app_history_populated() {
        let app = App::new_mock();
        assert_eq!(app.cpu_history.len(), 8);
        assert_eq!(app.mem_history.len(), 8);
        assert_eq!(app.per_core_percent.len(), 8);
    }

    #[test]
    fn test_mock_app_memory_values() {
        let app = App::new_mock();
        assert_eq!(app.mem_total, 16 * 1024 * 1024 * 1024);
        assert_eq!(app.mem_used, 10 * 1024 * 1024 * 1024);
        assert_eq!(app.mem_available, 6 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_update_frame_stats_empty() {
        let mut app = App::new_mock();
        app.update_frame_stats(&[]);
        // Should not panic, values unchanged
    }

    #[test]
    fn test_update_frame_stats_single() {
        let mut app = App::new_mock();
        app.update_frame_stats(&[Duration::from_micros(500)]);
        assert_eq!(app.avg_frame_time_us, 500);
        assert_eq!(app.max_frame_time_us, 500);
    }

    #[test]
    fn test_update_frame_stats_multiple() {
        let mut app = App::new_mock();
        let times = vec![
            Duration::from_micros(100),
            Duration::from_micros(200),
            Duration::from_micros(300),
        ];
        app.update_frame_stats(&times);
        assert_eq!(app.avg_frame_time_us, 200); // (100+200+300)/3
        assert_eq!(app.max_frame_time_us, 300);
    }

    #[test]
    fn test_visible_panels_default() {
        let app = App::new_mock();
        let visible = app.visible_panels();
        // Default: cpu, memory, disk, network, process (not files, battery/sensors may vary)
        assert!(visible.contains(&PanelType::Cpu));
        assert!(visible.contains(&PanelType::Memory));
        assert!(visible.contains(&PanelType::Disk));
        assert!(visible.contains(&PanelType::Network));
        assert!(visible.contains(&PanelType::Process));
        assert!(!visible.contains(&PanelType::Files)); // Off by default
    }

    #[test]
    fn test_visible_panels_with_files() {
        let mut app = App::new_mock();
        app.panels.files = true;
        let visible = app.visible_panels();
        assert!(visible.contains(&PanelType::Files));
    }

    #[test]
    fn test_visible_panels_all_disabled() {
        let mut app = App::new_mock();
        app.panels.cpu = false;
        app.panels.memory = false;
        app.panels.disk = false;
        app.panels.network = false;
        app.panels.process = false;
        app.panels.gpu = false;
        app.panels.battery = false;
        app.panels.sensors = false;
        app.panels.files = false;
        let visible = app.visible_panels();
        assert!(visible.is_empty());
    }

    #[test]
    fn test_first_visible_panel_default() {
        let app = App::new_mock();
        let first = app.first_visible_panel();
        assert_eq!(first, PanelType::Cpu);
    }

    #[test]
    fn test_first_visible_panel_when_cpu_disabled() {
        let mut app = App::new_mock();
        app.panels.cpu = false;
        let first = app.first_visible_panel();
        assert_eq!(first, PanelType::Memory);
    }

    #[test]
    fn test_is_panel_visible() {
        let app = App::new_mock();
        assert!(app.is_panel_visible(PanelType::Cpu));
        assert!(app.is_panel_visible(PanelType::Memory));
        assert!(!app.is_panel_visible(PanelType::Files));
    }

    #[test]
    fn test_handle_key_quit_q() {
        let mut app = App::new_mock();
        let quit = app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(quit);
    }

    #[test]
    fn test_handle_key_quit_ctrl_c() {
        let mut app = App::new_mock();
        let quit = app.handle_key(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(quit);
    }

    #[test]
    fn test_handle_key_quit_esc() {
        let mut app = App::new_mock();
        let quit = app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(quit);
    }

    #[test]
    fn test_handle_key_help_toggle() {
        let mut app = App::new_mock();
        assert!(!app.show_help);
        app.handle_key(KeyCode::Char('?'), KeyModifiers::NONE);
        assert!(app.show_help);
        app.handle_key(KeyCode::Char('?'), KeyModifiers::NONE);
        assert!(!app.show_help);
    }

    #[test]
    fn test_handle_key_panel_toggles() {
        let mut app = App::new_mock();

        // Toggle CPU off
        assert!(app.panels.cpu);
        app.handle_key(KeyCode::Char('1'), KeyModifiers::NONE);
        assert!(!app.panels.cpu);

        // Toggle memory off
        assert!(app.panels.memory);
        app.handle_key(KeyCode::Char('2'), KeyModifiers::NONE);
        assert!(!app.panels.memory);

        // Toggle files on (off by default)
        assert!(!app.panels.files);
        app.handle_key(KeyCode::Char('9'), KeyModifiers::NONE);
        assert!(app.panels.files);
    }

    #[test]
    fn test_handle_key_filter_mode() {
        let mut app = App::new_mock();
        assert!(!app.show_filter_input);

        // Enter filter mode
        app.handle_key(KeyCode::Char('/'), KeyModifiers::NONE);
        assert!(app.show_filter_input);

        // Type some text
        app.handle_key(KeyCode::Char('t'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('e'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('t'), KeyModifiers::NONE);
        assert_eq!(app.filter, "test");

        // Backspace
        app.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(app.filter, "tes");

        // Escape clears and exits
        app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(!app.show_filter_input);
        assert_eq!(app.filter, "");
    }

    #[test]
    fn test_handle_key_filter_enter_confirm() {
        let mut app = App::new_mock();
        app.handle_key(KeyCode::Char('f'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('a'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert!(!app.show_filter_input);
        assert_eq!(app.filter, "a"); // Preserved
    }

    #[test]
    fn test_handle_key_sort_toggle() {
        let mut app = App::new_mock();
        assert_eq!(app.sort_column, ProcessSortColumn::Cpu);

        app.handle_key(KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(app.sort_column, ProcessSortColumn::Mem);

        app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
        assert_eq!(app.sort_column, ProcessSortColumn::State);
    }

    #[test]
    fn test_handle_key_sort_reverse() {
        let mut app = App::new_mock();
        assert!(app.sort_descending);
        app.handle_key(KeyCode::Char('r'), KeyModifiers::NONE);
        assert!(!app.sort_descending);
    }

    #[test]
    fn test_handle_key_tree_toggle() {
        let mut app = App::new_mock();
        assert!(!app.show_tree);
        app.handle_key(KeyCode::Char('t'), KeyModifiers::NONE);
        assert!(app.show_tree);
    }

    #[test]
    fn test_handle_key_reset_view() {
        let mut app = App::new_mock();
        app.panels.cpu = false;
        app.focused_panel = Some(PanelType::Memory);
        app.exploded_panel = Some(PanelType::Disk);

        app.handle_key(KeyCode::Char('0'), KeyModifiers::NONE);

        assert!(app.panels.cpu);
        assert!(app.focused_panel.is_none());
        assert!(app.exploded_panel.is_none());
    }

    #[test]
    fn test_handle_key_focus_start_h() {
        let mut app = App::new_mock();
        assert!(app.focused_panel.is_none());

        app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
        assert!(app.focused_panel.is_some());
    }

    #[test]
    fn test_handle_key_focus_start_l() {
        let mut app = App::new_mock();
        assert!(app.focused_panel.is_none());

        app.handle_key(KeyCode::Char('l'), KeyModifiers::NONE);
        assert!(app.focused_panel.is_some());
    }

    #[test]
    fn test_handle_key_focus_navigation() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Cpu);

        // Navigate right
        app.handle_key(KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(app.focused_panel, Some(PanelType::Memory));

        // Navigate left
        app.handle_key(KeyCode::Left, KeyModifiers::NONE);
        assert_eq!(app.focused_panel, Some(PanelType::Cpu));
    }

    #[test]
    fn test_handle_key_explode_panel() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Cpu);

        // Explode with Enter
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(app.exploded_panel, Some(PanelType::Cpu));

        // Un-explode with Enter
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert!(app.exploded_panel.is_none());
    }

    #[test]
    fn test_handle_key_explode_with_z() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Memory);

        app.handle_key(KeyCode::Char('z'), KeyModifiers::NONE);
        assert_eq!(app.exploded_panel, Some(PanelType::Memory));
    }

    #[test]
    fn test_handle_key_esc_unexplode() {
        let mut app = App::new_mock();
        app.exploded_panel = Some(PanelType::Cpu);

        let quit = app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(!quit);
        assert!(app.exploded_panel.is_none());
    }

    #[test]
    fn test_handle_key_esc_unfocus() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Cpu);

        let quit = app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(!quit);
        assert!(app.focused_panel.is_none());
    }

    #[test]
    fn test_handle_key_files_view_mode() {
        let mut app = App::new_mock();
        use crate::state::FilesViewMode;

        assert_eq!(app.files_view_mode, FilesViewMode::Size);
        app.handle_key(KeyCode::Char('v'), KeyModifiers::NONE);
        assert_eq!(app.files_view_mode, FilesViewMode::Entropy);
    }

    #[test]
    fn test_navigate_panel_focus_wrap_right() {
        let mut app = App::new_mock();
        // Only keep CPU visible - disable everything else
        app.panels.memory = false;
        app.panels.disk = false;
        app.panels.network = false;
        app.panels.process = false;
        app.panels.gpu = false;
        app.panels.battery = false;
        app.panels.sensors = false;
        app.panels.files = false;

        app.focused_panel = Some(PanelType::Cpu);
        app.navigate_panel_focus(KeyCode::Right);
        // Should wrap to CPU (only visible panel)
        assert_eq!(app.focused_panel, Some(PanelType::Cpu));
    }

    #[test]
    fn test_navigate_panel_focus_wrap_left() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Cpu);
        app.navigate_panel_focus(KeyCode::Left);
        // Should wrap to last visible panel
        assert!(app.focused_panel.is_some());
    }

    #[test]
    fn test_navigate_panel_focus_empty() {
        let mut app = App::new_mock();
        app.panels.cpu = false;
        app.panels.memory = false;
        app.panels.disk = false;
        app.panels.network = false;
        app.panels.process = false;
        app.panels.gpu = false;
        app.panels.battery = false;
        app.panels.sensors = false;

        app.navigate_panel_focus(KeyCode::Right);
        // Should not panic
    }

    #[test]
    fn test_navigate_process_empty() {
        let mut app = App::new_mock();
        // Mock has no real processes
        app.navigate_process(1);
        app.navigate_process(-1);
        // Should not panic
    }

    #[test]
    fn test_process_count_empty() {
        let app = App::new_mock();
        assert_eq!(app.process_count(), 0);
    }

    #[test]
    fn test_sorted_processes_empty() {
        let app = App::new_mock();
        let procs = app.sorted_processes();
        assert!(procs.is_empty());
    }

    #[test]
    fn test_has_gpu_mock() {
        let app = App::new_mock();
        // Mock collectors are not "available"
        // This tests the has_gpu() method runs without panic
        let _has_gpu = app.has_gpu();
    }

    #[test]
    fn test_thrashing_severity() {
        let app = App::new_mock();
        let severity = app.thrashing_severity();
        assert_eq!(severity, ThrashingSeverity::None);
    }

    #[test]
    fn test_has_zram() {
        let app = App::new_mock();
        let _has = app.has_zram();
        // Just verify it doesn't panic
    }

    #[test]
    fn test_zram_ratio() {
        let app = App::new_mock();
        let ratio = app.zram_ratio();
        assert!(ratio >= 0.0);
    }

    #[test]
    fn test_selected_process_none() {
        let app = App::new_mock();
        assert!(app.selected_process().is_none());
    }

    #[test]
    fn test_request_signal_no_process() {
        let mut app = App::new_mock();
        app.request_signal(SignalType::Term);
        assert!(app.pending_signal.is_none()); // No process selected
    }

    #[test]
    fn test_cancel_signal() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test".to_string(), SignalType::Term));
        app.cancel_signal();
        assert!(app.pending_signal.is_none());
    }

    #[test]
    fn test_confirm_signal_none() {
        let mut app = App::new_mock();
        app.confirm_signal(); // No pending signal
        // Should not panic
    }

    #[test]
    fn test_clear_old_signal_result_none() {
        let mut app = App::new_mock();
        app.clear_old_signal_result();
        // Should not panic when no result
    }

    #[test]
    fn test_clear_old_signal_result_recent() {
        let mut app = App::new_mock();
        app.signal_result = Some((true, "test".to_string(), Instant::now()));
        app.clear_old_signal_result();
        assert!(app.signal_result.is_some()); // Not old enough
    }

    #[test]
    fn test_signal_menu_handling() {
        let mut app = App::new_mock();
        app.show_signal_menu = true;

        // ESC closes menu
        let quit = app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(!quit);
        assert!(!app.show_signal_menu);
    }

    #[test]
    fn test_signal_menu_keys() {
        let mut app = App::new_mock();

        // Test various signal menu keys
        app.show_signal_menu = true;
        app.handle_key(KeyCode::Char('x'), KeyModifiers::NONE);
        assert!(!app.show_signal_menu);

        app.show_signal_menu = true;
        app.handle_key(KeyCode::Char('K'), KeyModifiers::NONE);
        assert!(!app.show_signal_menu);

        app.show_signal_menu = true;
        app.handle_key(KeyCode::Char('H'), KeyModifiers::NONE);
        assert!(!app.show_signal_menu);
    }

    #[test]
    fn test_pending_signal_confirmation() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test".to_string(), SignalType::Term));

        // Y confirms
        let quit = app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);
        assert!(!quit);
        assert!(app.pending_signal.is_none());
    }

    #[test]
    fn test_pending_signal_cancel() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test".to_string(), SignalType::Term));

        // N cancels
        let quit = app.handle_key(KeyCode::Char('n'), KeyModifiers::NONE);
        assert!(!quit);
        assert!(app.pending_signal.is_none());
    }

    #[test]
    fn test_pending_signal_esc_cancels() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test".to_string(), SignalType::Term));

        let quit = app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(!quit);
        assert!(app.pending_signal.is_none());
    }

    #[test]
    fn test_process_navigation_keys() {
        let mut app = App::new_mock();

        // Home key
        app.process_selected = 5;
        app.handle_key(KeyCode::Home, KeyModifiers::NONE);
        assert_eq!(app.process_selected, 0);

        // g key
        app.process_selected = 5;
        app.handle_key(KeyCode::Char('g'), KeyModifiers::NONE);
        assert_eq!(app.process_selected, 0);
    }

    #[test]
    fn test_delete_clears_filter() {
        let mut app = App::new_mock();
        app.filter = "test".to_string();
        app.handle_key(KeyCode::Delete, KeyModifiers::NONE);
        assert!(app.filter.is_empty());
    }

    #[test]
    fn test_hjkl_focus_navigation() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Cpu);

        // l moves right in focus mode
        app.handle_key(KeyCode::Char('l'), KeyModifiers::NONE);
        assert_eq!(app.focused_panel, Some(PanelType::Memory));

        // h moves left
        app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
        assert_eq!(app.focused_panel, Some(PanelType::Cpu));
    }

    #[test]
    fn test_jk_process_nav_in_explode() {
        let mut app = App::new_mock();
        app.exploded_panel = Some(PanelType::Process);

        // j/k should navigate processes in explode mode
        app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
        // Should not panic
    }

    #[test]
    fn test_z_starts_focus() {
        let mut app = App::new_mock();
        assert!(app.focused_panel.is_none());
        assert!(app.exploded_panel.is_none());

        app.handle_key(KeyCode::Char('z'), KeyModifiers::NONE);
        assert!(app.focused_panel.is_some());
    }

    /// Test collect_metrics with real system data (for coverage)
    #[test]
    fn test_collect_metrics_real() {
        let mut app = App::new(false, false);
        // Run one real collection cycle for coverage
        app.collect_metrics();
        // Should complete without panic
        assert!(app.frame_id >= 1);
    }

    /// Test collect_metrics multiple cycles
    #[test]
    fn test_collect_metrics_cycles() {
        let mut app = App::new(false, false);
        let initial_frame = app.frame_id;
        app.collect_metrics();
        app.collect_metrics();
        assert_eq!(app.frame_id, initial_frame + 2);
    }

    /// Test history update in collect_metrics
    #[test]
    fn test_collect_metrics_history() {
        let mut app = App::new(false, false);
        let initial_cpu_len = app.cpu_history.len();
        app.collect_metrics();
        // History should have been updated (may or may not grow depending on collector state)
        assert!(app.cpu_history.len() >= initial_cpu_len);
    }

    /// Test push_to_history helper
    #[test]
    fn test_push_to_history() {
        let mut history = Vec::new();
        App::push_to_history(&mut history, 0.5);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0], 0.5);

        // Push more values
        for i in 0..400 {
            App::push_to_history(&mut history, i as f64 / 400.0);
        }
        // Should be capped at 300
        assert_eq!(history.len(), 300);
    }

    // === Micro-benchmark Performance Tests ===

    /// Verify collect_metrics completes within reasonable time
    /// Note: Real metrics collection involves many system calls (reading /proc, /sys, etc.)
    #[test]
    fn test_collect_metrics_performance() {
        use std::time::Instant;

        let mut app = App::new(false, false);
        let iterations = 5;
        let start = Instant::now();

        for _ in 0..iterations {
            app.collect_metrics();
        }

        let elapsed = start.elapsed();
        let avg_ms = elapsed.as_millis() / iterations as u128;

        // Real metrics collection with all collectors can take several seconds
        // on systems with many disks, network interfaces, and processes
        assert!(avg_ms < 5000, "collect_metrics too slow: {}ms avg", avg_ms);
    }

    /// Verify history push is O(1) amortized
    #[test]
    fn test_history_push_performance() {
        use std::time::Instant;

        let mut history = Vec::new();
        let iterations = 10000;
        let start = Instant::now();

        for i in 0..iterations {
            App::push_to_history(&mut history, i as f64 / iterations as f64);
        }

        let elapsed = start.elapsed();
        let per_op_ns = elapsed.as_nanos() / iterations as u128;

        // Each push should be sub-microsecond
        assert!(per_op_ns < 1000, "push_to_history too slow: {}ns per op", per_op_ns);
    }

    /// Verify App::new_mock is reasonably fast for testing
    /// Note: Mock creation initializes many analyzers which may read system state
    #[test]
    fn test_app_mock_creation_performance() {
        use std::time::Instant;

        let iterations = 50;
        let start = Instant::now();

        for _ in 0..iterations {
            let _ = App::new_mock();
        }

        let elapsed = start.elapsed();
        let avg_us = elapsed.as_micros() / iterations as u128;

        // Mock creation includes initializing analyzers which may touch system state
        // Allow up to 100ms each
        assert!(avg_us < 100000, "new_mock too slow: {}us avg", avg_us);
    }

    // === Additional Coverage Tests ===

    #[test]
    fn test_visible_panels_files_enabled() {
        let mut app = App::new_mock();
        app.panels.files = true;
        let panels = app.visible_panels();
        assert!(panels.contains(&PanelType::Files));
    }

    #[test]
    fn test_cancel_signal_with_pending() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test_proc".to_string(), SignalType::Term));
        app.cancel_signal();
        assert!(app.pending_signal.is_none());
    }

    #[test]
    fn test_pending_signal_hup() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "proc".to_string(), SignalType::Hup));
        assert!(app.pending_signal.is_some());
        if let Some((pid, name, signal)) = &app.pending_signal {
            assert_eq!(*pid, 1234);
            assert_eq!(name, "proc");
            assert_eq!(*signal, SignalType::Hup);
        }
    }

    #[test]
    fn test_pending_signal_int() {
        let mut app = App::new_mock();
        app.pending_signal = Some((5678, "daemon".to_string(), SignalType::Int));
        if let Some((_, _, signal)) = &app.pending_signal {
            assert_eq!(*signal, SignalType::Int);
        }
    }

    #[test]
    fn test_pending_signal_usr1() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1000, "app".to_string(), SignalType::Usr1));
        if let Some((_, _, signal)) = &app.pending_signal {
            assert_eq!(*signal, SignalType::Usr1);
        }
    }

    #[test]
    fn test_pending_signal_usr2() {
        let mut app = App::new_mock();
        app.pending_signal = Some((2000, "service".to_string(), SignalType::Usr2));
        if let Some((_, _, signal)) = &app.pending_signal {
            assert_eq!(*signal, SignalType::Usr2);
        }
    }

    #[test]
    fn test_pending_signal_stop() {
        let mut app = App::new_mock();
        app.pending_signal = Some((3000, "worker".to_string(), SignalType::Stop));
        if let Some((_, _, signal)) = &app.pending_signal {
            assert_eq!(*signal, SignalType::Stop);
        }
    }

    #[test]
    fn test_pending_signal_cont() {
        let mut app = App::new_mock();
        app.pending_signal = Some((4000, "bg_task".to_string(), SignalType::Cont));
        if let Some((_, _, signal)) = &app.pending_signal {
            assert_eq!(*signal, SignalType::Cont);
        }
    }

    #[test]
    fn test_pending_signal_kill() {
        let mut app = App::new_mock();
        app.pending_signal = Some((9999, "zombie".to_string(), SignalType::Kill));
        if let Some((_, _, signal)) = &app.pending_signal {
            assert_eq!(*signal, SignalType::Kill);
        }
    }

    #[test]
    fn test_signal_menu_key_i_int() {
        let mut app = App::new_mock();
        app.process_selected = 0;
        app.show_signal_menu = true;
        app.handle_key(KeyCode::Char('i'), KeyModifiers::NONE);
        assert!(!app.show_signal_menu);
    }

    #[test]
    fn test_signal_menu_key_p_stop() {
        let mut app = App::new_mock();
        app.process_selected = 0;
        app.show_signal_menu = true;
        app.handle_key(KeyCode::Char('p'), KeyModifiers::NONE);
        assert!(!app.show_signal_menu);
    }

    #[test]
    fn test_signal_menu_key_c_cont() {
        let mut app = App::new_mock();
        app.process_selected = 0;
        app.show_signal_menu = true;
        app.handle_key(KeyCode::Char('c'), KeyModifiers::NONE);
        assert!(!app.show_signal_menu);
    }

    #[test]
    fn test_signal_menu_key_unknown_ignored() {
        let mut app = App::new_mock();
        app.show_signal_menu = true;
        // Unknown key should not close menu
        app.handle_key(KeyCode::Char('z'), KeyModifiers::NONE);
        assert!(app.show_signal_menu);
    }

    #[test]
    fn test_filter_input_escape_clears() {
        let mut app = App::new_mock();
        app.show_filter_input = true;
        app.filter = "test".to_string();
        app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(!app.show_filter_input);
        assert!(app.filter.is_empty());
    }

    #[test]
    fn test_filter_input_backspace() {
        let mut app = App::new_mock();
        app.show_filter_input = true;
        app.filter = "test".to_string();
        app.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(app.filter, "tes");
    }

    #[test]
    fn test_filter_input_char_append() {
        let mut app = App::new_mock();
        app.show_filter_input = true;
        app.filter = "te".to_string();
        app.handle_key(KeyCode::Char('s'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('t'), KeyModifiers::NONE);
        assert_eq!(app.filter, "test");
    }

    #[test]
    fn test_filter_input_enter_closes() {
        let mut app = App::new_mock();
        app.show_filter_input = true;
        app.filter = "search".to_string();
        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert!(!app.show_filter_input);
        assert_eq!(app.filter, "search"); // Filter kept
    }

    // === Mock Data Verification Tests ===

    #[test]
    fn test_mock_gpus_populated() {
        let app = App::new_mock();
        assert!(!app.mock_gpus.is_empty(), "mock_gpus should be populated");
        assert_eq!(app.mock_gpus.len(), 2, "should have 2 mock GPUs");
        assert!(app.mock_gpus[0].name.contains("RTX"));
    }

    #[test]
    fn test_mock_battery_populated() {
        let app = App::new_mock();
        assert!(app.mock_battery.is_some(), "mock_battery should be populated");
        let bat = app.mock_battery.as_ref().expect("battery");
        assert!(bat.percent > 0.0);
        assert!(bat.health_percent > 0.0);
    }

    #[test]
    fn test_mock_sensors_populated() {
        let app = App::new_mock();
        assert!(!app.mock_sensors.is_empty(), "mock_sensors should be populated");
        assert!(app.mock_sensors.len() >= 3);
    }

    #[test]
    fn test_mock_containers_populated() {
        let app = App::new_mock();
        assert!(!app.mock_containers.is_empty(), "mock_containers should be populated");
        assert_eq!(app.mock_containers.len(), 3);
    }

    // === Additional Key Handling Tests ===

    #[test]
    fn test_panel_toggle_keys() {
        let mut app = App::new_mock();

        // Test panel 3 (disk)
        let original = app.panels.disk;
        app.handle_key(KeyCode::Char('3'), KeyModifiers::NONE);
        assert_ne!(app.panels.disk, original);

        // Test panel 4 (network)
        let original = app.panels.network;
        app.handle_key(KeyCode::Char('4'), KeyModifiers::NONE);
        assert_ne!(app.panels.network, original);

        // Test panel 5 (process)
        let original = app.panels.process;
        app.handle_key(KeyCode::Char('5'), KeyModifiers::NONE);
        assert_ne!(app.panels.process, original);

        // Test panel 6 (gpu)
        let original = app.panels.gpu;
        app.handle_key(KeyCode::Char('6'), KeyModifiers::NONE);
        assert_ne!(app.panels.gpu, original);

        // Test panel 7 (battery)
        let original = app.panels.battery;
        app.handle_key(KeyCode::Char('7'), KeyModifiers::NONE);
        assert_ne!(app.panels.battery, original);

        // Test panel 8 (sensors)
        let original = app.panels.sensors;
        app.handle_key(KeyCode::Char('8'), KeyModifiers::NONE);
        assert_ne!(app.panels.sensors, original);
    }

    #[test]
    fn test_navigation_when_focused() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Cpu);

        // h should navigate left
        app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
        // l should navigate right
        app.handle_key(KeyCode::Char('l'), KeyModifiers::NONE);
        // j should navigate down
        app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
        // k should navigate up
        app.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
    }

    #[test]
    fn test_navigation_when_not_focused() {
        let mut app = App::new_mock();
        app.focused_panel = None;

        // h should start focus
        app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
        assert!(app.focused_panel.is_some());

        // Reset and try l
        app.focused_panel = None;
        app.handle_key(KeyCode::Char('l'), KeyModifiers::NONE);
        assert!(app.focused_panel.is_some());
    }

    #[test]
    fn test_process_navigation_pageup_pagedown() {
        let mut app = App::new_mock();
        app.process_selected = 5;

        // Just test that keys are handled without panicking
        app.handle_key(KeyCode::PageDown, KeyModifiers::NONE);
        app.handle_key(KeyCode::PageUp, KeyModifiers::NONE);
    }

    #[test]
    fn test_process_navigation_home_end() {
        let mut app = App::new_mock();
        app.process_selected = 5;

        // g should go to start
        app.handle_key(KeyCode::Char('g'), KeyModifiers::NONE);
        assert_eq!(app.process_selected, 0);

        // G should go to end
        app.handle_key(KeyCode::Char('G'), KeyModifiers::NONE);
    }

    #[test]
    fn test_process_navigation_arrow_keys() {
        let mut app = App::new_mock();
        app.focused_panel = None;
        app.exploded_panel = None;
        app.process_selected = 5;

        // Just test that arrow keys are handled without panicking
        app.handle_key(KeyCode::Down, KeyModifiers::NONE);
        app.handle_key(KeyCode::Up, KeyModifiers::NONE);
    }

    #[test]
    fn test_process_navigation_with_exploded_panel() {
        let mut app = App::new_mock();
        app.exploded_panel = Some(PanelType::Process);
        app.process_selected = 0;

        // Just test that j/k are handled
        app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
    }

    #[test]
    fn test_help_toggle() {
        let mut app = App::new_mock();
        assert!(!app.show_help);

        app.handle_key(KeyCode::Char('?'), KeyModifiers::NONE);
        assert!(app.show_help);

        app.handle_key(KeyCode::Char('?'), KeyModifiers::NONE);
        assert!(!app.show_help);
    }

    #[test]
    fn test_f1_help_toggle() {
        let mut app = App::new_mock();
        assert!(!app.show_help);

        app.handle_key(KeyCode::F(1), KeyModifiers::NONE);
        assert!(app.show_help);
    }

    #[test]
    fn test_quit_key() {
        let mut app = App::new_mock();
        let quit = app.handle_key(KeyCode::Char('q'), KeyModifiers::NONE);
        assert!(quit);
    }

    #[test]
    fn test_esc_clears_focus() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Cpu);

        app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(app.focused_panel.is_none());
    }

    #[test]
    fn test_ctrl_c_returns_true() {
        let mut app = App::new_mock();
        let quit = app.handle_key(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(quit);
    }

    #[test]
    fn test_view_mode_cycle() {
        let mut app = App::new_mock();
        let original = app.files_view_mode;

        app.handle_key(KeyCode::Char('v'), KeyModifiers::NONE);
        // Should have cycled to next mode
        assert_ne!(app.files_view_mode, original);
    }

    #[test]
    fn test_pending_signal_confirm() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test".to_string(), SignalType::Term));

        // y confirms
        app.handle_key(KeyCode::Char('y'), KeyModifiers::NONE);
        assert!(app.pending_signal.is_none());
    }

    #[test]
    fn test_pending_signal_cancel_with_n_key() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test".to_string(), SignalType::Term));

        // n cancels
        app.handle_key(KeyCode::Char('n'), KeyModifiers::NONE);
        assert!(app.pending_signal.is_none());
    }

    #[test]
    fn test_panel_explode_toggle() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Cpu);
        app.exploded_panel = None;

        // z or Enter should toggle explode
        app.handle_key(KeyCode::Char('z'), KeyModifiers::NONE);
        assert!(app.exploded_panel.is_some());

        app.handle_key(KeyCode::Char('z'), KeyModifiers::NONE);
        assert!(app.exploded_panel.is_none());
    }

    #[test]
    fn test_signal_menu_keys_huk() {
        let mut app = App::new_mock();
        app.show_signal_menu = true;
        app.process_selected = 0;

        // Test H for HUP
        app.handle_key(KeyCode::Char('H'), KeyModifiers::NONE);

        app.show_signal_menu = true;
        // Test u for USR1
        app.handle_key(KeyCode::Char('u'), KeyModifiers::NONE);

        app.show_signal_menu = true;
        // Test U for USR2
        app.handle_key(KeyCode::Char('U'), KeyModifiers::NONE);
        // These keys should be handled without panicking
    }

    // === Additional Edge Case Tests ===

    #[test]
    fn test_signal_menu_all_signal_types() {
        let mut app = App::new_mock();

        // Test all signal menu keys
        for (key, _) in [
            ('x', SignalType::Term),
            ('K', SignalType::Kill),
            ('i', SignalType::Int),
            ('p', SignalType::Stop),
            ('c', SignalType::Cont),
        ] {
            app.show_signal_menu = true;
            app.pending_signal = None;
            app.handle_key(KeyCode::Char(key), KeyModifiers::NONE);
            assert!(!app.show_signal_menu);
        }
    }

    #[test]
    fn test_signal_menu_esc_closes_menu() {
        let mut app = App::new_mock();
        app.show_signal_menu = true;

        app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(!app.show_signal_menu);
    }

    #[test]
    fn test_filter_input_backspace_removal() {
        let mut app = App::new_mock();
        app.show_filter_input = true;
        app.filter = "test".to_string();

        app.handle_key(KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(app.filter, "tes");
    }

    #[test]
    fn test_filter_input_esc_clears_text() {
        let mut app = App::new_mock();
        app.show_filter_input = true;
        app.filter = "test".to_string();

        app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(!app.show_filter_input);
        assert!(app.filter.is_empty());
    }

    #[test]
    fn test_filter_input_enter_preserves() {
        let mut app = App::new_mock();
        app.show_filter_input = true;
        app.filter = "test".to_string();

        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert!(!app.show_filter_input);
        assert_eq!(app.filter, "test"); // Filter preserved
    }

    #[test]
    fn test_filter_input_add_char() {
        let mut app = App::new_mock();
        app.show_filter_input = true;
        app.filter = String::new();

        app.handle_key(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(app.filter, "a");
    }

    #[test]
    fn test_pending_signal_enter_confirms() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test".to_string(), SignalType::Term));

        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert!(app.pending_signal.is_none());
    }

    #[test]
    fn test_pending_signal_capital_y_confirms() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test".to_string(), SignalType::Term));

        app.handle_key(KeyCode::Char('Y'), KeyModifiers::NONE);
        assert!(app.pending_signal.is_none());
    }

    #[test]
    fn test_pending_signal_capital_n_cancels() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test".to_string(), SignalType::Term));

        app.handle_key(KeyCode::Char('N'), KeyModifiers::NONE);
        assert!(app.pending_signal.is_none());
    }

    #[test]
    fn test_pending_signal_esc_cancels_prompt() {
        let mut app = App::new_mock();
        app.pending_signal = Some((1234, "test".to_string(), SignalType::Term));

        app.handle_key(KeyCode::Esc, KeyModifiers::NONE);
        assert!(app.pending_signal.is_none());
    }

    #[test]
    fn test_exploded_panel_enter_exits() {
        let mut app = App::new_mock();
        app.exploded_panel = Some(PanelType::Cpu);

        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert!(app.exploded_panel.is_none());
    }

    #[test]
    fn test_focused_panel_arrow_navigation() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Cpu);
        app.exploded_panel = None;

        // Test all arrow directions
        app.handle_key(KeyCode::Left, KeyModifiers::NONE);
        app.handle_key(KeyCode::Right, KeyModifiers::NONE);
        app.handle_key(KeyCode::Up, KeyModifiers::NONE);
        app.handle_key(KeyCode::Down, KeyModifiers::NONE);
    }

    #[test]
    fn test_focused_panel_hjkl_navigation() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Memory);
        app.exploded_panel = None;

        // Test h/l navigation
        app.handle_key(KeyCode::Char('h'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('l'), KeyModifiers::NONE);
    }

    #[test]
    fn test_focused_panel_enter_explodes() {
        let mut app = App::new_mock();
        app.focused_panel = Some(PanelType::Disk);
        app.exploded_panel = None;

        app.handle_key(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(app.exploded_panel, Some(PanelType::Disk));
    }

    #[test]
    fn test_unfocused_process_navigation_jk() {
        let mut app = App::new_mock();
        app.focused_panel = None;
        app.exploded_panel = None;
        app.process_selected = 5;

        app.handle_key(KeyCode::Char('j'), KeyModifiers::NONE);
        app.handle_key(KeyCode::Char('k'), KeyModifiers::NONE);
    }

    #[test]
    fn test_push_to_history_overflow() {
        let mut history = Vec::new();
        for i in 0..305 {
            App::push_to_history(&mut history, i as f64);
        }
        assert_eq!(history.len(), 300);
        assert_eq!(history[0], 5.0); // First 5 elements should be removed
    }

    #[test]
    fn test_panel_visibility_all_fields() {
        let vis = PanelVisibility {
            cpu: false,
            memory: false,
            disk: false,
            network: false,
            process: false,
            gpu: false,
            battery: false,
            sensors: false,
            files: true,
        };
        assert!(!vis.cpu);
        assert!(vis.files);
    }

    #[test]
    fn test_mock_gpu_data_debug() {
        let gpu = MockGpuData {
            name: "Test GPU".to_string(),
            gpu_util: 50.0,
            vram_used: 1000,
            vram_total: 8000,
            temperature: 65.0,
            power_watts: 150,
            power_limit_watts: 300,
            clock_mhz: 1500,
            history: vec![0.5],
        };
        let debug = format!("{:?}", gpu);
        assert!(debug.contains("Test GPU"));
    }

    #[test]
    fn test_mock_battery_data_debug() {
        let bat = MockBatteryData {
            percent: 75.0,
            charging: true,
            time_remaining_mins: Some(120),
            power_watts: 45.0,
            health_percent: 95.0,
            cycle_count: 500,
        };
        let debug = format!("{:?}", bat);
        assert!(debug.contains("75"));
    }

    #[test]
    fn test_mock_sensor_data_debug() {
        let sensor = MockSensorData {
            name: "cpu/temp1".to_string(),
            label: "CPU".to_string(),
            value: 65.0,
            max: Some(90.0),
            crit: Some(100.0),
            sensor_type: MockSensorType::Temperature,
        };
        let debug = format!("{:?}", sensor);
        assert!(debug.contains("cpu/temp1"));
    }

    #[test]
    fn test_mock_container_data_debug() {
        let container = MockContainerData {
            name: "nginx".to_string(),
            status: "running".to_string(),
            cpu_percent: 5.0,
            mem_used: 100_000_000,
            mem_limit: 1_000_000_000,
        };
        let debug = format!("{:?}", container);
        assert!(debug.contains("nginx"));
    }

    #[test]
    fn test_mock_sensor_type_equality() {
        assert_eq!(MockSensorType::Temperature, MockSensorType::Temperature);
        assert_ne!(MockSensorType::Temperature, MockSensorType::Fan);
        assert_ne!(MockSensorType::Voltage, MockSensorType::Power);
    }

    #[test]
    fn test_toggle_panel_9_files_toggle() {
        let mut app = App::new_mock();
        let initial = app.panels.files;

        app.handle_key(KeyCode::Char('9'), KeyModifiers::NONE);
        assert_ne!(app.panels.files, initial);
    }

    // =========================================================================
    // TUI Load Testing (probar integration)
    // =========================================================================

    /// Test filter performance with large dataset using probar's TUI load testing.
    /// This test uses probar's synthetic data generator and detects hangs.
    #[test]
    fn test_filter_no_hang_with_5000_items() {
        use jugar_probar::tui_load::{DataGenerator, TuiLoadTest};
        use std::time::{Duration, Instant};

        // Generate 5000 synthetic process-like items
        let generator = DataGenerator::new(5000);
        let items = generator.generate();

        // Test filter performance with timeout
        let timeout = Duration::from_millis(1000);
        let filters = ["", "a", "sys", "chrome", "nonexistent_long_filter_string"];

        for filter in filters {
            let filter_lower = filter.to_lowercase();
            let start = Instant::now();

            // Simulate what sorted_processes does: filter then collect
            let filtered: Vec<_> = items
                .iter()
                .filter(|item| {
                    if filter_lower.is_empty() {
                        true
                    } else {
                        item.name.to_lowercase().contains(&filter_lower)
                            || item.description.to_lowercase().contains(&filter_lower)
                    }
                })
                .collect();

            let elapsed = start.elapsed();

            assert!(
                elapsed < timeout,
                "Filter '{}' took {:?} (timeout: {:?}) - HANG DETECTED with {} items, {} results",
                filter, elapsed, timeout, items.len(), filtered.len()
            );
        }
    }

    /// Test that filter performance is O(n) not O(n²) using probar load testing.
    #[test]
    fn test_filter_scales_linearly() {
        use jugar_probar::tui_load::DataGenerator;
        use std::time::Instant;

        let sizes = [100, 500, 1000, 2000, 5000];
        let mut times_us = Vec::new();

        for size in sizes {
            let items = DataGenerator::new(size).generate();
            let filter_lower = "sys".to_lowercase();

            let start = Instant::now();
            // Run filter 10 times for stable measurement
            for _ in 0..10 {
                let _: Vec<_> = items
                    .iter()
                    .filter(|item| {
                        item.name.to_lowercase().contains(&filter_lower)
                            || item.description.to_lowercase().contains(&filter_lower)
                    })
                    .collect();
            }
            let elapsed = start.elapsed().as_micros() as u64;
            times_us.push((size, elapsed));
        }

        // Check that time grows roughly linearly (< 3x for 5x data)
        // From 1000 to 5000 items should take roughly 5x longer (with tolerance)
        let time_1k = times_us.iter().find(|(s, _)| *s == 1000).map(|(_, t)| *t).unwrap_or(1);
        let time_5k = times_us.iter().find(|(s, _)| *s == 5000).map(|(_, t)| *t).unwrap_or(1);

        let ratio = time_5k as f64 / time_1k as f64;

        // Should scale roughly linearly: 5x data = ~5x time (allow up to 8x for overhead)
        assert!(
            ratio < 8.0,
            "Filter time scaled {}x from 1K to 5K items (expected ~5x). \
             Times: {:?}. May indicate O(n²) complexity.",
            ratio, times_us
        );
    }

    /// Stress test with probar's TuiLoadTest harness - tests for hangs, not microbenchmarks
    #[test]
    fn test_filter_stress_with_probar() {
        use jugar_probar::tui_load::TuiLoadTest;

        let load_test = TuiLoadTest::new()
            .with_item_count(5000)     // Test with 5000 items
            .with_timeout_ms(2000)      // 2 second timeout per frame
            .with_frames_per_filter(3);

        // Run filter stress test using allocation-free matching (like ttop's optimized code)
        let result = load_test.run_filter_stress(|items, filter| {
            let filter_lower = filter.to_lowercase();
            items
                .iter()
                .filter(|item| {
                    if filter_lower.is_empty() {
                        true
                    } else {
                        // Use allocation-free case-insensitive contains
                        contains_ignore_case(&item.name, &filter_lower)
                            || contains_ignore_case(&item.description, &filter_lower)
                    }
                })
                .cloned()
                .collect()
        });

        // The main assertion: no frame should timeout (no hangs)
        assert!(result.is_ok(), "TUI filter stress test detected hang: {:?}", result.err());

        // Verify we actually ran all the filters
        let results = result.expect("result should be ok");
        assert!(!results.is_empty(), "Should have run at least one filter");

        // Log performance for manual review (not a hard failure)
        for (filter, metrics) in &results {
            let avg = metrics.avg_frame_ms();
            assert!(
                avg < 500.0,
                "Filter '{}' took {:.1}ms avg - too slow for responsive UI",
                filter, avg
            );
        }
    }

    /// Integration load test that tests REAL App with REAL collectors.
    ///
    /// This test would have caught the container_analyzer hang because it:
    /// 1. Creates a real App (not mock)
    /// 2. Calls real collect methods
    /// 3. Measures component-level timings
    /// 4. Enforces per-component budgets (container_analyzer: 200ms max)
    ///
    /// The synthetic load tests missed the hang because they only tested
    /// filter performance with fake data, not actual system calls.
    #[test]
    fn test_integration_load_real_app_no_hang() {
        use jugar_probar::tui_load::{ComponentTimings, IntegrationLoadTest};

        // Set up integration test with component budgets
        let test = IntegrationLoadTest::new()
            .with_frame_budget_ms(500.0)   // 500ms total frame budget
            .with_timeout_ms(5000)          // 5 second timeout for hang detection
            .with_frame_count(3)            // Test 3 frames
            // Per-component budgets - this is what would catch the container_analyzer issue
            .with_component_budget("container_analyzer", 200.0)  // Max 200ms
            .with_component_budget("disk_analyzer", 200.0)
            .with_component_budget("network_analyzer", 200.0)
            .with_component_budget("sensor_analyzer", 100.0);

        // Track whether we're on first frame (initialization is slower)
        let first_frame = std::sync::atomic::AtomicBool::new(true);
        let app = std::sync::Mutex::new(None::<App>);

        let result = test.run(|| {
            let mut timings = ComponentTimings::new();

            // Get or create app
            let mut guard = app.lock().expect("lock");
            let app = guard.get_or_insert_with(|| {
                // First frame includes App::new() which is slower
                App::new(false, false) // deterministic=false, show_fps=false
            });

            // Measure individual analyzer times
            let start = Instant::now();
            app.container_analyzer.collect();
            timings.record("container_analyzer", start.elapsed().as_secs_f64() * 1000.0);

            let start = Instant::now();
            app.disk_io_analyzer.collect();
            timings.record("disk_analyzer", start.elapsed().as_secs_f64() * 1000.0);

            let start = Instant::now();
            app.network_stats.collect();
            timings.record("network_analyzer", start.elapsed().as_secs_f64() * 1000.0);

            let start = Instant::now();
            app.sensor_health.collect();
            timings.record("sensor_analyzer", start.elapsed().as_secs_f64() * 1000.0);

            // Skip strict budget check on first frame (initialization)
            if first_frame.swap(false, std::sync::atomic::Ordering::SeqCst) {
                // Return empty timings for first frame so budget checks are skipped
                return ComponentTimings::new();
            }

            timings
        });

        // This assertion WOULD HAVE FAILED before fixing container_analyzer
        // because docker stats --no-stream was blocking for 1.5+ seconds
        assert!(
            result.is_ok(),
            "Integration load test failed! This catches real collector hangs: {:?}",
            result.err()
        );

        let metrics = result.expect("test passed");
        assert!(
            metrics.p95_frame_ms() < 1000.0,
            "Frame time p95 {:.1}ms is too slow",
            metrics.p95_frame_ms()
        );
    }
}
