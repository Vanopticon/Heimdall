/**
 * Adapter interface for enrichment providers.
 * All enrichment adapters must implement this interface.
 */

export interface EnrichmentResult<T = unknown> {
	success: boolean;
	data?: T;
	error?: string;
	timestamp: number;
	provider: string;
}

export interface EnrichmentAdapter<TInput = unknown, TOutput = unknown> {
	/**
	 * Unique identifier for the adapter (e.g., 'asn', 'geoip')
	 */
	readonly name: string;

	/**
	 * Provider name (e.g., 'ipapi', 'maxmind', 'team-cymru')
	 */
	readonly provider: string;

	/**
	 * Perform enrichment on the given input
	 * @param input Input data to enrich
	 * @returns Promise resolving to enrichment result
	 */
	enrich(input: TInput): Promise<EnrichmentResult<TOutput>>;

	/**
	 * Check if the adapter is healthy and ready to process requests
	 * @returns Promise resolving to true if healthy, false otherwise
	 */
	healthCheck(): Promise<boolean>;
}

export interface EnrichmentTask<TInput = unknown> {
	id: string;
	type: string;
	input: TInput;
	sourceNodeId?: string;
	priority?: number;
	createdAt: number;
	attempts?: number;
}
