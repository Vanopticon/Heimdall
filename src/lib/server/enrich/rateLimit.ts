/**
 * Token bucket rate limiter implementation
 */

import type { RateLimitConfig } from './config.js';

export class RateLimiter {
	private tokens: number;
	private lastRefill: number;
	private readonly config: RateLimitConfig;

	constructor(config: RateLimitConfig) {
		this.config = config;
		this.tokens = config.burstSize;
		this.lastRefill = Date.now();
	}

	/**
	 * Try to acquire a token for rate limiting
	 * @returns true if token acquired, false if rate limit exceeded
	 */
	tryAcquire(): boolean {
		this.refill();
		if (this.tokens >= 1) {
			this.tokens -= 1;
			return true;
		}
		return false;
	}

	/**
	 * Acquire a token, waiting if necessary
	 * @param maxWaitMs Maximum time to wait in milliseconds
	 * @returns Promise resolving to true if acquired, false if timeout
	 */
	async acquire(maxWaitMs: number = 5000): Promise<boolean> {
		const startTime = Date.now();

		while (Date.now() - startTime < maxWaitMs) {
			if (this.tryAcquire()) {
				return true;
			}

			// Calculate wait time until next token is available
			const tokensPerMs = this.config.requestsPerSecond / 1000;
			const waitMs = Math.min(1 / tokensPerMs, maxWaitMs - (Date.now() - startTime));

			if (waitMs > 0) {
				await new Promise((resolve) => setTimeout(resolve, waitMs));
			}
		}

		return false;
	}

	/**
	 * Refill tokens based on elapsed time
	 */
	private refill(): void {
		const now = Date.now();
		const elapsedMs = now - this.lastRefill;
		const tokensToAdd = (elapsedMs / 1000) * this.config.requestsPerSecond;

		if (tokensToAdd > 0) {
			this.tokens = Math.min(this.tokens + tokensToAdd, this.config.burstSize);
			this.lastRefill = now;
		}
	}

	/**
	 * Get current token count (for testing)
	 */
	getTokens(): number {
		this.refill();
		return this.tokens;
	}

	/**
	 * Reset the rate limiter to full capacity
	 */
	reset(): void {
		this.tokens = this.config.burstSize;
		this.lastRefill = Date.now();
	}
}
