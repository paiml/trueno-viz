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

use crate::analyzers::{ContainerAnalyzer, DiskIoAnalyzer, GpuProcessAnalyzer, PsiAnalyzer, StorageAnalyzer, SwapAnalyzer, ThrashingSeverity};
use crate::state::ProcessSortColumn;

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

    // Frame timing
    pub frame_id: u64,
    pub last_collect: Instant,
    pub avg_frame_time_us: u64,
    pub max_frame_time_us: u64,
    pub show_fps: bool,

    // Mode flags
    pub deterministic: bool,
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

            panels: PanelVisibility::default(),
            process_selected: 0,
            process_scroll_offset: 0,
            sort_column: ProcessSortColumn::Cpu,
            sort_descending: true,
            filter: String::new(),
            show_filter_input: false,
            show_help: false,
            show_tree: false,

            frame_id: 0,
            last_collect: Instant::now(),
            avg_frame_time_us: 0,
            max_frame_time_us: 0,
            show_fps,

            deterministic,
        };

        // Initial collection (need 2 for deltas)
        debug::log(Level::Debug, "app", "Initial metrics collection (1/2)...");
        app.collect_metrics();
        debug::log(Level::Debug, "app", "Initial metrics collection (2/2)...");
        app.collect_metrics();
        debug::log(Level::Info, "app", "App initialization complete");

        app
    }

    /// Collect metrics from all collectors
    pub fn collect_metrics(&mut self) {
        use trueno_viz::monitor::debug::{self, Level};

        self.frame_id += 1;
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

        match code {
            // Quit
            KeyCode::Char('q') | KeyCode::Esc => return true,
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => return true,

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

            // Navigation
            KeyCode::Down | KeyCode::Char('j') => self.navigate_process(1),
            KeyCode::Up | KeyCode::Char('k') => self.navigate_process(-1),
            KeyCode::PageDown => self.navigate_process(10),
            KeyCode::PageUp => self.navigate_process(-10),
            KeyCode::Home | KeyCode::Char('g') => self.process_selected = 0,
            KeyCode::End | KeyCode::Char('G') => {
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

            // Filter
            KeyCode::Char('f') | KeyCode::Char('/') => {
                self.show_filter_input = true;
            }
            KeyCode::Delete => self.filter.clear(),

            // Reset view
            KeyCode::Char('0') => self.panels = PanelVisibility::default(),

            _ => {}
        }

        false
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
        self.process
            .processes()
            .values()
            .filter(|p| {
                if self.filter.is_empty() {
                    true
                } else {
                    p.name.to_lowercase().contains(&self.filter.to_lowercase())
                        || p.cmdline
                            .to_lowercase()
                            .contains(&self.filter.to_lowercase())
                }
            })
            .count()
    }

    /// Get sorted and filtered processes
    pub fn sorted_processes(&self) -> Vec<&trueno_viz::monitor::collectors::process::ProcessInfo> {
        let mut procs: Vec<_> = self
            .process
            .processes()
            .values()
            .filter(|p| {
                if self.filter.is_empty() {
                    true
                } else {
                    p.name.to_lowercase().contains(&self.filter.to_lowercase())
                        || p.cmdline
                            .to_lowercase()
                            .contains(&self.filter.to_lowercase())
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
}
