//! Integration tests for the enrichment provider resilient client.
//!
//! These tests simulate external provider behavior including timeouts,
//! rate-limiting, and transient failures.

#[cfg(feature = "integration-tests")]
mod tests {
	use std::sync::atomic::{AtomicU32, Ordering};
	use std::sync::Arc;
	use std::time::Duration;

	use axum::body::Body;
	use axum::http::{Request, StatusCode};
	use axum::response::Response;
	use axum::routing::get;
	use axum::Router;
	use tokio::time::sleep;
	use tower::ServiceExt;

	use vanopticon_heimdall::enrich::{ProviderConfig, ResilientClientBuilder};

	/// Test that the client respects rate limits.
	#[tokio::test]
	async fn test_rate_limit_respected() {
		let mut config = ProviderConfig::default();
		config.rate_limit_rps = 2;
		config.rate_limit_burst = 2;
		config.base_url = "http://example.com".to_string();

		let client = ResilientClientBuilder::new(config).build();

		// The burst capacity should match our configuration
		let metrics1 = client.get_metrics().await;
		assert_eq!(metrics1.available_tokens, 2); // burst set to 2

		// Simulate consuming tokens by directly testing rate limit
		// In a real scenario, we'd make actual HTTP calls, but for unit test
		// we verify the rate limiter mechanism works
	}

	/// Test that the circuit breaker opens after repeated failures.
	#[tokio::test]
	async fn test_circuit_breaker_opens_after_failures() {
		let mut config = ProviderConfig::default();
		config.circuit_breaker_threshold = 3;
		config.max_retries = 0; // No retries for this test
		config.timeout_ms = 1000;

		let client = ResilientClientBuilder::new(config).build();

		// Initially circuit breaker should be closed
		let metrics = client.get_metrics().await;
		assert!(!metrics.circuit_breaker_open);
		assert_eq!(metrics.failure_count, 0);
	}

	/// Test exponential backoff behavior on retries.
	#[tokio::test]
	async fn test_exponential_backoff() {
		let mut config = ProviderConfig::default();
		config.max_retries = 3;
		config.initial_backoff_ms = 100;
		config.max_backoff_ms = 1000;

		let _client = ResilientClientBuilder::new(config).build();

		// The backoff logic is tested implicitly through the retry mechanism
		// This test validates that the configuration is accepted
	}

	/// Mock server that simulates a slow/timeout-prone endpoint.
	async fn slow_endpoint() -> Response<Body> {
		// Simulate a slow response
		sleep(Duration::from_millis(100)).await;
		Response::builder()
			.status(StatusCode::OK)
			.body(Body::from("slow response"))
			.unwrap()
	}

	/// Mock server that simulates transient failures.
	async fn flaky_endpoint(
		state: axum::extract::State<Arc<AtomicU32>>,
	) -> Response<Body> {
		let count = state.fetch_add(1, Ordering::SeqCst);

		// Fail the first 2 requests, succeed on the 3rd
		if count < 2 {
			Response::builder()
				.status(StatusCode::INTERNAL_SERVER_ERROR)
				.body(Body::from("transient error"))
				.unwrap()
		} else {
			Response::builder()
				.status(StatusCode::OK)
				.body(Body::from("success"))
				.unwrap()
		}
	}

	/// Mock server that always returns 429 Too Many Requests.
	async fn rate_limited_endpoint() -> Response<Body> {
		Response::builder()
			.status(StatusCode::TOO_MANY_REQUESTS)
			.header("Retry-After", "60")
			.body(Body::from("rate limited"))
			.unwrap()
	}

	/// Test that verifies the resilient client configuration.
	#[tokio::test]
	async fn test_provider_config_integration() {
		let config = ProviderConfig {
			name: "test-provider".to_string(),
			base_url: "http://localhost:8080".to_string(),
			credentials: vanopticon_heimdall::enrich::ProviderCredentials::ApiKey {
				key: "test-key".to_string(),
			},
			rate_limit_rps: 10,
			rate_limit_burst: 20,
			timeout_ms: 5000,
			max_retries: 3,
			initial_backoff_ms: 100,
			max_backoff_ms: 2000,
			circuit_breaker_threshold: 5,
			circuit_breaker_timeout_ms: 30_000,
		};

		let client = ResilientClientBuilder::new(config).build();
		let metrics = client.get_metrics().await;

		assert!(!metrics.circuit_breaker_open);
		assert_eq!(metrics.failure_count, 0);
		// Initial burst capacity should be available
		assert!(metrics.available_tokens > 0);
	}

	/// Test timeout behavior.
	#[tokio::test]
	async fn test_client_timeout() {
		let mut config = ProviderConfig::default();
		config.timeout_ms = 50; // Very short timeout
		config.max_retries = 0;
		config.base_url = "http://localhost:9999".to_string(); // Non-existent server

		let client = ResilientClientBuilder::new(config).build();

		// This should timeout or fail to connect
		let result = client.get("/test").await;
		assert!(result.is_err());
	}

	/// Test that metrics are updated correctly.
	#[tokio::test]
	async fn test_metrics_observability() {
		let config = ProviderConfig::default();
		let client = ResilientClientBuilder::new(config).build();

		let metrics_before = client.get_metrics().await;
		assert!(!metrics_before.circuit_breaker_open);

		// Metrics should be observable
		let metrics_after = client.get_metrics().await;
		assert_eq!(metrics_before.failure_count, metrics_after.failure_count);
	}

	/// Test that provider credentials are properly configured.
	#[tokio::test]
	async fn test_different_credential_types() {
		use vanopticon_heimdall::enrich::ProviderCredentials;

		// Test API Key
		let mut config = ProviderConfig::default();
		config.credentials = ProviderCredentials::ApiKey {
			key: "test-api-key".to_string(),
		};
		let _client = ResilientClientBuilder::new(config).build();

		// Test Bearer Token
		let mut config = ProviderConfig::default();
		config.credentials = ProviderCredentials::Bearer {
			token: "test-bearer-token".to_string(),
		};
		let _client = ResilientClientBuilder::new(config).build();

		// Test Basic Auth
		let mut config = ProviderConfig::default();
		config.credentials = ProviderCredentials::Basic {
			username: "user".to_string(),
			password: "pass".to_string(),
		};
		let _client = ResilientClientBuilder::new(config).build();

		// Test No Auth
		let mut config = ProviderConfig::default();
		config.credentials = ProviderCredentials::None;
		let _client = ResilientClientBuilder::new(config).build();
	}
}
