use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

use crate::app_state::AppState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub checks: HealthChecks,
}

#[derive(Serialize)]
pub struct HealthChecks {
    pub database: ComponentHealth,
    pub redis: ComponentHealth,
}

#[derive(Serialize)]
pub struct ComponentHealth {
    pub status: String,
    pub latency_ms: Option<u64>,
}

/// GET /health — comprehensive health check with dependency status.
pub async fn health_check(
    State(state): State<AppState>,
) -> (StatusCode, Json<HealthResponse>) {
    let start = std::time::Instant::now();

    // Check database connectivity
    let db_check = match sqlx::query("SELECT 1").execute(&state.db).await {
        Ok(_) => ComponentHealth {
            status: "ok".to_string(),
            latency_ms: Some(start.elapsed().as_millis() as u64),
        },
        Err(_) => ComponentHealth {
            status: "error".to_string(),
            latency_ms: None,
        },
    };

    // Check Redis connectivity
    let redis_start = std::time::Instant::now();
    let redis_check = match state.queue.health_check().await {
        Ok(_) => ComponentHealth {
            status: "ok".to_string(),
            latency_ms: Some(redis_start.elapsed().as_millis() as u64),
        },
        Err(_) => ComponentHealth {
            status: "error".to_string(),
            latency_ms: None,
        },
    };

    let all_healthy = db_check.status == "ok" && redis_check.status == "ok";
    let status_code = if all_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let response = HealthResponse {
        status: if all_healthy {
            "ok".to_string()
        } else {
            "degraded".to_string()
        },
        version: env!("CARGO_PKG_VERSION").to_string(),
        checks: HealthChecks {
            database: db_check,
            redis: redis_check,
        },
    };

    (status_code, Json(response))
}
