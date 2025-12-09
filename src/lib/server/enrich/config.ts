/**
 * Provider configuration types and defaults
 */

export interface RateLimitConfig {
	/** Maximum requests per second */
	requestsPerSecond: number;
	/** Maximum burst size (tokens available at once) */
	burstSize: number;
}

export interface RetryConfig {
	/** Maximum number of retry attempts */
	maxAttempts: number;
	/** Initial delay in milliseconds */
	initialDelayMs: number;
	/** Maximum delay in milliseconds */
	maxDelayMs: number;
	/** Backoff multiplier (exponential backoff) */
	backoffMultiplier: number;
}

export interface CircuitBreakerConfig {
	/** Number of failures before opening circuit */
	failureThreshold: number;
	/** Time in milliseconds to wait before attempting reset */
	resetTimeoutMs: number;
	/** Minimum number of requests before checking failure rate */
	minimumRequests: number;
}

export interface ProviderConfig {
	/** Provider name */
	name: string;
	/** API endpoint base URL */
	baseUrl?: string;
	/** API key or authentication token */
	apiKey?: string;
	/** Request timeout in milliseconds */
	timeoutMs: number;
	/** Maximum concurrent requests to this provider */
	concurrency: number;
	/** Rate limiting configuration */
	rateLimit: RateLimitConfig;
	/** Retry configuration */
	retry: RetryConfig;
	/** Circuit breaker configuration */
	circuitBreaker: CircuitBreakerConfig;
	/** Additional provider-specific options */
	options?: Record<string, unknown>;
}

export const DEFAULT_PROVIDER_CONFIG: Omit<ProviderConfig, 'name'> = {
	timeoutMs: 5000,
	concurrency: 5,
	rateLimit: {
		requestsPerSecond: 10,
		burstSize: 20,
	},
	retry: {
		maxAttempts: 3,
		initialDelayMs: 1000,
		maxDelayMs: 30000,
		backoffMultiplier: 2,
	},
	circuitBreaker: {
		failureThreshold: 5,
		resetTimeoutMs: 60000,
		minimumRequests: 10,
	},
};

/**
 * Create a provider configuration with defaults
 */
export function createProviderConfig(
	name: string,
	overrides: Partial<ProviderConfig> = {}
): ProviderConfig {
	return {
		...DEFAULT_PROVIDER_CONFIG,
		name,
		...overrides,
		rateLimit: {
			...DEFAULT_PROVIDER_CONFIG.rateLimit,
			...(overrides.rateLimit || {}),
		},
		retry: {
			...DEFAULT_PROVIDER_CONFIG.retry,
			...(overrides.retry || {}),
		},
		circuitBreaker: {
			...DEFAULT_PROVIDER_CONFIG.circuitBreaker,
			...(overrides.circuitBreaker || {}),
		},
	};
}
