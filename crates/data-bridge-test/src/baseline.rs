//! Baseline metrics recording and regression detection
//!
//! Provides utilities for:
//! - Saving benchmark results as baselines
//! - Loading historical baselines
//! - Detecting performance regressions
//! - Generating regression reports

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use std::io;
use chrono::Utc;
use crate::benchmark::{BenchmarkResult, BenchmarkEnvironment};

/// Metadata for a baseline snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineMetadata {
    /// Baseline format version
    pub version: String,
    /// Timestamp when baseline was created (RFC3339)
    pub timestamp: String,
    /// Git metadata (if available)
    pub git_metadata: Option<GitMetadata>,
    /// Environment information
    pub environment: BenchmarkEnvironment,
}

/// Git repository metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitMetadata {
    /// Current commit SHA
    pub commit_sha: Option<String>,
    /// Current branch name
    pub branch: Option<String>,
    /// Whether working directory has uncommitted changes
    pub is_dirty: bool,
    /// Git user name
    pub author: Option<String>,
}

/// A snapshot of benchmark results at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineSnapshot {
    /// Metadata about this baseline
    pub metadata: BaselineMetadata,
    /// Benchmark results
    pub benchmarks: Vec<BenchmarkResult>,
}

/// Thresholds for regression detection
#[derive(Debug, Clone)]
pub struct RegressionThresholds {
    /// Warning threshold as percentage (e.g., 5.0 = 5%)
    pub warning_threshold_percent: f64,
    /// Failure threshold as percentage (e.g., 15.0 = 15%)
    pub failure_threshold_percent: f64,
    /// Require confidence interval overlap to avoid false positives
    pub ci_overlap_required: bool,
    /// Minimum number of samples required for comparison
    pub min_samples_for_comparison: u32,
}

impl Default for RegressionThresholds {
    fn default() -> Self {
        Self {
            warning_threshold_percent: 5.0,
            failure_threshold_percent: 15.0,
            ci_overlap_required: false,
            min_samples_for_comparison: 3,
        }
    }
}

/// Severity of a detected regression
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegressionSeverity {
    /// 0-5% slower
    Minor,
    /// 5-15% slower
    Moderate,
    /// >15% slower
    Severe,
}

/// A detected performance regression
#[derive(Debug, Clone)]
pub struct Regression {
    /// Name of the benchmark
    pub name: String,
    /// Baseline mean time in milliseconds
    pub baseline_mean_ms: f64,
    /// Current mean time in milliseconds
    pub current_mean_ms: f64,
    /// Percentage change (positive = slower)
    pub percent_change: f64,
    /// Whether confidence intervals overlap
    pub ci_overlap: bool,
    /// Severity of the regression
    pub severity: RegressionSeverity,
}

/// A detected performance improvement
#[derive(Debug, Clone)]
pub struct Improvement {
    /// Name of the benchmark
    pub name: String,
    /// Baseline mean time in milliseconds
    pub baseline_mean_ms: f64,
    /// Current mean time in milliseconds
    pub current_mean_ms: f64,
    /// Percentage change (negative = faster)
    pub percent_change: f64,
}

/// Summary statistics for regression analysis
#[derive(Debug, Clone, Default)]
pub struct RegressionSummary {
    /// Total number of benchmarks compared
    pub total_benchmarks: usize,
    /// Number of regressions found
    pub regressions_found: usize,
    /// Number of improvements found
    pub improvements_found: usize,
    /// Number of unchanged benchmarks
    pub unchanged: usize,
}

/// Report containing all regression analysis results
#[derive(Debug, Clone)]
pub struct RegressionReport {
    /// Baseline timestamp
    pub baseline_timestamp: String,
    /// Current run timestamp
    pub current_timestamp: String,
    /// Detected regressions
    pub regressions: Vec<Regression>,
    /// Detected improvements
    pub improvements: Vec<Improvement>,
    /// Summary statistics
    pub summary: RegressionSummary,
}

/// File-based baseline storage
pub struct FileBaselineStore {
    base_dir: PathBuf,
}

impl FileBaselineStore {
    /// Create a new file-based baseline store
    pub fn new(base_dir: impl AsRef<Path>) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    /// Create a default baseline store in .baselines directory
    pub fn default_store() -> Self {
        Self::new(".baselines")
    }

