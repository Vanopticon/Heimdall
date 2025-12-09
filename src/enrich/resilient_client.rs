use std::sync::Arc;
use std::time::{Duration, Instant};

use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::{Method, Request, StatusCode, Uri};
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use log::{debug, warn};
use rand::Rng;
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::time::sleep;

use super::provider_config::{ProviderConfig, ProviderCredentials};

/// Errors that can occur during resilient HTTP operations.
#[derive(Debug, Error)]
pub enum ResilientClientError {
	#[error("HTTP request failed: {0}")]
	HttpError(String),

	#[error("HTTP error status: {0}")]
	HttpStatus(StatusCode),

	#[error("circuit breaker is open")]
	CircuitBreakerOpen,

	#[error("rate limit exceeded")]
	RateLimitExceeded,

	#[error("request timeout")]
	Timeout,

	#[error("max retries exceeded")]
	MaxRetriesExceeded,

	#[error("invalid URI: {0}")]
	InvalidUri(#[from] hyper::http::uri::InvalidUri),

	#[error("invalid request: {0}")]
	InvalidRequest(String),
}

/// Circuit breaker state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CircuitState {
	Closed,
	Open { opened_at: Instant },
	HalfOpen,
}

/// Circuit breaker implementation.
struct CircuitBreaker {
	state: CircuitState,
	failure_count: u32,
	threshold: u32,
	timeout: Duration,
}

impl CircuitBreaker {
	fn new(threshold: u32, timeout: Duration) -> Self {
		Self {
			state: CircuitState::Closed,
			failure_count: 0,
			threshold,
			timeout,
		}
	}

	fn record_success(&mut self) {
		match self.state {
			CircuitState::HalfOpen => {
				debug!("Circuit breaker transitioning to Closed after success");
				self.state = CircuitState::Closed;
				self.failure_count = 0;
			}
			CircuitState::Closed => {
				self.failure_count = 0;
			}
			CircuitState::Open { .. } => {}
		}
	}

	fn record_failure(&mut self) {
		self.failure_count += 1;

		match self.state {
			CircuitState::Closed => {
				if self.failure_count >= self.threshold {
					warn!(
						"Circuit breaker opening after {} failures",
						self.failure_count
					);
					self.state = CircuitState::Open {
						opened_at: Instant::now(),
					};
				}
			}
			CircuitState::HalfOpen => {
				warn!("Circuit breaker reopening after failure in half-open state");
				self.state = CircuitState::Open {
					opened_at: Instant::now(),
				};
			}
			CircuitState::Open { .. } => {}
		}
	}

	fn can_attempt(&mut self) -> bool {
		match self.state {
			CircuitState::Closed => true,
			CircuitState::HalfOpen => true,
			CircuitState::Open { opened_at } => {
				if Instant::now().duration_since(opened_at) >= self.timeout {
					debug!("Circuit breaker transitioning to HalfOpen after timeout");
					self.state = CircuitState::HalfOpen;
					true
				} else {
					false
				}
			}
		}
	}

	fn is_open(&self) -> bool {
		matches!(self.state, CircuitState::Open { .. })
	}
}

/// Token bucket rate limiter.
struct TokenBucket {
	capacity: f64,
	tokens: f64,
	refill_per_sec: f64,
	last_refill: Instant,
}

impl TokenBucket {
	fn new(capacity: u32, refill_per_sec: u32) -> Self {
		let now = Instant::now();
		Self {
			capacity: capacity as f64,
			tokens: capacity as f64,
			refill_per_sec: refill_per_sec as f64,
			last_refill: now,
		}
	}

	fn try_acquire(&mut self) -> bool {
		let now = Instant::now();
		let elapsed = now.duration_since(self.last_refill).as_secs_f64();
		self.tokens = (self.tokens + elapsed * self.refill_per_sec).min(self.capacity);
		self.last_refill = now;

		if self.tokens >= 1.0 {
			self.tokens -= 1.0;
			true
		} else {
			false
		}
	}
}

/// Resilient HTTP client with rate-limiting, retry/backoff, and circuit-breaker.
pub struct ResilientClient {
	config: ProviderConfig,
	client: Client<hyper_util::client::legacy::connect::HttpConnector, Full<Bytes>>,
	rate_limiter: Arc<Mutex<TokenBucket>>,
	circuit_breaker: Arc<Mutex<CircuitBreaker>>,
}

impl ResilientClient {
	/// Execute a GET request with resilience features.
	pub async fn get(&self, path: &str) -> Result<Bytes, ResilientClientError> {
		self.execute_with_retry(Method::GET, path, None).await
	}

