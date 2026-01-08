//! State types for ttop.

/// Files panel view mode - toggleable outlier views
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilesViewMode {
    /// Top 25 by size (space hogs)
    #[default]
    Size,
    /// Top 25 by entropy/duplicates (suspicious/wasteful)
    Entropy,
    /// Top 25 by I/O activity (hot files)
    Io,
}

impl FilesViewMode {
    /// Cycle to next view mode
    pub fn next(&self) -> Self {
        match self {
            Self::Size => Self::Entropy,
            Self::Entropy => Self::Io,
            Self::Io => Self::Size,
        }
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Size => "SIZE",
            Self::Entropy => "ENTROPY",
            Self::Io => "I/O",
        }
    }

    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Size => "Top 25 largest files",
            Self::Entropy => "Top 25 high entropy/duplicates",
            Self::Io => "Top 25 recently modified",
        }
    }

    /// Get key hint
    pub fn key_hint(&self) -> &'static str {
        "Tab: cycle view"
    }
}

/// Panel types for focus/explode navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelType {
    Cpu,
    Memory,
    Disk,
    Network,
    Process,
    Gpu,
    Battery,
    Sensors,
    Files,
}

impl PanelType {
    /// Get all panel types in display order
    pub fn all() -> &'static [PanelType] {
        &[
            PanelType::Cpu,
            PanelType::Memory,
            PanelType::Disk,
            PanelType::Network,
            PanelType::Process,
            PanelType::Gpu,
            PanelType::Battery,
            PanelType::Sensors,
            PanelType::Files,
        ]
    }

    /// Get the panel number (1-9) for display
    pub fn number(&self) -> u8 {
        match self {
            PanelType::Cpu => 1,
            PanelType::Memory => 2,
            PanelType::Disk => 3,
            PanelType::Network => 4,
            PanelType::Process => 5,
            PanelType::Gpu => 6,
            PanelType::Battery => 7,
            PanelType::Sensors => 8,
            PanelType::Files => 9,
        }
    }

    /// Get panel name for display
    pub fn name(&self) -> &'static str {
        match self {
            PanelType::Cpu => "CPU",
            PanelType::Memory => "Memory",
            PanelType::Disk => "Disk",
            PanelType::Network => "Network",
            PanelType::Process => "Process",
            PanelType::Gpu => "GPU",
            PanelType::Battery => "Battery",
            PanelType::Sensors => "Sensors",
            PanelType::Files => "Files",
        }
    }

    /// Get next panel in sequence (wraps around)
    pub fn next(&self) -> Self {
        match self {
            PanelType::Cpu => PanelType::Memory,
            PanelType::Memory => PanelType::Disk,
            PanelType::Disk => PanelType::Network,
            PanelType::Network => PanelType::Process,
            PanelType::Process => PanelType::Gpu,
            PanelType::Gpu => PanelType::Battery,
            PanelType::Battery => PanelType::Sensors,
            PanelType::Sensors => PanelType::Files,
            PanelType::Files => PanelType::Cpu,
        }
    }

    /// Get previous panel in sequence (wraps around)
    pub fn prev(&self) -> Self {
        match self {
            PanelType::Cpu => PanelType::Files,
            PanelType::Memory => PanelType::Cpu,
            PanelType::Disk => PanelType::Memory,
            PanelType::Network => PanelType::Disk,
            PanelType::Process => PanelType::Network,
            PanelType::Gpu => PanelType::Process,
            PanelType::Battery => PanelType::Gpu,
            PanelType::Sensors => PanelType::Battery,
            PanelType::Files => PanelType::Sensors,
        }
    }
}

/// Unix signals for process control
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalType {
    /// SIGTERM (15) - Graceful termination
    Term,
    /// SIGKILL (9) - Force kill
    Kill,
    /// SIGHUP (1) - Hangup / reload config
    Hup,
    /// SIGINT (2) - Interrupt
    Int,
    /// SIGUSR1 (10) - User-defined signal 1
    Usr1,
    /// SIGUSR2 (12) - User-defined signal 2
    Usr2,
    /// SIGSTOP (19) - Pause process
    Stop,
    /// SIGCONT (18) - Continue paused process
    Cont,
}

