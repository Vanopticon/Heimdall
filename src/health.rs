use axum::{extract::State, http::StatusCode, response::IntoResponse};

/// DB health endpoint: returns 200 OK when the configured repo can run a
/// simple query, otherwise returns 503 Service Unavailable.
pub async fn db_health(State(state): State<crate::state::AppState>) -> impl IntoResponse {
	match state.repo.ping().await {
		Ok(()) => (StatusCode::OK, "OK").into_response(),
		Err(e) => (StatusCode::SERVICE_UNAVAILABLE, format!("db error: {}", e)).into_response(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::age_client::AgeRepo;
	use anyhow::{Result, anyhow};
	use async_trait::async_trait;
	use serde_json::Value;
	use std::sync::Arc;
	use tokio::sync::mpsc;

	// Mock repository for testing
	struct MockAgeRepo {
		should_succeed: bool,
	}

	#[async_trait]
	impl AgeRepo for MockAgeRepo {
		async fn merge_entity(&self, _label: &str, _key: &str, _props: &Value) -> Result<()> {
			Ok(())
		}

		async fn ping(&self) -> Result<()> {
			if self.should_succeed {
				Ok(())
			} else {
				Err(anyhow!("database unavailable"))
			}
		}

		async fn merge_batch(&self, _items: &[(String, String, Value)]) -> Result<()> {
			Ok(())
		}
	}

	#[tokio::test]
	async fn health_check_returns_ok_when_db_healthy() {
		let repo: Arc<dyn AgeRepo> = Arc::new(MockAgeRepo {
			should_succeed: true,
		});
		let (tx, _rx) = mpsc::channel(10);
		let state = crate::state::AppState {
			repo,
			persist_sender: tx,
		};

		let response = db_health(State(state)).await.into_response();
		assert_eq!(response.status(), StatusCode::OK);
	}

	#[tokio::test]
	async fn health_check_returns_service_unavailable_when_db_fails() {
		let repo: Arc<dyn AgeRepo> = Arc::new(MockAgeRepo {
			should_succeed: false,
		});
		let (tx, _rx) = mpsc::channel(10);
		let state = crate::state::AppState {
			repo,
			persist_sender: tx,
		};

		let response = db_health(State(state)).await.into_response();
		assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
	}
}
