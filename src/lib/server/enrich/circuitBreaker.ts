/**
 * Circuit breaker implementation for resilient API calls
 */

import type { CircuitBreakerConfig } from './config.js';

export enum CircuitState {
	CLOSED = 'closed',
	OPEN = 'open',
	HALF_OPEN = 'half_open',
}

export class CircuitBreaker {
	private state: CircuitState = CircuitState.CLOSED;
	private failureCount: number = 0;
	private successCount: number = 0;
	private requestCount: number = 0;
	private lastFailureTime: number = 0;
	private readonly config: CircuitBreakerConfig;

	constructor(config: CircuitBreakerConfig) {
		this.config = config;
	}

	/**
	 * Check if a request can proceed
	 * @returns true if request allowed, false if circuit is open
	 */
	canProceed(): boolean {
		if (this.state === CircuitState.CLOSED) {
			return true;
		}

		if (this.state === CircuitState.OPEN) {
			// Check if we should transition to half-open
			const timeSinceLastFailure = Date.now() - this.lastFailureTime;
			if (timeSinceLastFailure >= this.config.resetTimeoutMs) {
				this.state = CircuitState.HALF_OPEN;
				this.failureCount = 0;
				this.successCount = 0;
				this.requestCount = 0;
				return true;
			}
			return false;
		}

		// HALF_OPEN state - allow one request to test
		return true;
	}

	/**
	 * Record a successful request
	 */
	recordSuccess(): void {
		this.successCount += 1;
		this.requestCount += 1;

		if (this.state === CircuitState.HALF_OPEN) {
			// Success in half-open state transitions to closed
			this.state = CircuitState.CLOSED;
			this.failureCount = 0;
			this.successCount = 0;
			this.requestCount = 0;
		}
	}

	/**
	 * Record a failed request
	 */
	recordFailure(): void {
		this.failureCount += 1;
		this.requestCount += 1;
		this.lastFailureTime = Date.now();

		if (this.state === CircuitState.HALF_OPEN) {
			// Failure in half-open state transitions back to open
			this.state = CircuitState.OPEN;
			return;
		}

		// Check if we should open the circuit
		if (
			this.requestCount >= this.config.minimumRequests &&
			this.failureCount >= this.config.failureThreshold
		) {
			this.state = CircuitState.OPEN;
		}
	}

	/**
	 * Get current circuit state
	 */
	getState(): CircuitState {
		// Update state if needed (e.g., transition from open to half-open)
		this.canProceed();
		return this.state;
	}

	/**
	 * Get failure statistics
	 */
	getStats(): {
		state: CircuitState;
		failureCount: number;
		successCount: number;
		requestCount: number;
	} {
		return {
			state: this.getState(),
			failureCount: this.failureCount,
			successCount: this.successCount,
			requestCount: this.requestCount,
		};
	}

	/**
	 * Reset the circuit breaker to closed state
	 */
	reset(): void {
		this.state = CircuitState.CLOSED;
		this.failureCount = 0;
		this.successCount = 0;
		this.requestCount = 0;
		this.lastFailureTime = 0;
	}
}