	/// Execute a POST request with resilience features.
	pub async fn post(&self, path: &str, body: String) -> Result<Bytes, ResilientClientError> {
		self.execute_with_retry(Method::POST, path, Some(body))
			.await
	}

	/// Execute a request with retry logic and backoff.
	async fn execute_with_retry(
		&self,
		method: Method,
		path: &str,
		body: Option<String>,
	) -> Result<Bytes, ResilientClientError> {
		let mut attempts = 0;
		let mut backoff = self.config.initial_backoff();

		loop {
			// Check circuit breaker
			{
				let mut cb = self.circuit_breaker.lock().await;
				if !cb.can_attempt() {
					return Err(ResilientClientError::CircuitBreakerOpen);
				}
			}

			// Check rate limiter
			{
				let mut rl = self.rate_limiter.lock().await;
				if !rl.try_acquire() {
					return Err(ResilientClientError::RateLimitExceeded);
				}
			}

			// Execute the request
			match self.execute_once(&method, path, body.clone()).await {
				Ok(response) => {
					// Success: update circuit breaker
					let mut cb = self.circuit_breaker.lock().await;
					cb.record_success();
					return Ok(response);
				}
				Err(e) => {
					attempts += 1;

					// Determine if we should retry
					let should_retry = attempts <= self.config.max_retries
						&& self.is_retryable_error(&e);

					if !should_retry {
						// Record failure in circuit breaker
						let mut cb = self.circuit_breaker.lock().await;
						cb.record_failure();
						return Err(e);
					}

					// Log and backoff before retry
					warn!(
						"Request failed (attempt {}/{}): {:?}, retrying after {:?}",
						attempts, self.config.max_retries, e, backoff
					);

					sleep(backoff).await;

					// Exponential backoff with jitter
					backoff = (backoff * 2).min(self.config.max_backoff());
					let max_jitter = backoff.as_millis() as u64 / 4;
					let jitter_ms = rand::thread_rng().gen_range(0..=max_jitter);
					let jitter = Duration::from_millis(jitter_ms);
					backoff = backoff.saturating_add(jitter);

					// Record failure if this was the last retry
					if attempts >= self.config.max_retries {
						let mut cb = self.circuit_breaker.lock().await;
						cb.record_failure();
						return Err(ResilientClientError::MaxRetriesExceeded);
					}
				}
			}
		}
	}

	/// Execute a single HTTP request without retry logic.
	async fn execute_once(
		&self,
		method: &Method,
		path: &str,
		body: Option<String>,
	) -> Result<Bytes, ResilientClientError> {
		// Build the full URL
		let url = format!("{}{}", self.config.base_url.trim_end_matches('/'), path);
		let uri: Uri = url.parse()?;

		// Build the request
		let mut req = Request::builder().method(method).uri(uri);

		// Add authentication headers
		req = match &self.config.credentials {
			ProviderCredentials::None => req,
			ProviderCredentials::ApiKey { key } => {
				req.header("X-API-Key", key.as_str())
			}
			ProviderCredentials::Bearer { token } => {
				req.header("Authorization", format!("Bearer {}", token))
			}
			ProviderCredentials::Basic { username, password } => {
				let creds = format!("{}:{}", username, password);
				let encoded = base64_encode(&creds);
				req.header("Authorization", format!("Basic {}", encoded))
			}
		};

		// Set body if provided
		let request = if let Some(b) = body {
			req.header("Content-Type", "application/json")
				.body(Full::new(Bytes::from(b)))
				.map_err(|e| ResilientClientError::InvalidRequest(e.to_string()))?
		} else {
			req.body(Full::new(Bytes::new()))
				.map_err(|e| ResilientClientError::InvalidRequest(e.to_string()))?
		};

		// Execute with timeout
		let timeout_future = tokio::time::timeout(
			self.config.timeout(),
			self.client.request(request)
		);

		let response = match timeout_future.await {
			Ok(Ok(resp)) => resp,
			Ok(Err(e)) => return Err(ResilientClientError::HttpError(e.to_string())),
			Err(_) => return Err(ResilientClientError::Timeout),
		};

		// Check status code
		let status = response.status();
		if !status.is_success() {
			return Err(ResilientClientError::HttpStatus(status));
		}

		// Read response body
		let body_bytes = response
			.into_body()
			.collect()
			.await
			.map_err(|e| ResilientClientError::HttpError(e.to_string()))?
			.to_bytes();

		Ok(body_bytes)
	}

