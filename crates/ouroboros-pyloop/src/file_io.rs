//! Async file I/O integration
//!
//! Provides non-blocking file operations via thread pool,
//! compatible with asyncio's file I/O patterns.

use std::io::{self, SeekFrom};
use std::path::{Path, PathBuf};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::task;

// ============================================================================
// Async File Handle
// ============================================================================

/// Async file handle for non-blocking I/O operations
pub struct AsyncFile {
    file: File,
    path: PathBuf,
}

impl AsyncFile {
    /// Open a file for reading
    pub async fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path).await?;
        Ok(Self { file, path })
    }

    /// Create a new file for writing (truncates if exists)
    pub async fn create(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::create(&path).await?;
        Ok(Self { file, path })
    }

    /// Open a file with custom options
    pub async fn open_with(path: impl AsRef<Path>, options: &OpenOptions) -> io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = options.open(&path).await?;
        Ok(Self { file, path })
    }

    /// Get the file path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read the entire file contents
    pub async fn read_all(&mut self) -> io::Result<Vec<u8>> {
        let mut buffer = Vec::new();
        self.file.read_to_end(&mut buffer).await?;
        Ok(buffer)
    }

    /// Read the entire file as string
    pub async fn read_string(&mut self) -> io::Result<String> {
        let mut buffer = String::new();
        self.file.read_to_string(&mut buffer).await?;
        Ok(buffer)
    }

    /// Read up to `count` bytes
    pub async fn read(&mut self, count: usize) -> io::Result<Vec<u8>> {
        let mut buffer = vec![0u8; count];
        let bytes_read = self.file.read(&mut buffer).await?;
        buffer.truncate(bytes_read);
        Ok(buffer)
    }

    /// Read exactly `count` bytes
    pub async fn read_exact(&mut self, count: usize) -> io::Result<Vec<u8>> {
        let mut buffer = vec![0u8; count];
        self.file.read_exact(&mut buffer).await?;
        Ok(buffer)
    }

    /// Read until a specific byte (like readline for '\n')
    pub async fn read_until(&mut self, byte: u8) -> io::Result<Vec<u8>> {
        let mut buffer = Vec::new();
        let mut single = [0u8; 1];

        loop {
            match self.file.read(&mut single).await? {
                0 => break, // EOF
                _ => {
                    buffer.push(single[0]);
                    if single[0] == byte {
                        break;
                    }
                }
            }
        }

        Ok(buffer)
    }

    /// Read a line (reads until '\n')
    pub async fn readline(&mut self) -> io::Result<String> {
        let bytes = self.read_until(b'\n').await?;
        String::from_utf8(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Write bytes to the file
    pub async fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.file.write(data).await
    }

    /// Write all bytes to the file
    pub async fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        self.file.write_all(data).await
    }

    /// Write a string to the file
    pub async fn write_string(&mut self, data: &str) -> io::Result<()> {
        self.file.write_all(data.as_bytes()).await
    }

    /// Seek to a position
    pub async fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.file.seek(pos).await
    }

    /// Get current position
    pub async fn tell(&mut self) -> io::Result<u64> {
        self.file.stream_position().await
    }

    /// Flush the file
    pub async fn flush(&mut self) -> io::Result<()> {
        self.file.flush().await
    }

    /// Sync all data to disk
    pub async fn sync_all(&self) -> io::Result<()> {
        self.file.sync_all().await
    }

    /// Get file metadata
    pub async fn metadata(&self) -> io::Result<std::fs::Metadata> {
        self.file.metadata().await
    }
}

// ============================================================================
// File Builder
// ============================================================================

/// Builder for file open options
#[derive(Default)]
pub struct FileBuilder {
    read: bool,
    write: bool,
    append: bool,
    create: bool,
    create_new: bool,
    truncate: bool,
}

impl FileBuilder {
    /// Create a new file builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Open for reading
    pub fn read(mut self) -> Self {
        self.read = true;
        self
    }

    /// Open for writing
    pub fn write(mut self) -> Self {
        self.write = true;
        self
    }

    /// Open for appending
    pub fn append(mut self) -> Self {
        self.append = true;
        self.write = true;
        self
    }

    /// Create file if it doesn't exist
    pub fn create(mut self) -> Self {
        self.create = true;
        self
    }

    /// Create new file, error if exists
    pub fn create_new(mut self) -> Self {
        self.create_new = true;
        self
    }

    /// Truncate file to 0 length
    pub fn truncate(mut self) -> Self {
        self.truncate = true;
        self
    }

