/**
 * Worker pool for managing concurrent enrichment tasks
 */

import type { EnrichmentAdapter, EnrichmentResult, EnrichmentTask } from './adapter.js';
import type { ProviderConfig } from './config.js';
import { RateLimiter } from './rateLimit.js';
import { CircuitBreaker } from './circuitBreaker.js';
import { withRetry } from './retry.js';

interface WorkerPoolTask {
	task: EnrichmentTask;
	adapter: EnrichmentAdapter;
	resolve: (result: EnrichmentResult) => void;
	reject: (error: Error) => void;
}

export class WorkerPool {
	private readonly adapters: Map<string, EnrichmentAdapter> = new Map();
	private readonly configs: Map<string, ProviderConfig> = new Map();
	private readonly rateLimiters: Map<string, RateLimiter> = new Map();
	private readonly circuitBreakers: Map<string, CircuitBreaker> = new Map();
	private readonly queue: WorkerPoolTask[] = [];
	private readonly activeWorkers: Set<string> = new Set();
	private isShuttingDown = false;

	constructor() {}

	/**
	 * Register an adapter with its configuration
	 */
	registerAdapter(adapter: EnrichmentAdapter, config: ProviderConfig): void {
		const key = `${adapter.name}:${adapter.provider}`;
		this.adapters.set(key, adapter);
		this.configs.set(key, config);
		this.rateLimiters.set(key, new RateLimiter(config.rateLimit));
		this.circuitBreakers.set(key, new CircuitBreaker(config.circuitBreaker));
	}

	/**
	 * Submit a task for enrichment
	 */
	async submitTask(task: EnrichmentTask): Promise<EnrichmentResult> {
		if (this.isShuttingDown) {
			throw new Error('Worker pool is shutting down');
		}

		const adapter = this.adapters.get(task.type);
		if (!adapter) {
			return {
				success: false,
				error: `No adapter registered for type: ${task.type}`,
				timestamp: Date.now(),
				provider: 'unknown',
			};
		}

		return new Promise((resolve, reject) => {
			this.queue.push({ task, adapter, resolve, reject });
			this.processQueue();
		});
	}

	/**
	 * Process queued tasks
	 */
	private async processQueue(): Promise<void> {
		while (this.queue.length > 0 && !this.isShuttingDown) {
			const item = this.queue[0];
			if (!item) break;

			const key = `${item.adapter.name}:${item.adapter.provider}`;
			const config = this.configs.get(key);
			if (!config) {
				this.queue.shift();
				item.reject(new Error(`No configuration for adapter: ${key}`));
				continue;
			}

			// Check concurrency limit
			const activeCount = Array.from(this.activeWorkers).filter((w) =>
				w.startsWith(key)
			).length;
			if (activeCount >= config.concurrency) {
				break;
			}

			// Remove from queue and process
			this.queue.shift();
			this.processTask(item, key, config).catch((error) => {
				item.reject(error instanceof Error ? error : new Error(String(error)));
			});
		}
	}

	/**
	 * Process a single task
	 */
	private async processTask(
		item: WorkerPoolTask,
		key: string,
		config: ProviderConfig
	): Promise<void> {
		const workerId = `${key}:${Date.now()}:${Math.random()}`;
		this.activeWorkers.add(workerId);

		try {
			const rateLimiter = this.rateLimiters.get(key);
			const circuitBreaker = this.circuitBreakers.get(key);

			if (!rateLimiter || !circuitBreaker) {
				throw new Error(`Missing rate limiter or circuit breaker for ${key}`);
			}

			// Check circuit breaker
			if (!circuitBreaker.canProceed()) {
				item.resolve({
					success: false,
					error: 'Circuit breaker is open',
					timestamp: Date.now(),
					provider: item.adapter.provider,
				});
				return;
			}

			// Acquire rate limit token
			const acquired = await rateLimiter.acquire(5000);
			if (!acquired) {
				item.resolve({
					success: false,
					error: 'Rate limit exceeded',
					timestamp: Date.now(),
					provider: item.adapter.provider,
				});
				return;
			}

			// Execute with retry
			const result = await withRetry(
				async () => {
					const timeoutPromise = new Promise<never>((_, reject) =>
						setTimeout(() => reject(new Error('Timeout')), config.timeoutMs)
					);

					const enrichPromise = item.adapter.enrich(item.task.input);

					return Promise.race([enrichPromise, timeoutPromise]);
				},
				config.retry
			);

			circuitBreaker.recordSuccess();
			item.resolve(result);
		} catch (error) {
			const circuitBreaker = this.circuitBreakers.get(key);
			circuitBreaker?.recordFailure();

			item.resolve({
				success: false,
				error: error instanceof Error ? error.message : String(error),
				timestamp: Date.now(),
				provider: item.adapter.provider,
			});
		} finally {
			this.activeWorkers.delete(workerId);
			// Process next item in queue
			this.processQueue();
		}
	}

	/**
	 * Get statistics about the worker pool
	 */
	getStats(): {
		queueLength: number;
		activeWorkers: number;
		adapters: number;
	} {
		return {
			queueLength: this.queue.length,
			activeWorkers: this.activeWorkers.size,
			adapters: this.adapters.size,
		};
	}

	/**
	 * Gracefully shutdown the worker pool
	 */
	async shutdown(): Promise<void> {
		this.isShuttingDown = true;

		// Wait for active workers to complete (with timeout)
		const maxWaitMs = 30000;
		const startTime = Date.now();

		while (this.activeWorkers.size > 0 && Date.now() - startTime < maxWaitMs) {
			await new Promise((resolve) => setTimeout(resolve, 100));
		}

		// Clear queue
		this.queue.length = 0;
	}
}