    /// Save benchmark results as a baseline
    ///
    /// # Arguments
    /// * `name` - Name of the baseline (e.g., "main", "feature-branch")
    /// * `results` - Benchmark results to save
    /// * `env` - Environment information
    ///
    /// # Returns
    /// The filename of the saved baseline
    pub fn save_baseline(
        &self,
        name: &str,
        results: &[BenchmarkResult],
        env: &BenchmarkEnvironment,
    ) -> io::Result<String> {
        fs::create_dir_all(&self.base_dir)?;

        let timestamp = Utc::now().to_rfc3339();
        let snapshot = BaselineSnapshot {
            metadata: BaselineMetadata {
                version: "1.0".to_string(),
                timestamp: timestamp.clone(),
                git_metadata: Self::get_git_metadata().ok(),
                environment: env.clone(),
            },
            benchmarks: results.to_vec(),
        };

        let filename = format!("{}_{}.json", name, timestamp.replace(':', "-"));
        let path = self.base_dir.join(&filename);

        let json = serde_json::to_string_pretty(&snapshot)
            .map_err(io::Error::other)?;
        fs::write(&path, json)?;

        // Update latest symlink/copy
        let latest = self.base_dir.join(format!("{}_latest.json", name));
        let _ = fs::remove_file(&latest);
        fs::copy(&path, &latest)?;

        Ok(filename)
    }

    /// Load a baseline by name and ID
    ///
    /// # Arguments
    /// * `name` - Name of the baseline
    /// * `id` - Baseline ID (filename or "latest")
    ///
    /// # Returns
    /// The loaded baseline snapshot
    pub fn load_baseline(&self, name: &str, id: &str) -> io::Result<BaselineSnapshot> {
        let path = if id == "latest" {
            self.base_dir.join(format!("{}_latest.json", name))
        } else {
            self.base_dir.join(id)
        };

        let json = fs::read_to_string(&path)?;
        let snapshot: BaselineSnapshot = serde_json::from_str(&json)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(snapshot)
    }

    /// List all baselines for a given name
    ///
    /// # Arguments
    /// * `name` - Name of the baseline group
    ///
    /// # Returns
    /// Vector of metadata for all matching baselines (sorted newest first)
    pub fn list_baselines(&self, name: &str) -> io::Result<Vec<BaselineMetadata>> {
        if !self.base_dir.exists() {
            return Ok(Vec::new());
        }

        let mut metadata = Vec::new();

        for entry in fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let filename = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                if filename.starts_with(name) && !filename.ends_with("_latest.json") {
                    if let Ok(json) = fs::read_to_string(&path) {
                        if let Ok(snapshot) = serde_json::from_str::<BaselineSnapshot>(&json) {
                            metadata.push(snapshot.metadata);
                        }
                    }
                }
            }
        }

        // Sort by timestamp (newest first)
        metadata.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(metadata)
    }

    /// Attempt to extract git metadata from current repository
    fn get_git_metadata() -> io::Result<GitMetadata> {
        use std::process::Command;

        let commit_sha = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());

        let branch = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());

        let is_dirty = Command::new("git")
            .args(["status", "--porcelain"])
            .output()
            .ok()
            .map(|o| !o.stdout.is_empty())
            .unwrap_or(false);

        let author = Command::new("git")
            .args(["config", "user.name"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());

        Ok(GitMetadata {
            commit_sha,
            branch,
            is_dirty,
            author,
        })
    }
}

/// Regression detection engine
pub struct RegressionDetector;

