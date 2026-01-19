//! Streaming file upload support
//!
//! Provides memory-efficient streaming upload handling for large files.
//! Supports progress tracking, size limits, and direct-to-storage uploads.

use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};
use bytes::Bytes;
use futures_util::Stream;
use tokio::io::{AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc;
use crate::error::{ApiError, ApiResult};

// ============================================================================
// Upload Configuration
// ============================================================================

/// Upload configuration
#[derive(Debug, Clone)]
pub struct UploadConfig {
    /// Maximum file size (bytes), None for unlimited
    pub max_size: Option<u64>,
    /// Maximum total upload size (bytes), None for unlimited
    pub max_total_size: Option<u64>,
    /// Allowed MIME types, empty for all
    pub allowed_types: Vec<String>,
    /// Chunk size for streaming (default 64KB)
    pub chunk_size: usize,
    /// Temporary directory for buffering
    pub temp_dir: Option<PathBuf>,
}

impl Default for UploadConfig {
    fn default() -> Self {
        Self {
            max_size: Some(100 * 1024 * 1024), // 100MB default
            max_total_size: Some(500 * 1024 * 1024), // 500MB total
            allowed_types: Vec::new(),
            chunk_size: 64 * 1024, // 64KB chunks
            temp_dir: None,
        }
    }
}

impl UploadConfig {
    /// Create a new upload configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum file size
    pub fn max_size(mut self, size: u64) -> Self {
        self.max_size = Some(size);
        self
    }

    /// Set unlimited file size
    pub fn unlimited(mut self) -> Self {
        self.max_size = None;
        self
    }

    /// Set maximum total upload size
    pub fn max_total_size(mut self, size: u64) -> Self {
        self.max_total_size = Some(size);
        self
    }

    /// Set allowed MIME types
    pub fn allowed_types(mut self, types: Vec<String>) -> Self {
        self.allowed_types = types;
        self
    }

    /// Allow specific MIME type
    pub fn allow_type(mut self, mime_type: impl Into<String>) -> Self {
        self.allowed_types.push(mime_type.into());
        self
    }

    /// Allow all image types
    pub fn allow_images(mut self) -> Self {
        self.allowed_types.extend([
            "image/jpeg".to_string(),
            "image/png".to_string(),
            "image/gif".to_string(),
            "image/webp".to_string(),
            "image/svg+xml".to_string(),
        ]);
        self
    }

    /// Allow all video types
    pub fn allow_videos(mut self) -> Self {
        self.allowed_types.extend([
            "video/mp4".to_string(),
            "video/webm".to_string(),
            "video/quicktime".to_string(),
            "video/x-msvideo".to_string(),
        ]);
        self
    }

    /// Set chunk size for streaming
    pub fn chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    /// Set temporary directory
    pub fn temp_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.temp_dir = Some(dir.into());
        self
    }
}

// ============================================================================
// Progress Tracking
// ============================================================================

/// Upload progress information
#[derive(Debug, Clone)]
pub struct UploadProgress {
    /// Current bytes received
    pub bytes_received: u64,
    /// Total expected bytes (if known)
    pub total_bytes: Option<u64>,
    /// Current file being uploaded
    pub current_file: Option<String>,
    /// Number of files processed
    pub files_processed: usize,
    /// Total files expected
    pub total_files: Option<usize>,
}

impl UploadProgress {
    /// Create new progress tracker
    pub fn new() -> Self {
        Self {
            bytes_received: 0,
            total_bytes: None,
            current_file: None,
            files_processed: 0,
            total_files: None,
        }
    }

    /// Get progress percentage (0-100)
    pub fn percentage(&self) -> Option<f64> {
        self.total_bytes.map(|total| {
            if total == 0 {
                100.0
            } else {
                (self.bytes_received as f64 / total as f64) * 100.0
            }
        })
    }
}

impl Default for UploadProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// Progress callback type
pub type ProgressCallback = Box<dyn Fn(&UploadProgress) + Send + Sync>;

// ============================================================================
// Streaming Upload
// ============================================================================

/// Streaming file upload
pub struct StreamingUpload {
    config: UploadConfig,
    progress: UploadProgress,
    progress_callback: Option<ProgressCallback>,
}

