//! Static file serving
//!
//! Provides functionality to serve static files from directories,
//! with support for MIME type detection, caching, and SPA fallback.

use crate::error::{ApiError, ApiResult};
use crate::request::Request;
use crate::response::Response;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncReadExt;

// ============================================================================
// Static File Configuration
// ============================================================================

/// Configuration for static file serving
#[derive(Debug, Clone)]
pub struct StaticFilesConfig {
    /// Directory to serve files from
    pub directory: PathBuf,
    /// URL prefix (e.g., "/static")
    pub prefix: String,
    /// Index file for directory requests
    pub index_file: Option<String>,
    /// Enable SPA mode (fallback to index for 404s)
    pub spa_mode: bool,
    /// Enable ETag header
    pub etag: bool,
    /// Cache-Control max-age in seconds
    pub max_age: Option<u32>,
    /// Custom MIME type mappings
    pub mime_types: HashMap<String, String>,
}

impl Default for StaticFilesConfig {
    fn default() -> Self {
        Self {
            directory: PathBuf::from("static"),
            prefix: "/static".to_string(),
            index_file: Some("index.html".to_string()),
            spa_mode: false,
            etag: true,
            max_age: Some(3600),
            mime_types: HashMap::new(),
        }
    }
}

impl StaticFilesConfig {
    /// Create a new static files configuration
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
            ..Default::default()
        }
    }

    /// Set the URL prefix
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Set the index file
    pub fn index_file(mut self, file: impl Into<String>) -> Self {
        self.index_file = Some(file.into());
        self
    }

    /// Enable SPA mode (serves index.html for 404s)
    pub fn spa(mut self, enable: bool) -> Self {
        self.spa_mode = enable;
        self
    }

    /// Set cache max age
    pub fn max_age(mut self, seconds: u32) -> Self {
        self.max_age = Some(seconds);
        self
    }

    /// Disable caching
    pub fn no_cache(mut self) -> Self {
        self.max_age = None;
        self.etag = false;
        self
    }

    /// Add custom MIME type mapping
    pub fn mime_type(mut self, extension: impl Into<String>, mime: impl Into<String>) -> Self {
        self.mime_types.insert(extension.into(), mime.into());
        self
    }
}

// ============================================================================
// Static File Handler
// ============================================================================

/// Static file handler
pub struct StaticFiles {
    config: StaticFilesConfig,
}

impl StaticFiles {
    /// Create a new static file handler
    pub fn new(config: StaticFilesConfig) -> Self {
        Self { config }
    }

    /// Serve a request for a static file
    pub async fn serve(&self, req: &Request) -> ApiResult<Response> {
        let path = req.path();

        // Check if path starts with prefix
        let relative_path = if let Some(stripped) = path.strip_prefix(&self.config.prefix) {
            stripped.trim_start_matches('/')
        } else {
            return Err(ApiError::NotFound("Path not found".into()));
        };

        // Resolve the file path
        let file_path = self.resolve_path(relative_path)?;

        // Try to serve the file
        match self.serve_file(&file_path, req).await {
            Ok(response) => Ok(response),
            Err(e) => {
                if self.config.spa_mode {
                    // In SPA mode, try to serve index.html
                    if let Some(ref index) = self.config.index_file {
                        let index_path = self.config.directory.join(index);
                        return self.serve_file(&index_path, req).await;
                    }
                }
                Err(e)
            }
        }
    }

    /// Resolve a relative path to an absolute file path
    fn resolve_path(&self, relative: &str) -> ApiResult<PathBuf> {
        // Prevent path traversal attacks
        let normalized = PathBuf::from(relative);

        // Check for path traversal
        for component in normalized.components() {
            if let std::path::Component::ParentDir = component {
                return Err(ApiError::BadRequest("Invalid path".into()));
            }
        }

        let full_path = self.config.directory.join(&normalized);

        // Ensure the resolved path is within the directory
        let canonical_dir = self.config.directory.canonicalize().unwrap_or_default();
        if let Ok(canonical_path) = full_path.canonicalize() {
            if !canonical_path.starts_with(&canonical_dir) {
                return Err(ApiError::BadRequest("Invalid path".into()));
            }
        }

        Ok(full_path)
    }

