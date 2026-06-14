use async_trait::async_trait;
use std::sync::Mutex;

#[async_trait]
pub trait BlobStore: Send + Sync {
    async fn put(&self, key: &str, bytes: Vec<u8>, content_type: &str) -> anyhow::Result<String>;
}

#[derive(Default)]
pub struct MockBlob {
    pub puts: Mutex<Vec<(String, usize)>>,
}

#[async_trait]
impl BlobStore for MockBlob {
    async fn put(&self, key: &str, bytes: Vec<u8>, _ct: &str) -> anyhow::Result<String> {
        self.puts
            .lock()
            .unwrap()
            .push((key.to_string(), bytes.len()));
        Ok(format!("mock://{key}"))
    }
}
