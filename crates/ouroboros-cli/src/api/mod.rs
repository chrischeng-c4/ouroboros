//! ob api CLI commands
//!
//! Provides commands for managing ouroboros API projects:
//! - `ob api init` - Initialize a new project
//! - `ob api app` - Manage apps
//! - `ob api core` - Manage core modules
//! - `ob api feat` - Manage feature modules
//! - `ob api g` - Code generation (legacy, for backward compatibility)
//! - `ob api serve` - Start API server

pub mod config;
pub mod fields;
pub mod codegen;
pub mod init;
pub mod app;
pub mod core;
pub mod feat;
pub mod generate;
pub mod serve;
pub mod test;

use clap::Subcommand;

pub use config::DbType;

/// API project management commands
#[derive(Debug, Subcommand)]
pub enum ApiAction {
    /// Initialize a new ouroboros API project
    Init(init::InitArgs),

    /// Manage apps (API entry points)
    #[command(subcommand)]
    App(AppAction),

    /// Manage core modules (shared, widely-depended modules)
    #[command(subcommand)]
    Core(CoreAction),

    /// Manage feature modules (business domain modules)
    #[command(subcommand)]
    Feat(FeatAction),

    /// Code generation (shorthand for common operations)
    #[command(subcommand)]
    G(GenerateAction),

    /// Start API server (supports both dev and production modes)
    Serve(serve::ServeArgs),
}

/// App management commands
#[derive(Debug, Subcommand)]
pub enum AppAction {
    /// Create a new app
    Create(app::CreateArgs),

    /// List all apps
    List,
}

/// Core module management commands
#[derive(Debug, Subcommand)]
pub enum CoreAction {
    /// Create a new core module
    Create(core::CreateArgs),

    /// Add a model to a core module
    Model(core::ModelArgs),

    /// Add a service to a core module
    Service(core::ServiceArgs),

    /// Initialize routes for a core module
    Route(core::RouteArgs),

    /// Add an endpoint to a core module's routes
    Endpoint(core::EndpointArgs),

    /// Add a schema to a core module
    Schema(core::SchemaArgs),

    /// List all core modules
    List,
}

/// Feature module management commands
#[derive(Debug, Subcommand)]
pub enum FeatAction {
    /// Create a new feature module
    Create(feat::CreateArgs),

    /// Add a model to a feature module
    Model(feat::ModelArgs),

    /// Add a service to a feature module
    Service(feat::ServiceArgs),

    /// Initialize routes for a feature module
    Route(feat::RouteArgs),

    /// Add an endpoint to a feature module's routes
    Endpoint(feat::EndpointArgs),

    /// Add a schema to a feature module
    Schema(feat::SchemaArgs),

    /// List all feature modules
    List,
}

/// Code generation commands (shorthand)
#[derive(Debug, Subcommand)]
pub enum GenerateAction {
    /// Generate an app (alias for `ob api app create`)
    App {
        /// Name of the app to generate
        name: String,
        /// Port for the app (optional)
        #[arg(long)]
        port: Option<u16>,
    },

    /// Generate a feature module (alias for `ob api feat create`)
    Feature {
        /// Name of the feature to generate
        name: String,
        /// Database type override
        #[arg(long)]
        db: Option<DbType>,
    },

    /// Generate a route for a module
    Route {
        /// Name of the module
        module: String,
        /// Target app name
        #[arg(long)]
        app: String,
        /// Whether this is a core module (default: feature)
        #[arg(long)]
        core: bool,
    },

    /// Generate tests for a module using ouroboros-qc
    Test(test::TestArgs),
}

/// Execute an API action
pub async fn execute(action: ApiAction) -> anyhow::Result<()> {
    match action {
        ApiAction::Init(args) => init::execute(args).await,
        ApiAction::App(action) => app::execute(action).await,
        ApiAction::Core(action) => core::execute(action).await,
        ApiAction::Feat(action) => feat::execute(action).await,
        ApiAction::G(action) => generate::execute(action).await,
        ApiAction::Serve(args) => serve::execute(args),
    }
}
