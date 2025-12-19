// Discovery module for dbtest CLI
//
// This module provides fast file-system discovery using the walkdir crate,
// storing file paths and metadata for lazy loading during execution.

use std::path::{Path, PathBuf};
use std::time::Instant;
use walkdir::WalkDir;

/// File type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Test,
    Benchmark,
}

/// Configuration for file discovery
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Root path to start discovery
    pub root_path: PathBuf,
    /// File patterns to match (e.g., ["test_*.py", "bench_*.py"])
    pub patterns: Vec<String>,
    /// Directories to exclude (e.g., ["__pycache__", ".git"])
    pub exclusions: Vec<String>,
    /// Maximum directory depth
    pub max_depth: usize,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            root_path: PathBuf::from("tests/"),
            patterns: vec!["test_*.py".to_string(), "bench_*.py".to_string()],
            exclusions: vec!["__pycache__".to_string(), ".git".to_string()],
            max_depth: 10,
        }
    }
}

/// File information discovered by walkdir
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// Absolute path to the file
    pub path: PathBuf,
    /// Python module name (derived from path)
    pub module_name: String,
    /// File type (Test or Benchmark)
    pub file_type: FileType,
}

impl FileInfo {
    /// Create FileInfo from path
    pub fn from_path(path: &Path, root: &Path) -> Result<Self, String> {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| format!("Invalid file name: {:?}", path))?;

        // Determine file type
        let file_type = if file_name.starts_with("test_") {
            FileType::Test
        } else if file_name.starts_with("bench_") {
            FileType::Benchmark
        } else {
            return Err(format!("Unknown file type: {}", file_name));
        };

        // Generate module name (e.g., "tests.mongo.unit.test_document")
        let rel_path = path
            .strip_prefix(root)
            .map_err(|e| format!("Failed to strip prefix: {}", e))?;

        let module_name = rel_path
            .with_extension("")
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(".");

        Ok(Self {
            path: path.to_path_buf(),
            module_name,
            file_type,
        })
    }

    /// Check if file matches a glob pattern
    pub fn matches_pattern(&self, pattern: &str) -> bool {
        let file_name = self
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // Simple glob matching (supports * wildcard)
        if let Some(star_pos) = pattern.find('*') {
            let prefix = &pattern[..star_pos];
            let suffix = &pattern[star_pos + 1..];
            file_name.starts_with(prefix) && file_name.ends_with(suffix)
        } else {
            file_name == pattern
        }
    }
}

/// Statistics from discovery process
#[derive(Debug, Clone)]
pub struct DiscoveryStats {
    pub files_found: usize,
    pub filtered_count: usize,
    pub discovery_time_ms: u64,
}

/// Registry for test files
#[derive(Debug, Clone)]
pub struct TestRegistry {
    files: Vec<FileInfo>,
}

impl TestRegistry {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn register(&mut self, file: FileInfo) {
        if file.file_type == FileType::Test {
            self.files.push(file);
        }
    }

    pub fn get_all(&self) -> &[FileInfo] {
        &self.files
    }

    pub fn filter_by_pattern(&mut self, pattern: &str) -> &mut Self {
        self.files.retain(|f| f.matches_pattern(pattern));
        self
    }

    pub fn count(&self) -> usize {
        self.files.len()
    }

    pub fn clear(&mut self) {
        self.files.clear();
    }
}

impl Default for TestRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry for benchmark files
#[derive(Debug, Clone)]
pub struct BenchmarkRegistry {
    files: Vec<FileInfo>,
}

impl BenchmarkRegistry {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn register(&mut self, file: FileInfo) {
        if file.file_type == FileType::Benchmark {
            self.files.push(file);
        }
    }

    pub fn get_all(&self) -> &[FileInfo] {
        &self.files
    }

    pub fn filter_by_pattern(&mut self, pattern: &str) -> &mut Self {
        self.files.retain(|f| f.matches_pattern(pattern));
        self
    }

    pub fn count(&self) -> usize {
        self.files.len()
    }

    pub fn clear(&mut self) {
        self.files.clear();
    }
}

