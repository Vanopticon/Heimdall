/**
 * ASN lookup adapter (sample implementation with mock provider)
 */

import type { EnrichmentAdapter, EnrichmentResult } from '../adapter.js';

export interface ASNInput {
	ipAddress: string;
}

export interface ASNOutput {
	asn: number;
	organization: string;
	country?: string;
	registrationDate?: string;
}

/**
 * Mock ASN lookup adapter for testing
 * In production, this would call a real ASN lookup service (e.g., Team Cymru, IPInfo)
 */
export class MockASNAdapter implements EnrichmentAdapter<ASNInput, ASNOutput> {
	readonly name = 'asn';
	readonly provider = 'mock';
	private shouldFail = false;

	/**
	 * Set whether the adapter should simulate failures (for testing)
	 */
	setShouldFail(shouldFail: boolean): void {
		this.shouldFail = shouldFail;
	}

	async enrich(input: ASNInput): Promise<EnrichmentResult<ASNOutput>> {
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

		// Mock ASN data based on IP address
		const asn = this.mockASNLookup(input.ipAddress);

		return {
			success: true,
			data: asn,
			timestamp: Date.now(),
			provider: this.provider,
		};
	}

	async healthCheck(): Promise<boolean> {
		return !this.shouldFail;
	}

	/**
	 * Mock ASN lookup - in production this would query a real service
	 */
	private mockASNLookup(ipAddress: string): ASNOutput {
		// Simple mock logic for testing
		const hash = ipAddress.split('.').reduce((acc, octet) => acc + parseInt(octet), 0);
		const asnNumber = 64512 + (hash % 1000); // Use private ASN range for mocking

		const organizations = [
			'Example Corp',
			'Test Networks Inc',
			'Mock ISP',
			'Sample Telecom',
			'Demo Hosting',
		];

		const countries = ['US', 'GB', 'DE', 'JP', 'CA'];

		return {
			asn: asnNumber,
			organization: organizations[hash % organizations.length] || 'Unknown',
			country: countries[hash % countries.length],
			registrationDate: '2020-01-01',
		};
	}
}
