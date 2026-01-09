//! File analysis for treemap visualization.
//!
//! Provides file metadata for enhanced treemap display:
//! - Size-based treemap data
//! - Recently modified file detection
//! - Duplicate file detection (by size + partial hash)
//! - Directory depth analysis
//! - Large file growth tracking
//! - Per-file I/O activity tracking
//! - Per-file entropy estimation

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::fs;

#[cfg(target_os = "linux")]
use std::io::{Seek, SeekFrom};

/// File type classification for coloring
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileType {
    Code,       // .rs, .py, .js, .go, .c, .cpp, etc.
    Config,     // .json, .yaml, .toml, .xml, etc.
    Log,        // .log, syslog, journal
    Media,      // images, video, audio
    Archive,    // .tar, .zip, .gz, etc.
    Document,   // .pdf, .doc, .txt, .md
    Data,       // .db, .sqlite, .csv
    Binary,     // executables, .so, .dll
    NodeModules,// node_modules (special case)
    Other,
}

impl FileType {
    pub fn from_path(path: &Path) -> Self {
        let path_str = path.to_string_lossy().to_lowercase();

        // Special directories
        if path_str.contains("node_modules") {
            return Self::NodeModules;
        }

        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        match ext.as_str() {
            // Code
            "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "go" | "c" | "cpp" | "h" | "hpp" |
            "java" | "kt" | "swift" | "rb" | "php" | "sh" | "bash" | "zsh" => Self::Code,

            // Config
            "json" | "yaml" | "yml" | "toml" | "xml" | "ini" | "cfg" | "conf" | "env" => Self::Config,

            // Log
            "log" => Self::Log,

            // Media
            "jpg" | "jpeg" | "png" | "gif" | "svg" | "webp" | "ico" |
            "mp3" | "wav" | "flac" | "ogg" | "m4a" |
            "mp4" | "mkv" | "avi" | "mov" | "webm" => Self::Media,

            // Archive
            "tar" | "gz" | "xz" | "bz2" | "zip" | "rar" | "7z" | "zst" => Self::Archive,

            // Document
            "pdf" | "doc" | "docx" | "txt" | "md" | "rst" | "odt" => Self::Document,

            // Data
            "db" | "sqlite" | "sqlite3" | "csv" | "parquet" | "arrow" => Self::Data,

            // Binary
            "so" | "dll" | "dylib" | "exe" | "o" | "a" => Self::Binary,

            _ => {
                // Check for log files without extension
                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                if name.contains("log") || name == "syslog" || name == "messages" {
                    Self::Log
                } else {
                    Self::Other
                }
            }
        }
    }

    /// Get color for this file type (RGB)
    pub fn color(&self) -> (u8, u8, u8) {
        match self {
            Self::Code => (100, 200, 100),       // Green
            Self::Config => (200, 200, 100),    // Yellow
            Self::Log => (255, 150, 50),        // Orange
            Self::Media => (100, 150, 255),     // Blue
            Self::Archive => (150, 100, 200),   // Purple
            Self::Document => (200, 200, 200),  // Gray
            Self::Data => (100, 200, 200),      // Cyan
            Self::Binary => (200, 100, 100),    // Red
            Self::NodeModules => (150, 150, 150), // Dark gray
            Self::Other => (128, 128, 128),     // Mid gray
        }
    }

    /// Get icon character for this file type
    pub fn icon(&self) -> char {
        match self {
            Self::Code => '⌘',       // Code/programming
            Self::Config => '⚙',     // Config/gear
            Self::Log => '≡',        // Log/list
            Self::Media => '◈',      // Media/image
            Self::Archive => '▣',    // Archive/box
            Self::Document => '◲',   // Document
            Self::Data => '⌗',       // Database
            Self::Binary => '◉',     // Binary/executable
            Self::NodeModules => '▦', // Package
            Self::Other => '◌',      // Generic file
        }
    }

    /// Get short label for this file type
    pub fn label(&self) -> &'static str {
        match self {
            Self::Code => "code",
            Self::Config => "cfg",
            Self::Log => "log",
            Self::Media => "media",
            Self::Archive => "arch",
            Self::Document => "doc",
            Self::Data => "data",
            Self::Binary => "bin",
            Self::NodeModules => "npm",
            Self::Other => "file",
        }
    }
}

/// I/O activity level for a file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IoActivity {
    #[default]
    None,
    Low,       // Some recent I/O
    Medium,    // Moderate I/O
    High,      // Heavy I/O (hot file)
}