impl RegressionDetector {
    /// Detect regressions by comparing current results against baseline
    ///
    /// # Arguments
    /// * `baseline` - Baseline snapshot to compare against
    /// * `current` - Current benchmark results
    /// * `thresholds` - Thresholds for regression detection
    ///
    /// # Returns
    /// A regression report containing all findings
    pub fn detect_regressions(
        baseline: &BaselineSnapshot,
        current: &[BenchmarkResult],
        thresholds: &RegressionThresholds,
    ) -> RegressionReport {
        let mut regressions = Vec::new();
        let mut improvements = Vec::new();
        let mut unchanged = 0;

        for current_result in current {
            if let Some(baseline_result) =
                baseline.benchmarks.iter().find(|b| b.name == current_result.name) {

                let baseline_mean = baseline_result.stats.mean_ms;
                let current_mean = current_result.stats.mean_ms;
                let percent_change = ((current_mean - baseline_mean) / baseline_mean) * 100.0;

                // Check CI overlap
                let ci_overlap = !(baseline_result.stats.ci_upper_ms < current_result.stats.ci_lower_ms
                    || baseline_result.stats.ci_lower_ms > current_result.stats.ci_upper_ms);

                if percent_change > thresholds.warning_threshold_percent {
                    if !ci_overlap || !thresholds.ci_overlap_required {
                        let severity = if percent_change > thresholds.failure_threshold_percent {
                            RegressionSeverity::Severe
                        } else if percent_change > thresholds.warning_threshold_percent {
                            RegressionSeverity::Moderate
                        } else {
                            RegressionSeverity::Minor
                        };

                        regressions.push(Regression {
                            name: current_result.name.clone(),
                            baseline_mean_ms: baseline_mean,
                            current_mean_ms: current_mean,
                            percent_change,
                            ci_overlap,
                            severity,
                        });
                    } else {
                        unchanged += 1;
                    }
                } else if percent_change < -2.0 {
                    improvements.push(Improvement {
                        name: current_result.name.clone(),
                        baseline_mean_ms: baseline_mean,
                        current_mean_ms: current_mean,
                        percent_change,
                    });
                } else {
                    unchanged += 1;
                }
            }
        }

        RegressionReport {
            baseline_timestamp: baseline.metadata.timestamp.clone(),
            current_timestamp: Utc::now().to_rfc3339(),
            summary: RegressionSummary {
                total_benchmarks: current.len(),
                regressions_found: regressions.len(),
                improvements_found: improvements.len(),
                unchanged,
            },
            regressions,
            improvements,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::benchmark::{BenchmarkStats};

    fn create_test_result(name: &str, mean_ms: f64, std_dev: f64) -> BenchmarkResult {
        BenchmarkResult {
            name: name.to_string(),
            stats: BenchmarkStats {
                iterations: 10,
                rounds: 1,
                warmup: 0,
                total_runs: 10,
                mean_ms,
                min_ms: mean_ms * 0.9,
                max_ms: mean_ms * 1.1,
                stddev_ms: std_dev,
                median_ms: mean_ms,
                total_ms: mean_ms * 10.0,
                p25_ms: mean_ms * 0.95,
                p75_ms: mean_ms * 1.05,
                p95_ms: mean_ms * 1.08,
                p99_ms: mean_ms * 1.10,
                iqr_ms: mean_ms * 0.1,
                outliers: 0,
                outliers_low: 0,
                outliers_high: 0,
                std_error_ms: std_dev / 10f64.sqrt(),
                ci_lower_ms: mean_ms - 1.96 * std_dev / 10f64.sqrt(),
                ci_upper_ms: mean_ms + 1.96 * std_dev / 10f64.sqrt(),
                all_times_ms: vec![mean_ms; 10],
                adaptive_stopped_early: false,
                adaptive_reason: None,
                adaptive_iterations_used: 10,
            },
            success: true,
            error: None,
        }
    }

    #[test]
    fn test_regression_severity_classification() {
        let baseline = BaselineSnapshot {
            metadata: BaselineMetadata {
                version: "1.0".to_string(),
                timestamp: "2026-01-01T00:00:00Z".to_string(),
                git_metadata: None,
                environment: BenchmarkEnvironment::default(),
            },
            benchmarks: vec![
                create_test_result("test1", 10.0, 1.0),
            ],
        };

        let current_minor = vec![create_test_result("test1", 10.4, 1.0)];  // 4% slower
        let current_moderate = vec![create_test_result("test1", 11.0, 1.0)];  // 10% slower
        let current_severe = vec![create_test_result("test1", 12.0, 1.0)];  // 20% slower

        let thresholds = RegressionThresholds::default();

        let report_minor = RegressionDetector::detect_regressions(&baseline, &current_minor, &thresholds);
        assert_eq!(report_minor.regressions.len(), 0);  // Below 5% threshold

        let report_moderate = RegressionDetector::detect_regressions(&baseline, &current_moderate, &thresholds);
        assert_eq!(report_moderate.regressions.len(), 1);
        assert_eq!(report_moderate.regressions[0].severity, RegressionSeverity::Moderate);

        let report_severe = RegressionDetector::detect_regressions(&baseline, &current_severe, &thresholds);
        assert_eq!(report_severe.regressions.len(), 1);
        assert_eq!(report_severe.regressions[0].severity, RegressionSeverity::Severe);
    }

    #[test]
    fn test_improvement_detection() {
        let baseline = BaselineSnapshot {
            metadata: BaselineMetadata {
                version: "1.0".to_string(),
                timestamp: "2026-01-01T00:00:00Z".to_string(),
                git_metadata: None,
                environment: BenchmarkEnvironment::default(),
            },
            benchmarks: vec![
                create_test_result("test1", 10.0, 1.0),
            ],
        };

        let current = vec![create_test_result("test1", 9.0, 1.0)];  // 10% faster

        let thresholds = RegressionThresholds::default();
        let report = RegressionDetector::detect_regressions(&baseline, &current, &thresholds);

        assert_eq!(report.improvements.len(), 1);
        assert_eq!(report.improvements[0].percent_change, -10.0);
    }

    #[test]
    fn test_baseline_store_default() {
        let store = FileBaselineStore::default_store();
        assert_eq!(store.base_dir, PathBuf::from(".baselines"));
    }
}