impl StreamingUpload {
    /// Create a new streaming upload handler
    pub fn new(config: UploadConfig) -> Self {
        Self {
            config,
            progress: UploadProgress::new(),
            progress_callback: None,
        }
    }

    /// Set progress callback
    pub fn on_progress<F>(mut self, callback: F) -> Self
    where
        F: Fn(&UploadProgress) + Send + Sync + 'static,
    {
        self.progress_callback = Some(Box::new(callback));
        self
    }

    /// Set total expected bytes
    pub fn expect_bytes(mut self, total: u64) -> Self {
        self.progress.total_bytes = Some(total);
        self
    }

    /// Set total expected files
    pub fn expect_files(mut self, count: usize) -> Self {
        self.progress.total_files = Some(count);
        self
    }

    /// Validate file type
    fn validate_type(&self, content_type: &str) -> ApiResult<()> {
        if self.config.allowed_types.is_empty() {
            return Ok(());
        }

        if self.config.allowed_types.iter().any(|t| t == content_type) {
            Ok(())
        } else {
            Err(ApiError::BadRequest(format!(
                "File type '{}' not allowed. Allowed types: {:?}",
                content_type, self.config.allowed_types
            )))
        }
    }

    /// Process a stream of chunks
    pub async fn process_stream<S, W>(
        &mut self,
        mut stream: S,
        mut writer: W,
        filename: Option<&str>,
        content_type: Option<&str>,
    ) -> ApiResult<UploadedFile>
    where
        S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
        W: AsyncWrite + Unpin,
    {
        use futures_util::StreamExt;

        // Validate content type
        if let Some(ct) = content_type {
            self.validate_type(ct)?;
        }

        self.progress.current_file = filename.map(String::from);
        let mut size: u64 = 0;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| ApiError::Internal(e.to_string()))?;
            let chunk_len = chunk.len() as u64;

            // Check size limit
            if let Some(max_size) = self.config.max_size {
                if size + chunk_len > max_size {
                    return Err(ApiError::BadRequest(format!(
                        "File size exceeds maximum allowed ({} bytes)",
                        max_size
                    )));
                }
            }

            // Write chunk
            writer
                .write_all(&chunk)
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;

            size += chunk_len;
            self.progress.bytes_received += chunk_len;

            // Report progress
            if let Some(ref callback) = self.progress_callback {
                callback(&self.progress);
            }
        }

        writer
            .flush()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        self.progress.files_processed += 1;

        Ok(UploadedFile {
            filename: filename.map(String::from),
            content_type: content_type.map(String::from),
            size,
        })
    }

    /// Save stream directly to file
    pub async fn save_to_file<S>(
        &mut self,
        stream: S,
        path: impl AsRef<Path>,
        filename: Option<&str>,
        content_type: Option<&str>,
    ) -> ApiResult<UploadedFile>
    where
        S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
    {
        let file = tokio::fs::File::create(path.as_ref())
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        self.process_stream(stream, file, filename, content_type)
            .await
    }

    /// Get current progress
    pub fn progress(&self) -> &UploadProgress {
        &self.progress
    }
}

// ============================================================================
// Uploaded File
// ============================================================================

/// Result of a file upload
#[derive(Debug, Clone)]
pub struct UploadedFile {
    /// Original filename
    pub filename: Option<String>,
    /// Content type
    pub content_type: Option<String>,
    /// File size in bytes
    pub size: u64,
}

impl UploadedFile {
    /// Get the file extension
    pub fn extension(&self) -> Option<&str> {
        self.filename
            .as_ref()
            .and_then(|f| Path::new(f).extension())
            .and_then(|e| e.to_str())
    }

    /// Check if file is an image
    pub fn is_image(&self) -> bool {
        self.content_type
            .as_ref()
            .map(|ct| ct.starts_with("image/"))
            .unwrap_or(false)
    }

    /// Check if file is a video
    pub fn is_video(&self) -> bool {
        self.content_type
            .as_ref()
            .map(|ct| ct.starts_with("video/"))
            .unwrap_or(false)
    }

    /// Human-readable size
    pub fn human_size(&self) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if self.size >= GB {
            format!("{:.2} GB", self.size as f64 / GB as f64)
        } else if self.size >= MB {
            format!("{:.2} MB", self.size as f64 / MB as f64)
        } else if self.size >= KB {
            format!("{:.2} KB", self.size as f64 / KB as f64)
        } else {
            format!("{} bytes", self.size)
        }
    }
}

