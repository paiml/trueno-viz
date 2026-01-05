//! State types for ttop.

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
}