	/// Check if an error is retryable.
	fn is_retryable_error(&self, error: &ResilientClientError) -> bool {
		match error {
			ResilientClientError::HttpStatus(status) => {
				// Retry on 5xx server errors and some 4xx errors
				status.is_server_error()
					|| *status == StatusCode::TOO_MANY_REQUESTS
					|| *status == StatusCode::REQUEST_TIMEOUT
			}
			ResilientClientError::Timeout => true,
			ResilientClientError::HttpError(_) => true,
			_ => false,
		}
	}

	/// Get metrics about the client state.
	pub async fn get_metrics(&self) -> ClientMetrics {
		let cb = self.circuit_breaker.lock().await;
		let rl = self.rate_limiter.lock().await;

		ClientMetrics {
			circuit_breaker_open: cb.is_open(),
			failure_count: cb.failure_count,
			available_tokens: rl.tokens as u32,
		}
	}
}

/// Metrics for observability.
#[derive(Debug, Clone)]
pub struct ClientMetrics {
	pub circuit_breaker_open: bool,
	pub failure_count: u32,
	pub available_tokens: u32,
}

/// Builder for ResilientClient.
pub struct ResilientClientBuilder {
	config: ProviderConfig,
}

impl ResilientClientBuilder {
	/// Create a new builder with the given provider configuration.
	pub fn new(config: ProviderConfig) -> Self {
		Self { config }
	}

	/// Build the ResilientClient.
	pub fn build(self) -> ResilientClient {
		let client = Client::builder(TokioExecutor::new())
			.build_http();

		let rate_limiter = Arc::new(Mutex::new(TokenBucket::new(
			self.config.rate_limit_burst,
			self.config.rate_limit_rps,
		)));

		let circuit_breaker = Arc::new(Mutex::new(CircuitBreaker::new(
			self.config.circuit_breaker_threshold,
			self.config.circuit_breaker_timeout(),
		)));

		ResilientClient {
			config: self.config,
			client,
			rate_limiter,
			circuit_breaker,
		}
	}
}

/// Simple base64 encoding for Basic auth (avoiding extra dependency).
fn base64_encode(input: &str) -> String {
	const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
	let bytes = input.as_bytes();
	let mut result = String::new();

	for chunk in bytes.chunks(3) {
		let b1 = chunk[0];
		let b2 = chunk.get(1).copied().unwrap_or(0);
		let b3 = chunk.get(2).copied().unwrap_or(0);

		result.push(CHARSET[(b1 >> 2) as usize] as char);
		result.push(CHARSET[(((b1 & 0x03) << 4) | (b2 >> 4)) as usize] as char);

		if chunk.len() > 1 {
			result.push(CHARSET[(((b2 & 0x0f) << 2) | (b3 >> 6)) as usize] as char);
		} else {
			result.push('=');
		}

		if chunk.len() > 2 {
			result.push(CHARSET[(b3 & 0x3f) as usize] as char);
		} else {
			result.push('=');
		}
	}

	result
}

#[cfg(test)]
#[cfg(feature = "unit-tests")]
mod tests {
	use super::*;

	#[test]
	fn test_circuit_breaker_closed_to_open() {
		let mut cb = CircuitBreaker::new(3, Duration::from_secs(60));
		assert!(cb.can_attempt());

		cb.record_failure();
		assert!(cb.can_attempt());

		cb.record_failure();
		assert!(cb.can_attempt());

		cb.record_failure();
		assert!(!cb.can_attempt());
		assert!(cb.is_open());
	}

	#[test]
	fn test_circuit_breaker_success_resets() {
		let mut cb = CircuitBreaker::new(3, Duration::from_secs(60));

		cb.record_failure();
		cb.record_failure();
		assert_eq!(cb.failure_count, 2);

		cb.record_success();
		assert_eq!(cb.failure_count, 0);
	}

	#[test]
	fn test_token_bucket_rate_limiting() {
		let mut bucket = TokenBucket::new(2, 0);

		assert!(bucket.try_acquire());
		assert!(bucket.try_acquire());
		assert!(!bucket.try_acquire());
	}

	#[test]
	fn test_base64_encode() {
		assert_eq!(base64_encode("hello"), "aGVsbG8=");
		assert_eq!(base64_encode("user:pass"), "dXNlcjpwYXNz");
		assert_eq!(base64_encode("a"), "YQ==");
	}

	#[test]
	fn test_resilient_client_builder() {
		let config = ProviderConfig::default();
		let builder = ResilientClientBuilder::new(config);
		let _client = builder.build();
	}

	#[tokio::test]
	async fn test_client_metrics() {
		let config = ProviderConfig::default();
		let client = ResilientClientBuilder::new(config).build();

		let metrics = client.get_metrics().await;
		assert!(!metrics.circuit_breaker_open);
		assert_eq!(metrics.failure_count, 0);
	}
}
