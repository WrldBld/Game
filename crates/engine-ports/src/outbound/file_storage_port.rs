//! File storage port for abstracting file system operations.
//!
//! This allows application layer services to interact with the file system
//! without direct I/O dependencies, enabling testing and alternative storage backends.

use async_trait::async_trait;
use std::path::Path;

/// Port for file system operations.
///
/// Abstracts file I/O so application services don't depend on std::fs directly.
/// Implementations can provide local filesystem, cloud storage, or mock storage.
#[async_trait]
pub trait FileStoragePort: Send + Sync {
    /// Create all directories in the path if they don't exist.
    async fn create_dir_all(&self, path: &Path) -> anyhow::Result<()>;

    /// Write bytes to a file, creating it if it doesn't exist.
    async fn write(&self, path: &Path, data: &[u8]) -> anyhow::Result<()>;

    /// Write string to a file, creating it if it doesn't exist.
    async fn write_str(&self, path: &Path, data: &str) -> anyhow::Result<()>;

    /// Read file contents as string.
    async fn read_to_string(&self, path: &Path) -> anyhow::Result<String>;

    /// Read file contents as bytes.
    async fn read(&self, path: &Path) -> anyhow::Result<Vec<u8>>;

    /// Check if a path exists.
    async fn exists(&self, path: &Path) -> anyhow::Result<bool>;

    /// Remove a file.
    async fn remove_file(&self, path: &Path) -> anyhow::Result<()>;

    /// Remove a directory and all its contents.
    async fn remove_dir_all(&self, path: &Path) -> anyhow::Result<()>;
}
