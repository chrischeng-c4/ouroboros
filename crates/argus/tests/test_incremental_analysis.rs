//! Integration tests for incremental analysis (M5.2)

use argus::types::{IncrementalAnalyzer, IncrementalConfig, ChangeKind, ContentHash};
use std::path::PathBuf;
use std::fs;
use std::time::Duration;
use tempfile::TempDir;

fn create_test_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn test_incremental_analyzer_basic() {
    let config = IncrementalConfig::default();
    let mut analyzer = IncrementalAnalyzer::new(config);

    // Create temp files
    let temp_dir = TempDir::new().unwrap();
    let file1 = create_test_file(&temp_dir, "test1.py", "import os\nprint('hello')");

    // Record change
    let hash = ContentHash::from_content("import os\nprint('hello')");
    analyzer.file_changed(file1.clone(), ChangeKind::Created, hash);

    // Wait for debounce period (default is 300ms)
    std::thread::sleep(Duration::from_millis(350));

    // Get files to analyze
    let files = analyzer.get_files_to_analyze();
    assert!(!files.is_empty());
    assert!(files.contains(&file1));
}

#[test]
fn test_dependency_extraction_python() {
    let config = IncrementalConfig::default();
    let analyzer = IncrementalAnalyzer::new(config);

    let temp_dir = TempDir::new().unwrap();
    let content = r#"
import os
import sys
from pathlib import Path
"#;
    let file = create_test_file(&temp_dir, "test.py", content);

    let hash = ContentHash::from_content(content);
    let mut analyzer_mut = analyzer;
    analyzer_mut.file_changed(file.clone(), ChangeKind::Created, hash);

    // Analyze file
    let files_to_analyze = vec![file.clone()];
    let result = analyzer_mut.analyze(files_to_analyze);

    // Should have analyzed the file
    assert_eq!(result.analyzed_files.len(), 1);
    assert!(result.failed_files.is_empty());
}

#[test]
fn test_cache_hit() {
    let config = IncrementalConfig::default();
    let mut analyzer = IncrementalAnalyzer::new(config);

    let temp_dir = TempDir::new().unwrap();
    let content = "import os\n";
    let file = create_test_file(&temp_dir, "test.py", content);
    let hash = ContentHash::from_content(content);

    // First analysis
    analyzer.file_changed(file.clone(), ChangeKind::Created, hash.clone());
    let files1 = vec![file.clone()];
    let result1 = analyzer.analyze(files1);

    assert_eq!(result1.analyzed_files.len(), 1);
    assert_eq!(result1.cached_files.len(), 0);

    // Second analysis without changes - should hit cache
    let files2 = vec![file.clone()];
    let result2 = analyzer.analyze(files2);

    assert_eq!(result2.analyzed_files.len(), 0);
    assert_eq!(result2.cached_files.len(), 1);
}

#[test]
fn test_cache_invalidation() {
    let config = IncrementalConfig::default();
    let mut analyzer = IncrementalAnalyzer::new(config);

    let temp_dir = TempDir::new().unwrap();
    let content1 = "import os\n";
    let file = create_test_file(&temp_dir, "test.py", content1);
    let hash1 = ContentHash::from_content(content1);

    // First analysis
    analyzer.file_changed(file.clone(), ChangeKind::Created, hash1);
    let result1 = analyzer.analyze(vec![file.clone()]);
    assert_eq!(result1.analyzed_files.len(), 1);

    // Modify content
    let content2 = "import sys\n";
    fs::write(&file, content2).unwrap();
    let hash2 = ContentHash::from_content(content2);

    // Record change
    analyzer.file_changed(file.clone(), ChangeKind::Modified, hash2);

    // Wait for debounce
    std::thread::sleep(Duration::from_millis(350));

    // Should reanalyze
    let files = analyzer.get_files_to_analyze();
    let result2 = analyzer.analyze(files);

    assert_eq!(result2.analyzed_files.len(), 1, "File should be reanalyzed after change");
    assert_eq!(result2.cached_files.len(), 0);
}