impl IoActivity {
    /// Get icon for I/O activity level
    pub fn icon(&self) -> char {
        match self {
            Self::None => ' ',
            Self::Low => '▁',
            Self::Medium => '▃',
            Self::High => '▇',
        }
    }

    /// Get color for I/O activity (RGB)
    pub fn color(&self) -> (u8, u8, u8) {
        match self {
            Self::None => (80, 80, 80),       // Gray
            Self::Low => (100, 180, 100),     // Green
            Self::Medium => (220, 180, 80),   // Yellow
            Self::High => (255, 100, 80),     // Red-orange
        }
    }
}

/// Entropy level for file content
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EntropyLevel {
    #[default]
    Unknown,
    Low,       // 0-0.3: highly repetitive/compressible
    Medium,    // 0.3-0.7: mixed content
    High,      // 0.7-0.9: diverse/unique
    VeryHigh,  // 0.9-1.0: random/encrypted
}

impl EntropyLevel {
    /// Get icon for entropy level
    pub fn icon(&self) -> char {
        match self {
            Self::Unknown => '?',
            Self::Low => '○',      // Low entropy = duplicates likely
            Self::Medium => '◐',
            Self::High => '●',
            Self::VeryHigh => '◉', // Very high = encrypted/compressed
        }
    }

    /// Get color for entropy level (RGB)
    pub fn color(&self) -> (u8, u8, u8) {
        match self {
            Self::Unknown => (100, 100, 100),
            Self::Low => (100, 200, 100),     // Green (compressible = good for dedup)
            Self::Medium => (180, 180, 100),  // Yellow
            Self::High => (200, 140, 100),    // Orange
            Self::VeryHigh => (200, 100, 150), // Pink (encrypted/random)
        }
    }

    /// Create from normalized entropy value (0.0-1.0)
    pub fn from_entropy(entropy: f64) -> Self {
        if entropy >= 0.9 {
            Self::VeryHigh
        } else if entropy >= 0.7 {
            Self::High
        } else if entropy >= 0.3 {
            Self::Medium
        } else if entropy > 0.0 {
            Self::Low
        } else {
            Self::Unknown
        }
    }
}

/// File entry with analysis metadata
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
    pub file_type: FileType,
    pub depth: u32,
    pub modified: Option<SystemTime>,
    pub is_recent: bool,      // Modified in last N minutes
    pub growth_rate: f64,     // Bytes per second (for watched files)
    pub io_activity: IoActivity,  // Current I/O activity level
    pub entropy: f64,         // Shannon entropy (normalized 0-1)
    pub entropy_level: EntropyLevel, // Categorical entropy level
    pub is_duplicate: bool,   // Part of a duplicate group
}

/// Potential duplicate group
#[derive(Debug, Clone)]
pub struct DuplicateGroup {
    pub size: u64,
    pub paths: Vec<PathBuf>,
    pub wasted_bytes: u64,
}

/// Large file being watched for growth
#[derive(Debug, Clone)]
pub struct WatchedFile {
    pub path: PathBuf,
    pub size_history: Vec<(Instant, u64)>,
    pub growth_rate: f64,  // bytes/sec
    pub alert_threshold: f64, // bytes/sec to trigger alert
}

impl WatchedFile {
    pub fn new(path: PathBuf, threshold: f64) -> Self {
        Self {
            path,
            size_history: Vec::new(),
            growth_rate: 0.0,
            alert_threshold: threshold,
        }
    }

    pub fn update(&mut self, size: u64) {
        let now = Instant::now();
        self.size_history.push((now, size));

        // Keep last 60 samples
        if self.size_history.len() > 60 {
            self.size_history.remove(0);
        }

        // Calculate growth rate
        if let (Some((t1, s1)), Some((t2, s2))) =
            (self.size_history.first(), self.size_history.last())
        {
            let duration = t2.duration_since(*t1).as_secs_f64();
            if duration > 0.0 {
                self.growth_rate = (*s2 as f64 - *s1 as f64) / duration;
            }
        }
    }

    pub fn is_alerting(&self) -> bool {
        self.growth_rate > self.alert_threshold
    }

    /// Estimate time until disk full given available space
    pub fn time_to_full(&self, available_bytes: u64) -> Option<Duration> {
        if self.growth_rate > 0.0 {
            let secs = available_bytes as f64 / self.growth_rate;
            Some(Duration::from_secs_f64(secs))
        } else {
            None
        }
    }
}

