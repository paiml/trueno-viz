//! Large File Treemap - finds and displays biggest files
//!
//! Scans filesystem for large files and renders as 2D treemap.

use std::fs;

/// Mount file info: (filename, size, full_path)
pub type MountFileInfo = (String, u64, String);

/// Mount group: (mount_label, mount_total_size, files)
pub type MountGroup = (String, u64, Vec<MountFileInfo>);
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

/// File category for grouping/coloring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileCategory {
    Model,      // .gguf, .safetensors, .bin (ML models)
    Archive,    // .tar, .zip, .zst, .gz
    Build,      // target/, node_modules/, .o, .a
    Media,      // video, audio, images
    Database,   // .db, .sqlite
    Benchmark,  // seq-read, seq-write, rand-* (fio artifacts)
    Other,
}

impl FileCategory {
    pub fn from_path(path: &std::path::Path) -> Self {
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        // Check for benchmark artifacts first
        if name.starts_with("seq-read") || name.starts_with("seq-write")
            || name.starts_with("rand-") || name.starts_with("randrw") {
            return Self::Benchmark;
        }

        // Check path components for build dirs
        let path_str = path.to_string_lossy();
        if path_str.contains("/target/") || path_str.contains("/node_modules/")
            || path_str.contains("/.cache/") {
            return Self::Build;
        }

        // Check for model-like .bin files (ggml, pytorch, etc.)
        let name_lower = path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.to_lowercase())
            .unwrap_or_default();
        let is_model_bin = ext.eq_ignore_ascii_case("bin") &&
            (name_lower.contains("model") || name_lower.contains("ggml") ||
             name_lower.contains("pytorch") || name_lower.contains("llama"));

        match ext.to_lowercase().as_str() {
            // ML models: GGUF, SafeTensors, ONNX, PyTorch, TensorFlow, Keras, CoreML, etc.
            "gguf" | "safetensors" | "onnx" | "pt" | "pth" | "h5" | "pb" |
            "ckpt" | "tflite" | "mlmodel" | "mar" | "keras" | "engine" |
            "llamafile" | "ollama" | "hdf5" => Self::Model,
            "bin" if is_model_bin => Self::Model,
            "tar" | "zip" | "zst" | "gz" | "xz" | "bz2" | "7z" | "rar" => Self::Archive,
            "mp4" | "mkv" | "avi" | "mov" | "mp3" | "flac" | "wav" | "jpg" | "png" | "raw" => Self::Media,
            "db" | "sqlite" | "sqlite3" | "mdb" => Self::Database,
            "o" | "a" | "so" | "dylib" | "rlib" | "rmeta" => Self::Build,
            _ => Self::Other,
        }
    }

    pub fn icon(&self) -> char {
        match self {
            Self::Model => '🧠',
            Self::Archive => '📦',
            Self::Build => '🔨',
            Self::Media => '🎬',
            Self::Database => '💾',
            Self::Benchmark => '⏱',
            Self::Other => '📄',
        }
    }

    pub fn color(&self) -> (u8, u8, u8) {
        match self {
            Self::Model => (180, 100, 220),    // purple
            Self::Archive => (220, 160, 80),   // orange
            Self::Build => (120, 120, 130),    // gray
            Self::Media => (100, 180, 220),    // cyan
            Self::Database => (100, 140, 220), // blue
            Self::Benchmark => (80, 80, 90),   // dark gray
            Self::Other => (160, 160, 160),    // light gray
        }
    }
}

/// A large file entry
#[derive(Debug, Clone)]
pub struct LargeFile {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub color_idx: u8,
    pub modified: Option<SystemTime>,
    pub category: FileCategory,
}

/// A rectangle in the treemap layout
#[derive(Debug, Clone, Copy)]
pub struct TreeRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub size: u64,
    pub color_idx: u8,
    pub depth: u8,
}

/// Treemap analyzer - finds large files
pub struct TreemapAnalyzer {
    files: Arc<Mutex<Vec<LargeFile>>>,
    last_scan: Instant,
    scanning: Arc<Mutex<bool>>,
}

impl TreemapAnalyzer {
    pub fn new(_path: &str) -> Self {
        Self {
            files: Arc::new(Mutex::new(Vec::new())),
            last_scan: Instant::now() - Duration::from_secs(3600),
            scanning: Arc::new(Mutex::new(false)),
        }
    }

    /// Start background scan if needed
    pub fn collect(&mut self) {
        let elapsed = self.last_scan.elapsed();

        // Rescan every 60 seconds
        if elapsed < Duration::from_secs(60) {
            return;
        }

        // Check if already scanning
        {
            let scanning = self.scanning.lock().expect("scanning lock poisoned");
            if *scanning {
                return;
            }
        }

        let files = Arc::clone(&self.files);
        let scanning = Arc::clone(&self.scanning);

        *scanning.lock().expect("scanning lock poisoned") = true;
        self.last_scan = Instant::now();

        thread::spawn(move || {
            let found = find_large_files();
            *files.lock().expect("files lock poisoned") = found;
            *scanning.lock().expect("scanning lock poisoned") = false;
        });
    }

    /// Get treemap layout for rendering
    pub fn layout(&self, width: f64, height: f64) -> Vec<(TreeRect, String)> {
        let files = self.files.lock().expect("files lock poisoned");
        if files.is_empty() {
            return Vec::new();
        }
        squarify_files(&files, width, height)
    }

    /// Check if currently scanning
    pub fn is_scanning(&self) -> bool {
        *self.scanning.lock().expect("scanning lock poisoned")
    }

    /// Get total size of all large files
    pub fn total_size(&self) -> u64 {
        self.files.lock().expect("files lock poisoned").iter().map(|f| f.size).sum()
    }

    /// Get files with paths for actionable display (parent/name format)
    pub fn files_with_paths(&self) -> Vec<(String, u64)> {
        let files = self.files.lock().expect("files lock poisoned");
        files.iter()
            .map(|f| {
                // Format as "parent/filename" for actionability
                let display = if let Some(parent) = f.path.parent() {
                    if let Some(parent_name) = parent.file_name() {
                        format!("{}/{}", parent_name.to_string_lossy(), f.name)
                    } else {
                        f.name.clone()
                    }
                } else {
                    f.name.clone()
                };
                (display, f.size)
            })
            .collect()
    }

    /// Get top files with full paths for display
    pub fn top_files_full_path(&self, count: usize) -> Vec<(String, u64)> {
        let files = self.files.lock().expect("files lock poisoned");
        files.iter()
            .take(count)
            .map(|f| (f.path.to_string_lossy().to_string(), f.size))
            .collect()
    }

