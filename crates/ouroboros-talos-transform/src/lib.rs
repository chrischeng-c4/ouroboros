use anyhow::Result;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

pub mod jsx;
pub mod typescript;
pub mod transform_tsx;
pub mod css;
pub mod incremental;
pub mod modules;

/// Code transformer using SWC
pub struct Transformer {
    options: TransformOptions,
}

/// Transform options
#[derive(Debug, Clone)]
pub struct TransformOptions {
    /// JSX pragma (default: React.createElement)
    pub jsx_pragma: Option<String>,

    /// JSX fragment pragma (default: React.Fragment)
    pub jsx_fragment: Option<String>,

    /// Enable JSX automatic runtime
    pub jsx_automatic: bool,

    /// TypeScript target
    pub ts_target: TypeScriptTarget,

    /// Enable source maps
    pub source_maps: bool,

    /// Enable minification
    pub minify: bool,
}

/// TypeScript compilation target
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeScriptTarget {
    ES5,
    ES2015,
    ES2016,
    ES2017,
    ES2018,
    ES2019,
    ES2020,
    ES2021,
    ES2022,
    ESNext,
}

/// Transform result
#[derive(Debug, Clone)]
pub struct TransformResult {
    /// Transformed code
    pub code: String,

    /// Source map (if enabled)
    pub source_map: Option<String>,
}

impl Transformer {
    /// Create a new transformer
    pub fn new(options: TransformOptions) -> Self {
        Self { options }
    }

    /// Transform JavaScript/TypeScript file
    pub fn transform_js(&self, source: &str, filename: &Path) -> Result<TransformResult> {
        let ext = filename.extension().and_then(|e| e.to_str()).unwrap_or("");

        match ext {
            "jsx" => jsx::transform_jsx(source, &self.options),
            "tsx" => {
                // ✅ Single-pass transformation: removes types + transforms JSX
                transform_tsx::transform_tsx(source, &self.options)
            }
            "ts" => typescript::transform_typescript(source, &self.options),
            "js" | "mjs" | "cjs" => {
                // Plain JavaScript, just parse and emit
                Ok(TransformResult {
                    code: source.to_string(),
                    source_map: None,
                })
            }
            _ => anyhow::bail!("Unsupported file extension: {}", ext),
        }
    }

    /// Transform JavaScript/TypeScript file with module context
    /// This applies ES6 module transformation after JSX/TypeScript transformation
    pub fn transform_js_with_context(
        &self,
        source: &str,
        filename: &Path,
        module_map: &HashMap<PathBuf, usize>,
    ) -> Result<TransformResult> {
        let ext = filename.extension().and_then(|e| e.to_str()).unwrap_or("");

        // 1. First, apply TypeScript/JSX transformation
        let transformed = match ext {
            "jsx" => jsx::transform_jsx(source, &self.options)?,
            "tsx" => {
                // ✅ Single-pass transformation: removes types + transforms JSX
                transform_tsx::transform_tsx(source, &self.options)?
            }
            "ts" => typescript::transform_typescript(source, &self.options)?,
            "js" | "mjs" | "cjs" => {
                // Check if it needs transformation (has ES6 modules or is CommonJS)
                if source.contains("import ") || source.contains("export ") {
                    // Has ES6 modules, will be transformed in step 2
                    TransformResult {
                        code: source.to_string(),
                        source_map: None,
                    }
                } else if source.contains("module.exports") || source.contains("require(") {
                    // Already CommonJS, no transformation needed
                    return Ok(TransformResult {
                        code: source.to_string(),
                        source_map: None,
                    });
                } else {
                    // Plain JavaScript
                    TransformResult {
                        code: source.to_string(),
                        source_map: None,
                    }
                }
            }
            _ => anyhow::bail!("Unsupported file extension: {}", ext),
        };

        // 2. Apply ES6 module transformation
        modules::transform_modules(&transformed.code, module_map)
    }

    /// Transform CSS file
    pub fn transform_css(&self, source: &str) -> Result<TransformResult> {
        css::transform_css(source, &self.options)
    }
}

impl Default for TransformOptions {
    fn default() -> Self {
        Self {
            jsx_pragma: None,
            jsx_fragment: None,
            jsx_automatic: true, // Use React 17+ automatic runtime
            ts_target: TypeScriptTarget::ES2020,
            source_maps: true,
            minify: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transformer_creation() {
        let transformer = Transformer::new(TransformOptions::default());
        assert!(transformer.options.jsx_automatic);
    }
}