/// Global file activity metrics for sparklines
#[derive(Debug, Clone, Default)]
pub struct FileActivityMetrics {
    /// Count of files with high I/O activity
    pub high_io_count: usize,
    /// Count of files with high entropy
    pub high_entropy_count: usize,
    /// Count of duplicate files
    pub duplicate_count: usize,
    /// Count of recently modified files
    pub recent_count: usize,
    /// Total bytes in duplicates (wasted)
    pub duplicate_bytes: u64,
    /// Average entropy across sampled files
    pub avg_entropy: f64,
}

/// File analyzer for treemap enhancements
pub struct FileAnalyzer {
    /// All scanned files
    files: Vec<FileEntry>,
    /// Files modified recently (within threshold)
    recent_files: Vec<PathBuf>,
    /// Potential duplicate groups
    duplicates: Vec<DuplicateGroup>,
    /// Files being watched for growth
    watchlist: HashMap<PathBuf, WatchedFile>,
    /// Max directory depth found
    max_depth: u32,
    /// Recent file threshold
    recent_threshold: Duration,
    /// Last scan time
    last_scan: Instant,
    /// Scan interval
    scan_interval: Duration,
    /// History of activity metrics for sparklines
    activity_history: Vec<FileActivityMetrics>,
    /// Entropy sample size (bytes per file)
    entropy_sample_size: usize,
}

