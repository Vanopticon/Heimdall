/**
 * Scheduler for enrichment tasks
 */

import type { EnrichmentTask } from './adapter.js';
import type { WorkerPool } from './workerPool.js';

export interface SchedulerConfig {
	/** Polling interval in milliseconds */
	pollingIntervalMs: number;
	/** Maximum tasks to fetch per poll */
	batchSize: number;
	/** Enable/disable scheduler */
	enabled: boolean;
}

export const DEFAULT_SCHEDULER_CONFIG: SchedulerConfig = {
	pollingIntervalMs: 5000,
	batchSize: 10,
	enabled: true,
};

/**
 * Scheduler that polls for pending enrichment tasks and dispatches to worker pool
 */
export class Scheduler {
	private readonly workerPool: WorkerPool;
	private readonly config: SchedulerConfig;
	private intervalId?: NodeJS.Timeout;
	private isRunning = false;
	private taskProvider?: () => Promise<EnrichmentTask[]>;

	constructor(workerPool: WorkerPool, config: SchedulerConfig = DEFAULT_SCHEDULER_CONFIG) {
		this.workerPool = workerPool;
		this.config = config;
	}

	/**
	 * Set the task provider function that fetches pending tasks
	 * @param provider Function that returns a promise of enrichment tasks
	 */
	setTaskProvider(provider: () => Promise<EnrichmentTask[]>): void {
		this.taskProvider = provider;
	}

	/**
	 * Start the scheduler
	 */
	start(): void {
		if (this.isRunning) {
			return;
		}

		if (!this.config.enabled) {
			return;
		}

		if (!this.taskProvider) {
			throw new Error('Task provider must be set before starting scheduler');
		}

		this.isRunning = true;
		this.intervalId = setInterval(() => {
			this.poll().catch((error) => {
				console.error('Scheduler poll error:', error);
			});
		}, this.config.pollingIntervalMs);

		// Run first poll immediately
		this.poll().catch((error) => {
			console.error('Initial scheduler poll error:', error);
		});
	}

	/**
	 * Stop the scheduler
	 */
	stop(): void {
		if (!this.isRunning) {
			return;
		}

		this.isRunning = false;
		if (this.intervalId) {
			clearInterval(this.intervalId);
			this.intervalId = undefined;
		}
	}

	/**
	 * Poll for pending tasks and submit to worker pool
	 */
	private async poll(): Promise<void> {
		if (!this.taskProvider || !this.isRunning) {
			return;
		}

		try {
			const tasks = await this.taskProvider();
			const batch = tasks.slice(0, this.config.batchSize);

			// Submit tasks to worker pool (fire and forget)
			for (const task of batch) {
				this.workerPool
					.submitTask(task)
					.then((result) => {
						if (!result.success) {
							console.warn(
								`Enrichment task ${task.id} failed: ${result.error}`
							);
						}
					})
					.catch((error) => {
						console.error(`Error processing task ${task.id}:`, error);
					});
			}
		} catch (error) {
			console.error('Error fetching tasks:', error);
		}
	}

	/**
	 * Check if scheduler is running
	 */
	isActive(): boolean {
		return this.isRunning;
	}
}
