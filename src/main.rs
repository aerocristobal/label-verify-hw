mod config;
mod models;
mod routes;
mod services;

use axum::{routing::get, routing::post, Router};
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();

    // Build API routes
    let app = Router::new()
        .route("/health", get(routes::health::health_check))
        .route("/api/v1/verify", post(routes::verify::submit_verification))
        .route(
            "/api/v1/verify/{job_id}",
            get(routes::verify::get_job_status),
        )
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024)); // 10 MB limit

    let bind_addr = "0.0.0.0:3000";
    tracing::info!("Starting label-verify-hw on {}", bind_addr);

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}