impl FileAnalyzer {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            recent_files: Vec::new(),
            duplicates: Vec::new(),
            watchlist: HashMap::new(),
            max_depth: 0,
            recent_threshold: Duration::from_secs(300), // 5 minutes
            last_scan: Instant::now() - Duration::from_secs(3600),
            scan_interval: Duration::from_secs(30),
            activity_history: Vec::with_capacity(60),
            entropy_sample_size: 2048, // 2KB sample per file
        }
    }

    /// Collect file data from a directory
    #[cfg(target_os = "linux")]
    pub fn collect(&mut self, root: &str) {
        // Rate limit scanning
        if self.last_scan.elapsed() < self.scan_interval {
            // Just update watchlist
            self.update_watchlist();
            return;
        }
        self.last_scan = Instant::now();

        self.files.clear();
        self.recent_files.clear();
        self.max_depth = 0;

        let root_path = Path::new(root);
        self.scan_directory(root_path, 0);

        // Find duplicates and mark files
        self.find_duplicates();
        self.mark_duplicate_files();

        // Sample entropy for largest files
        self.sample_file_entropy();

        // Update watchlist
        self.update_watchlist();

        // Track activity metrics for sparklines
        self.update_activity_history();
    }

    #[cfg(target_os = "macos")]
    pub fn collect(&mut self, root: &str) {
        // Rate limit scanning
        if self.last_scan.elapsed() < self.scan_interval {
            return;
        }
        self.last_scan = Instant::now();

        self.files.clear();
        self.recent_files.clear();
        self.max_depth = 0;

        let root_path = Path::new(root);
        self.scan_directory(root_path, 0);

        // Find duplicates by size (simplified for macOS)
        self.find_duplicates();
        self.mark_duplicate_files();

        // Track activity metrics for sparklines
        self.update_activity_history();
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    pub fn collect(&mut self, _root: &str) {
        // Not implemented on this platform
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn scan_directory(&mut self, dir: &Path, depth: u32) {
        const MAX_FILES: usize = 10000;
        const MAX_DEPTH: u32 = 20;

        if self.files.len() >= MAX_FILES || depth > MAX_DEPTH {
            return;
        }

        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            if self.files.len() >= MAX_FILES {
                break;
            }

            let path = entry.path();
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            if metadata.is_file() {
                let modified = metadata.modified().ok();
                let is_recent = modified
                    .and_then(|m| m.elapsed().ok())
                    .map(|d| d < self.recent_threshold)
                    .unwrap_or(false);

                if is_recent {
                    self.recent_files.push(path.clone());
                }

                let file_type = FileType::from_path(&path);

                // Determine I/O activity based on modification recency
                let io_activity = if is_recent {
                    // Recently modified = some I/O activity
                    let elapsed = modified.and_then(|m| m.elapsed().ok()).unwrap_or_default();
                    if elapsed < Duration::from_secs(60) {
                        IoActivity::High
                    } else if elapsed < Duration::from_secs(180) {
                        IoActivity::Medium
                    } else {
                        IoActivity::Low
                    }
                } else {
                    IoActivity::None
                };

                self.files.push(FileEntry {
                    path,
                    size: metadata.len(),
                    file_type,
                    depth,
                    modified,
                    is_recent,
                    growth_rate: 0.0,
                    io_activity,
                    entropy: 0.0,
                    entropy_level: EntropyLevel::Unknown,
                    is_duplicate: false,
                });

                self.max_depth = self.max_depth.max(depth);
            } else if metadata.is_dir() {
                // Skip common large/uninteresting directories
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str == ".git" || name_str == "target" || name_str == "__pycache__" {
                    continue;
                }
                self.scan_directory(&path, depth + 1);
            }
        }
    }

    /// Find potential duplicates by size
    fn find_duplicates(&mut self) {
        self.duplicates.clear();

        // Group files by size
        let mut by_size: HashMap<u64, Vec<PathBuf>> = HashMap::new();
        for file in &self.files {
            if file.size > 1024 { // Only consider files > 1KB
                by_size.entry(file.size).or_default().push(file.path.clone());
            }
        }

        // Find groups with multiple files
        for (size, paths) in by_size {
            if paths.len() > 1 {
                let wasted = size * (paths.len() as u64 - 1);
                self.duplicates.push(DuplicateGroup {
                    size,
                    paths,
                    wasted_bytes: wasted,
                });
            }
        }

        // Sort by wasted space
        self.duplicates.sort_by(|a, b| b.wasted_bytes.cmp(&a.wasted_bytes));
    }

    /// Mark files that are part of duplicate groups
    fn mark_duplicate_files(&mut self) {
        // Collect all paths in duplicate groups
        let dup_paths: std::collections::HashSet<PathBuf> = self
            .duplicates
            .iter()
            .flat_map(|g| g.paths.iter().cloned())
            .collect();

        // Mark files
        for file in &mut self.files {
            file.is_duplicate = dup_paths.contains(&file.path);
        }
    }

    /// Sample entropy for the largest files
    #[cfg(target_os = "linux")]
    fn sample_file_entropy(&mut self) {
        // Get indices of largest files (sample top 20)
        let mut indices: Vec<usize> = (0..self.files.len()).collect();
        indices.sort_by(|&a, &b| self.files[b].size.cmp(&self.files[a].size));
        indices.truncate(20);

        for idx in indices {
            if let Some(entropy) = self.calculate_file_entropy(&self.files[idx].path.clone()) {
                self.files[idx].entropy = entropy;
                self.files[idx].entropy_level = EntropyLevel::from_entropy(entropy);
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn sample_file_entropy(&mut self) {
        // Not implemented on non-Linux
    }

    /// Calculate Shannon entropy for a file sample
    #[cfg(target_os = "linux")]
    fn calculate_file_entropy(&self, path: &Path) -> Option<f64> {
        let mut file = fs::File::open(path).ok()?;
        let meta = file.metadata().ok()?;
        let file_size = meta.len();

        if file_size == 0 {
            return None;
        }

        let mut byte_counts = [0u64; 256];
        let mut total_bytes = 0u64;
        let sample_size = self.entropy_sample_size.min(file_size as usize);

        // Sample from beginning
        let mut buf = vec![0u8; sample_size];
        if std::io::Read::read(&mut file, &mut buf).is_ok() {
            for &byte in &buf {
                byte_counts[byte as usize] += 1;
                total_bytes += 1;
            }
        }

        // Sample from middle if file is large enough
        if file_size > (sample_size * 2) as u64 {
            let mid_pos = file_size / 2;
            if file.seek(SeekFrom::Start(mid_pos)).is_ok() {
                let mut mid_buf = vec![0u8; sample_size];
                if std::io::Read::read(&mut file, &mut mid_buf).is_ok() {
                    for &byte in &mid_buf {
                        byte_counts[byte as usize] += 1;
                        total_bytes += 1;
                    }
                }
            }
        }

        if total_bytes == 0 {
            return None;
        }

        // Calculate Shannon entropy
        let mut entropy = 0.0;
        let total_f64 = total_bytes as f64;

        for &count in &byte_counts {
            if count > 0 {
                let probability = count as f64 / total_f64;
                entropy -= probability * probability.log2();
            }
        }

        // Normalize to 0-1 (max entropy for bytes is 8 bits)
        Some(entropy / 8.0)
    }

    /// Update activity history for sparklines
    fn update_activity_history(&mut self) {
        let metrics = FileActivityMetrics {
            high_io_count: self.files.iter().filter(|f| f.io_activity == IoActivity::High).count(),
            high_entropy_count: self.files.iter().filter(|f| matches!(f.entropy_level, EntropyLevel::High | EntropyLevel::VeryHigh)).count(),
            duplicate_count: self.files.iter().filter(|f| f.is_duplicate).count(),
            recent_count: self.recent_files.len(),
            duplicate_bytes: self.total_wasted(),
            avg_entropy: {
                let sampled: Vec<_> = self.files.iter().filter(|f| f.entropy > 0.0).collect();
                if sampled.is_empty() {
                    0.0
                } else {
                    sampled.iter().map(|f| f.entropy).sum::<f64>() / sampled.len() as f64
                }
            },
        };

        self.activity_history.push(metrics);
        // Keep last 60 samples
        if self.activity_history.len() > 60 {
            self.activity_history.remove(0);
        }
    }

    /// Add a file to the watchlist
    pub fn watch(&mut self, path: PathBuf, threshold_bytes_per_sec: f64) {
        self.watchlist.insert(
            path.clone(),
            WatchedFile::new(path, threshold_bytes_per_sec),
        );
    }

    /// Remove a file from the watchlist
    pub fn unwatch(&mut self, path: &Path) {
        self.watchlist.remove(path);
    }

    /// Update watchlist file sizes
    fn update_watchlist(&mut self) {
        for watched in self.watchlist.values_mut() {
            if let Ok(metadata) = std::fs::metadata(&watched.path) {
                watched.update(metadata.len());
            }
        }
    }

    /// Get all files
    pub fn files(&self) -> &[FileEntry] {
        &self.files
    }

    /// Get recently modified files
    pub fn recent_files(&self) -> &[PathBuf] {
        &self.recent_files
    }

    /// Get duplicate groups
    pub fn duplicates(&self) -> &[DuplicateGroup] {
        &self.duplicates
    }

    /// Get total wasted space from duplicates
    pub fn total_wasted(&self) -> u64 {
        self.duplicates.iter().map(|d| d.wasted_bytes).sum()
    }

    /// Get watchlist
    pub fn watchlist(&self) -> &HashMap<PathBuf, WatchedFile> {
        &self.watchlist
    }

    /// Get alerting watched files
    pub fn alerting_files(&self) -> Vec<&WatchedFile> {
        self.watchlist.values().filter(|w| w.is_alerting()).collect()
    }

    /// Get max directory depth
    pub fn max_depth(&self) -> u32 {
        self.max_depth
    }

    /// Get depth color intensity (0.0 - 1.0)
    pub fn depth_intensity(&self, depth: u32) -> f64 {
        if self.max_depth == 0 {
            return 0.0;
        }
        depth as f64 / self.max_depth as f64
    }

    /// Get files sorted by size (largest first)
    pub fn largest_files(&self, count: usize) -> Vec<&FileEntry> {
        let mut sorted: Vec<_> = self.files.iter().collect();
        sorted.sort_by(|a, b| b.size.cmp(&a.size));
        sorted.truncate(count);
        sorted
    }

    /// Get files with high I/O activity
    pub fn hot_files(&self, count: usize) -> Vec<&FileEntry> {
        let mut hot: Vec<_> = self.files.iter()
            .filter(|f| f.io_activity != IoActivity::None)
            .collect();
        hot.sort_by(|a, b| {
            let a_score = match a.io_activity {
                IoActivity::High => 3,
                IoActivity::Medium => 2,
                IoActivity::Low => 1,
                IoActivity::None => 0,
            };
            let b_score = match b.io_activity {
                IoActivity::High => 3,
                IoActivity::Medium => 2,
                IoActivity::Low => 1,
                IoActivity::None => 0,
            };
            b_score.cmp(&a_score).then_with(|| b.size.cmp(&a.size))
        });
        hot.truncate(count);
        hot
    }

    /// Get files with high entropy (encrypted/compressed)
    pub fn high_entropy_files(&self, count: usize) -> Vec<&FileEntry> {
        let mut high: Vec<_> = self.files.iter()
            .filter(|f| matches!(f.entropy_level, EntropyLevel::High | EntropyLevel::VeryHigh))
            .collect();
        high.sort_by(|a, b| b.entropy.partial_cmp(&a.entropy).unwrap_or(std::cmp::Ordering::Equal));
        high.truncate(count);
        high
    }

    /// Get files with low entropy (good dedup candidates)
    pub fn low_entropy_files(&self, count: usize) -> Vec<&FileEntry> {
        let mut low: Vec<_> = self.files.iter()
            .filter(|f| f.entropy_level == EntropyLevel::Low)
            .collect();
        low.sort_by(|a, b| a.entropy.partial_cmp(&b.entropy).unwrap_or(std::cmp::Ordering::Equal));
        low.truncate(count);
        low
    }

    /// Get duplicate files sorted by wasted space
    pub fn duplicate_files(&self, count: usize) -> Vec<&FileEntry> {
        let mut dups: Vec<_> = self.files.iter()
            .filter(|f| f.is_duplicate)
            .collect();
        dups.sort_by(|a, b| b.size.cmp(&a.size));
        dups.truncate(count);
        dups
    }

    /// Get activity history for sparklines
    pub fn activity_history(&self) -> &[FileActivityMetrics] {
        &self.activity_history
    }

    /// Get current activity metrics
    pub fn current_metrics(&self) -> FileActivityMetrics {
        self.activity_history.last().cloned().unwrap_or_default()
    }

    /// Get history of a specific metric as f64 for sparkline (normalized 0-1)
    pub fn metric_history(&self, metric: &str) -> Vec<f64> {
        let max_val = |vals: &[usize]| vals.iter().max().copied().unwrap_or(1).max(1) as f64;

        match metric {
            "high_io" => {
                let vals: Vec<_> = self.activity_history.iter().map(|m| m.high_io_count).collect();
                let max = max_val(&vals);
                vals.into_iter().map(|v| v as f64 / max).collect()
            }
            "high_entropy" => {
                let vals: Vec<_> = self.activity_history.iter().map(|m| m.high_entropy_count).collect();
                let max = max_val(&vals);
                vals.into_iter().map(|v| v as f64 / max).collect()
            }
            "duplicates" => {
                let vals: Vec<_> = self.activity_history.iter().map(|m| m.duplicate_count).collect();
                let max = max_val(&vals);
                vals.into_iter().map(|v| v as f64 / max).collect()
            }
            "recent" => {
                let vals: Vec<_> = self.activity_history.iter().map(|m| m.recent_count).collect();
                let max = max_val(&vals);
                vals.into_iter().map(|v| v as f64 / max).collect()
            }
            "avg_entropy" => {
                self.activity_history.iter().map(|m| m.avg_entropy).collect()
            }
            _ => Vec::new(),
        }
    }

    /// Set recent file threshold
    pub fn set_recent_threshold(&mut self, duration: Duration) {
        self.recent_threshold = duration;
    }
}

impl Default for FileAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_type_detection() {
        assert_eq!(FileType::from_path(Path::new("main.rs")), FileType::Code);
        assert_eq!(FileType::from_path(Path::new("config.json")), FileType::Config);
        assert_eq!(FileType::from_path(Path::new("app.log")), FileType::Log);
        assert_eq!(FileType::from_path(Path::new("photo.jpg")), FileType::Media);
        assert_eq!(FileType::from_path(Path::new("backup.tar.gz")), FileType::Archive);
    }

    #[test]
    fn test_file_type_all_types() {
        // Test all file types
        assert_eq!(FileType::from_path(Path::new("script.py")), FileType::Code);
        assert_eq!(FileType::from_path(Path::new("settings.yaml")), FileType::Config);
        assert_eq!(FileType::from_path(Path::new("video.mp4")), FileType::Media);
        assert_eq!(FileType::from_path(Path::new("readme.md")), FileType::Document);
        assert_eq!(FileType::from_path(Path::new("data.csv")), FileType::Data);
        assert_eq!(FileType::from_path(Path::new("app.so")), FileType::Binary);
        assert_eq!(FileType::from_path(Path::new("random.xyz")), FileType::Other);
        // Log detection without extension
        assert_eq!(FileType::from_path(Path::new("syslog")), FileType::Log);
    }

    #[test]
    fn test_file_type_icon() {
        assert_eq!(FileType::Code.icon(), '⌘');
        assert_eq!(FileType::Config.icon(), '⚙');
        assert_eq!(FileType::Log.icon(), '≡');
        assert_eq!(FileType::Media.icon(), '◈');
        assert_eq!(FileType::Archive.icon(), '▣');
        assert_eq!(FileType::Document.icon(), '◲');
        assert_eq!(FileType::Data.icon(), '⌗');
        assert_eq!(FileType::Binary.icon(), '◉');
        assert_eq!(FileType::NodeModules.icon(), '▦');
        assert_eq!(FileType::Other.icon(), '◌');
    }

    #[test]
    fn test_file_type_label() {
        assert_eq!(FileType::Code.label(), "code");
        assert_eq!(FileType::Config.label(), "cfg");
        assert_eq!(FileType::Log.label(), "log");
        assert_eq!(FileType::Media.label(), "media");
        assert_eq!(FileType::Archive.label(), "arch");
        assert_eq!(FileType::Document.label(), "doc");
        assert_eq!(FileType::Data.label(), "data");
        assert_eq!(FileType::Binary.label(), "bin");
        assert_eq!(FileType::NodeModules.label(), "npm");
        assert_eq!(FileType::Other.label(), "file");
    }

    #[test]
    fn test_file_type_color() {
        // Just ensure all colors are defined and different
        let colors: Vec<_> = [
            FileType::Code.color(),
            FileType::Config.color(),
            FileType::Log.color(),
            FileType::Media.color(),
            FileType::Archive.color(),
            FileType::Document.color(),
            FileType::Data.color(),
            FileType::Binary.color(),
            FileType::NodeModules.color(),
            FileType::Other.color(),
        ].to_vec();

        // All should have valid RGB values
        for (r, g, b) in &colors {
            assert!(*r <= 255 && *g <= 255 && *b <= 255);
        }
    }

    #[test]
    fn test_node_modules_detection() {
        assert_eq!(
            FileType::from_path(Path::new("/app/node_modules/lodash/index.js")),
            FileType::NodeModules
        );
    }

    #[test]
    fn test_io_activity_icon() {
        assert_eq!(IoActivity::None.icon(), ' ');
        assert_eq!(IoActivity::Low.icon(), '▁');
        assert_eq!(IoActivity::Medium.icon(), '▃');
        assert_eq!(IoActivity::High.icon(), '▇');
    }

    #[test]
    fn test_io_activity_color() {
        // Test that all activity levels have valid colors
        let (r, g, b) = IoActivity::None.color();
        assert!(r == 80 && g == 80 && b == 80);

        let (r, g, b) = IoActivity::Low.color();
        assert!(r == 100 && g == 180 && b == 100);

        let (r, g, b) = IoActivity::Medium.color();
        assert!(r == 220 && g == 180 && b == 80);

        let (r, g, b) = IoActivity::High.color();
        assert!(r == 255 && g == 100 && b == 80);
    }

    #[test]
    fn test_entropy_level_from_entropy() {
        assert_eq!(EntropyLevel::from_entropy(0.0), EntropyLevel::Unknown);
        assert_eq!(EntropyLevel::from_entropy(0.1), EntropyLevel::Low);
        assert_eq!(EntropyLevel::from_entropy(0.29), EntropyLevel::Low);
        assert_eq!(EntropyLevel::from_entropy(0.5), EntropyLevel::Medium);
        assert_eq!(EntropyLevel::from_entropy(0.7), EntropyLevel::High);
        assert_eq!(EntropyLevel::from_entropy(0.85), EntropyLevel::High);
        assert_eq!(EntropyLevel::from_entropy(0.9), EntropyLevel::VeryHigh);
        assert_eq!(EntropyLevel::from_entropy(1.0), EntropyLevel::VeryHigh);
    }

    #[test]
    fn test_entropy_level_icon() {
        assert_eq!(EntropyLevel::Unknown.icon(), '?');
        assert_eq!(EntropyLevel::Low.icon(), '○');
        assert_eq!(EntropyLevel::Medium.icon(), '◐');
        assert_eq!(EntropyLevel::High.icon(), '●');
        assert_eq!(EntropyLevel::VeryHigh.icon(), '◉');
    }

    #[test]
    fn test_entropy_level_color() {
        // Test all entropy levels have valid colors
        let levels = [
            EntropyLevel::Unknown,
            EntropyLevel::Low,
            EntropyLevel::Medium,
            EntropyLevel::High,
            EntropyLevel::VeryHigh,
        ];

        for level in levels {
            let (r, g, b) = level.color();
            assert!(r <= 255 && g <= 255 && b <= 255);
        }
    }

    #[test]
    fn test_watched_file_growth() {
        let mut watched = WatchedFile::new(PathBuf::from("/var/log/test.log"), 1000.0);

        watched.update(1000);
        std::thread::sleep(std::time::Duration::from_millis(100));
        watched.update(2000);

        assert!(watched.growth_rate > 0.0);
    }

    #[test]
    fn test_watched_file_alerting() {
        let mut watched = WatchedFile::new(PathBuf::from("/tmp/test.log"), 100.0);

        // Initially not alerting
        assert!(!watched.is_alerting());

        // Simulate high growth
        watched.growth_rate = 200.0;
        assert!(watched.is_alerting());

        watched.growth_rate = 50.0;
        assert!(!watched.is_alerting());
    }

    #[test]
    fn test_watched_file_time_to_full() {
        let mut watched = WatchedFile::new(PathBuf::from("/tmp/test.log"), 100.0);

        // No growth = no time estimate
        watched.growth_rate = 0.0;
        assert!(watched.time_to_full(1000).is_none());

        // With growth, should have estimate
        watched.growth_rate = 100.0;
        let ttf = watched.time_to_full(1000).unwrap();
        assert!((ttf.as_secs_f64() - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_file_analyzer_creation() {
        let analyzer = FileAnalyzer::new();
        assert!(analyzer.files().is_empty());
        assert!(analyzer.duplicates().is_empty());
        assert!(analyzer.activity_history().is_empty());
    }

    #[test]
    fn test_depth_intensity() {
        let mut analyzer = FileAnalyzer::new();
        analyzer.max_depth = 10;

        assert!((analyzer.depth_intensity(0) - 0.0).abs() < 0.01);
        assert!((analyzer.depth_intensity(5) - 0.5).abs() < 0.01);
        assert!((analyzer.depth_intensity(10) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_depth_intensity_zero_depth() {
        let analyzer = FileAnalyzer::new();
        // max_depth is 0 by default
        assert_eq!(analyzer.depth_intensity(5), 0.0);
    }

    #[test]
    fn test_watchlist() {
        let mut analyzer = FileAnalyzer::new();
        let path = PathBuf::from("/tmp/test.log");

        analyzer.watch(path.clone(), 1000.0);
        assert!(analyzer.watchlist().contains_key(&path));

        analyzer.unwatch(&path);
        assert!(!analyzer.watchlist().contains_key(&path));
    }

    #[test]
    fn test_file_activity_metrics_default() {
        let metrics = FileActivityMetrics::default();
        assert_eq!(metrics.high_io_count, 0);
        assert_eq!(metrics.high_entropy_count, 0);
        assert_eq!(metrics.duplicate_count, 0);
        assert_eq!(metrics.recent_count, 0);
        assert_eq!(metrics.duplicate_bytes, 0);
        assert_eq!(metrics.avg_entropy, 0.0);
    }

    #[test]
    fn test_current_metrics_empty() {
        let analyzer = FileAnalyzer::new();
        let metrics = analyzer.current_metrics();
        assert_eq!(metrics.high_io_count, 0);
    }

    #[test]
    fn test_metric_history_empty() {
        let analyzer = FileAnalyzer::new();
        assert!(analyzer.metric_history("high_io").is_empty());
        assert!(analyzer.metric_history("high_entropy").is_empty());
        assert!(analyzer.metric_history("duplicates").is_empty());
        assert!(analyzer.metric_history("recent").is_empty());
        assert!(analyzer.metric_history("avg_entropy").is_empty());
        assert!(analyzer.metric_history("unknown").is_empty());
    }

    #[test]
    fn test_hot_files_empty() {
        let analyzer = FileAnalyzer::new();
        assert!(analyzer.hot_files(10).is_empty());
    }

    #[test]
    fn test_high_entropy_files_empty() {
        let analyzer = FileAnalyzer::new();
        assert!(analyzer.high_entropy_files(10).is_empty());
    }

    #[test]
    fn test_low_entropy_files_empty() {
        let analyzer = FileAnalyzer::new();
        assert!(analyzer.low_entropy_files(10).is_empty());
    }

    #[test]
    fn test_duplicate_files_empty() {
        let analyzer = FileAnalyzer::new();
        assert!(analyzer.duplicate_files(10).is_empty());
    }

    #[test]
    fn test_largest_files_empty() {
        let analyzer = FileAnalyzer::new();
        assert!(analyzer.largest_files(10).is_empty());
    }

    #[test]
    fn test_recent_threshold() {
        let mut analyzer = FileAnalyzer::new();
        analyzer.set_recent_threshold(Duration::from_secs(600));
        // Just verify it doesn't panic
    }

    #[test]
    fn test_total_wasted_empty() {
        let analyzer = FileAnalyzer::new();
        assert_eq!(analyzer.total_wasted(), 0);
    }

    #[test]
    fn test_alerting_files_empty() {
        let analyzer = FileAnalyzer::new();
        assert!(analyzer.alerting_files().is_empty());
    }

    #[test]
    fn test_recent_files_empty() {
        let analyzer = FileAnalyzer::new();
        assert!(analyzer.recent_files().is_empty());
    }

    #[test]
    fn test_max_depth_initial() {
        let analyzer = FileAnalyzer::new();
        assert_eq!(analyzer.max_depth(), 0);
    }

    #[test]
    fn test_duplicate_group() {
        let group = DuplicateGroup {
            size: 1024,
            paths: vec![PathBuf::from("/a"), PathBuf::from("/b")],
            wasted_bytes: 1024,
        };
        assert_eq!(group.size, 1024);
        assert_eq!(group.paths.len(), 2);
        assert_eq!(group.wasted_bytes, 1024);
    }
}
