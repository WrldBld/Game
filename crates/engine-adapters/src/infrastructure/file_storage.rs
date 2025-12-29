//! File storage adapter using tokio::fs for async file operations.

use async_trait::async_trait;
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use wrldbldr_engine_ports::outbound::FileStoragePort;

/// Tokio-based file storage implementation.
///
/// Uses tokio::fs for async file operations on the local filesystem.
#[derive(Debug, Clone, Default)]
pub struct TokioFileStorageAdapter;

impl TokioFileStorageAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl FileStoragePort for TokioFileStorageAdapter {
    async fn create_dir_all(&self, path: &Path) -> anyhow::Result<()> {
        fs::create_dir_all(path).await?;
        Ok(())
    }

    async fn write(&self, path: &Path, data: &[u8]) -> anyhow::Result<()> {
        let mut file = fs::File::create(path).await?;
        file.write_all(data).await?;
        Ok(())
    }

    async fn write_str(&self, path: &Path, data: &str) -> anyhow::Result<()> {
        self.write(path, data.as_bytes()).await
    }

    async fn read_to_string(&self, path: &Path) -> anyhow::Result<String> {
        let content = fs::read_to_string(path).await?;
        Ok(content)
    }

    async fn read(&self, path: &Path) -> anyhow::Result<Vec<u8>> {
        let content = fs::read(path).await?;
        Ok(content)
    }

    async fn exists(&self, path: &Path) -> anyhow::Result<bool> {
        Ok(path.exists())
    }

    async fn remove_file(&self, path: &Path) -> anyhow::Result<()> {
        fs::remove_file(path).await?;
        Ok(())
    }

    async fn remove_dir_all(&self, path: &Path) -> anyhow::Result<()> {
        fs::remove_dir_all(path).await?;
        Ok(())
    }
}
