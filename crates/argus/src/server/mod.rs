//! Argus Daemon Server
//!
//! Provides a long-running daemon for code analysis with:
//! - In-memory code index
//! - File watching with incremental updates
//! - JSON-RPC over Unix socket

pub mod daemon;
pub mod handler;
pub mod protocol;

pub use daemon::{ArgusDaemon, DaemonClient, DaemonConfig};
pub use handler::RequestHandler;
pub use protocol::{Request, Response, RpcError, CheckResult, DiagnosticInfo, SymbolInfo, IndexStatus};
