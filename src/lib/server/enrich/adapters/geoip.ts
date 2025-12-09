/**
 * GeoIP lookup adapter (sample implementation with mock provider)
 */

import type { EnrichmentAdapter, EnrichmentResult } from '../adapter.js';

export interface GeoIPInput {
	ipAddress: string;
}

export interface GeoIPOutput {
	country: string;
	countryCode: string;
	city?: string;
	latitude?: number;
	longitude?: number;
	timezone?: string;
}

/**
 * Mock GeoIP lookup adapter for testing
 * In production, this would call a real GeoIP service (e.g., MaxMind, IP2Location)
 */
export class MockGeoIPAdapter implements EnrichmentAdapter<GeoIPInput, GeoIPOutput> {
	readonly name = 'geoip';
	readonly provider = 'mock';
	private shouldFail = false;

	/**
	 * Set whether the adapter should simulate failures (for testing)
	 */
	setShouldFail(shouldFail: boolean): void {
		this.shouldFail = shouldFail;
	}

	async enrich(input: GeoIPInput): Promise<EnrichmentResult<GeoIPOutput>> {
		if (this.shouldFail) {
			return {
				success: false,
				error: 'Mock failure',
				timestamp: Date.now(),
				provider: this.provider,
			};
		}

		// Simulate network delay
		await new Promise((resolve) => setTimeout(resolve, 50));

		// Mock GeoIP data based on IP address
		const geo = this.mockGeoIPLookup(input.ipAddress);

		return {
			success: true,
			data: geo,
			timestamp: Date.now(),
			provider: this.provider,
		};
	}

	async healthCheck(): Promise<boolean> {
		return !this.shouldFail;
	}

	/**
	 * Mock GeoIP lookup - in production this would query a real service
	 */
	private mockGeoIPLookup(ipAddress: string): GeoIPOutput {
		// Simple mock logic for testing
		const hash = ipAddress.split('.').reduce((acc, octet) => acc + parseInt(octet), 0);

		const locations = [
			{
				country: 'United States',
				countryCode: 'US',
				city: 'New York',
				latitude: 40.7128,
				longitude: -74.006,
				timezone: 'America/New_York',
			},
			{
				country: 'United Kingdom',
				countryCode: 'GB',
				city: 'London',
				latitude: 51.5074,
				longitude: -0.1278,
				timezone: 'Europe/London',
			},
			{
				country: 'Germany',
				countryCode: 'DE',
				city: 'Berlin',
				latitude: 52.52,
				longitude: 13.405,
				timezone: 'Europe/Berlin',
			},
			{
				country: 'Japan',
				countryCode: 'JP',
				city: 'Tokyo',
				latitude: 35.6762,
				longitude: 139.6503,
				timezone: 'Asia/Tokyo',
			},
			{
				country: 'Canada',
				countryCode: 'CA',
				city: 'Toronto',
				latitude: 43.6532,
				longitude: -79.3832,
				timezone: 'America/Toronto',
			},
		];

		return locations[hash % locations.length] || locations[0]!;
	}
}