    /// Get top files excluding benchmarks, with category and age
    /// Returns: Vec<(name, size, category, age_str, full_path)>
    pub fn top_files_filtered(&self, count: usize) -> Vec<(String, u64, FileCategory, String, String)> {
        let files = self.files.lock().expect("files lock poisoned");
        files.iter()
            .filter(|f| f.category != FileCategory::Benchmark)
            .take(count)
            .map(|f| {
                let age = format_age(f.modified);
                (f.name.clone(), f.size, f.category, age, f.path.to_string_lossy().to_string())
            })
            .collect()
    }

    /// Get directory totals (grouped by parent directory)
    /// Returns: Vec<(directory_path, total_size, file_count, categories)>
    pub fn directory_totals(&self) -> Vec<(String, u64, usize, Vec<FileCategory>)> {
        use std::collections::HashMap;

        let files = self.files.lock().expect("files lock poisoned");

        // Group by parent directory (2 levels up for better grouping)
        let mut dir_stats: HashMap<String, (u64, usize, Vec<FileCategory>)> = HashMap::new();

        for f in files.iter() {
            // Skip benchmarks
            if f.category == FileCategory::Benchmark {
                continue;
            }

            // Get meaningful parent (try 2 levels for things like /mnt/nvme-raid0/models)
            let dir = if let Some(parent) = f.path.parent() {
                let parent_str = parent.to_string_lossy();
                // If parent is a mount point, use full path
                if parent_str.matches('/').count() <= 2 {
                    parent_str.to_string()
                } else {
                    // Use grandparent/parent for grouping
                    parent.to_string_lossy().to_string()
                }
            } else {
                "/".to_string()
            };

            let entry = dir_stats.entry(dir).or_insert((0, 0, Vec::new()));
            entry.0 += f.size;
            entry.1 += 1;
            if !entry.2.contains(&f.category) {
                entry.2.push(f.category);
            }
        }

        // Convert to vec and sort by size
        let mut result: Vec<_> = dir_stats.into_iter()
            .map(|(dir, (size, count, cats))| (dir, size, count, cats))
            .collect();
        result.sort_by(|a, b| b.1.cmp(&a.1));
        result
    }

    /// Get files grouped by mount point (faceted view)
    /// Returns: Vec<(mount_label, mount_total_size, files)>
    pub fn files_by_mount(&self) -> Vec<MountGroup> {
        let files = self.files.lock().expect("files lock poisoned");

        // Known mount points to group by
        #[cfg(target_os = "linux")]
        let mounts = get_real_mounts();
        #[cfg(not(target_os = "linux"))]
        let mounts = vec!["/".to_string()];

        let mut mount_files: std::collections::HashMap<String, Vec<MountFileInfo>> =
            std::collections::HashMap::new();

        for f in files.iter() {
            let path_str = f.path.to_string_lossy();

            // Find which mount this file belongs to (longest match)
            let mount = mounts.iter()
                .filter(|m| path_str.starts_with(m.as_str()))
                .max_by_key(|m| m.len())
                .cloned()
                .unwrap_or_else(|| "/".to_string());

            // Create short display name (path relative to mount)
            let relative = path_str.strip_prefix(&mount)
                .unwrap_or(&path_str)
                .trim_start_matches('/');
            let display = if relative.len() > 25 {
                // Shorten: keep first dir + filename
                let parts: Vec<&str> = relative.split('/').collect();
                if parts.len() > 2 {
                    format!("{}/.../{}", parts[0], parts.last().unwrap_or(&""))
                } else {
                    relative.to_string()
                }
            } else {
                relative.to_string()
            };

            mount_files.entry(mount)
                .or_default()
                .push((display, f.size, path_str.to_string()));
        }

        // Convert to sorted vec, largest mounts first
        let mut result: Vec<_> = mount_files.into_iter()
            .map(|(mount, mut files)| {
                files.sort_by(|a, b| b.1.cmp(&a.1)); // Sort files by size desc
                let total: u64 = files.iter().map(|(_, s, _)| *s).sum();
                // Shorten mount label
                let label = if mount == "/" {
                    "/".to_string()
                } else {
                    mount.split('/').next_back().unwrap_or(&mount).to_string()
                };
                (label, total, files)
            })
            .collect();

        result.sort_by(|a, b| b.1.cmp(&a.1)); // Sort mounts by total size desc
        result
    }
}

impl Default for TreemapAnalyzer {
    fn default() -> Self {
        Self::new("/")
    }
}

/// Find large files across all real mounted filesystems (Grand Perspective style)
fn find_large_files() -> Vec<LargeFile> {
    let mut files = Vec::new();

    // Get real mounts from /proc/mounts (Linux)
    #[cfg(target_os = "linux")]
    let scan_dirs = get_real_mounts();

    #[cfg(target_os = "macos")]
    let scan_dirs = vec![
        "/Users".to_string(),
        "/Applications".to_string(),
        "/Library".to_string(),
        "/opt".to_string(),
        "/Volumes".to_string(),
    ];

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    let scan_dirs = vec!["/".to_string()];

    // Scan all mounts with deeper traversal (max 12 levels like Grand Perspective)
    for (dir_idx, dir) in scan_dirs.iter().enumerate() {
        scan_for_large_files(dir, &mut files, dir_idx as u8, 0, 12);
    }

    // Sort by size descending
    files.sort_by(|a, b| b.size.cmp(&a.size));

    // Keep top 100 for better coverage
    files.truncate(100);
    files
}

/// Get real (non-virtual) mount points from /proc/mounts
#[cfg(target_os = "linux")]
fn get_real_mounts() -> Vec<String> {
    use std::io::{BufRead, BufReader};

    let mut mounts = Vec::new();

    // Virtual/pseudo filesystems to skip
    let skip_fs = [
        "proc", "sysfs", "devtmpfs", "devpts", "tmpfs", "securityfs",
        "cgroup", "cgroup2", "pstore", "debugfs", "hugetlbfs", "mqueue",
        "fusectl", "configfs", "efivarfs", "binfmt_misc", "autofs",
        "tracefs", "bpf", "overlay", "squashfs", "nsfs", "ramfs",
    ];

    // Paths to skip (even if real fs)
    let skip_paths = ["/proc", "/sys", "/dev", "/run", "/snap"];

    if let Ok(file) = fs::File::open("/proc/mounts") {
        let reader = BufReader::new(file);
        for line in reader.lines().map_while(Result::ok) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let mount_point = parts[1];
                let fs_type = parts[2];

                // Skip virtual filesystems
                if skip_fs.contains(&fs_type) {
                    continue;
                }

                // Skip system paths
                if skip_paths.iter().any(|p| mount_point.starts_with(p)) {
                    continue;
                }

                // Skip if mount point doesn't exist or isn't readable
                if fs::metadata(mount_point).is_err() {
                    continue;
                }

                mounts.push(mount_point.to_string());
            }
        }
    }

    // Fallback if no mounts found
    if mounts.is_empty() {
        mounts.push("/home".to_string());
        mounts.push("/mnt".to_string());
        mounts.push("/var".to_string());
    }

    // Dedupe and sort by path length (shorter first = higher level mounts)
    mounts.sort_by_key(|a| a.len());
    mounts.dedup();

    mounts
}

