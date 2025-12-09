use axum::{extract::State, http::StatusCode, response::IntoResponse};

/// DB health endpoint: returns 200 OK when the configured repo can run a
/// simple query, otherwise returns 503 Service Unavailable.
pub async fn db_health(State(state): State<crate::state::AppState>) -> impl IntoResponse {
	match state.repo.ping().await {
		Ok(()) => (StatusCode::OK, "OK").into_response(),
		Err(e) => (StatusCode::SERVICE_UNAVAILABLE, format!("db error: {}", e)).into_response(),
	}
}

/// Prometheus metrics endpoint: returns metrics in Prometheus text format
pub async fn metrics_handler(State(state): State<crate::state::AppState>) -> impl IntoResponse {
	let metrics_text = state.metrics.encode();
	(StatusCode::OK, metrics_text).into_response()
}
