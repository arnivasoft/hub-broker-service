use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use axum::{response::IntoResponse, http::StatusCode};

/// Initialize Prometheus metrics
pub fn init_metrics() -> PrometheusHandle {
    const EXPONENTIAL_SECONDS: &[f64] = &[
        0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
    ];

    PrometheusBuilder::new()
        .set_buckets_for_metric(
            Matcher::Full("hub_broker_message_duration_seconds".to_string()),
            EXPONENTIAL_SECONDS,
        )
        .unwrap()
        .install_recorder()
        .unwrap()
}

/// Metrics handler endpoint
pub async fn metrics_handler() -> impl IntoResponse {
    match metrics_exporter_prometheus::render() {
        Ok(body) => (StatusCode::OK, body),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, String::new()),
    }
}

// Metric recording functions
pub fn record_connection(tenant_id: &str) {
    counter!("hub_broker_connections_total", "tenant_id" => tenant_id.to_string()).increment(1);
}

pub fn record_disconnection(tenant_id: &str) {
    counter!("hub_broker_disconnections_total", "tenant_id" => tenant_id.to_string()).increment(1);
}

pub fn record_message(tenant_id: &str, message_type: &str) {
    counter!(
        "hub_broker_messages_total",
        "tenant_id" => tenant_id.to_string(),
        "type" => message_type.to_string()
    )
    .increment(1);
}

pub fn record_message_duration(duration_secs: f64) {
    histogram!("hub_broker_message_duration_seconds").record(duration_secs);
}

pub fn set_active_connections(tenant_id: &str, count: usize) {
    gauge!(
        "hub_broker_active_connections",
        "tenant_id" => tenant_id.to_string()
    )
    .set(count as f64);
}

pub fn record_routing_error(tenant_id: &str, error_type: &str) {
    counter!(
        "hub_broker_routing_errors_total",
        "tenant_id" => tenant_id.to_string(),
        "error" => error_type.to_string()
    )
    .increment(1);
}
