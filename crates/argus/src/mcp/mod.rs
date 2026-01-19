//! MCP (Model Context Protocol) Server for Argus
//!
//! Provides AI-friendly tools for code analysis via the MCP protocol.
//! The MCP server connects to the Argus daemon for fast cached analysis.

pub mod server;
pub mod tools;

pub use server::McpServer;
pub use tools::ArgusTools;