impl SignalType {
    /// Get the Unix signal number
    #[cfg(unix)]
    pub fn number(&self) -> i32 {
        match self {
            SignalType::Term => 15,
            SignalType::Kill => 9,
            SignalType::Hup => 1,
            SignalType::Int => 2,
            SignalType::Usr1 => 10,
            SignalType::Usr2 => 12,
            SignalType::Stop => 19,
            SignalType::Cont => 18,
        }
    }

    #[cfg(not(unix))]
    pub fn number(&self) -> i32 {
        0
    }

    /// Get the display name
    pub fn name(&self) -> &'static str {
        match self {
            SignalType::Term => "TERM",
            SignalType::Kill => "KILL",
            SignalType::Hup => "HUP",
            SignalType::Int => "INT",
            SignalType::Usr1 => "USR1",
            SignalType::Usr2 => "USR2",
            SignalType::Stop => "STOP",
            SignalType::Cont => "CONT",
        }
    }

    /// Get key binding for this signal
    pub fn key(&self) -> char {
        match self {
            SignalType::Term => 'x',
            SignalType::Kill => 'K',
            SignalType::Hup => 'H',
            SignalType::Int => 'i',
            SignalType::Usr1 => '1',
            SignalType::Usr2 => '2',
            SignalType::Stop => 'p',
            SignalType::Cont => 'c',
        }
    }

    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            SignalType::Term => "Graceful shutdown",
            SignalType::Kill => "Force kill (cannot be caught)",
            SignalType::Hup => "Reload config / hangup",
            SignalType::Int => "Interrupt (like Ctrl+C)",
            SignalType::Usr1 => "User signal 1",
            SignalType::Usr2 => "User signal 2",
            SignalType::Stop => "Pause process",
            SignalType::Cont => "Resume paused process",
        }
    }

    /// All available signals
    pub fn all() -> &'static [SignalType] {
        &[
            SignalType::Term,
            SignalType::Kill,
            SignalType::Hup,
            SignalType::Int,
            SignalType::Usr1,
            SignalType::Usr2,
            SignalType::Stop,
            SignalType::Cont,
        ]
    }
}

/// Process sort column
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProcessSortColumn {
    Pid,
    Name,
    #[default]
    Cpu,
    Mem,
    State,
    User,
    Threads,
}

