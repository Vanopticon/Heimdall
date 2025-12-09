# Provider Configuration Guide

This document describes how to configure external enrichment providers in Heimdall.

## Overview

Heimdall uses a resilient HTTP client to interact with external enrichment providers. The client includes:

- **Rate Limiting**: Token bucket algorithm to respect provider limits
- **Circuit Breaker**: Automatic failure isolation to prevent cascading failures
- **Retry/Backoff**: Exponential backoff with jitter for transient failures
- **Multiple Auth Types**: Support for API keys, Bearer tokens, and Basic auth

## Configuration Schema

### Provider Config

Each provider is configured with the following fields:

```json
{
	"name": "example-provider",
	"base_url": "https://api.example.com",
	"credentials": {
		"type": "api_key",
		"key": "your-api-key-here"
	},
	"rate_limit_rps": 10,
	"rate_limit_burst": 20,
	"timeout_ms": 30000,
	"max_retries": 3,
	"initial_backoff_ms": 100,
	"max_backoff_ms": 10000,
	"circuit_breaker_threshold": 5,
	"circuit_breaker_timeout_ms": 60000
}
```

### Configuration Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | required | Human-readable provider name |
| `base_url` | string | required | Base URL for the provider's API |
| `credentials` | object | `{"type": "none"}` | Authentication credentials (see below) |
| `rate_limit_rps` | integer | 10 | Maximum requests per second |
| `rate_limit_burst` | integer | 20 | Burst capacity (token bucket size) |
| `timeout_ms` | integer | 30000 | Request timeout in milliseconds |
| `max_retries` | integer | 3 | Maximum number of retry attempts |
| `initial_backoff_ms` | integer | 100 | Initial backoff delay in milliseconds |
| `max_backoff_ms` | integer | 10000 | Maximum backoff delay in milliseconds |
| `circuit_breaker_threshold` | integer | 5 | Failures before opening circuit |
| `circuit_breaker_timeout_ms` | integer | 60000 | Time before attempting to close circuit |

## Authentication Types

### No Authentication

```json
{
	"credentials": {
		"type": "none"
	}
}
```

### API Key

API keys are sent as the `X-API-Key` header.

```json
{
	"credentials": {
		"type": "api_key",
		"key": "your-api-key-here"
	}
}
```

### Bearer Token

Bearer tokens are sent in the `Authorization` header.

```json
{
	"credentials": {
		"type": "bearer",
		"token": "your-bearer-token-here"
	}
}
```

### Basic Authentication

Username and password are base64-encoded and sent in the `Authorization` header.

```json
{
	"credentials": {
		"type": "basic",
		"username": "your-username",
		"password": "your-password"
	}
}
```

## Rate Limiting

The client uses a token bucket algorithm to enforce rate limits:

- **Rate Limit RPS**: Tokens are refilled at this rate per second
- **Burst Capacity**: Maximum number of tokens available at once
- Requests consume one token each
- If no tokens are available, the client returns `RateLimitExceeded` error

### Example

With `rate_limit_rps: 10` and `rate_limit_burst: 20`:

- You can make 20 requests immediately (burst)
- After that, you can make 10 requests per second
- If you pause, the bucket refills up to 20 tokens

## Circuit Breaker

The circuit breaker prevents cascading failures by automatically stopping requests to failing providers.

### States

1. **Closed** (normal operation)
	- Requests are allowed
	- Failures increment the failure counter
	- Once threshold is reached, transitions to Open

2. **Open** (provider is failing)
	- All requests immediately return `CircuitBreakerOpen` error
	- After timeout period, transitions to Half-Open

3. **Half-Open** (testing recovery)
	- One request is allowed as a test
	- On success: transitions back to Closed
	- On failure: transitions back to Open

### Configuration

- `circuit_breaker_threshold`: Number of consecutive failures before opening
- `circuit_breaker_timeout_ms`: Time to wait before attempting recovery

## Retry and Backoff

The client automatically retries failed requests with exponential backoff.

### Retryable Errors

- 5xx server errors
- 429 Too Many Requests
- 408 Request Timeout
- Network errors (connection refused, timeout)

### Non-Retryable Errors

- 4xx client errors (except 408, 429)
- Circuit breaker open
- Rate limit exceeded

### Backoff Algorithm

1. Start with `initial_backoff_ms`
2. After each retry: `backoff = min(backoff * 2, max_backoff_ms)`
3. Add jitter: `backoff += random(0, backoff / 4)`
4. Sleep for backoff duration
5. Retry request

## Usage Example

```rust
use vanopticon_heimdall::enrich::{ProviderConfig, ResilientClientBuilder, ProviderCredentials};

// Configure the provider
let config = ProviderConfig {
    name: "geoip-provider".to_string(),
    base_url: "https://api.geoip.example.com".to_string(),
    credentials: ProviderCredentials::ApiKey {
        key: "your-api-key".to_string(),
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

// Build the client
let client = ResilientClientBuilder::new(config).build();

// Make requests
match client.get("/lookup?ip=1.2.3.4").await {
    Ok(response) => {
        // Handle successful response
        println!("Response: {:?}", response);
    }
    Err(e) => {
        // Handle error
        eprintln!("Request failed: {:?}", e);
    }
}

// Get metrics for observability
let metrics = client.get_metrics().await;
println!("Circuit breaker open: {}", metrics.circuit_breaker_open);
println!("Failure count: {}", metrics.failure_count);
println!("Available tokens: {}", metrics.available_tokens);
```

## Observability

The client exposes metrics for monitoring:

```rust
pub struct ClientMetrics {
    pub circuit_breaker_open: bool,
    pub failure_count: u32,
    pub available_tokens: u32,
}
```

These metrics can be integrated with your monitoring system to track:

- Circuit breaker state changes
- Failure rates
- Rate limit usage
- Provider health

## Best Practices

1. **Set Conservative Rate Limits**: Start with lower limits and increase gradually
2. **Monitor Circuit Breaker**: Alert when circuits open frequently
3. **Configure Appropriate Timeouts**: Balance responsiveness with provider SLAs
4. **Use Environment Variables**: Store credentials in env vars, not config files
5. **Test Configurations**: Use integration tests to validate provider configs
6. **Log Retries**: Monitor retry patterns to identify problematic providers

## Security Considerations

- Never commit credentials to version control
- Use environment variables or secret management systems
- Rotate API keys regularly
- Use TLS/HTTPS for all provider connections
- Monitor for credential leaks in logs