    /// Serve a specific file
    async fn serve_file(&self, path: &Path, req: &Request) -> ApiResult<Response> {
        // Resolve path, handling directory -> index file redirection
        let mut current_path = path.to_path_buf();

        loop {
            // Check if file exists
            let metadata = fs::metadata(&current_path)
                .await
                .map_err(|_| ApiError::NotFound("File not found".into()))?;

            // Handle directory requests
            if metadata.is_dir() {
                if let Some(ref index) = self.config.index_file {
                    current_path = current_path.join(index);
                    continue;  // Loop to check the index file
                }
                return Err(ApiError::NotFound("Directory listing not allowed".into()));
            }

            // Get file size and continue with serving the file
            return self.serve_regular_file(&current_path, &metadata, req).await;
        }
    }

    /// Serve a regular file (non-directory)
    async fn serve_regular_file(
        &self,
        path: &Path,
        metadata: &std::fs::Metadata,
        req: &Request,
    ) -> ApiResult<Response> {
        // Get file size
        let file_size = metadata.len();

        // Calculate ETag
        let etag = if self.config.etag {
            Some(self.calculate_etag(path, &metadata))
        } else {
            None
        };

        // Check If-None-Match
        if let Some(ref tag) = etag {
            if let Some(client_etag) = req.header("if-none-match") {
                if client_etag.trim_matches('"') == tag.trim_matches('"') {
                    let mut response = Response::new().status(304);
                    response.set_header("ETag", tag);
                    return Ok(response);
                }
            }
        }

        // Parse Range header for partial content
        let range = self.parse_range_header(req.header("range"), file_size);

        // Read file content
        let (content, start, end) = match range {
            Some((start, end)) => {
                let content = self.read_file_range(path, start, end).await?;
                (content, start, end)
            }
            None => {
                let content = self.read_file(path).await?;
                (content, 0, file_size.saturating_sub(1))
            }
        };

        // Build response
        let status = if range.is_some() { 206 } else { 200 };
        let mut response = Response::new().status(status).with_bytes(content, "application/octet-stream");

        // Set Content-Type
        let mime_type = self.get_mime_type(path);
        response.set_header("Content-Type", &mime_type);

        // Set Content-Length
        response.set_header("Content-Length", &(end - start + 1).to_string());

        // Set Content-Range for partial content
        if range.is_some() {
            response.set_header(
                "Content-Range",
                &format!("bytes {}-{}/{}", start, end, file_size),
            );
        }

        // Set Accept-Ranges
        response.set_header("Accept-Ranges", "bytes");

        // Set ETag
        if let Some(tag) = etag {
            response.set_header("ETag", &tag);
        }

        // Set Cache-Control
        if let Some(max_age) = self.config.max_age {
            response.set_header("Cache-Control", &format!("max-age={}", max_age));
        }

        Ok(response)
    }

    /// Read entire file
    async fn read_file(&self, path: &Path) -> ApiResult<Vec<u8>> {
        fs::read(path)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to read file: {}", e)))
    }

    /// Read a range of bytes from file
    async fn read_file_range(&self, path: &Path, start: u64, end: u64) -> ApiResult<Vec<u8>> {
        let mut file = fs::File::open(path)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to open file: {}", e)))?;

        use tokio::io::AsyncSeekExt;
        file.seek(std::io::SeekFrom::Start(start))
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to seek: {}", e)))?;

        let len = (end - start + 1) as usize;
        let mut buffer = vec![0u8; len];
        file.read_exact(&mut buffer)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to read: {}", e)))?;

        Ok(buffer)
    }

    /// Calculate ETag for a file
    fn calculate_etag(&self, path: &Path, metadata: &std::fs::Metadata) -> String {
        use std::time::UNIX_EPOCH;

        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let size = metadata.len();
        let name = path.file_name().unwrap_or_default().to_string_lossy();

        format!("\"{:x}-{:x}-{}\"", modified, size, name.len())
    }

    /// Parse Range header
    fn parse_range_header(&self, header: Option<&str>, file_size: u64) -> Option<(u64, u64)> {
        let header = header?;
        let range_str = header.strip_prefix("bytes=")?;

        let parts: Vec<&str> = range_str.split('-').collect();
        if parts.len() != 2 {
            return None;
        }

        let start: u64 = if parts[0].is_empty() {
            // Suffix range: -500 means last 500 bytes
            let suffix: u64 = parts[1].parse().ok()?;
            file_size.saturating_sub(suffix)
        } else {
            parts[0].parse().ok()?
        };

        let end: u64 = if parts[1].is_empty() {
            file_size - 1
        } else {
            parts[1].parse().ok()?
        };

        if start > end || start >= file_size {
            return None;
        }

        Some((start, end.min(file_size - 1)))
    }

