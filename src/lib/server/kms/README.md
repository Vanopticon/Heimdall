# KMS Integration Module

This module provides pluggable Key Management Service (KMS) integration for Heimdall's envelope encryption and key management needs.

## Overview

Heimdall uses envelope encryption to protect sensitive field-level data (PII, credentials, etc.) stored in the graph database:

1. **Data Encryption Keys (DEKs)** — AES-256-GCM keys generated per-field or per-batch to encrypt sensitive data
2. **Key Encryption Keys (KEKs)** — Master keys stored in a KMS that encrypt the DEKs
3. **Encrypted DEKs** — Stored alongside encrypted data in the database for decryption

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     Heimdall Application                │
│                                                         │
│  ┌─────────────────────────────────────────────────┐  │
│  │            Field Encryption Layer               │  │
│  │                                                 │  │
│  │  1. Generate DEK via KMS.generateDataKey()     │  │
│  │  2. Encrypt field data with DEK (AES-GCM)      │  │
│  │  3. Store encrypted data + encrypted DEK       │  │
│  └─────────────────────────────────────────────────┘  │
│                          │                             │
│                          ▼                             │
│  ┌─────────────────────────────────────────────────┐  │
│  │            KMS Provider Interface               │  │
│  │  • encrypt(plaintext, context)                  │  │
│  │  • decrypt(ciphertext, context)                 │  │
│  │  • generateDataKey(context)                     │  │
│  │  • healthCheck()                                │  │
│  └─────────────────────────────────────────────────┘  │
│                          │                             │
└──────────────────────────┼─────────────────────────────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
        ▼                  ▼                  ▼
  ┌──────────┐      ┌──────────┐      ┌──────────┐
  │ AWS KMS  │      │ GCP KMS  │      │  Vault   │
  └──────────┘      └──────────┘      └──────────┘
```

## Usage

### Configuration

Configure KMS provider using environment variables:

```bash
# Provider type
export HMD_KMS_PROVIDER=vault  # or aws, gcp, local

# Key ID (format depends on provider)
export HMD_KMS_KEY_ID=heimdall-kek

# Provider-specific configuration
export HMD_KMS_REGION=us-west-2  # For AWS/GCP
export HMD_KMS_ENDPOINT=https://vault.example.com:8200  # For Vault
```

### Basic Usage

```typescript
import { createKMSProvider, loadKMSConfig } from '$lib/server/kms';

// Load configuration from environment
const config = loadKMSConfig();

// Create KMS provider
const kms = createKMSProvider(config);

// Verify KMS connectivity at startup
await kms.healthCheck();

// Encrypt sensitive data using envelope encryption
async function encryptField(fieldData: string): Promise<{ ciphertext: string; encryptedDEK: string }> {
	// Generate a new DEK (both plaintext and encrypted versions)
	const { plaintext: dek, encrypted: encryptedDEK } = await kms.generateDataKey({
		field: 'email',
		recordId: '12345'
	});

	// Encrypt field data with DEK using AES-256-GCM
	const iv = crypto.randomBytes(12);
	const cipher = crypto.createCipheriv('aes-256-gcm', dek, iv);
	const encrypted = Buffer.concat([
		cipher.update(fieldData, 'utf8'),
		cipher.final()
	]);
	const tag = cipher.getAuthTag();

	// Combine IV + tag + ciphertext
	const ciphertext = Buffer.concat([iv, tag, encrypted]).toString('base64');

	// Securely erase DEK from memory
	dek.fill(0);

	// Return encrypted data and encrypted DEK for storage
	return {
		ciphertext,
		encryptedDEK: encryptedDEK.toString('base64')
	};
}

