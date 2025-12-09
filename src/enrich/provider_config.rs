use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Credentials for authenticating with external enrichment providers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProviderCredentials {
	/// No authentication required
	None,
	/// API key authentication (typically sent as header or query parameter)
	ApiKey { key: String },
	/// Basic authentication with username and password
	Basic { username: String, password: String },
	/// Bearer token authentication
	Bearer { token: String },
}

impl Default for ProviderCredentials {
	fn default() -> Self {
		Self::None
	}
}

/// Configuration for a single external enrichment provider.
///
/// This structure defines rate limits, timeouts, retry behavior, and
/// authentication credentials for interacting with an external API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
	/// Human-readable name for the provider
	pub name: String,

	/// Base URL for the provider's API
	pub base_url: String,

	/// Authentication credentials
	#[serde(default)]
	pub credentials: ProviderCredentials,

	/// Rate limit: maximum requests per second
	#[serde(default = "default_rate_limit_rps")]
	pub rate_limit_rps: u32,

	/// Rate limit: burst capacity (max tokens)
	#[serde(default = "default_rate_limit_burst")]
	pub rate_limit_burst: u32,

	/// Request timeout in milliseconds
	#[serde(default = "default_timeout_ms")]
	pub timeout_ms: u64,

	/// Maximum number of retry attempts
	#[serde(default = "default_max_retries")]
	pub max_retries: u32,

	/// Initial backoff delay in milliseconds
	#[serde(default = "default_initial_backoff_ms")]
	pub initial_backoff_ms: u64,

	/// Maximum backoff delay in milliseconds
	#[serde(default = "default_max_backoff_ms")]
	pub max_backoff_ms: u64,

	/// Circuit breaker: failure threshold before opening circuit
	#[serde(default = "default_circuit_breaker_threshold")]
	pub circuit_breaker_threshold: u32,

	/// Circuit breaker: timeout before attempting to close circuit (in milliseconds)
	#[serde(default = "default_circuit_breaker_timeout_ms")]
	pub circuit_breaker_timeout_ms: u64,
}

impl Default for ProviderConfig {
	fn default() -> Self {
		Self {
			name: "default".to_string(),
			base_url: "http://localhost".to_string(),
			credentials: ProviderCredentials::None,
			rate_limit_rps: default_rate_limit_rps(),
			rate_limit_burst: default_rate_limit_burst(),
			timeout_ms: default_timeout_ms(),
			max_retries: default_max_retries(),
			initial_backoff_ms: default_initial_backoff_ms(),
			max_backoff_ms: default_max_backoff_ms(),
			circuit_breaker_threshold: default_circuit_breaker_threshold(),
			circuit_breaker_timeout_ms: default_circuit_breaker_timeout_ms(),
		}
	}
}

impl ProviderConfig {
	/// Get the timeout as a Duration
	pub fn timeout(&self) -> Duration {
		Duration::from_millis(self.timeout_ms)
	}

	/// Get the initial backoff as a Duration
	pub fn initial_backoff(&self) -> Duration {
		Duration::from_millis(self.initial_backoff_ms)
	}

	/// Get the maximum backoff as a Duration
	pub fn max_backoff(&self) -> Duration {
		Duration::from_millis(self.max_backoff_ms)
	}

	/// Get the circuit breaker timeout as a Duration
	pub fn circuit_breaker_timeout(&self) -> Duration {
		Duration::from_millis(self.circuit_breaker_timeout_ms)
	}
}

// Default value functions for serde
fn default_rate_limit_rps() -> u32 {
	10
}

fn default_rate_limit_burst() -> u32 {
	20
}

fn default_timeout_ms() -> u64 {
	30_000 // 30 seconds
}

fn default_max_retries() -> u32 {
	3
}

fn default_initial_backoff_ms() -> u64 {
	100
}

fn default_max_backoff_ms() -> u64 {
	10_000 // 10 seconds
}

fn default_circuit_breaker_threshold() -> u32 {
	5
}

fn default_circuit_breaker_timeout_ms() -> u64 {
	60_000 // 60 seconds
}

#[cfg(test)]
#[cfg(feature = "unit-tests")]
mod tests {
	use super::*;

	#[test]
	fn test_provider_config_default() {
		let config = ProviderConfig::default();
		assert_eq!(config.name, "default");
		assert_eq!(config.rate_limit_rps, 10);
		assert_eq!(config.rate_limit_burst, 20);
		assert_eq!(config.timeout_ms, 30_000);
		assert_eq!(config.max_retries, 3);
		assert!(matches!(config.credentials, ProviderCredentials::None));
	}

	#[test]
	fn test_provider_config_serialize() {
		let config = ProviderConfig {
			name: "test-provider".to_string(),
			base_url: "https://api.example.com".to_string(),
			credentials: ProviderCredentials::ApiKey {
				key: "secret-key".to_string(),
			},
			rate_limit_rps: 5,
			rate_limit_burst: 10,
			timeout_ms: 5000,
			max_retries: 2,
			initial_backoff_ms: 200,
			max_backoff_ms: 5000,
			circuit_breaker_threshold: 3,
			circuit_breaker_timeout_ms: 30_000,
		};

		let json = serde_json::to_string(&config).expect("should serialize");
		assert!(json.contains("test-provider"));
		assert!(json.contains("api_key"));
	}

	#[test]
	fn test_provider_config_deserialize() {
		let json = r#"{
			"name": "test-provider",
			"base_url": "https://api.example.com",
			"credentials": {"type": "api_key", "key": "secret"},
			"rate_limit_rps": 5,
			"rate_limit_burst": 10,
			"timeout_ms": 5000
		}"#;

		let config: ProviderConfig = serde_json::from_str(json).expect("should deserialize");
		assert_eq!(config.name, "test-provider");
		assert_eq!(config.rate_limit_rps, 5);
		if let ProviderCredentials::ApiKey { key } = &config.credentials {
			assert_eq!(key, "secret");
		} else {
			panic!("Expected ApiKey credentials");
		}
	}

	#[test]
	fn test_provider_credentials_variants() {
		let none = ProviderCredentials::None;
		let json = serde_json::to_string(&none).unwrap();
		assert!(json.contains("none"));

		let bearer = ProviderCredentials::Bearer {
			token: "token123".to_string(),
		};
		let json = serde_json::to_string(&bearer).unwrap();
		assert!(json.contains("bearer"));

		let basic = ProviderCredentials::Basic {
			username: "user".to_string(),
			password: "pass".to_string(),
		};
		let json = serde_json::to_string(&basic).unwrap();
		assert!(json.contains("basic"));
	}

	#[test]
	fn test_provider_config_duration_helpers() {
		let config = ProviderConfig::default();
		assert_eq!(config.timeout(), Duration::from_millis(30_000));
		assert_eq!(config.initial_backoff(), Duration::from_millis(100));
		assert_eq!(config.max_backoff(), Duration::from_millis(10_000));
		assert_eq!(
			config.circuit_breaker_timeout(),
			Duration::from_millis(60_000)
		);
	}
}