    /// Get MIME type for a file
    fn get_mime_type(&self, path: &Path) -> String {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Check custom mappings first
        if let Some(mime) = self.config.mime_types.get(&extension) {
            return mime.clone();
        }

        // Default MIME types
        match extension.as_str() {
            // Text
            "html" | "htm" => "text/html; charset=utf-8",
            "css" => "text/css; charset=utf-8",
            "js" | "mjs" => "application/javascript; charset=utf-8",
            "json" => "application/json; charset=utf-8",
            "xml" => "application/xml; charset=utf-8",
            "txt" => "text/plain; charset=utf-8",
            "md" => "text/markdown; charset=utf-8",
            "csv" => "text/csv; charset=utf-8",

            // Images
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "svg" => "image/svg+xml",
            "webp" => "image/webp",
            "ico" => "image/x-icon",
            "bmp" => "image/bmp",
            "avif" => "image/avif",

            // Fonts
            "woff" => "font/woff",
            "woff2" => "font/woff2",
            "ttf" => "font/ttf",
            "otf" => "font/otf",
            "eot" => "application/vnd.ms-fontobject",

            // Media
            "mp3" => "audio/mpeg",
            "mp4" => "video/mp4",
            "webm" => "video/webm",
            "ogg" => "audio/ogg",
            "wav" => "audio/wav",
            "avi" => "video/x-msvideo",
            "mov" => "video/quicktime",

            // Archives
            "zip" => "application/zip",
            "tar" => "application/x-tar",
            "gz" => "application/gzip",
            "7z" => "application/x-7z-compressed",

            // Documents
            "pdf" => "application/pdf",
            "doc" => "application/msword",
            "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "xls" => "application/vnd.ms-excel",
            "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",

            // WebAssembly
            "wasm" => "application/wasm",

            // Source maps
            "map" => "application/json",

            // Default
            _ => "application/octet-stream",
        }
        .to_string()
    }
}


// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = StaticFilesConfig::new("./public")
            .prefix("/assets")
            .index_file("index.html")
            .spa(true)
            .max_age(7200)
            .mime_type("ts", "application/typescript");

        assert_eq!(config.directory, PathBuf::from("./public"));
        assert_eq!(config.prefix, "/assets");
        assert_eq!(config.index_file, Some("index.html".to_string()));
        assert!(config.spa_mode);
        assert_eq!(config.max_age, Some(7200));
        assert_eq!(
            config.mime_types.get("ts"),
            Some(&"application/typescript".to_string())
        );
    }

    #[test]
    fn test_mime_type_detection() {
        let config = StaticFilesConfig::new("./static");
        let handler = StaticFiles::new(config);

        assert_eq!(
            handler.get_mime_type(Path::new("file.html")),
            "text/html; charset=utf-8"
        );
        assert_eq!(handler.get_mime_type(Path::new("image.png")), "image/png");
        assert_eq!(
            handler.get_mime_type(Path::new("script.js")),
            "application/javascript; charset=utf-8"
        );
        assert_eq!(
            handler.get_mime_type(Path::new("unknown.xyz")),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_range_parsing() {
        let config = StaticFilesConfig::new("./static");
        let handler = StaticFiles::new(config);
        let file_size = 1000;

        assert_eq!(
            handler.parse_range_header(Some("bytes=0-499"), file_size),
            Some((0, 499))
        );
        assert_eq!(
            handler.parse_range_header(Some("bytes=500-999"), file_size),
            Some((500, 999))
        );
        assert_eq!(
            handler.parse_range_header(Some("bytes=500-"), file_size),
            Some((500, 999))
        );
        assert_eq!(
            handler.parse_range_header(Some("bytes=-500"), file_size),
            Some((500, 999))
        );
        assert_eq!(handler.parse_range_header(None, file_size), None);
        assert_eq!(
            handler.parse_range_header(Some("invalid"), file_size),
            None
        );
    }

    #[test]
    fn test_etag_generation() {
        let config = StaticFilesConfig::new("./static");
        let handler = StaticFiles::new(config);

        // Create temp file for testing
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_etag.txt");
        std::fs::write(&temp_file, "test content").unwrap();

        let metadata = std::fs::metadata(&temp_file).unwrap();
        let etag = handler.calculate_etag(&temp_file, &metadata);

        assert!(etag.starts_with('"'));
        assert!(etag.ends_with('"'));

        // Cleanup
        std::fs::remove_file(temp_file).ok();
    }
}
