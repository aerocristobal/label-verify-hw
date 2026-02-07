use s3::creds::Credentials;
use s3::{Bucket, Region};

/// Client for Cloudflare R2 object storage (S3-compatible).
pub struct R2Client {
    bucket: Box<Bucket>,
}

impl R2Client {
    pub fn new(
        bucket_name: &str,
        endpoint: &str,
        access_key: &str,
        secret_key: &str,
    ) -> Result<Self, StorageError> {
        let region = Region::Custom {
            region: "auto".to_string(),
            endpoint: endpoint.to_string(),
        };

        let credentials =
            Credentials::new(Some(access_key), Some(secret_key), None, None, None)
                .map_err(|e| StorageError::Config(e.to_string()))?;

        let bucket = Bucket::new(bucket_name, region, credentials)
            .map_err(|e| StorageError::Config(e.to_string()))?;

        Ok(Self { bucket })
    }

    /// Upload encrypted image bytes to R2.
    pub async fn upload(&self, key: &str, data: &[u8], content_type: &str) -> Result<(), StorageError> {
        self.bucket
            .put_object_with_content_type(key, data, content_type)
            .await
            .map_err(StorageError::S3)?;
        Ok(())
    }

    /// Download encrypted image bytes from R2.
    pub async fn download(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let response = self.bucket.get_object(key).await.map_err(StorageError::S3)?;
        Ok(response.to_vec())
    }

    /// Delete an object from R2.
    pub async fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.bucket.delete_object(key).await.map_err(StorageError::S3)?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("S3 operation failed: {0}")]
    S3(#[from] s3::error::S3Error),

    #[error("Storage configuration error: {0}")]
    Config(String),
}