    /// Open the file with the configured options
    pub async fn open(self, path: impl AsRef<Path>) -> io::Result<AsyncFile> {
        let mut options = OpenOptions::new();
        options
            .read(self.read)
            .write(self.write)
            .append(self.append)
            .create(self.create)
            .create_new(self.create_new)
            .truncate(self.truncate);

        AsyncFile::open_with(path, &options).await
    }
}

// ============================================================================
// Standalone Functions
// ============================================================================

/// Read entire file contents (convenience function)
pub async fn read_file(path: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    tokio::fs::read(path).await
}

/// Read file as string (convenience function)
pub async fn read_to_string(path: impl AsRef<Path>) -> io::Result<String> {
    tokio::fs::read_to_string(path).await
}

/// Write bytes to file (convenience function)
pub async fn write_file(path: impl AsRef<Path>, contents: &[u8]) -> io::Result<()> {
    tokio::fs::write(path, contents).await
}

/// Append bytes to file
pub async fn append_file(path: impl AsRef<Path>, contents: &[u8]) -> io::Result<()> {
    let mut file = FileBuilder::new()
        .write()
        .append()
        .create()
        .open(path)
        .await?;
    file.write_all(contents).await
}

/// Copy file from src to dst
pub async fn copy_file(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<u64> {
    tokio::fs::copy(src, dst).await
}

/// Remove a file
pub async fn remove_file(path: impl AsRef<Path>) -> io::Result<()> {
    tokio::fs::remove_file(path).await
}

/// Create a directory
pub async fn create_dir(path: impl AsRef<Path>) -> io::Result<()> {
    tokio::fs::create_dir(path).await
}

/// Create directory and all parent directories
pub async fn create_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    tokio::fs::create_dir_all(path).await
}

/// Remove a directory
pub async fn remove_dir(path: impl AsRef<Path>) -> io::Result<()> {
    tokio::fs::remove_dir(path).await
}

/// Remove directory and all contents
pub async fn remove_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    tokio::fs::remove_dir_all(path).await
}

/// Check if path exists
pub async fn exists(path: impl AsRef<Path>) -> bool {
    tokio::fs::try_exists(path).await.unwrap_or(false)
}

/// Get file metadata
pub async fn metadata(path: impl AsRef<Path>) -> io::Result<std::fs::Metadata> {
    tokio::fs::metadata(path).await
}

/// Rename/move a file
pub async fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    tokio::fs::rename(from, to).await
}

// ============================================================================
// Blocking I/O via Thread Pool
// ============================================================================

/// Run a blocking file operation in a thread pool
///
/// Use this for operations that don't have async equivalents
/// or when working with synchronous file APIs.
pub async fn run_blocking<F, T>(f: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    task::spawn_blocking(f)
        .await
        .expect("spawn_blocking panicked")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_read_file() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"hello world").unwrap();
        temp.flush().unwrap();

        let contents = read_file(temp.path()).await.unwrap();
        assert_eq!(contents, b"hello world");
    }

    #[tokio::test]
    async fn test_write_file() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path().to_path_buf();

        write_file(&path, b"test content").await.unwrap();
        let contents = read_file(&path).await.unwrap();
        assert_eq!(contents, b"test content");
    }

    #[tokio::test]
    async fn test_async_file_read_write() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path().to_path_buf();

        // Write
        {
            let mut file = AsyncFile::create(&path).await.unwrap();
            file.write_all(b"line1\nline2\n").await.unwrap();
        }

        // Read
        {
            let mut file = AsyncFile::open(&path).await.unwrap();
            let content = file.read_string().await.unwrap();
            assert_eq!(content, "line1\nline2\n");
        }
    }

    #[tokio::test]
    async fn test_file_builder() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path().to_path_buf();

        // Create and write
        {
            let mut file = FileBuilder::new()
                .write()
                .create()
                .truncate()
                .open(&path)
                .await
                .unwrap();
            file.write_all(b"content").await.unwrap();
        }

        // Read
        let contents = read_file(&path).await.unwrap();
        assert_eq!(contents, b"content");
    }

    #[tokio::test]
    async fn test_seek_and_tell() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"0123456789").unwrap();
        temp.flush().unwrap();

        let mut file = AsyncFile::open(temp.path()).await.unwrap();

        // Initial position
        assert_eq!(file.tell().await.unwrap(), 0);

        // Seek to position 5
        file.seek(SeekFrom::Start(5)).await.unwrap();
        assert_eq!(file.tell().await.unwrap(), 5);

        // Read remaining
        let data = file.read_all().await.unwrap();
        assert_eq!(data, b"56789");
    }
}
