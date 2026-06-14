use super::blob::BlobStore;
use crate::config::Config;
use async_trait::async_trait;

pub struct LocalFs {
    dir: String,
    public_url: String,
}

impl LocalFs {
    pub fn from_cfg(cfg: &Config) -> Self {
        LocalFs {
            dir: cfg.localfs_dir.clone(),
            public_url: cfg.public_url.clone(),
        }
    }
}

#[async_trait]
impl BlobStore for LocalFs {
    async fn put(&self, key: &str, bytes: Vec<u8>, _ct: &str) -> anyhow::Result<String> {
        let path = std::path::Path::new(&self.dir).join(key);
        if let Some(p) = path.parent() {
            tokio::fs::create_dir_all(p).await?;
        }
        tokio::fs::write(&path, bytes).await?;
        Ok(format!(
            "{}/static/{key}",
            self.public_url.trim_end_matches('/')
        ))
    }
}
