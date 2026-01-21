use anyhow::Result;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use tokio::sync::broadcast;

/// File watcher for detecting changes
pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    tx: broadcast::Sender<PathBuf>,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new(root_dir: PathBuf) -> Result<Self> {
        let (tx, _) = broadcast::channel(100);
        let tx_clone = tx.clone();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                for path in event.paths {
                    // Skip node_modules and other common excludes
                    if should_ignore(&path) {
                        continue;
                    }

                    let _ = tx_clone.send(path);
                }
            }
        })?;

        watcher.watch(&root_dir, RecursiveMode::Recursive)?;

        Ok(Self {
            _watcher: watcher,
            tx,
        })
    }

    /// Subscribe to file change events
    pub fn subscribe(&self) -> broadcast::Receiver<PathBuf> {
        self.tx.subscribe()
    }
}

/// Check if path should be ignored
fn should_ignore(path: &PathBuf) -> bool {
    let path_str = path.to_string_lossy();

    // Common directories to ignore
    const IGNORE_PATTERNS: &[&str] = &[
        "node_modules",
        ".git",
        "dist",
        "build",
        ".talos-cache",
        "target",
        ".DS_Store",
    ];

    IGNORE_PATTERNS
        .iter()
        .any(|pattern| path_str.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_ignore() {
        assert!(should_ignore(&PathBuf::from("node_modules/react/index.js")));
        assert!(should_ignore(&PathBuf::from(".git/config")));
        assert!(!should_ignore(&PathBuf::from("src/App.tsx")));
    }
}
