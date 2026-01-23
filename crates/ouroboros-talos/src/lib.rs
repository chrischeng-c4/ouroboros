// Ouroboros-Talos
//
// This crate provides the Talos build tool functionality.
// All commands are accessed through the unified CLI: `ob talos <command>`
//
// See: ouroboros-cli for the CLI integration

// Re-export main components
pub use ouroboros_talos_bundler as bundler;
pub use ouroboros_talos_dev_server as dev_server;
pub use ouroboros_talos_pkg_manager as pkg_manager;
pub use ouroboros_talos_resolver as resolver;
pub use ouroboros_talos_transform as transform;
pub use ouroboros_talos_asset as asset;
