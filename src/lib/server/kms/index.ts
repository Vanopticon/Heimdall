/**
 * KMS (Key Management Service) Integration Module
 *
 * This module provides pluggable KMS integration for envelope encryption and key management.
 * It defines the interface for KMS operations and provides factory functions for creating
 * KMS clients based on configuration.
 *
 * Envelope encryption flow:
 * 1. Generate a Data Encryption Key (DEK) for encrypting sensitive data
 * 2. Encrypt the data with the DEK using AES-256-GCM
 * 3. Encrypt the DEK with a Key Encryption Key (KEK) from the KMS
 * 4. Store encrypted data + encrypted DEK in the database
 * 5. To decrypt: retrieve encrypted DEK, decrypt with KMS, decrypt data with DEK
 */

/**
 * Encryption context for additional authenticated data (AAD).
 * Provides additional security by binding encrypted data to specific context.
 */
export interface EncryptionContext {
	[key: string]: string;
}

/**
 * Data key pair containing both plaintext and encrypted versions.
 * Used for envelope encryption where the plaintext key encrypts data
 * and the encrypted key is stored alongside the encrypted data.
 */
export interface DataKey {
	/** Plaintext DEK for immediate use (should be securely erased after use) */
	plaintext: Buffer;
	/** Encrypted DEK to be stored with encrypted data */
	encrypted: Buffer;
}

/**
 * KMS Provider interface defining operations for key management and envelope encryption.
 * Implementations should support various KMS backends (AWS KMS, GCP KMS, HashiCorp Vault, etc.)
 */
export interface KMSProvider {
	/**
	 * Encrypt a plaintext buffer using the configured KEK.
	 * Used to encrypt Data Encryption Keys (DEKs) in envelope encryption.
	 *
	 * @param plaintext - The plaintext to encrypt (typically a DEK)
	 * @param context - Optional encryption context for additional authenticated data
	 * @returns Promise resolving to encrypted ciphertext
	 */
	encrypt(plaintext: Buffer, context?: EncryptionContext): Promise<Buffer>;

	/**
	 * Decrypt a ciphertext buffer using the configured KEK.
	 * Used to decrypt Data Encryption Keys (DEKs) in envelope encryption.
	 *
	 * @param ciphertext - The ciphertext to decrypt (typically an encrypted DEK)
	 * @param context - Optional encryption context (must match context used during encryption)
	 * @returns Promise resolving to decrypted plaintext
	 */
	decrypt(ciphertext: Buffer, context?: EncryptionContext): Promise<Buffer>;

	/**
	 * Generate a new Data Encryption Key (DEK) and return both plaintext and encrypted versions.
	 * This is the primary method for envelope encryption workflows.
	 *
	 * The plaintext DEK should be used immediately to encrypt data, then securely erased.
	 * The encrypted DEK should be stored alongside the encrypted data.
	 *
	 * @param context - Optional encryption context for the DEK
	 * @returns Promise resolving to DataKey containing plaintext and encrypted DEK
	 */
	generateDataKey(context?: EncryptionContext): Promise<DataKey>;

	/**
	 * Verify KMS connectivity and permissions.
	 * Should be called during application startup to fail fast if KMS is unavailable.
	 *
	 * @returns Promise resolving to true if KMS is accessible and operational
	 * @throws Error if KMS is unavailable or credentials are invalid
	 */
	healthCheck(): Promise<boolean>;
}

/**
 * KMS configuration interface.
 * Different KMS providers require different configuration options.
 */
export interface KMSConfig {
	/** KMS provider type (aws, gcp, vault, etc.) */
	provider: string;
	/** Key ID or ARN for the Key Encryption Key (KEK) */
	keyId: string;
	/** Optional region (for cloud providers) */
	region?: string;
	/** Optional endpoint URL (for custom/local KMS endpoints) */
	endpoint?: string;
	/** Provider-specific configuration options */
	[key: string]: unknown;
}

/**
 * Factory function to create a KMS provider instance based on configuration.
 *
 * @param config - KMS configuration
 * @returns KMS provider instance
 * @throws Error if provider is not supported or configuration is invalid
 */
export function createKMSProvider(config: KMSConfig): KMSProvider {
	switch (config.provider.toLowerCase()) {
		case 'aws':
		case 'aws-kms':
			// Lazy-load AWS KMS provider
			// eslint-disable-next-line @typescript-eslint/no-require-imports
			const { AWSKMSProvider } = require('./providers/aws');
			return new AWSKMSProvider(config);

		case 'gcp':
		case 'gcp-kms':
			// Lazy-load GCP KMS provider
			// eslint-disable-next-line @typescript-eslint/no-require-imports
			const { GCPKMSProvider } = require('./providers/gcp');
			return new GCPKMSProvider(config);

		case 'vault':
		case 'hashicorp-vault':
			// Lazy-load Vault provider
			// eslint-disable-next-line @typescript-eslint/no-require-imports
			const { VaultKMSProvider } = require('./providers/vault');
			return new VaultKMSProvider(config);

		case 'local':
		case 'local-kms':
			// Lazy-load local KMS provider (for development/testing only)
			// eslint-disable-next-line @typescript-eslint/no-require-imports
			const { LocalKMSProvider } = require('./providers/local');
			return new LocalKMSProvider(config);

		default:
			throw new Error(
				`Unsupported KMS provider: ${config.provider}. Supported providers: aws, gcp, vault, local`,
			);
	}
}

/**
 * Load KMS configuration from environment variables.
 * Follows the HMD_* environment variable naming convention.
 *
 * Environment variables:
 * - HMD_KMS_PROVIDER: KMS provider type (aws, gcp, vault, local)
 * - HMD_KMS_KEY_ID: Key ID or ARN for the KEK
 * - HMD_KMS_REGION: Region (for cloud providers)
 * - HMD_KMS_ENDPOINT: Custom endpoint URL (optional)
 *
 * @returns KMS configuration object
 * @throws Error if required configuration is missing
 */
export function loadKMSConfig(): KMSConfig {
	const provider = process.env.HMD_KMS_PROVIDER;
	const keyId = process.env.HMD_KMS_KEY_ID;

	if (!provider) {
		throw new Error('HMD_KMS_PROVIDER environment variable is required');
	}

	if (!keyId) {
		throw new Error('HMD_KMS_KEY_ID environment variable is required');
	}

	return {
		provider,
		keyId,
		region: process.env.HMD_KMS_REGION,
		endpoint: process.env.HMD_KMS_ENDPOINT,
	};
}