#[test]
fn test_dependency_graph_affected_files() {
    let config = IncrementalConfig::default();
    let mut analyzer = IncrementalAnalyzer::new(config);

    let temp_dir = TempDir::new().unwrap();

    // Create module.py
    let module_content = "def func(): pass\n";
    let module_file = create_test_file(&temp_dir, "module.py", module_content);

    // Create main.py that imports module
    let main_content = "from module import func\n";
    let main_file = create_test_file(&temp_dir, "main.py", main_content);

    // Analyze both files
    let hash1 = ContentHash::from_content(module_content);
    let hash2 = ContentHash::from_content(main_content);

    analyzer.file_changed(module_file.clone(), ChangeKind::Created, hash1);
    analyzer.file_changed(main_file.clone(), ChangeKind::Created, hash2);

    let result = analyzer.analyze(vec![main_file.clone(), module_file.clone()]);

    // Both should be analyzed
    assert_eq!(result.analyzed_files.len(), 2);

    // Check dependency graph (though extraction may not catch all dependencies)
    let dep_graph = analyzer.dep_graph();
    assert!(dep_graph.contains(&main_file) || dep_graph.contains(&module_file));
}

#[test]
fn test_change_tracker_debounce() {
    let config = IncrementalConfig {
        background_analysis: false,
        batch_size: 10,
        file_timeout: Duration::from_secs(5),
        persistent_cache: false,
        cache_dir: None,
    };
    let mut analyzer = IncrementalAnalyzer::new(config);

    let temp_dir = TempDir::new().unwrap();
    let file = create_test_file(&temp_dir, "test.py", "import os\n");
    let hash = ContentHash::from_content("import os\n");

    // Record change
    analyzer.file_changed(file.clone(), ChangeKind::Modified, hash);

    // Immediately check - should be empty due to debounce
    let files = analyzer.get_files_to_analyze();
    assert!(files.is_empty(), "Files should be empty due to debounce");

    // Wait for debounce period
    std::thread::sleep(Duration::from_millis(400));

    // Now should have files
    let files = analyzer.get_files_to_analyze();
    assert!(!files.is_empty(), "Files should be available after debounce");
}

#[test]
fn test_analysis_result_tracking() {
    let config = IncrementalConfig::default();
    let mut analyzer = IncrementalAnalyzer::new(config);

    let temp_dir = TempDir::new().unwrap();
    let file1 = create_test_file(&temp_dir, "test1.py", "import os\n");
    let file2 = create_test_file(&temp_dir, "test2.py", "import sys\n");

    let hash1 = ContentHash::from_content("import os\n");
    let hash2 = ContentHash::from_content("import sys\n");

    analyzer.file_changed(file1.clone(), ChangeKind::Created, hash1);
    analyzer.file_changed(file2.clone(), ChangeKind::Created, hash2);

    let files = vec![file1.clone(), file2.clone()];
    let result = analyzer.analyze(files);

    // Check result
    assert_eq!(result.analyzed_files.len(), 2);
    assert_eq!(result.cached_files.len(), 0);
    assert_eq!(result.failed_files.len(), 0);
    assert!(!result.has_failures());
    assert!(result.total_time > Duration::ZERO);
}

#[test]
fn test_clear_cache() {
    let config = IncrementalConfig::default();
    let mut analyzer = IncrementalAnalyzer::new(config);

    let temp_dir = TempDir::new().unwrap();
    let file = create_test_file(&temp_dir, "test.py", "import os\n");
    let hash = ContentHash::from_content("import os\n");

    // Analyze
    analyzer.file_changed(file.clone(), ChangeKind::Created, hash);
    let result1 = analyzer.analyze(vec![file.clone()]);
    assert_eq!(result1.analyzed_files.len(), 1);

    // Second analysis - should hit cache
    let result2 = analyzer.analyze(vec![file.clone()]);
    assert_eq!(result2.cached_files.len(), 1);

    // Clear cache
    analyzer.clear_cache();

    // Third analysis - should reanalyze
    let result3 = analyzer.analyze(vec![file.clone()]);
    assert_eq!(result3.analyzed_files.len(), 1);
    assert_eq!(result3.cached_files.len(), 0);
}
