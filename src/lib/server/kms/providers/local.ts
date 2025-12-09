import crypto from 'crypto';
import type { KMSProvider, DataKey, EncryptionContext, KMSConfig } from '../index';

/**
 * Local KMS Provider - FOR DEVELOPMENT AND TESTING ONLY
 *
 * This provider implements the KMS interface using local cryptography without
 * external KMS service. It is suitable for development and testing but should
 * NEVER be used in production as it does not provide the same security guarantees.
 *
 * Security considerations:
 * - Key material is stored in memory or local files
 * - No hardware security module (HSM) backing
 * - No audit logging
 * - No automatic key rotation
 * - No access control beyond file system permissions
 *
 * For production, use AWS KMS, GCP KMS, or HashiCorp Vault providers.
 */
export class LocalKMSProvider implements KMSProvider {
	private kek: Buffer;
	private keyId: string;

	/**
	 * Create a LocalKMSProvider instance.
	 *
	 * @param config - KMS configuration
	 *   - keyId: Path to key file or "generate" to create a new key
	 *   - For testing, keyId can be a base64-encoded key
	 */
	constructor(config: KMSConfig) {
		this.keyId = config.keyId;

		// Load or generate KEK
		if (config.keyId === 'generate') {
			// Generate a new random KEK
			this.kek = crypto.randomBytes(32); // 256-bit key
			console.warn(
				'[LocalKMSProvider] Generated ephemeral KEK (lost on restart). Use keyId with base64 key or file path for persistence.',
			);
		} else if (config.keyId.startsWith('base64:')) {
			// Load KEK from base64-encoded string (for testing)
			const base64Key = config.keyId.substring(7);
			this.kek = Buffer.from(base64Key, 'base64');
			if (this.kek.length !== 32) {
				throw new Error('Local KMS key must be 32 bytes (256 bits)');
			}
		} else {
			// Load KEK from file (for development with persistent key)
			try {
				const keyMaterial = require('fs').readFileSync(config.keyId);
				if (keyMaterial.length !== 32) {
					throw new Error('Local KMS key file must contain exactly 32 bytes');
				}
				this.kek = keyMaterial;
			} catch (error) {
				throw new Error(
					`Failed to load local KMS key from ${config.keyId}: ${error instanceof Error ? error.message : String(error)}`,
				);
			}
		}
	}

	/**
	 * Encrypt plaintext using AES-256-GCM with the KEK.
	 *
	 * @param plaintext - Data to encrypt (typically a DEK)
	 * @param context - Optional encryption context (stored as AAD)
	 * @returns Encrypted ciphertext (format: version:iv:tag:ciphertext, base64-encoded)
	 */
	async encrypt(plaintext: Buffer, context?: EncryptionContext): Promise<Buffer> {
		// Generate random IV (12 bytes for GCM)
		const iv = crypto.randomBytes(12);

		// Create cipher
		const cipher = crypto.createCipheriv('aes-256-gcm', this.kek, iv);

		// Add encryption context as additional authenticated data (AAD)
		if (context) {
			const contextString = JSON.stringify(context);
			cipher.setAAD(Buffer.from(contextString, 'utf-8'));
		}

		// Encrypt
		const encrypted = Buffer.concat([cipher.update(plaintext), cipher.final()]);
		const tag = cipher.getAuthTag();

		// Format: version(1) + iv(12) + tag(16) + ciphertext(variable)
		const version = Buffer.from([0x01]); // Version 1
		const result = Buffer.concat([version, iv, tag, encrypted]);

		return result;
	}

	/**
	 * Decrypt ciphertext using AES-256-GCM with the KEK.
	 *
	 * @param ciphertext - Encrypted data to decrypt
	 * @param context - Optional encryption context (must match encryption context)
	 * @returns Decrypted plaintext
	 */
	async decrypt(ciphertext: Buffer, context?: EncryptionContext): Promise<Buffer> {
		// Parse format: version(1) + iv(12) + tag(16) + ciphertext(variable)
		if (ciphertext.length < 29) {
			throw new Error('Invalid ciphertext format: too short');
		}

		const version = ciphertext[0];
		if (version !== 0x01) {
			throw new Error(`Unsupported ciphertext version: ${version}`);
		}

		const iv = ciphertext.subarray(1, 13);
		const tag = ciphertext.subarray(13, 29);
		const encrypted = ciphertext.subarray(29);

		// Create decipher
		const decipher = crypto.createDecipheriv('aes-256-gcm', this.kek, iv);
		decipher.setAuthTag(tag);

		// Add encryption context as AAD
		if (context) {
			const contextString = JSON.stringify(context);
			decipher.setAAD(Buffer.from(contextString, 'utf-8'));
		}

		// Decrypt
		try {
			const decrypted = Buffer.concat([decipher.update(encrypted), decipher.final()]);
			return decrypted;
		} catch (error) {
			throw new Error(
				`Decryption failed (wrong key or tampered data): ${error instanceof Error ? error.message : String(error)}`,
			);
		}
	}

	/**
	 * Generate a new Data Encryption Key (DEK).
	 *
	 * @param context - Optional encryption context for the DEK
	 * @returns DataKey containing plaintext and encrypted DEK
	 */
	async generateDataKey(context?: EncryptionContext): Promise<DataKey> {
		// Generate random 256-bit DEK
		const plaintext = crypto.randomBytes(32);

		// Encrypt DEK with KEK
		const encrypted = await this.encrypt(plaintext, context);

		return {
			plaintext,
			encrypted,
		};
	}

	/**
	 * Health check (always succeeds for local provider).
	 *
	 * @returns Promise resolving to true
	 */
	async healthCheck(): Promise<boolean> {
		// Verify we can encrypt and decrypt
		const testData = Buffer.from('health-check', 'utf-8');
		const encrypted = await this.encrypt(testData);
		const decrypted = await this.decrypt(encrypted);

		if (!testData.equals(decrypted)) {
			throw new Error('Local KMS health check failed: encrypt/decrypt mismatch');
		}

		return true;
	}
}
