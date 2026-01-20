//! Signal handling for the event loop
//!
//! Provides asyncio-compatible signal handling with `add_signal_handler`
//! and `remove_signal_handler` APIs.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};

// ============================================================================
// Signal Types
// ============================================================================

/// Common Unix signals
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SignalType {
    /// Interrupt signal (Ctrl+C)
    Int,
    /// Terminate signal
    Term,
    /// Hangup signal
    Hup,
    /// User-defined signal 1
    Usr1,
    /// User-defined signal 2
    Usr2,
    /// Child process status changed
    Chld,
    /// Alarm signal
    Alrm,
    /// I/O possible signal
    Io,
    /// Window size changed
    Winch,
}

impl SignalType {
    /// Get the signal number
    #[cfg(unix)]
    pub fn as_raw(&self) -> i32 {
        match self {
            SignalType::Int => libc::SIGINT,
            SignalType::Term => libc::SIGTERM,
            SignalType::Hup => libc::SIGHUP,
            SignalType::Usr1 => libc::SIGUSR1,
            SignalType::Usr2 => libc::SIGUSR2,
            SignalType::Chld => libc::SIGCHLD,
            SignalType::Alrm => libc::SIGALRM,
            SignalType::Io => libc::SIGIO,
            SignalType::Winch => libc::SIGWINCH,
        }
    }

    /// Get the signal name
    pub fn name(&self) -> &'static str {
        match self {
            SignalType::Int => "SIGINT",
            SignalType::Term => "SIGTERM",
            SignalType::Hup => "SIGHUP",
            SignalType::Usr1 => "SIGUSR1",
            SignalType::Usr2 => "SIGUSR2",
            SignalType::Chld => "SIGCHLD",
            SignalType::Alrm => "SIGALRM",
            SignalType::Io => "SIGIO",
            SignalType::Winch => "SIGWINCH",
        }
    }

    /// Convert from signal number
    #[cfg(unix)]
    pub fn from_raw(signum: i32) -> Option<Self> {
        match signum {
            x if x == libc::SIGINT => Some(SignalType::Int),
            x if x == libc::SIGTERM => Some(SignalType::Term),
            x if x == libc::SIGHUP => Some(SignalType::Hup),
            x if x == libc::SIGUSR1 => Some(SignalType::Usr1),
            x if x == libc::SIGUSR2 => Some(SignalType::Usr2),
            x if x == libc::SIGCHLD => Some(SignalType::Chld),
            x if x == libc::SIGALRM => Some(SignalType::Alrm),
            x if x == libc::SIGIO => Some(SignalType::Io),
            x if x == libc::SIGWINCH => Some(SignalType::Winch),
            _ => None,
        }
    }

    #[cfg(unix)]
    fn to_signal_kind(self) -> SignalKind {
        match self {
            SignalType::Int => SignalKind::interrupt(),
            SignalType::Term => SignalKind::terminate(),
            SignalType::Hup => SignalKind::hangup(),
            SignalType::Usr1 => SignalKind::user_defined1(),
            SignalType::Usr2 => SignalKind::user_defined2(),
            SignalType::Chld => SignalKind::child(),
            SignalType::Alrm => SignalKind::alarm(),
            SignalType::Io => SignalKind::io(),
            SignalType::Winch => SignalKind::window_change(),
        }
    }
}

// ============================================================================
// Signal Handler Callback
// ============================================================================

/// Signal handler callback type
pub type SignalCallback = Box<dyn Fn() + Send + Sync + 'static>;

// ============================================================================
// Signal Handler
// ============================================================================

/// Signal handler entry
struct SignalEntry {
    callbacks: Vec<SignalCallback>,
    #[cfg(unix)]
    cancel_tx: Option<mpsc::Sender<()>>,
}

/// Signal handler manager
///
/// Manages signal handlers in an asyncio-compatible way.
pub struct SignalHandler {
    handlers: Arc<Mutex<HashMap<SignalType, SignalEntry>>>,
}