impl ProcessSortColumn {
    /// Get the display name for this column
    pub fn name(&self) -> &'static str {
        match self {
            Self::Pid => "PID",
            Self::Name => "NAME",
            Self::Cpu => "CPU%",
            Self::Mem => "MEM%",
            Self::State => "STATE",
            Self::User => "USER",
            Self::Threads => "THR",
        }
    }

    /// Cycle to the next column
    pub fn next(&self) -> Self {
        match self {
            Self::Pid => Self::Name,
            Self::Name => Self::Cpu,
            Self::Cpu => Self::Mem,
            Self::Mem => Self::State,
            Self::State => Self::User,
            Self::User => Self::Threads,
            Self::Threads => Self::Pid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_column_cycle() {
        let mut col = ProcessSortColumn::Pid;
        col = col.next();
        assert_eq!(col, ProcessSortColumn::Name);
        col = col.next();
        assert_eq!(col, ProcessSortColumn::Cpu);
    }

    #[test]
    fn test_sort_column_name() {
        assert_eq!(ProcessSortColumn::Cpu.name(), "CPU%");
        assert_eq!(ProcessSortColumn::Mem.name(), "MEM%");
    }

    #[test]
    fn test_sort_column_full_cycle() {
        let start = ProcessSortColumn::Pid;
        let mut col = start;
        for _ in 0..7 {
            col = col.next();
        }
        assert_eq!(col, start);
    }

    #[test]
    fn test_sort_column_all_names() {
        assert_eq!(ProcessSortColumn::Pid.name(), "PID");
        assert_eq!(ProcessSortColumn::Name.name(), "NAME");
        assert_eq!(ProcessSortColumn::State.name(), "STATE");
        assert_eq!(ProcessSortColumn::User.name(), "USER");
        assert_eq!(ProcessSortColumn::Threads.name(), "THR");
    }

    #[test]
    fn test_files_view_mode_default() {
        assert_eq!(FilesViewMode::default(), FilesViewMode::Size);
    }

    #[test]
    fn test_files_view_mode_cycle() {
        let mode = FilesViewMode::Size;
        assert_eq!(mode.next(), FilesViewMode::Entropy);
        assert_eq!(mode.next().next(), FilesViewMode::Io);
        assert_eq!(mode.next().next().next(), FilesViewMode::Size);
    }

    #[test]
    fn test_files_view_mode_names() {
        assert_eq!(FilesViewMode::Size.name(), "SIZE");
        assert_eq!(FilesViewMode::Entropy.name(), "ENTROPY");
        assert_eq!(FilesViewMode::Io.name(), "I/O");
    }

    #[test]
    fn test_files_view_mode_descriptions() {
        assert_eq!(FilesViewMode::Size.description(), "Top 25 largest files");
        assert_eq!(FilesViewMode::Entropy.description(), "Top 25 high entropy/duplicates");
        assert_eq!(FilesViewMode::Io.description(), "Top 25 recently modified");
    }

    #[test]
    fn test_files_view_mode_key_hint() {
        assert_eq!(FilesViewMode::Size.key_hint(), "Tab: cycle view");
        assert_eq!(FilesViewMode::Entropy.key_hint(), "Tab: cycle view");
        assert_eq!(FilesViewMode::Io.key_hint(), "Tab: cycle view");
    }

    #[test]
    fn test_panel_type_all() {
        let all = PanelType::all();
        assert_eq!(all.len(), 9);
        assert_eq!(all[0], PanelType::Cpu);
        assert_eq!(all[8], PanelType::Files);
    }

    #[test]
    fn test_panel_type_numbers() {
        assert_eq!(PanelType::Cpu.number(), 1);
        assert_eq!(PanelType::Memory.number(), 2);
        assert_eq!(PanelType::Disk.number(), 3);
        assert_eq!(PanelType::Network.number(), 4);
        assert_eq!(PanelType::Process.number(), 5);
        assert_eq!(PanelType::Gpu.number(), 6);
        assert_eq!(PanelType::Battery.number(), 7);
        assert_eq!(PanelType::Sensors.number(), 8);
        assert_eq!(PanelType::Files.number(), 9);
    }

    #[test]
    fn test_panel_type_names() {
        assert_eq!(PanelType::Cpu.name(), "CPU");
        assert_eq!(PanelType::Memory.name(), "Memory");
        assert_eq!(PanelType::Disk.name(), "Disk");
        assert_eq!(PanelType::Network.name(), "Network");
        assert_eq!(PanelType::Process.name(), "Process");
        assert_eq!(PanelType::Gpu.name(), "GPU");
        assert_eq!(PanelType::Battery.name(), "Battery");
        assert_eq!(PanelType::Sensors.name(), "Sensors");
        assert_eq!(PanelType::Files.name(), "Files");
    }

    #[test]
    fn test_panel_type_next() {
        assert_eq!(PanelType::Cpu.next(), PanelType::Memory);
        assert_eq!(PanelType::Memory.next(), PanelType::Disk);
        assert_eq!(PanelType::Disk.next(), PanelType::Network);
        assert_eq!(PanelType::Network.next(), PanelType::Process);
        assert_eq!(PanelType::Process.next(), PanelType::Gpu);
        assert_eq!(PanelType::Gpu.next(), PanelType::Battery);
        assert_eq!(PanelType::Battery.next(), PanelType::Sensors);
        assert_eq!(PanelType::Sensors.next(), PanelType::Files);
        assert_eq!(PanelType::Files.next(), PanelType::Cpu);
    }

    #[test]
    fn test_panel_type_prev() {
        assert_eq!(PanelType::Cpu.prev(), PanelType::Files);
        assert_eq!(PanelType::Memory.prev(), PanelType::Cpu);
        assert_eq!(PanelType::Disk.prev(), PanelType::Memory);
        assert_eq!(PanelType::Network.prev(), PanelType::Disk);
        assert_eq!(PanelType::Process.prev(), PanelType::Network);
        assert_eq!(PanelType::Gpu.prev(), PanelType::Process);
        assert_eq!(PanelType::Battery.prev(), PanelType::Gpu);
        assert_eq!(PanelType::Sensors.prev(), PanelType::Battery);
        assert_eq!(PanelType::Files.prev(), PanelType::Sensors);
    }

    #[test]
    fn test_panel_type_full_cycle_next() {
        let start = PanelType::Cpu;
        let mut panel = start;
        for _ in 0..9 {
            panel = panel.next();
        }
        assert_eq!(panel, start);
    }

    #[test]
    fn test_panel_type_full_cycle_prev() {
        let start = PanelType::Cpu;
        let mut panel = start;
        for _ in 0..9 {
            panel = panel.prev();
        }
        assert_eq!(panel, start);
    }

    #[test]
    fn test_signal_type_numbers() {
        assert_eq!(SignalType::Term.number(), 15);
        assert_eq!(SignalType::Kill.number(), 9);
        assert_eq!(SignalType::Hup.number(), 1);
        assert_eq!(SignalType::Int.number(), 2);
        assert_eq!(SignalType::Usr1.number(), 10);
        assert_eq!(SignalType::Usr2.number(), 12);
        assert_eq!(SignalType::Stop.number(), 19);
        assert_eq!(SignalType::Cont.number(), 18);
    }

    #[test]
    fn test_signal_type_names() {
        assert_eq!(SignalType::Term.name(), "TERM");
        assert_eq!(SignalType::Kill.name(), "KILL");
        assert_eq!(SignalType::Hup.name(), "HUP");
        assert_eq!(SignalType::Int.name(), "INT");
        assert_eq!(SignalType::Usr1.name(), "USR1");
        assert_eq!(SignalType::Usr2.name(), "USR2");
        assert_eq!(SignalType::Stop.name(), "STOP");
        assert_eq!(SignalType::Cont.name(), "CONT");
    }

    #[test]
    fn test_signal_type_keys() {
        assert_eq!(SignalType::Term.key(), 'x');
        assert_eq!(SignalType::Kill.key(), 'K');
        assert_eq!(SignalType::Hup.key(), 'H');
        assert_eq!(SignalType::Int.key(), 'i');
        assert_eq!(SignalType::Usr1.key(), '1');
        assert_eq!(SignalType::Usr2.key(), '2');
        assert_eq!(SignalType::Stop.key(), 'p');
        assert_eq!(SignalType::Cont.key(), 'c');
    }

    #[test]
    fn test_signal_type_descriptions() {
        assert_eq!(SignalType::Term.description(), "Graceful shutdown");
        assert_eq!(SignalType::Kill.description(), "Force kill (cannot be caught)");
        assert_eq!(SignalType::Hup.description(), "Reload config / hangup");
        assert_eq!(SignalType::Int.description(), "Interrupt (like Ctrl+C)");
        assert_eq!(SignalType::Usr1.description(), "User signal 1");
        assert_eq!(SignalType::Usr2.description(), "User signal 2");
        assert_eq!(SignalType::Stop.description(), "Pause process");
        assert_eq!(SignalType::Cont.description(), "Resume paused process");
    }

    #[test]
    fn test_signal_type_all() {
        let all = SignalType::all();
        assert_eq!(all.len(), 8);
        assert_eq!(all[0], SignalType::Term);
        assert_eq!(all[7], SignalType::Cont);
    }

    #[test]
    fn test_process_sort_column_default() {
        assert_eq!(ProcessSortColumn::default(), ProcessSortColumn::Cpu);
    }
}
