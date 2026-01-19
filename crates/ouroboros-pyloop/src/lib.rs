//! ouroboros-pyloop: Rust-native Python asyncio event loop
//!
//! Provides a high-performance event loop implementation for Python's asyncio,
//! backed by Tokio runtime with native Rust integration.
//!
//! # Architecture
//!
//! This crate bridges Python's asyncio event loop protocol with Tokio's runtime,
//! allowing Python coroutines to run on a Rust-backed event loop for improved
//! performance and better integration with Rust async code.
//!
//! # Usage
//!
//! ```python
//! import ouroboros.pyloop
//!
//! # Install as default event loop
//! ouroboros.pyloop.install()
//!
//! # Now all asyncio code uses Tokio-backed loop
//! import asyncio
//!
//! async def main():
//!     await asyncio.sleep(1)
//!     print("Running on Tokio!")
//!
//! asyncio.run(main())
//! ```

use std::sync::Arc;
use tokio::runtime::Runtime;

mod error;
mod loop_impl;
mod future;
mod handle;
mod network;
mod subprocess;
mod task;
mod timer_wheel;
pub mod file_io;
pub mod signal;
pub mod executor;
#[cfg(unix)]
pub mod unix_socket;

pub use error::PyLoopError;
pub use loop_impl::PyLoop;
pub use future::PyFuture;
pub use handle::{Handle, TimerHandle};
pub use network::{
    TcpTransport, TcpServer, StreamReader, StreamWriter,
    create_connection, create_connection_with_timeout,
    create_server, open_connection, open_connection_with_timeout,
};
pub use subprocess::{
    Process, ProcessConfig, ProcessOutput,
    create_subprocess_exec, create_subprocess_exec_with_config,
    create_subprocess_shell, create_subprocess_shell_with_config,
    run, run_with_input, run_shell, run_with_timeout,
};
pub use task::{PyCancelledError, Task};
pub use timer_wheel::{TimerWheel, TimerEntry, ScheduledCallback};

// File I/O re-exports
pub use file_io::{AsyncFile, FileBuilder, read_file, write_file, append_file, copy_file, remove_file};

// Signal handling re-exports
pub use signal::{SignalType, SignalHandler, ctrl_c};

// Executor re-exports
pub use executor::{ThreadPoolExecutor, run_in_executor, spawn_blocking, set_default_executor};

// Unix socket re-exports
#[cfg(unix)]
pub use unix_socket::{
    UnixTransport, UnixServer, UnixStreamReader, UnixStreamWriter,
    create_unix_connection, create_unix_connection_with_timeout,
    create_unix_server, create_unix_server_with_permissions,
    open_unix_connection,
};

/// Initialize the Tokio runtime for use with PyLoop.
///
/// This should be called once at module initialization to set up
/// the global Tokio runtime that will be shared across all PyLoop instances.
fn init_runtime() -> Result<Arc<Runtime>, PyLoopError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|e| PyLoopError::RuntimeInit(e.to_string()))?;

    Ok(Arc::new(runtime))
}

/// Get or create the global Tokio runtime.
///
/// Uses once_cell to ensure the runtime is initialized exactly once
/// and shared across all PyLoop instances.
pub fn get_runtime() -> Result<Arc<Runtime>, PyLoopError> {
    use once_cell::sync::Lazy;

    static RUNTIME: Lazy<Result<Arc<Runtime>, PyLoopError>> = Lazy::new(init_runtime);

    match &*RUNTIME {
        Ok(rt) => Ok(Arc::clone(rt)),
        Err(e) => Err(PyLoopError::RuntimeInit(format!("Failed to initialize runtime: {:?}", e))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_initialization() {
        let runtime = get_runtime();
        assert!(runtime.is_ok(), "Runtime should initialize successfully");
    }

    #[test]
    fn test_runtime_is_singleton() {
        let rt1 = get_runtime().unwrap();
        let rt2 = get_runtime().unwrap();

        // Check that both references point to the same runtime
        assert!(Arc::ptr_eq(&rt1, &rt2), "Runtime should be a singleton");
    }
}