impl SignalHandler {
    /// Create a new signal handler manager
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add a signal handler callback
    ///
    /// # Arguments
    /// * `sig` - The signal type to handle
    /// * `callback` - The callback to invoke when signal is received
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err` if signal registration fails
    #[cfg(unix)]
    pub async fn add_signal_handler(
        &self,
        sig: SignalType,
        callback: impl Fn() + Send + Sync + 'static,
    ) -> std::io::Result<()> {
        let mut handlers = self.handlers.lock().await;

        // Check if we need to start a new listener
        let entry = handlers.entry(sig).or_insert_with(|| SignalEntry {
            callbacks: Vec::new(),
            cancel_tx: None,
        });

        entry.callbacks.push(Box::new(callback));

        // Start listener if this is the first handler
        if entry.cancel_tx.is_none() {
            let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);
            entry.cancel_tx = Some(cancel_tx);

            let handlers_clone = Arc::clone(&self.handlers);
            let sig_kind = sig.to_signal_kind();

            // Create signal listener
            let mut stream = signal(sig_kind)?;

            // Spawn task to handle signals
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        _ = stream.recv() => {
                            let handlers = handlers_clone.lock().await;
                            if let Some(entry) = handlers.get(&sig) {
                                for callback in &entry.callbacks {
                                    callback();
                                }
                            }
                        }
                        _ = cancel_rx.recv() => {
                            break;
                        }
                    }
                }
            });
        }

        Ok(())
    }

    /// Remove all signal handlers for a signal
    ///
    /// Returns true if handlers were removed, false if none existed.
    #[cfg(unix)]
    pub async fn remove_signal_handler(&self, sig: SignalType) -> bool {
        let mut handlers = self.handlers.lock().await;

        if let Some(entry) = handlers.remove(&sig) {
            // Cancel the listener task
            if let Some(cancel_tx) = entry.cancel_tx {
                let _ = cancel_tx.send(()).await;
            }
            true
        } else {
            false
        }
    }

    /// Check if a signal has handlers
    pub async fn has_handlers(&self, sig: SignalType) -> bool {
        let handlers = self.handlers.lock().await;
        handlers
            .get(&sig)
            .map(|e| !e.callbacks.is_empty())
            .unwrap_or(false)
    }

    /// Get the number of handlers for a signal
    pub async fn handler_count(&self, sig: SignalType) -> usize {
        let handlers = self.handlers.lock().await;
        handlers.get(&sig).map(|e| e.callbacks.len()).unwrap_or(0)
    }

    /// Remove all signal handlers
    #[cfg(unix)]
    pub async fn clear(&self) {
        let mut handlers = self.handlers.lock().await;
        for (_, entry) in handlers.drain() {
            if let Some(cancel_tx) = entry.cancel_tx {
                let _ = cancel_tx.send(()).await;
            }
        }
    }
}

impl Default for SignalHandler {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Wait for a signal (one-shot)
#[cfg(unix)]
pub async fn wait_for_signal(sig: SignalType) -> std::io::Result<()> {
    let mut stream = signal(sig.to_signal_kind())?;
    stream.recv().await;
    Ok(())
}

/// Wait for SIGINT (Ctrl+C)
pub async fn ctrl_c() -> std::io::Result<()> {
    tokio::signal::ctrl_c().await
}

/// Wait for termination signal (SIGTERM on Unix)
#[cfg(unix)]
pub async fn terminate() -> std::io::Result<()> {
    wait_for_signal(SignalType::Term).await
}

/// Wait for either SIGINT or SIGTERM
#[cfg(unix)]
pub async fn shutdown_signal() -> std::io::Result<SignalType> {
    let ctrl_c = ctrl_c();
    let terminate = wait_for_signal(SignalType::Term);

    tokio::select! {
        result = ctrl_c => {
            result?;
            Ok(SignalType::Int)
        }
        result = terminate => {
            result?;
            Ok(SignalType::Term)
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_signal_type_name() {
        assert_eq!(SignalType::Int.name(), "SIGINT");
        assert_eq!(SignalType::Term.name(), "SIGTERM");
        assert_eq!(SignalType::Hup.name(), "SIGHUP");
    }

    #[cfg(unix)]
    #[test]
    fn test_signal_type_raw() {
        assert_eq!(SignalType::Int.as_raw(), libc::SIGINT);
        assert_eq!(SignalType::Term.as_raw(), libc::SIGTERM);
    }

    #[cfg(unix)]
    #[test]
    fn test_signal_type_from_raw() {
        assert_eq!(SignalType::from_raw(libc::SIGINT), Some(SignalType::Int));
        assert_eq!(SignalType::from_raw(libc::SIGTERM), Some(SignalType::Term));
        assert_eq!(SignalType::from_raw(999), None);
    }

    #[tokio::test]
    async fn test_signal_handler_new() {
        let handler = SignalHandler::new();
        assert!(!handler.has_handlers(SignalType::Int).await);
        assert_eq!(handler.handler_count(SignalType::Int).await, 0);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_add_and_count_handlers() {
        let handler = SignalHandler::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = Arc::clone(&counter);
        handler
            .add_signal_handler(SignalType::Usr1, move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            })
            .await
            .unwrap();

        assert!(handler.has_handlers(SignalType::Usr1).await);
        assert_eq!(handler.handler_count(SignalType::Usr1).await, 1);

        // Add another handler
        let counter_clone = Arc::clone(&counter);
        handler
            .add_signal_handler(SignalType::Usr1, move || {
                counter_clone.fetch_add(10, Ordering::SeqCst);
            })
            .await
            .unwrap();

        assert_eq!(handler.handler_count(SignalType::Usr1).await, 2);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_remove_signal_handler() {
        let handler = SignalHandler::new();

        handler
            .add_signal_handler(SignalType::Usr2, || {})
            .await
            .unwrap();
        assert!(handler.has_handlers(SignalType::Usr2).await);

        let removed = handler.remove_signal_handler(SignalType::Usr2).await;
        assert!(removed);
        assert!(!handler.has_handlers(SignalType::Usr2).await);

        // Try removing again
        let removed = handler.remove_signal_handler(SignalType::Usr2).await;
        assert!(!removed);
    }
}
