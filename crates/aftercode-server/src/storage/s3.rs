use super::blob::BlobStore;
use crate::config::Config;
use async_trait::async_trait;

pub struct S3Store {
    client: aws_sdk_s3::Client,
    bucket: String,
    public_url: String,
}

impl S3Store {
    pub async fn from_cfg(cfg: &Config) -> anyhow::Result<Self> {
        let bucket = cfg
            .s3_bucket
            .clone()
            .ok_or_else(|| anyhow::anyhow!("S3_BUCKET not set"))?;
        let conf = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        Ok(S3Store {
            client: aws_sdk_s3::Client::new(&conf),
            bucket,
            public_url: cfg.public_url.clone(),
        })
    }
}

#[async_trait]
impl BlobStore for S3Store {
    async fn put(&self, key: &str, bytes: Vec<u8>, ct: &str) -> anyhow::Result<String> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(bytes.into())
            .content_type(ct)
            .send()
            .await?;
        Ok(format!("{}/{key}", self.public_url.trim_end_matches('/')))
    }
}
