use std::path::PathBuf;
use std::collections::HashSet;

pub use crate::graph::ModuleId;

/// Bundle configuration options
#[derive(Debug, Clone)]
pub struct BundleOptions {
    /// Entry point
    pub entry: PathBuf,

    /// Output directory
    pub output_dir: PathBuf,

    /// Enable source maps
    pub source_maps: bool,

    /// Enable minification
    pub minify: bool,

    /// Module resolution options
    pub resolve_options: ouroboros_talos_resolver::ResolveOptions,

    /// Transform options
    pub transform_options: ouroboros_talos_transform::TransformOptions,

    /// Asset processing options
    pub asset_options: ouroboros_talos_asset::AssetOptions,

    /// Packages to mark as external (not bundled)
    /// Example: HashSet::from(["react", "react-dom"]) to use CDN
    /// Empty set means bundle everything
    pub externals: HashSet<String>,
}

/// Bundle output
#[derive(Debug, Clone)]
pub struct BundleOutput {
    /// Bundled JavaScript code
    pub code: String,

    /// Source map (if enabled)
    pub source_map: Option<String>,

    /// Generated assets
    pub assets: Vec<Asset>,
}

/// Asset output
#[derive(Debug, Clone)]
pub struct Asset {
    /// Asset file name
    pub filename: String,

    /// Asset content
    pub content: Vec<u8>,

    /// Asset type
    pub asset_type: AssetType,
}

/// Type of asset
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetType {
    /// CSS file
    Css,

    /// Image
    Image,

    /// Font
    Font,

    /// Other
    Other,
}

impl Default for BundleOptions {
    fn default() -> Self {
        Self {
            entry: PathBuf::from("src/index.js"),
            output_dir: PathBuf::from("dist"),
            source_maps: true,
            minify: false,
            resolve_options: Default::default(),
            transform_options: Default::default(),
            asset_options: Default::default(),
            externals: HashSet::new(), // Empty = bundle everything
        }
    }
}
