use axum::response::IntoResponse;
use metrics_exporter_prometheus::PrometheusHandle;
use std::sync::Arc;

/// Prometheus metrics scrape endpoint.
/// Returns metrics in Prometheus text exposition format.
pub async fn prometheus_metrics(
    axum::extract::State(handle): axum::extract::State<Arc<PrometheusHandle>>,
) -> impl IntoResponse {
    handle.render()
}
