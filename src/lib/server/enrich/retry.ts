/**
 * Retry logic with exponential backoff
 */

import type { RetryConfig } from './config.js';

export class RetryError extends Error {
	constructor(
		message: string,
		public readonly attempts: number,
		public readonly lastError?: Error
	) {
		super(message);
		this.name = 'RetryError';
	}
}

/**
 * Execute a function with retry logic and exponential backoff
 * @param fn Function to execute
 * @param config Retry configuration
 * @returns Promise resolving to function result
 * @throws RetryError if all attempts fail
 */
export async function withRetry<T>(
	fn: () => Promise<T>,
	config: RetryConfig
): Promise<T> {
	let lastError: Error | undefined;
	let delay = config.initialDelayMs;

	for (let attempt = 1; attempt <= config.maxAttempts; attempt++) {
		try {
			return await fn();
		} catch (error) {
			lastError = error instanceof Error ? error : new Error(String(error));

			// Don't retry on last attempt
			if (attempt >= config.maxAttempts) {
				break;
			}

			// Wait before next attempt with exponential backoff
			await sleep(delay);
			delay = Math.min(delay * config.backoffMultiplier, config.maxDelayMs);
		}
	}

	throw new RetryError(
		`Failed after ${config.maxAttempts} attempts`,
		config.maxAttempts,
		lastError
	);
}

/**
 * Calculate the delay for a given attempt with exponential backoff
 * @param attempt Attempt number (1-indexed)
 * @param config Retry configuration
 * @returns Delay in milliseconds
 */
export function calculateBackoff(attempt: number, config: RetryConfig): number {
	if (attempt <= 1) {
		return config.initialDelayMs;
	}

	const delay = config.initialDelayMs * Math.pow(config.backoffMultiplier, attempt - 1);
	return Math.min(delay, config.maxDelayMs);
}

/**
 * Sleep for a given duration
 * @param ms Duration in milliseconds
 */
function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}