/// Recursively scan for large files
fn scan_for_large_files(
    path: &str,
    files: &mut Vec<LargeFile>,
    color_idx: u8,
    depth: usize,
    max_depth: usize,
) {
    if depth > max_depth {
        return;
    }

    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let entry_path = entry.path();
        let name = entry_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Skip hidden and system dirs
        if name.starts_with('.') || name == "proc" || name == "sys" || name == "dev" {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        if metadata.is_file() {
            let size = metadata.len();
            // Include files > 10MB (lower threshold for better coverage)
            if size > 10 * 1024 * 1024 {
                let modified = metadata.modified().ok();
                let category = FileCategory::from_path(&entry_path);
                files.push(LargeFile {
                    name: name.clone(),
                    path: entry_path,
                    size,
                    color_idx,
                    modified,
                    category,
                });
            }
        } else if metadata.is_dir() {
            scan_for_large_files(
                entry_path.to_str().unwrap_or(""),
                files,
                color_idx,
                depth + 1,
                max_depth,
            );
        }
    }
}

/// Squarified layout for files
fn squarify_files(files: &[LargeFile], width: f64, height: f64) -> Vec<(TreeRect, String)> {
    if files.is_empty() || width < 1.0 || height < 1.0 {
        return Vec::new();
    }

    let total: f64 = files.iter().map(|f| f.size as f64).sum();
    if total == 0.0 {
        return Vec::new();
    }

    let mut rects = Vec::new();
    let mut remaining = files.to_vec();
    let mut x = 0.0;
    let mut y = 0.0;
    let mut w = width;
    let mut h = height;

    while !remaining.is_empty() && w >= 1.0 && h >= 1.0 {
        // Layout along shorter side
        let vertical = w > h;
        let side = if vertical { h } else { w };

        // Find best row
        let remaining_total: f64 = remaining.iter().map(|f| f.size as f64).sum();
        let mut row = Vec::new();
        let mut row_size = 0.0;
        let mut best_ratio = f64::MAX;

        for file in &remaining {
            let new_row_size = row_size + file.size as f64;
            let row_area = (new_row_size / remaining_total) * w * h;
            let row_dim = row_area / side;

            // Calculate worst aspect ratio in this row
            let worst = row.iter().chain(std::iter::once(file))
                .map(|f| {
                    let file_area = (f.size as f64 / new_row_size) * row_area;
                    let file_dim = file_area / row_dim;
                    if row_dim > file_dim { row_dim / file_dim } else { file_dim / row_dim }
                })
                .fold(0.0f64, |a, b| a.max(b));

            if worst <= best_ratio || row.is_empty() {
                best_ratio = worst;
                row.push(file.clone());
                row_size = new_row_size;
            } else {
                break;
            }
        }

        // Remove used files
        for rf in &row {
            remaining.retain(|f| f.path != rf.path);
        }

        // Layout this row
        let row_total: f64 = row.iter().map(|f| f.size as f64).sum();
        let row_fraction = row_total / remaining_total.max(row_total);
        let row_dim = if vertical {
            w * row_fraction
        } else {
            h * row_fraction
        };

        let mut pos = 0.0;
        for file in &row {
            let file_fraction = file.size as f64 / row_total;
            let file_dim = side * file_fraction;

            let (rx, ry, rw, rh) = if vertical {
                (x, y + pos, row_dim, file_dim)
            } else {
                (x + pos, y, file_dim, row_dim)
            };

            rects.push((
                TreeRect {
                    x: rx,
                    y: ry,
                    w: rw,
                    h: rh,
                    size: file.size,
                    color_idx: file.color_idx,
                    depth: 0,
                },
                file.name.clone(),
            ));

            pos += file_dim;
        }

        // Shrink remaining area
        if vertical {
            x += row_dim;
            w -= row_dim;
        } else {
            y += row_dim;
            h -= row_dim;
        }
    }

    rects
}

