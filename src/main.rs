mod app_state;
mod config;
mod db;
mod models;
mod routes;
mod services;

use axum::{routing::get, routing::post, Router};
use axum::response::Html;
use metrics_exporter_prometheus::PrometheusBuilder;
use std::sync::Arc;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use app_state::AppState;
use config::AppConfig;
use services::{
    encryption::EncryptionService,
    ocr::WorkersAiClient,
    queue::JobQueue,
    storage::R2Client,
};

#[tokio::main]
async fn main() {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .json()
        .init();

    // Load configuration from environment
    let config = AppConfig::from_env().expect("Failed to load configuration from environment");

    tracing::info!("Initializing label-verify-hw server");

    // Initialize Prometheus metrics recorder
    let prometheus_handle = PrometheusBuilder::new()
        .install_recorder()
        .expect("Failed to install Prometheus metrics recorder");
    let prometheus_handle = Arc::new(prometheus_handle);

    // Register application metrics
    metrics::describe_histogram!(
        "verification_processing_seconds",
        "Time to process a label verification job"
    );
    metrics::describe_counter!(
        "verification_jobs_total",
        "Total verification jobs submitted"
    );
    metrics::describe_counter!(
        "verification_jobs_completed",
        "Total verification jobs completed"
    );
    metrics::describe_counter!(
        "verification_jobs_failed",
        "Total verification jobs that failed"
    );
    metrics::describe_gauge!(
        "verification_queue_depth",
        "Current number of pending jobs in the queue"
    );

    // Initialize database connection pool
    tracing::info!("Connecting to PostgreSQL database");
    let db_pool = db::init_pool(&config.database_url)
        .await
        .expect("Failed to connect to database");

    // Run database migrations
    tracing::info!("Running database migrations");
    db::run_migrations(&db_pool)
        .await
        .expect("Failed to run database migrations");

    // Initialize R2 storage client
    tracing::info!("Initializing R2 storage client");
    let r2_client = R2Client::new(
        &config.r2_bucket,
        &config.r2_endpoint,
        &config.r2_access_key,
        &config.r2_secret_key,
    )
    .expect("Failed to initialize R2 client");

    // Initialize encryption service
    tracing::info!("Initializing AES-256-GCM encryption");
    let encryption =
        EncryptionService::new(&config.encryption_key).expect("Failed to initialize encryption");

    // Initialize Redis job queue
    tracing::info!("Connecting to Redis job queue");
    let queue = JobQueue::new(&config.redis_url).expect("Failed to initialize job queue");

    // Initialize Workers AI client
    tracing::info!("Initializing Cloudflare Workers AI client");
    let ocr_client = WorkersAiClient::new(&config.cf_account_id, &config.cf_api_token)
        .expect("Failed to initialize Workers AI client");

    // Create shared application state
    let state = AppState::new(db_pool, r2_client, encryption, queue, ocr_client);

    // Build API routes
    let app = Router::new()
        // Static UI (embedded at compile time)
        .route("/", get(|| async { Html(include_str!("../static/index.html")) }))
        // API endpoints
        .route("/health", get(routes::health::health_check))
        .route("/api/v1/verify", post(routes::verify::submit_verification))
        .route(
            "/api/v1/verify/:job_id",
            get(routes::verify::get_job_status),
        )
        .with_state(state)
        // Prometheus metrics endpoint (separate state)
        .route(
            "/metrics",
            get(routes::metrics::prometheus_metrics).with_state(prometheus_handle),
        )
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024)); // 10 MB limit

    tracing::info!("Starting label-verify-hw on {}", config.bind_addr);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .expect("Failed to bind to address");

    tracing::info!("Server listening on {}", config.bind_addr);

    axum::serve(listener, app)
        .await
        .expect("Server error");
}