// Decrypt sensitive data
async function decryptField(ciphertext: string, encryptedDEK: string, context: Record<string, string>): Promise<string> {
	// Decrypt DEK using KMS
	const dek = await kms.decrypt(
		Buffer.from(encryptedDEK, 'base64'),
		context
	);

	// Parse ciphertext: IV (12) + tag (16) + encrypted data
	const buffer = Buffer.from(ciphertext, 'base64');
	const iv = buffer.subarray(0, 12);
	const tag = buffer.subarray(12, 28);
	const encrypted = buffer.subarray(28);

	// Decrypt field data with DEK
	const decipher = crypto.createDecipheriv('aes-256-gcm', dek, iv);
	decipher.setAuthTag(tag);
	const decrypted = Buffer.concat([
		decipher.update(encrypted),
		decipher.final()
	]);

	// Securely erase DEK from memory
	dek.fill(0);

	return decrypted.toString('utf8');
}
```

## Providers

### Local Provider (Development Only)

For development and testing. **DO NOT use in production.**

```bash
export HMD_KMS_PROVIDER=local
export HMD_KMS_KEY_ID=generate  # or base64:... or /path/to/key/file
```

### AWS KMS

For production use with AWS.

```bash
export HMD_KMS_PROVIDER=aws
export HMD_KMS_KEY_ID=arn:aws:kms:us-west-2:123456789012:key/abc-def-ghi
export HMD_KMS_REGION=us-west-2
```

**Prerequisites**:

- Install `@aws-sdk/client-kms`
- Implement `src/lib/server/kms/providers/aws.ts` (stub currently provided)
- Configure IAM permissions: `kms:Encrypt`, `kms:Decrypt`, `kms:GenerateDataKey`

### GCP KMS

For production use with Google Cloud.

```bash
export HMD_KMS_PROVIDER=gcp
export HMD_KMS_KEY_ID=projects/my-project/locations/global/keyRings/heimdall/cryptoKeys/kek
export GCP_PROJECT_ID=my-project
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
```

**Prerequisites**:

- Install `@google-cloud/kms`
- Implement `src/lib/server/kms/providers/gcp.ts` (stub currently provided)
- Configure IAM permissions: `cloudkms.cryptoKeys.encrypt`, `cloudkms.cryptoKeys.decrypt`

### HashiCorp Vault

For production use with Vault Transit secrets engine.

```bash
export HMD_KMS_PROVIDER=vault
export HMD_KMS_KEY_ID=heimdall-kek
export VAULT_ADDR=https://vault.example.com:8200
export VAULT_TOKEN=hvs.CAESIJY...  # or use Kubernetes auth
export HMD_VAULT_MOUNT_PATH=transit  # optional, defaults to "transit"
```

**Prerequisites**:

- Install `node-vault`
- Implement `src/lib/server/kms/providers/vault.ts` (stub currently provided)
- Enable Transit secrets engine: `vault secrets enable transit`
- Create encryption key: `vault write -f transit/keys/heimdall-kek`
- Configure Vault policy (see provider stub for policy example)

## Encryption Context

Encryption context provides additional authenticated data (AAD) for encryption operations. It binds encrypted data to specific metadata, preventing ciphertext from being used in unintended contexts.

**Example context**:

```typescript
const context = {
	field: 'email',
	recordId: '12345',
	schema: 'v1',
	purpose: 'pii'
};

const { plaintext, encrypted } = await kms.generateDataKey(context);
```

**Best practices**:

- Include identifiers that uniquely identify the data (record ID, field name)
- Include schema version to prevent decryption after schema changes
- Use consistent context for encrypt/decrypt operations
- Context is not encrypted and may appear in logs — do not include sensitive data

## Key Rotation

For key rotation procedures, see [docs/ops/key-rotation.md](../../../../docs/ops/key-rotation.md).

**Summary**:

1. Generate new KEK in KMS
2. Update `HMD_KMS_KEY_ID` configuration
3. Restart application (new encryptions use new KEK)
4. Optionally re-encrypt existing data with new KEK (background job)
5. Disable old KEK after re-encryption complete

## Security Considerations

### DEK Handling

- DEKs are generated randomly and used once per encryption operation
- Plaintext DEKs must be securely erased after use: `dek.fill(0)`
- Never log or persist plaintext DEKs
- Encrypted DEKs are stored alongside encrypted data

### KEK Management

- KEKs are never exposed to the application
- KEKs remain in the KMS (HSM-backed for AWS/GCP)
- Rotate KEKs regularly (annually or per compliance requirements)
- Monitor KMS audit logs for unauthorized access

### Access Control

- Use least-privilege IAM policies for KMS access
- Separate keys per environment (dev, staging, prod)
- Use encryption context to enforce usage boundaries
- Enable KMS audit logging

### Compliance

- Most KMS providers support FIPS 140-2 Level 3 (HSM-backed)
- Audit logs available for all KMS operations
- Key rotation and versioning supported
- Automatic key backup and recovery (cloud providers)

## Testing

Test KMS integration using the local provider:

```typescript
import { createKMSProvider } from '$lib/server/kms';

describe('KMS Integration', () => {
	const kms = createKMSProvider({
		provider: 'local',
		keyId: 'generate'
	});

	it('should generate and use data key', async () => {
		const { plaintext, encrypted } = await kms.generateDataKey();
		expect(plaintext).toHaveLength(32);
		expect(encrypted.length).toBeGreaterThan(0);

		// Verify we can decrypt the DEK
		const decrypted = await kms.decrypt(encrypted);
		expect(decrypted).toEqual(plaintext);
	});

	it('should use encryption context', async () => {
		const context = { field: 'email', recordId: '123' };
		const { encrypted } = await kms.generateDataKey(context);

		// Decryption with correct context succeeds
		await expect(kms.decrypt(encrypted, context)).resolves.toBeDefined();

		// Decryption with wrong context fails
		await expect(kms.decrypt(encrypted, { field: 'wrong' })).rejects.toThrow();
	});
});
```

## Implementation Status

| Provider | Status      | Notes                                      |
| -------- | ----------- | ------------------------------------------ |
| Local    | ✅ Complete | For development/testing only               |
| AWS      | 📝 Stub     | Install @aws-sdk/client-kms to implement   |
| GCP      | 📝 Stub     | Install @google-cloud/kms to implement     |
| Vault    | 📝 Stub     | Install node-vault to implement            |

## Resources

- [Envelope Encryption Pattern](https://cloud.google.com/kms/docs/envelope-encryption)
- [AWS KMS Best Practices](https://docs.aws.amazon.com/kms/latest/developerguide/best-practices.html)
- [GCP KMS Documentation](https://cloud.google.com/kms/docs)
- [HashiCorp Vault Transit Engine](https://www.vaultproject.io/docs/secrets/transit)
- [NIST Key Management Guidelines](https://csrc.nist.gov/publications/detail/sp/800-57-part-1/rev-5/final)