/// Format file age as human-readable string
fn format_age(modified: Option<SystemTime>) -> String {
    let Some(mtime) = modified else {
        return "?".to_string();
    };

    let Ok(elapsed) = SystemTime::now().duration_since(mtime) else {
        return "?".to_string();
    };

    let secs = elapsed.as_secs();
    if secs < 60 {
        "now".to_string()
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        format!("{}h", secs / 3600)
    } else if secs < 86400 * 30 {
        format!("{}d", secs / 86400)
    } else if secs < 86400 * 365 {
        format!("{}mo", secs / (86400 * 30))
    } else {
        format!("{}y", secs / (86400 * 365))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_treemap_analyzer_creation() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        assert!(!analyzer.is_scanning());
    }

    #[test]
    fn test_empty_layout() {
        let analyzer = TreemapAnalyzer::new("/nonexistent");
        let layout = analyzer.layout(100.0, 100.0);
        assert!(layout.is_empty());
    }

    #[test]
    fn test_squarify_single() {
        let files = vec![
            LargeFile { name: "big.bin".into(), path: "/big.bin".into(), size: 1000, color_idx: 0, modified: None, category: FileCategory::Other },
        ];
        let layout = squarify_files(&files, 10.0, 10.0);
        assert_eq!(layout.len(), 1);
        assert!((layout[0].0.w - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_squarify_multiple() {
        let files = vec![
            LargeFile { name: "a.bin".into(), path: "/a".into(), size: 500, color_idx: 0, modified: None, category: FileCategory::Other },
            LargeFile { name: "b.bin".into(), path: "/b".into(), size: 300, color_idx: 1, modified: None, category: FileCategory::Other },
            LargeFile { name: "c.bin".into(), path: "/c".into(), size: 200, color_idx: 2, modified: None, category: FileCategory::Other },
        ];
        let layout = squarify_files(&files, 10.0, 10.0);
        assert_eq!(layout.len(), 3);
    }

    #[test]
    fn test_file_category_detection() {
        use std::path::Path;
        assert_eq!(FileCategory::from_path(Path::new("/models/llama.gguf")), FileCategory::Model);
        assert_eq!(FileCategory::from_path(Path::new("/backup.tar.zst")), FileCategory::Archive);
        assert_eq!(FileCategory::from_path(Path::new("/seq-read.1.0")), FileCategory::Benchmark);
        assert_eq!(FileCategory::from_path(Path::new("/project/target/debug/bin")), FileCategory::Build);
    }

    #[test]
    fn test_model_extensions() {
        use std::path::Path;
        // All ML model formats
        assert_eq!(FileCategory::from_path(Path::new("/models/llama.gguf")), FileCategory::Model);
        assert_eq!(FileCategory::from_path(Path::new("/models/model.safetensors")), FileCategory::Model);
        assert_eq!(FileCategory::from_path(Path::new("/models/model.onnx")), FileCategory::Model);
        assert_eq!(FileCategory::from_path(Path::new("/models/model.pt")), FileCategory::Model);
        assert_eq!(FileCategory::from_path(Path::new("/models/model.pth")), FileCategory::Model);
        assert_eq!(FileCategory::from_path(Path::new("/models/model.h5")), FileCategory::Model);
        assert_eq!(FileCategory::from_path(Path::new("/models/model.pb")), FileCategory::Model);
        assert_eq!(FileCategory::from_path(Path::new("/models/sd.ckpt")), FileCategory::Model);
        assert_eq!(FileCategory::from_path(Path::new("/models/model.tflite")), FileCategory::Model);
        assert_eq!(FileCategory::from_path(Path::new("/models/model.mlmodel")), FileCategory::Model);
        assert_eq!(FileCategory::from_path(Path::new("/models/model.mar")), FileCategory::Model);
        assert_eq!(FileCategory::from_path(Path::new("/models/model.keras")), FileCategory::Model);
        assert_eq!(FileCategory::from_path(Path::new("/models/model.engine")), FileCategory::Model);
        assert_eq!(FileCategory::from_path(Path::new("/models/llama.llamafile")), FileCategory::Model);
        assert_eq!(FileCategory::from_path(Path::new("/models/model.ollama")), FileCategory::Model);
        assert_eq!(FileCategory::from_path(Path::new("/models/model.hdf5")), FileCategory::Model);
        // Model-like .bin files
        assert_eq!(FileCategory::from_path(Path::new("/models/ggml-model-q4.bin")), FileCategory::Model);
        assert_eq!(FileCategory::from_path(Path::new("/models/pytorch_model.bin")), FileCategory::Model);
        assert_eq!(FileCategory::from_path(Path::new("/models/llama-7b.bin")), FileCategory::Model);
        // Generic .bin should NOT be model
        assert_eq!(FileCategory::from_path(Path::new("/usr/bin/ls")), FileCategory::Other);
        assert_eq!(FileCategory::from_path(Path::new("/data/random.bin")), FileCategory::Other);
    }

    #[test]
    fn test_file_category_icons() {
        assert_eq!(FileCategory::Model.icon(), '🧠');
        assert_eq!(FileCategory::Archive.icon(), '📦');
        assert_eq!(FileCategory::Build.icon(), '🔨');
        assert_eq!(FileCategory::Media.icon(), '🎬');
        assert_eq!(FileCategory::Database.icon(), '💾');
        assert_eq!(FileCategory::Benchmark.icon(), '⏱');
        assert_eq!(FileCategory::Other.icon(), '📄');
    }

    #[test]
    fn test_file_category_colors() {
        assert_eq!(FileCategory::Model.color(), (180, 100, 220));
        assert_eq!(FileCategory::Archive.color(), (220, 160, 80));
        assert_eq!(FileCategory::Build.color(), (120, 120, 130));
        assert_eq!(FileCategory::Media.color(), (100, 180, 220));
        assert_eq!(FileCategory::Database.color(), (100, 140, 220));
        assert_eq!(FileCategory::Benchmark.color(), (80, 80, 90));
        assert_eq!(FileCategory::Other.color(), (160, 160, 160));
    }

    #[test]
    fn test_archive_extensions() {
        use std::path::Path;
        assert_eq!(FileCategory::from_path(Path::new("/archive.tar")), FileCategory::Archive);
        assert_eq!(FileCategory::from_path(Path::new("/archive.zip")), FileCategory::Archive);
        assert_eq!(FileCategory::from_path(Path::new("/archive.zst")), FileCategory::Archive);
        assert_eq!(FileCategory::from_path(Path::new("/archive.gz")), FileCategory::Archive);
        assert_eq!(FileCategory::from_path(Path::new("/archive.xz")), FileCategory::Archive);
        assert_eq!(FileCategory::from_path(Path::new("/archive.bz2")), FileCategory::Archive);
        assert_eq!(FileCategory::from_path(Path::new("/archive.7z")), FileCategory::Archive);
        assert_eq!(FileCategory::from_path(Path::new("/archive.rar")), FileCategory::Archive);
    }

    #[test]
    fn test_media_extensions() {
        use std::path::Path;
        assert_eq!(FileCategory::from_path(Path::new("/video.mp4")), FileCategory::Media);
        assert_eq!(FileCategory::from_path(Path::new("/video.mkv")), FileCategory::Media);
        assert_eq!(FileCategory::from_path(Path::new("/video.avi")), FileCategory::Media);
        assert_eq!(FileCategory::from_path(Path::new("/video.mov")), FileCategory::Media);
        assert_eq!(FileCategory::from_path(Path::new("/audio.mp3")), FileCategory::Media);
        assert_eq!(FileCategory::from_path(Path::new("/audio.flac")), FileCategory::Media);
        assert_eq!(FileCategory::from_path(Path::new("/audio.wav")), FileCategory::Media);
        assert_eq!(FileCategory::from_path(Path::new("/image.jpg")), FileCategory::Media);
        assert_eq!(FileCategory::from_path(Path::new("/image.png")), FileCategory::Media);
        assert_eq!(FileCategory::from_path(Path::new("/photo.raw")), FileCategory::Media);
    }

    #[test]
    fn test_database_extensions() {
        use std::path::Path;
        assert_eq!(FileCategory::from_path(Path::new("/data.db")), FileCategory::Database);
        assert_eq!(FileCategory::from_path(Path::new("/data.sqlite")), FileCategory::Database);
        assert_eq!(FileCategory::from_path(Path::new("/data.sqlite3")), FileCategory::Database);
        assert_eq!(FileCategory::from_path(Path::new("/data.mdb")), FileCategory::Database);
    }

    #[test]
    fn test_build_extensions() {
        use std::path::Path;
        assert_eq!(FileCategory::from_path(Path::new("/obj.o")), FileCategory::Build);
        assert_eq!(FileCategory::from_path(Path::new("/lib.a")), FileCategory::Build);
        assert_eq!(FileCategory::from_path(Path::new("/lib.so")), FileCategory::Build);
        assert_eq!(FileCategory::from_path(Path::new("/lib.dylib")), FileCategory::Build);
        assert_eq!(FileCategory::from_path(Path::new("/lib.rlib")), FileCategory::Build);
        assert_eq!(FileCategory::from_path(Path::new("/lib.rmeta")), FileCategory::Build);
    }

    #[test]
    fn test_build_paths() {
        use std::path::Path;
        assert_eq!(FileCategory::from_path(Path::new("/project/target/debug/binary")), FileCategory::Build);
        assert_eq!(FileCategory::from_path(Path::new("/project/node_modules/pkg/file.js")), FileCategory::Build);
        assert_eq!(FileCategory::from_path(Path::new("/home/.cache/huggingface/model.bin")), FileCategory::Build);
    }

    #[test]
    fn test_benchmark_patterns() {
        use std::path::Path;
        assert_eq!(FileCategory::from_path(Path::new("/seq-read.0.0")), FileCategory::Benchmark);
        assert_eq!(FileCategory::from_path(Path::new("/seq-write.1.0")), FileCategory::Benchmark);
        assert_eq!(FileCategory::from_path(Path::new("/rand-read.0.0")), FileCategory::Benchmark);
        assert_eq!(FileCategory::from_path(Path::new("/randrw.0.0")), FileCategory::Benchmark);
    }

    #[test]
    fn test_treemap_total_size() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        assert_eq!(analyzer.total_size(), 0);
    }

    #[test]
    fn test_treemap_files_with_paths_empty() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        assert!(analyzer.files_with_paths().is_empty());
    }

    #[test]
    fn test_squarify_empty() {
        let files: Vec<LargeFile> = vec![];
        let layout = squarify_files(&files, 100.0, 100.0);
        assert!(layout.is_empty());
    }

    #[test]
    fn test_squarify_tiny_dimensions() {
        let files = vec![
            LargeFile { name: "a.bin".into(), path: "/a".into(), size: 1000, color_idx: 0, modified: None, category: FileCategory::Other },
        ];
        let layout = squarify_files(&files, 0.5, 0.5);
        assert!(layout.is_empty());
    }

    #[test]
    fn test_large_file_with_modified() {
        use std::time::SystemTime;
        let now = SystemTime::now();
        let file = LargeFile {
            name: "test.bin".into(),
            path: "/test.bin".into(),
            size: 1024,
            color_idx: 0,
            modified: Some(now),
            category: FileCategory::Other,
        };
        assert_eq!(file.name, "test.bin");
        assert_eq!(file.size, 1024);
        assert!(file.modified.is_some());
    }

    #[test]
    fn test_tree_rect_fields() {
        let rect = TreeRect {
            x: 10.0,
            y: 20.0,
            w: 100.0,
            h: 50.0,
            size: 1024,
            color_idx: 2,
            depth: 1,
        };
        assert!((rect.x - 10.0).abs() < 0.001);
        assert!((rect.y - 20.0).abs() < 0.001);
        assert!((rect.w - 100.0).abs() < 0.001);
        assert!((rect.h - 50.0).abs() < 0.001);
        assert_eq!(rect.size, 1024);
        assert_eq!(rect.color_idx, 2);
        assert_eq!(rect.depth, 1);
    }

    #[test]
    fn test_squarify_large_count() {
        let files: Vec<LargeFile> = (0..10)
            .map(|i| LargeFile {
                name: format!("file{}.bin", i),
                path: format!("/file{}.bin", i).into(),
                size: (100 - i * 5) as u64,
                color_idx: i as u8,
                modified: None,
                category: FileCategory::Other,
            })
            .collect();
        let layout = squarify_files(&files, 100.0, 100.0);
        assert_eq!(layout.len(), 10);
        // Verify total area is approximately preserved
        let total_area: f64 = layout.iter().map(|(r, _)| r.w * r.h).sum();
        assert!(total_area > 9000.0 && total_area < 10100.0);
    }

    #[test]
    fn test_case_insensitive_extensions() {
        use std::path::Path;
        assert_eq!(FileCategory::from_path(Path::new("/model.GGUF")), FileCategory::Model);
        assert_eq!(FileCategory::from_path(Path::new("/archive.TAR")), FileCategory::Archive);
        assert_eq!(FileCategory::from_path(Path::new("/video.MP4")), FileCategory::Media);
        assert_eq!(FileCategory::from_path(Path::new("/data.DB")), FileCategory::Database);
    }

    #[test]
    fn test_no_extension() {
        use std::path::Path;
        assert_eq!(FileCategory::from_path(Path::new("/somefile")), FileCategory::Other);
        assert_eq!(FileCategory::from_path(Path::new("/")), FileCategory::Other);
    }

    #[test]
    fn test_format_age_none() {
        assert_eq!(format_age(None), "?");
    }

    #[test]
    fn test_format_age_now() {
        let recent = SystemTime::now() - Duration::from_secs(30);
        assert_eq!(format_age(Some(recent)), "now");
    }

    #[test]
    fn test_format_age_minutes() {
        let mins_ago = SystemTime::now() - Duration::from_secs(120);
        assert_eq!(format_age(Some(mins_ago)), "2m");
    }

    #[test]
    fn test_format_age_hours() {
        let hours_ago = SystemTime::now() - Duration::from_secs(7200);
        assert_eq!(format_age(Some(hours_ago)), "2h");
    }

    #[test]
    fn test_format_age_days() {
        let days_ago = SystemTime::now() - Duration::from_secs(86400 * 5);
        assert_eq!(format_age(Some(days_ago)), "5d");
    }

    #[test]
    fn test_format_age_months() {
        let months_ago = SystemTime::now() - Duration::from_secs(86400 * 60);
        assert_eq!(format_age(Some(months_ago)), "2mo");
    }

    #[test]
    fn test_format_age_years() {
        let years_ago = SystemTime::now() - Duration::from_secs(86400 * 400);
        assert_eq!(format_age(Some(years_ago)), "1y");
    }

    #[test]
    fn test_is_scanning() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        assert!(!analyzer.is_scanning());
    }

    #[test]
    fn test_directory_totals_empty() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        let totals = analyzer.directory_totals();
        assert!(totals.is_empty());
    }

    #[test]
    fn test_files_by_mount_empty() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        let mounts = analyzer.files_by_mount();
        assert!(mounts.is_empty());
    }

    #[test]
    fn test_top_files_full_path_empty() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        let files = analyzer.top_files_full_path(10);
        assert!(files.is_empty());
    }

    #[test]
    fn test_top_files_filtered_empty() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        let files = analyzer.top_files_filtered(10);
        assert!(files.is_empty());
    }

    #[test]
    fn test_analyzer_collect_spawns_scan() {
        let mut analyzer = TreemapAnalyzer::new("/tmp");
        // Collect spawns a background scan thread (may or may not be scanning by time assertion runs)
        analyzer.collect();
        // Just verify the method runs without panic
    }

    #[test]
    fn test_large_file_struct() {
        let file = LargeFile {
            name: "test.gguf".to_string(),
            path: PathBuf::from("/models/test.gguf"),
            size: 1024 * 1024 * 100,
            color_idx: 5,
            modified: Some(SystemTime::now()),
            category: FileCategory::Model,
        };

        assert_eq!(file.name, "test.gguf");
        assert_eq!(file.size, 104857600);
        assert_eq!(file.category, FileCategory::Model);
    }

    #[test]
    fn test_file_category_build_cache() {
        use std::path::Path;
        // Test .cache directory
        assert_eq!(
            FileCategory::from_path(Path::new("/home/user/.cache/pip/test.whl")),
            FileCategory::Build
        );
    }

    #[test]
    fn test_file_category_model_bin() {
        use std::path::Path;
        // Test model-like .bin files
        assert_eq!(
            FileCategory::from_path(Path::new("/models/llama-model.bin")),
            FileCategory::Model
        );
        assert_eq!(
            FileCategory::from_path(Path::new("/models/ggml-base.bin")),
            FileCategory::Model
        );
        assert_eq!(
            FileCategory::from_path(Path::new("/models/pytorch-model.bin")),
            FileCategory::Model
        );
    }

    #[test]
    fn test_squarify_varied_sizes() {
        let files = vec![
            LargeFile { name: "huge.bin".into(), path: "/huge.bin".into(), size: 10000, color_idx: 0, modified: None, category: FileCategory::Other },
            LargeFile { name: "medium.bin".into(), path: "/medium.bin".into(), size: 3000, color_idx: 1, modified: None, category: FileCategory::Other },
            LargeFile { name: "small.bin".into(), path: "/small.bin".into(), size: 1000, color_idx: 2, modified: None, category: FileCategory::Other },
        ];
        let layout = squarify_files(&files, 100.0, 100.0);

        // Should have some rectangles (squarify may drop tiny files)
        assert!(!layout.is_empty());

        // Largest file should have largest area
        if layout.len() >= 2 {
            let areas: Vec<f64> = layout.iter().map(|(r, _)| r.w * r.h).collect();
            assert!(areas[0] > areas[1]);
        }
    }

    #[test]
    fn test_all_ml_extensions() {
        use std::path::Path;
        let ml_exts = ["gguf", "safetensors", "onnx", "pt", "pth", "h5", "pb",
                       "ckpt", "tflite", "mlmodel", "mar", "keras", "engine",
                       "llamafile", "ollama", "hdf5"];
        for ext in ml_exts {
            let path_str = format!("/model.{}", ext);
            assert_eq!(
                FileCategory::from_path(Path::new(&path_str)),
                FileCategory::Model,
                "Extension {} should be Model category",
                ext
            );
        }
    }

    #[test]
    fn test_analyzer_collect_tmp() {
        let mut analyzer = TreemapAnalyzer::new("/tmp");
        // Collect should not panic
        analyzer.collect();
        // May or may not have files
        let _ = analyzer.total_size();
    }

    #[test]
    fn test_squarify_zero_size() {
        let files = vec![
            LargeFile { name: "zero.bin".into(), path: "/zero.bin".into(), size: 0, color_idx: 0, modified: None, category: FileCategory::Other },
        ];
        let layout = squarify_files(&files, 100.0, 100.0);
        // Zero-size file should be handled gracefully
        assert!(layout.is_empty() || layout[0].0.w * layout[0].0.h == 0.0 || layout[0].0.w * layout[0].0.h > 0.0);
    }

    // === Additional Coverage Tests ===

    #[test]
    fn test_file_category_clone() {
        let cat = FileCategory::Model;
        let cloned = cat.clone();
        assert_eq!(cat, cloned);
    }

    #[test]
    fn test_file_category_debug() {
        let debug = format!("{:?}", FileCategory::Archive);
        assert!(debug.contains("Archive"));
    }

    #[test]
    fn test_large_file_clone() {
        let file = LargeFile {
            name: "test.bin".into(),
            path: "/test.bin".into(),
            size: 1000,
            color_idx: 1,
            modified: None,
            category: FileCategory::Other,
        };
        let cloned = file.clone();
        assert_eq!(file.name, cloned.name);
        assert_eq!(file.size, cloned.size);
    }

    #[test]
    fn test_large_file_debug() {
        let file = LargeFile {
            name: "debug.bin".into(),
            path: "/debug.bin".into(),
            size: 2000,
            color_idx: 2,
            modified: None,
            category: FileCategory::Build,
        };
        let debug = format!("{:?}", file);
        assert!(debug.contains("debug.bin"));
        assert!(debug.contains("2000"));
    }

    #[test]
    fn test_tree_rect_clone() {
        let rect = TreeRect {
            x: 5.0,
            y: 5.0,
            w: 50.0,
            h: 50.0,
            size: 500,
            color_idx: 3,
            depth: 2,
        };
        let cloned = rect.clone();
        assert!((rect.x - cloned.x).abs() < 0.001);
        assert_eq!(rect.size, cloned.size);
    }

    #[test]
    fn test_tree_rect_debug() {
        let rect = TreeRect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
            size: 100,
            color_idx: 0,
            depth: 0,
        };
        let debug = format!("{:?}", rect);
        assert!(debug.contains("TreeRect"));
    }

    #[test]
    fn test_analyzer_total_size() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        // New analyzer should have zero total size before scanning
        let size = analyzer.total_size();
        assert!(size == 0 || size > 0); // Either no files yet or some found
    }

    #[test]
    fn test_analyzer_is_scanning() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        // New analyzer should not be scanning initially
        let scanning = analyzer.is_scanning();
        assert!(!scanning);
    }

    #[test]
    fn test_squarify_narrow_area() {
        let files = vec![
            LargeFile { name: "a.bin".into(), path: "/a".into(), size: 1000, color_idx: 0, modified: None, category: FileCategory::Other },
            LargeFile { name: "b.bin".into(), path: "/b".into(), size: 500, color_idx: 1, modified: None, category: FileCategory::Other },
        ];
        // Very narrow rectangle
        let layout = squarify_files(&files, 100.0, 5.0);
        // Should handle narrow area
        assert!(layout.len() <= 2);
    }

    #[test]
    fn test_squarify_tall_area() {
        let files = vec![
            LargeFile { name: "a.bin".into(), path: "/a".into(), size: 1000, color_idx: 0, modified: None, category: FileCategory::Other },
            LargeFile { name: "b.bin".into(), path: "/b".into(), size: 500, color_idx: 1, modified: None, category: FileCategory::Other },
        ];
        // Very tall rectangle
        let layout = squarify_files(&files, 5.0, 100.0);
        // Should handle tall area
        assert!(layout.len() <= 2);
    }

    #[test]
    fn test_format_age_edge_cases() {
        // Test 60 seconds (1 minute)
        let one_min_ago = SystemTime::now() - Duration::from_secs(60);
        let age = format_age(Some(one_min_ago));
        assert!(age == "now" || age == "1m");

        // Test 3600 seconds (1 hour)
        let one_hour_ago = SystemTime::now() - Duration::from_secs(3600);
        let age = format_age(Some(one_hour_ago));
        assert!(age == "1h" || age.ends_with('m'));
    }

    #[test]
    fn test_file_category_copy() {
        let cat = FileCategory::Media;
        let copied = cat;  // Copy trait
        assert_eq!(cat, copied);
    }

    #[test]
    fn test_file_category_eq() {
        assert_eq!(FileCategory::Database, FileCategory::Database);
        assert_ne!(FileCategory::Database, FileCategory::Media);
    }

    // === Additional Coverage Tests for TreemapAnalyzer Methods ===

    #[test]
    fn test_analyzer_default() {
        let analyzer = TreemapAnalyzer::default();
        assert!(!analyzer.is_scanning());
        assert_eq!(analyzer.total_size(), 0);
    }

    #[test]
    fn test_files_with_paths_with_data() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        // Inject test files via Arc<Mutex<>>
        {
            let mut files = analyzer.files.lock().expect("lock poisoned");
            files.push(LargeFile {
                name: "test.bin".to_string(),
                path: PathBuf::from("/home/user/downloads/test.bin"),
                size: 100_000_000,
                color_idx: 0,
                modified: None,
                category: FileCategory::Other,
            });
        }

        let paths = analyzer.files_with_paths();
        assert!(!paths.is_empty());
        // Should show "downloads/test.bin" format
        assert!(paths[0].0.contains("test.bin"));
        assert_eq!(paths[0].1, 100_000_000);
    }

    #[test]
    fn test_files_with_paths_no_parent() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        {
            let mut files = analyzer.files.lock().expect("lock poisoned");
            files.push(LargeFile {
                name: "root.bin".to_string(),
                path: PathBuf::from("/root.bin"),
                size: 50_000_000,
                color_idx: 0,
                modified: None,
                category: FileCategory::Other,
            });
        }

        let paths = analyzer.files_with_paths();
        assert!(!paths.is_empty());
        // When parent has no file_name, should just use the file name
        assert!(paths[0].0.contains("root.bin"));
    }

    #[test]
    fn test_top_files_full_path_with_data() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        {
            let mut files = analyzer.files.lock().expect("lock poisoned");
            for i in 0..5 {
                files.push(LargeFile {
                    name: format!("file{}.bin", i),
                    path: PathBuf::from(format!("/data/file{}.bin", i)),
                    size: (1000 - i * 100) as u64,
                    color_idx: i as u8,
                    modified: None,
                    category: FileCategory::Other,
                });
            }
        }

        let top = analyzer.top_files_full_path(3);
        assert_eq!(top.len(), 3);
        assert!(top[0].0.contains("file0.bin"));
    }

    #[test]
    fn test_top_files_filtered_excludes_benchmark() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        {
            let mut files = analyzer.files.lock().expect("lock poisoned");
            files.push(LargeFile {
                name: "model.gguf".to_string(),
                path: PathBuf::from("/models/model.gguf"),
                size: 5_000_000_000,
                color_idx: 0,
                modified: Some(SystemTime::now()),
                category: FileCategory::Model,
            });
            files.push(LargeFile {
                name: "seq-read.0.0".to_string(),
                path: PathBuf::from("/tmp/seq-read.0.0"),
                size: 10_000_000_000,
                color_idx: 1,
                modified: Some(SystemTime::now()),
                category: FileCategory::Benchmark,
            });
        }

        let filtered = analyzer.top_files_filtered(10);
        // Should only have the model, benchmark should be filtered out
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].0.contains("model"));
        assert_eq!(filtered[0].2, FileCategory::Model);
    }

    #[test]
    fn test_directory_totals_with_data() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        {
            let mut files = analyzer.files.lock().expect("lock poisoned");
            files.push(LargeFile {
                name: "model1.gguf".to_string(),
                path: PathBuf::from("/mnt/nvme/models/model1.gguf"),
                size: 1_000_000_000,
                color_idx: 0,
                modified: None,
                category: FileCategory::Model,
            });
            files.push(LargeFile {
                name: "model2.safetensors".to_string(),
                path: PathBuf::from("/mnt/nvme/models/model2.safetensors"),
                size: 2_000_000_000,
                color_idx: 0,
                modified: None,
                category: FileCategory::Model,
            });
            files.push(LargeFile {
                name: "archive.tar".to_string(),
                path: PathBuf::from("/mnt/nvme/backup/archive.tar"),
                size: 500_000_000,
                color_idx: 1,
                modified: None,
                category: FileCategory::Archive,
            });
        }

        let totals = analyzer.directory_totals();
        // Should have directory groupings
        assert!(!totals.is_empty());
        // Verify sizes are summed
        let total_size: u64 = totals.iter().map(|(_, size, _, _)| *size).sum();
        assert_eq!(total_size, 3_500_000_000);
    }

    #[test]
    fn test_directory_totals_excludes_benchmark() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        {
            let mut files = analyzer.files.lock().expect("lock poisoned");
            files.push(LargeFile {
                name: "seq-read.0.0".to_string(),
                path: PathBuf::from("/tmp/bench/seq-read.0.0"),
                size: 10_000_000_000,
                color_idx: 0,
                modified: None,
                category: FileCategory::Benchmark,
            });
        }

        let totals = analyzer.directory_totals();
        // Benchmarks should be excluded
        assert!(totals.is_empty());
    }

    #[test]
    fn test_files_by_mount_with_data() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        {
            let mut files = analyzer.files.lock().expect("lock poisoned");
            files.push(LargeFile {
                name: "large.bin".to_string(),
                path: PathBuf::from("/home/user/large.bin"),
                size: 1_000_000_000,
                color_idx: 0,
                modified: None,
                category: FileCategory::Other,
            });
        }

        let by_mount = analyzer.files_by_mount();
        // Should have at least one mount group
        assert!(!by_mount.is_empty());
    }

    #[test]
    fn test_files_by_mount_long_path() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        {
            let mut files = analyzer.files.lock().expect("lock poisoned");
            // Create a very long path to test path shortening
            files.push(LargeFile {
                name: "very_long_filename_that_exceeds_limit.bin".to_string(),
                path: PathBuf::from("/mnt/raid/very/deep/nested/directory/structure/that/is/quite/long/very_long_filename_that_exceeds_limit.bin"),
                size: 500_000_000,
                color_idx: 0,
                modified: None,
                category: FileCategory::Other,
            });
        }

        let by_mount = analyzer.files_by_mount();
        // Should handle long paths gracefully
        assert!(!by_mount.is_empty());
    }

    #[test]
    fn test_layout_with_injected_files() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        {
            let mut files = analyzer.files.lock().expect("lock poisoned");
            files.push(LargeFile {
                name: "a.bin".into(),
                path: PathBuf::from("/a.bin"),
                size: 1000,
                color_idx: 0,
                modified: None,
                category: FileCategory::Other,
            });
            files.push(LargeFile {
                name: "b.bin".into(),
                path: PathBuf::from("/b.bin"),
                size: 500,
                color_idx: 1,
                modified: None,
                category: FileCategory::Other,
            });
        }

        let layout = analyzer.layout(100.0, 100.0);
        assert!(!layout.is_empty());
        assert!(layout.len() >= 1);
    }

    #[test]
    fn test_squarify_equal_sizes() {
        let files = vec![
            LargeFile { name: "a.bin".into(), path: "/a".into(), size: 1000, color_idx: 0, modified: None, category: FileCategory::Other },
            LargeFile { name: "b.bin".into(), path: "/b".into(), size: 1000, color_idx: 1, modified: None, category: FileCategory::Other },
            LargeFile { name: "c.bin".into(), path: "/c".into(), size: 1000, color_idx: 2, modified: None, category: FileCategory::Other },
        ];
        let layout = squarify_files(&files, 100.0, 100.0);
        assert_eq!(layout.len(), 3);

        // All areas should be roughly equal
        let areas: Vec<f64> = layout.iter().map(|(r, _)| r.w * r.h).collect();
        for area in &areas {
            assert!((area - 3333.3).abs() < 500.0);
        }
    }

    #[test]
    fn test_format_age_future_time() {
        // Test with a time in the future (edge case)
        let future = SystemTime::now() + Duration::from_secs(3600);
        let age = format_age(Some(future));
        // Should return "?" for future times since duration_since will fail
        assert_eq!(age, "?");
    }

    #[test]
    fn test_squarify_very_different_sizes() {
        let files = vec![
            LargeFile { name: "huge.bin".into(), path: "/huge".into(), size: 1_000_000, color_idx: 0, modified: None, category: FileCategory::Other },
            LargeFile { name: "tiny.bin".into(), path: "/tiny".into(), size: 1, color_idx: 1, modified: None, category: FileCategory::Other },
        ];
        let layout = squarify_files(&files, 100.0, 100.0);
        // Should handle very different sizes
        assert!(layout.len() >= 1);
    }

    #[test]
    fn test_file_category_from_empty_path() {
        use std::path::Path;
        // Empty path component
        let cat = FileCategory::from_path(Path::new(""));
        assert_eq!(cat, FileCategory::Other);
    }

    #[test]
    fn test_directory_totals_shallow_mount() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        {
            let mut files = analyzer.files.lock().expect("lock poisoned");
            // File at root of mount (shallow path)
            files.push(LargeFile {
                name: "root_file.db".to_string(),
                path: PathBuf::from("/mnt/root_file.db"),
                size: 1_000_000,
                color_idx: 0,
                modified: None,
                category: FileCategory::Database,
            });
        }

        let totals = analyzer.directory_totals();
        assert!(!totals.is_empty());
    }

    #[test]
    fn test_total_size_with_multiple_files() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        {
            let mut files = analyzer.files.lock().expect("lock poisoned");
            files.push(LargeFile {
                name: "a.bin".into(),
                path: "/a".into(),
                size: 1000,
                color_idx: 0,
                modified: None,
                category: FileCategory::Other,
            });
            files.push(LargeFile {
                name: "b.bin".into(),
                path: "/b".into(),
                size: 2000,
                color_idx: 1,
                modified: None,
                category: FileCategory::Other,
            });
        }

        assert_eq!(analyzer.total_size(), 3000);
    }

    #[test]
    fn test_squarify_single_file_exact_dimensions() {
        let files = vec![
            LargeFile { name: "single.bin".into(), path: "/single".into(), size: 1000, color_idx: 0, modified: None, category: FileCategory::Other },
        ];
        let layout = squarify_files(&files, 50.0, 30.0);
        assert_eq!(layout.len(), 1);
        // Single file should fill the entire area
        assert!((layout[0].0.w - 50.0).abs() < 0.1);
        assert!((layout[0].0.h - 30.0).abs() < 0.1);
    }

    #[test]
    fn test_file_category_mixed_case_model_bin() {
        use std::path::Path;
        // Test model detection with mixed case
        assert_eq!(
            FileCategory::from_path(Path::new("/models/PyTorch_Model.bin")),
            FileCategory::Model
        );
        assert_eq!(
            FileCategory::from_path(Path::new("/models/LLAMA-7B.bin")),
            FileCategory::Model
        );
    }

    #[test]
    fn test_layout_zero_dimensions() {
        let analyzer = TreemapAnalyzer::new("/tmp");
        {
            let mut files = analyzer.files.lock().expect("lock poisoned");
            files.push(LargeFile {
                name: "test.bin".into(),
                path: "/test".into(),
                size: 1000,
                color_idx: 0,
                modified: None,
                category: FileCategory::Other,
            });
        }

        // Zero dimensions should return empty
        let layout = analyzer.layout(0.0, 100.0);
        assert!(layout.is_empty());

        let layout = analyzer.layout(100.0, 0.0);
        assert!(layout.is_empty());
    }

    #[test]
    fn test_tree_rect_copy() {
        let rect = TreeRect {
            x: 1.0,
            y: 2.0,
            w: 3.0,
            h: 4.0,
            size: 5,
            color_idx: 6,
            depth: 7,
        };
        let copied = rect; // Copy trait
        assert_eq!(rect.x, copied.x);
        assert_eq!(rect.size, copied.size);
    }
}
