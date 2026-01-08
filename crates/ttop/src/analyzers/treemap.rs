//! Large File Treemap - finds and displays biggest files
//!
//! Scans filesystem for large files and renders as 2D treemap.

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// A large file entry
#[derive(Debug, Clone)]
pub struct LargeFile {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub color_idx: u8,
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
            let scanning = self.scanning.lock().unwrap();
            if *scanning {
                return;
            }
        }

        let files = Arc::clone(&self.files);
        let scanning = Arc::clone(&self.scanning);

        *scanning.lock().unwrap() = true;
        self.last_scan = Instant::now();

        thread::spawn(move || {
            let found = find_large_files();
            *files.lock().unwrap() = found;
            *scanning.lock().unwrap() = false;
        });
    }

    /// Get treemap layout for rendering
    pub fn layout(&self, width: f64, height: f64) -> Vec<(TreeRect, String)> {
        let files = self.files.lock().unwrap();
        if files.is_empty() {
            return Vec::new();
        }
        squarify_files(&files, width, height)
    }

    /// Check if currently scanning
    pub fn is_scanning(&self) -> bool {
        *self.scanning.lock().unwrap()
    }

    /// Get total size of all large files
    pub fn total_size(&self) -> u64 {
        self.files.lock().unwrap().iter().map(|f| f.size).sum()
    }
}

impl Default for TreemapAnalyzer {
    fn default() -> Self {
        Self::new("/")
    }
}

/// Find large files (>50MB) in common locations
fn find_large_files() -> Vec<LargeFile> {
    let mut files = Vec::new();

    // Scan common directories for large files
    let scan_dirs = [
        "/home",
        "/var",
        "/opt",
        "/usr",
        "/tmp",
    ];

    for (dir_idx, dir) in scan_dirs.iter().enumerate() {
        scan_for_large_files(dir, &mut files, dir_idx as u8, 0, 4);
    }

    // Sort by size descending
    files.sort_by(|a, b| b.size.cmp(&a.size));

    // Keep top 50
    files.truncate(50);
    files
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
            // Include files > 50MB
            if size > 50 * 1024 * 1024 {
                files.push(LargeFile {
                    name: name.clone(),
                    path: entry_path,
                    size,
                    color_idx,
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
            LargeFile { name: "big.bin".into(), path: "/big.bin".into(), size: 1000, color_idx: 0 },
        ];
        let layout = squarify_files(&files, 10.0, 10.0);
        assert_eq!(layout.len(), 1);
        assert!((layout[0].0.w - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_squarify_multiple() {
        let files = vec![
            LargeFile { name: "a.bin".into(), path: "/a".into(), size: 500, color_idx: 0 },
            LargeFile { name: "b.bin".into(), path: "/b".into(), size: 300, color_idx: 1 },
            LargeFile { name: "c.bin".into(), path: "/c".into(), size: 200, color_idx: 2 },
        ];
        let layout = squarify_files(&files, 10.0, 10.0);
        assert_eq!(layout.len(), 3);
    }
}
