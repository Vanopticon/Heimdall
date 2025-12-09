# PII Policy Engine

This module implements a comprehensive PII (Personally Identifiable Information) policy engine with envelope encryption for Heimdall's data ingestion pipeline.

## Features

- **Field-level PII policies**: Configure actions per field type (scrub, hash, encrypt, passthrough)
- **Envelope encryption**: AES-256-GCM encryption using the vetted `ring` crate
- **One-way hashing**: SHA-256 hashing for fields that need de-duplication but not recovery
- **Audit logging**: All decryption operations are logged with actor, timestamp, and reason
- **Pre/post validation**: Methods to ensure no plaintext PII persists

## Configuration

### Master Key

Set the PII master key via environment variable:

```bash
export HMD_PII_MASTER_KEY="0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
```

The key must be exactly 64 hexadecimal characters (32 bytes for AES-256).

### Policy Configuration

Policies are configured via `PiiPolicyConfig`:

```rust
use std::collections::HashMap;
use vanopticon_heimdall::pii::pii_policy::{PiiAction, PiiPolicyConfig};

let mut rules = HashMap::new();
rules.insert("email".to_string(), PiiAction::Hash);
rules.insert("ssn".to_string(), PiiAction::Encrypt);
rules.insert("password".to_string(), PiiAction::Scrub);

let config = PiiPolicyConfig {
    rules,
    default_action: PiiAction::Passthrough,
};
```

## PII Actions

### Scrub

Replaces the field value with `[REDACTED]`.

**Use case**: Passwords, API keys, or data that should never be stored.

### Hash

Applies SHA-256 one-way hash with `sha256:` prefix.

**Use case**: Email addresses, usernames, or fields needed for de-duplication but not recovery.

### Encrypt

Uses AES-256-GCM envelope encryption with a random nonce.

**Use case**: SSNs, credit card numbers, or data that may need to be decrypted later.

**Output format**:

```json
{
  "ciphertext": "base64-encoded-ciphertext",
  "nonce": "base64-encoded-nonce",
  "key_id": "default-key-v1",
  "algorithm": "AES-256-GCM"
}
```

### Passthrough

Stores the field value as-is without protection.

**Use case**: Non-sensitive metadata like domain names, IP addresses (when not PII).

## Usage Example

```rust
use vanopticon_heimdall::pii::pii_policy::{PiiPolicyConfig, PiiPolicyEngine};

// Parse master key
let master_key = PiiPolicyEngine::parse_master_key_hex(
    &std::env::var("HMD_PII_MASTER_KEY")?
)?;

// Create engine
let config = PiiPolicyConfig::default(); // or custom config
let engine = PiiPolicyEngine::new(config, master_key, "key-v1".to_string())?;

// Apply policy to a field
let protected = engine.apply_policy("email", "user@example.com")?;
// Returns: "sha256:..." (if email is configured to hash)

// Validate no plaintext PII
let json = serde_json::json!({
    "email": protected,
    "domain": "example.com"
});
engine.validate_no_plaintext_pii(&json)?;

// Decrypt (with audit trail)
let envelope = engine.encrypt("sensitive-data")?;
let plaintext = engine.decrypt(&envelope, "admin-user", "debugging")?;
// Logs: AUDIT: decrypt {"actor":"admin-user","reason":"debugging",...}
```

## Integration

The PII engine is automatically integrated into the ingest pipeline when `HMD_PII_MASTER_KEY` is set. Each field's raw value is processed according to its policy before persistence.

## Security Considerations

- **Key Management**: The master key is environment-based. For production, consider using a KMS (Key Management Service) or HashiCorp Vault.
- **Key Rotation**: The `key_id` field supports key rotation. Implement rotation by:
  1. Setting a new key with a different ID
  2. Re-encrypting old data with the new key
  3. Updating the key_id in the configuration
- **Audit Logs**: Decryption operations log to stderr. In production, redirect to a secure audit log storage.
- **Nonce Uniqueness**: Each encryption operation uses a random nonce. Never reuse nonces with the same key.

## Testing

Run unit tests:

```bash
cargo test --lib --features unit-tests pii_policy
```

Run integration tests:

```bash
cargo test --test integration_pii_policy --features integration-tests
```

## Dependencies

- `ring` 0.17: AES-256-GCM encryption
- `sha2` 0.10: SHA-256 hashing
- `base64` 0.22: Encoding/decoding
- `chrono` 0.4: RFC3339 timestamps for audit logs