// ============================================================================
// Chunk Stream
// ============================================================================

/// Stream of byte chunks
pub struct ChunkStream {
    receiver: mpsc::Receiver<Bytes>,
}

impl ChunkStream {
    /// Create a new chunk stream
    pub fn new(receiver: mpsc::Receiver<Bytes>) -> Self {
        Self { receiver }
    }
}

impl Stream for ChunkStream {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.receiver).poll_recv(cx) {
            Poll::Ready(Some(bytes)) => Poll::Ready(Some(Ok(bytes))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Create a channel for streaming uploads
pub fn upload_channel(buffer_size: usize) -> (mpsc::Sender<Bytes>, ChunkStream) {
    let (tx, rx) = mpsc::channel(buffer_size);
    (tx, ChunkStream::new(rx))
}

// ============================================================================
// Memory Buffer Writer
// ============================================================================

/// In-memory buffer for collecting chunks
pub struct MemoryBuffer {
    data: Vec<u8>,
    max_size: Option<usize>,
}

impl MemoryBuffer {
    /// Create a new memory buffer
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            max_size: None,
        }
    }

    /// Create with size limit
    pub fn with_limit(max_size: usize) -> Self {
        Self {
            data: Vec::with_capacity(max_size.min(1024 * 1024)), // Cap pre-alloc at 1MB
            max_size: Some(max_size),
        }
    }

    /// Get the collected data
    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }

    /// Get data reference
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }
}

impl Default for MemoryBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncWrite for MemoryBuffer {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        if let Some(max) = self.max_size {
            if self.data.len() + buf.len() > max {
                return Poll::Ready(Err(std::io::Error::new(
                    std::io::ErrorKind::OutOfMemory,
                    "Buffer size limit exceeded",
                )));
            }
        }
        self.data.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }
}

// ============================================================================
// Multipart Streaming
// ============================================================================

/// Stream multipart fields with streaming body processing
pub struct MultipartStream<'a> {
    inner: multer::Multipart<'a>,
    config: UploadConfig,
}

impl<'a> MultipartStream<'a> {
    /// Create from multer multipart
    pub fn new(multipart: multer::Multipart<'a>, config: UploadConfig) -> Self {
        Self {
            inner: multipart,
            config,
        }
    }

    /// Get next field
    pub async fn next_field(&mut self) -> ApiResult<Option<StreamingField<'a>>> {
        match self.inner.next_field().await {
            Ok(Some(field)) => {
                let name = field.name().map(String::from);
                let filename = field.file_name().map(String::from);
                let content_type = field.content_type().map(|m| m.to_string());

                // Validate content type
                if let Some(ref ct) = content_type {
                    if !self.config.allowed_types.is_empty()
                        && !self.config.allowed_types.iter().any(|t| t == ct)
                    {
                        return Err(ApiError::BadRequest(format!(
                            "File type '{}' not allowed",
                            ct
                        )));
                    }
                }

                Ok(Some(StreamingField {
                    inner: field,
                    name,
                    filename,
                    content_type,
                    max_size: self.config.max_size,
                }))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(ApiError::BadRequest(e.to_string())),
        }
    }
}

/// A streaming multipart field
pub struct StreamingField<'a> {
    inner: multer::Field<'a>,
    name: Option<String>,
    filename: Option<String>,
    content_type: Option<String>,
    max_size: Option<u64>,
}

