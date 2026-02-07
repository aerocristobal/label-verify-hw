use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    /// Server bind address (e.g., "0.0.0.0:3000"). Optional for worker processes.
    #[serde(default = "default_bind_addr")]
    pub bind_addr: String,

    /// PostgreSQL connection string
    pub database_url: String,

    /// Redis connection string for job queue
    pub redis_url: String,

    /// Cloudflare account ID
    pub cf_account_id: String,

    /// Cloudflare Workers AI API token
    pub cf_api_token: String,

    /// R2 bucket name
    pub r2_bucket: String,

    /// R2 access key ID (S3-compatible)
    pub r2_access_key: String,

    /// R2 secret access key (S3-compatible)
    pub r2_secret_key: String,

    /// R2 endpoint URL
    pub r2_endpoint: String,

    /// AES-256-GCM encryption key (base64-encoded, 32 bytes)
    pub encryption_key: String,
}

fn default_bind_addr() -> String {
    "0.0.0.0:3000".to_string()
}

impl AppConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        dotenvy::dotenv().ok();
        envy::from_env()
    }
}