impl Default for BenchmarkRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Walk file system and discover test/benchmark files
pub fn walk_files(config: &DiscoveryConfig) -> Result<Vec<FileInfo>, String> {
    let start = Instant::now();
    let mut files = Vec::new();

    let walker = WalkDir::new(&config.root_path)
        .follow_links(false)
        .max_depth(config.max_depth)
        .into_iter()
        .filter_entry(|e| {
            // Exclude specified directories
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy();
                !config.exclusions.iter().any(|ex| name.contains(ex))
            } else {
                true
            }
        });

    for entry in walker {
        let entry = entry.map_err(|e| format!("Walk error: {}", e))?;

        // Skip directories
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // Check if matches any pattern
        let matches = config
            .patterns
            .iter()
            .any(|pat| pattern_matches(file_name, pat));

        if matches {
            match FileInfo::from_path(path, &config.root_path) {
                Ok(file_info) => files.push(file_info),
                Err(e) => eprintln!("Warning: {}", e),
            }
        }
    }

    let elapsed = start.elapsed().as_millis() as u64;
    tracing::debug!(
        "Discovery completed: {} files in {}ms",
        files.len(),
        elapsed
    );

    Ok(files)
}

/// Simple glob pattern matching
fn pattern_matches(text: &str, pattern: &str) -> bool {
    if let Some(star_pos) = pattern.find('*') {
        let prefix = &pattern[..star_pos];
        let suffix = &pattern[star_pos + 1..];
        text.starts_with(prefix) && text.ends_with(suffix)
    } else {
        text == pattern
    }
}

/// Filter files by pattern
pub fn filter_files(files: Vec<FileInfo>, pattern: &str) -> Vec<FileInfo> {
    files
        .into_iter()
        .filter(|f| f.matches_pattern(pattern))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matches() {
        assert!(pattern_matches("test_foo.py", "test_*.py"));
        assert!(pattern_matches("bench_insert.py", "bench_*.py"));
        assert!(!pattern_matches("foo_test.py", "test_*.py"));
        assert!(pattern_matches("exact.py", "exact.py"));
        assert!(!pattern_matches("exact.py", "other.py"));
    }

    #[test]
    fn test_file_info_matches_pattern() {
        let path = PathBuf::from("/tmp/test_example.py");
        let root = PathBuf::from("/tmp");
        let file_info = FileInfo::from_path(&path, &root).unwrap();

        assert!(file_info.matches_pattern("test_*.py"));
        assert!(!file_info.matches_pattern("bench_*.py"));
        assert!(file_info.matches_pattern("test_example.py"));
    }

    #[test]
    fn test_test_registry() {
        let mut registry = TestRegistry::new();

        let path1 = PathBuf::from("/tmp/test_one.py");
        let path2 = PathBuf::from("/tmp/test_two.py");
        let root = PathBuf::from("/tmp");

        let file1 = FileInfo::from_path(&path1, &root).unwrap();
        let file2 = FileInfo::from_path(&path2, &root).unwrap();

        registry.register(file1);
        registry.register(file2);

        assert_eq!(registry.count(), 2);

        registry.filter_by_pattern("test_one.py");
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_benchmark_registry() {
        let mut registry = BenchmarkRegistry::new();

        let path1 = PathBuf::from("/tmp/bench_insert.py");
        let path2 = PathBuf::from("/tmp/bench_find.py");
        let root = PathBuf::from("/tmp");

        let file1 = FileInfo::from_path(&path1, &root).unwrap();
        let file2 = FileInfo::from_path(&path2, &root).unwrap();

        registry.register(file1);
        registry.register(file2);

        assert_eq!(registry.count(), 2);

        registry.filter_by_pattern("bench_insert.py");
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_filter_files() {
        let root = PathBuf::from("/tmp");
        let files = vec![
            FileInfo::from_path(&PathBuf::from("/tmp/test_foo.py"), &root).unwrap(),
            FileInfo::from_path(&PathBuf::from("/tmp/test_bar.py"), &root).unwrap(),
        ];

        let filtered = filter_files(files, "test_foo.py");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].module_name, "test_foo");
    }
}