impl<'a> StreamingField<'a> {
    /// Get field name
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get filename
    pub fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }

    /// Get content type
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    /// Check if this is a file field
    pub fn is_file(&self) -> bool {
        self.filename.is_some()
    }

    /// Read next chunk
    pub async fn chunk(&mut self) -> ApiResult<Option<Bytes>> {
        self.inner
            .chunk()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))
    }

    /// Collect all bytes into memory
    pub async fn bytes(self) -> ApiResult<Bytes> {
        self.inner
            .bytes()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))
    }

    /// Read as text
    pub async fn text(self) -> ApiResult<String> {
        self.inner
            .text()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))
    }

    /// Stream to a writer
    pub async fn stream_to<W: AsyncWrite + Unpin>(mut self, mut writer: W) -> ApiResult<u64> {
        let mut size: u64 = 0;

        while let Some(chunk) = self.chunk().await? {
            let chunk_len = chunk.len() as u64;

            // Check size limit
            if let Some(max_size) = self.max_size {
                if size + chunk_len > max_size {
                    return Err(ApiError::BadRequest(format!(
                        "File size exceeds maximum allowed ({} bytes)",
                        max_size
                    )));
                }
            }

            writer
                .write_all(&chunk)
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;

            size += chunk_len;
        }

        writer
            .flush()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        Ok(size)
    }

    /// Save directly to file
    pub async fn save_to(self, path: impl AsRef<Path>) -> ApiResult<UploadedFile> {
        let file = tokio::fs::File::create(path.as_ref())
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        let filename = self.filename.clone();
        let content_type = self.content_type.clone();
        let size = self.stream_to(file).await?;

        Ok(UploadedFile {
            filename,
            content_type,
            size,
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;

    #[test]
    fn test_upload_config_defaults() {
        let config = UploadConfig::default();
        assert_eq!(config.max_size, Some(100 * 1024 * 1024));
        assert_eq!(config.chunk_size, 64 * 1024);
        assert!(config.allowed_types.is_empty());
    }

    #[test]
    fn test_upload_config_builder() {
        let config = UploadConfig::new()
            .max_size(50 * 1024 * 1024)
            .allow_images()
            .chunk_size(32 * 1024);

        assert_eq!(config.max_size, Some(50 * 1024 * 1024));
        assert!(config.allowed_types.contains(&"image/jpeg".to_string()));
        assert_eq!(config.chunk_size, 32 * 1024);
    }

    #[test]
    fn test_upload_progress() {
        let mut progress = UploadProgress::new();
        progress.bytes_received = 50;
        progress.total_bytes = Some(100);

        assert_eq!(progress.percentage(), Some(50.0));
    }

    #[test]
    fn test_uploaded_file() {
        let file = UploadedFile {
            filename: Some("test.jpg".to_string()),
            content_type: Some("image/jpeg".to_string()),
            size: 1024 * 1024 * 5, // 5MB
        };

        assert_eq!(file.extension(), Some("jpg"));
        assert!(file.is_image());
        assert!(!file.is_video());
        assert_eq!(file.human_size(), "5.00 MB");
    }

    #[test]
    fn test_human_size() {
        assert_eq!(
            UploadedFile {
                filename: None,
                content_type: None,
                size: 500
            }
            .human_size(),
            "500 bytes"
        );

        assert_eq!(
            UploadedFile {
                filename: None,
                content_type: None,
                size: 1536
            }
            .human_size(),
            "1.50 KB"
        );

        assert_eq!(
            UploadedFile {
                filename: None,
                content_type: None,
                size: 2 * 1024 * 1024 * 1024
            }
            .human_size(),
            "2.00 GB"
        );
    }

    #[tokio::test]
    async fn test_memory_buffer() {
        let mut buffer = MemoryBuffer::new();
        buffer
            .write_all(b"hello ")
            .await
            .unwrap();
        buffer.write_all(b"world").await.unwrap();

        assert_eq!(buffer.as_bytes(), b"hello world");
    }

    #[tokio::test]
    async fn test_memory_buffer_limit() {
        let mut buffer = MemoryBuffer::with_limit(5);
        assert!(buffer.write_all(b"hi").await.is_ok());
        assert!(buffer.write_all(b"there").await.is_err());
    }

    #[tokio::test]
    async fn test_streaming_upload() {
        let config = UploadConfig::new().max_size(1024);
        let mut upload = StreamingUpload::new(config);

        let data = vec![
            Ok(Bytes::from("hello ")),
            Ok(Bytes::from("world")),
        ];
        let stream = stream::iter(data);

        let mut buffer = MemoryBuffer::new();
        let result = upload
            .process_stream(stream, &mut buffer, Some("test.txt"), Some("text/plain"))
            .await
            .unwrap();

        assert_eq!(result.size, 11);
        assert_eq!(result.filename, Some("test.txt".to_string()));
        assert_eq!(buffer.as_bytes(), b"hello world");
    }
}
