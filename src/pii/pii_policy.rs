use anyhow::{anyhow, Result};
use ring::aead::{Aad, BoundKey, Nonce, NonceSequence, OpeningKey, SealingKey, UnboundKey, AES_256_GCM};
use ring::error::Unspecified;
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;

/// PII field action: how to handle sensitive data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PiiAction {
	/// Remove the field entirely
	Scrub,
	/// Replace with one-way SHA-256 hash
	Hash,
	/// Store encrypted using envelope encryption
	Encrypt,
	/// Store as-is (no protection)
	Passthrough,
}

/// Policy configuration for PII handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiPolicyConfig {
	/// Map of field names/patterns to actions
	pub rules: HashMap<String, PiiAction>,
	/// Default action when no rule matches
	#[serde(default = "default_action")]
	pub default_action: PiiAction,
}

fn default_action() -> PiiAction {
	PiiAction::Passthrough
}

impl Default for PiiPolicyConfig {
	fn default() -> Self {
		Self {
			rules: HashMap::new(),
			default_action: PiiAction::Passthrough,
		}
	}
}

/// Encrypted data envelope containing ciphertext and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedEnvelope {
	/// Base64-encoded ciphertext (includes authentication tag)
	pub ciphertext: String,
	/// Base64-encoded nonce (12 bytes for AES-GCM)
	pub nonce: String,
	/// Key identifier for key rotation support
	pub key_id: String,
	/// Algorithm identifier
	pub algorithm: String,
}

/// Audit record for decryption operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecryptAuditLog {
	/// Actor performing decryption (user ID, service account, etc.)
	pub actor: String,
	/// Reason for decryption
	pub reason: String,
	/// Timestamp (ISO 8601)
	pub timestamp: String,
	/// Field name that was decrypted
	pub field_name: String,
	/// Key ID used
	pub key_id: String,
}

/// PII policy engine with envelope encryption support
pub struct PiiPolicyEngine {
	config: PiiPolicyConfig,
	master_key: Arc<Vec<u8>>,
	key_id: String,
	rng: SystemRandom,
}

/// Nonce generator for AES-GCM (generates random nonces)
struct RandomNonceSequence {
	rng: SystemRandom,
}

impl RandomNonceSequence {
	fn new(rng: SystemRandom) -> Self {
		Self { rng }
	}
}

impl NonceSequence for RandomNonceSequence {
	fn advance(&mut self) -> Result<Nonce, Unspecified> {
		let mut nonce_bytes = [0u8; 12]; // AES-GCM nonce size
		self.rng.fill(&mut nonce_bytes).map_err(|_| Unspecified)?;
		Nonce::try_assume_unique_for_key(&nonce_bytes)
	}
}

impl PiiPolicyEngine {
	/// Create a new PII policy engine
	///
	/// # Arguments
	/// * `config` - Policy configuration
	/// * `master_key` - 32-byte AES-256 key for envelope encryption
	/// * `key_id` - Identifier for the master key (for key rotation)
	pub fn new(config: PiiPolicyConfig, master_key: Vec<u8>, key_id: String) -> Result<Self> {
		if master_key.len() != 32 {
			return Err(anyhow!("master key must be exactly 32 bytes for AES-256"));
		}

		Ok(Self {
			config,
			master_key: Arc::new(master_key),
			key_id,
			rng: SystemRandom::new(),
		})
	}

	/// Parse master key from hex string
	pub fn parse_master_key_hex(hex: &str) -> Result<Vec<u8>> {
		if hex.len() != 64 {
			return Err(anyhow!("hex master key must be 64 characters (32 bytes)"));
		}

		let mut bytes = Vec::with_capacity(32);
		for i in 0..32 {
			let byte_str = &hex[i * 2..i * 2 + 2];
			let byte = u8::from_str_radix(byte_str, 16)
				.map_err(|_| anyhow!("invalid hex in master key"))?;
			bytes.push(byte);
		}
		Ok(bytes)
	}

	/// Get the action for a given field name
	pub fn get_action(&self, field_name: &str) -> PiiAction {
		self.config
			.rules
			.get(field_name)
			.copied()
			.unwrap_or(self.config.default_action)
	}

	/// Apply PII policy to a field value
	pub fn apply_policy(&self, field_name: &str, value: &str) -> Result<String> {
		let action = self.get_action(field_name);

		match action {
			PiiAction::Scrub => Ok(String::from("[REDACTED]")),
			PiiAction::Hash => {
				let hash = self.hash_value(value);
				Ok(format!("sha256:{}", hash))
			}
			PiiAction::Encrypt => {
				let envelope = self.encrypt(value)?;
				Ok(serde_json::to_string(&envelope)?)
			}
			PiiAction::Passthrough => Ok(value.to_string()),
		}
	}

	/// One-way hash of a value using SHA-256
	pub fn hash_value(&self, value: &str) -> String {
		let mut hasher = Sha256::new();
		hasher.update(value.as_bytes());
		let result = hasher.finalize();
		hex_helper::encode(result)
	}

	/// Encrypt a plaintext value using envelope encryption (AES-256-GCM)
	pub fn encrypt(&self, plaintext: &str) -> Result<EncryptedEnvelope> {
		let plaintext_bytes = plaintext.as_bytes();

		// Generate random nonce (12 bytes for AES-GCM)
		let mut nonce_bytes = [0u8; 12];
		self.rng.fill(&mut nonce_bytes)
			.map_err(|_| anyhow!("failed to generate nonce"))?;

		// Create sealing key
		let unbound_key = UnboundKey::new(&AES_256_GCM, &self.master_key)
			.map_err(|_| anyhow!("failed to create encryption key"))?;

		let nonce = Nonce::try_assume_unique_for_key(&nonce_bytes)
			.map_err(|_| anyhow!("failed to create nonce"))?;

		let mut sealing_key = SealingKey::new(unbound_key, OneTimeNonce::new(nonce));

		// Prepare buffer for in-place encryption (plaintext + tag space)
		let mut in_out = plaintext_bytes.to_vec();
		in_out.reserve(AES_256_GCM.tag_len());

		// Encrypt in-place
		sealing_key
			.seal_in_place_append_tag(Aad::empty(), &mut in_out)
			.map_err(|_| anyhow!("encryption failed"))?;

		Ok(EncryptedEnvelope {
			ciphertext: base64_helper::encode(&in_out),
			nonce: base64_helper::encode(&nonce_bytes),
			key_id: self.key_id.clone(),
			algorithm: "AES-256-GCM".to_string(),
		})
	}

	/// Decrypt an encrypted envelope
	///
	/// # Arguments
	/// * `envelope` - The encrypted envelope
	/// * `audit_info` - Audit information (actor, reason)
	///
	/// Returns the plaintext and logs the audit record
	pub fn decrypt(
		&self,
		envelope: &EncryptedEnvelope,
		actor: &str,
		reason: &str,
	) -> Result<String> {
		// Verify key ID matches
		if envelope.key_id != self.key_id {
			return Err(anyhow!(
				"key ID mismatch: expected {}, got {}",
				self.key_id,
				envelope.key_id
			));
		}

		// Decode ciphertext and nonce
		let ciphertext = base64_helper::decode(&envelope.ciphertext)
			.map_err(|_| anyhow!("failed to decode ciphertext"))?;
		let nonce_bytes = base64_helper::decode(&envelope.nonce)
			.map_err(|_| anyhow!("failed to decode nonce"))?;

		if nonce_bytes.len() != 12 {
			return Err(anyhow!("invalid nonce length"));
		}

		// Create opening key
		let unbound_key = UnboundKey::new(&AES_256_GCM, &self.master_key)
			.map_err(|_| anyhow!("failed to create decryption key"))?;

		let mut nonce_arr = [0u8; 12];
		nonce_arr.copy_from_slice(&nonce_bytes);
		let nonce = Nonce::try_assume_unique_for_key(&nonce_arr)
			.map_err(|_| anyhow!("failed to create nonce"))?;

		let mut opening_key = OpeningKey::new(unbound_key, OneTimeNonce::new(nonce));

		// Decrypt in-place
		let mut in_out = ciphertext;
		let plaintext_bytes = opening_key
			.open_in_place(Aad::empty(), &mut in_out)
			.map_err(|_| anyhow!("decryption failed"))?;

		// Log audit record
		let audit = DecryptAuditLog {
			actor: actor.to_string(),
			reason: reason.to_string(),
			timestamp: time_helper::now_rfc3339(),
			field_name: String::from("unknown"), // caller should set this
			key_id: envelope.key_id.clone(),
		};

		// Log to stderr for now (in production, this should go to a secure audit log)
		eprintln!("AUDIT: decrypt {}", serde_json::to_string(&audit)?);

		String::from_utf8(plaintext_bytes.to_vec())
			.map_err(|_| anyhow!("decrypted data is not valid UTF-8"))
	}

	/// Check if a value is encrypted (heuristic)
	pub fn is_encrypted(value: &str) -> bool {
		// Check if value looks like a JSON envelope
		if let Ok(envelope) = serde_json::from_str::<EncryptedEnvelope>(value) {
			envelope.algorithm == "AES-256-GCM"
		} else {
			false
		}
	}

	/// Check if a value is hashed (heuristic)
	pub fn is_hashed(value: &str) -> bool {
		value.starts_with("sha256:")
	}

	/// Validate that no plaintext PII exists in a JSON value
	pub fn validate_no_plaintext_pii(&self, json: &serde_json::Value) -> Result<()> {
		match json {
			serde_json::Value::Object(map) => {
				for (key, value) in map {
					let action = self.get_action(key);
					
					// If field requires protection, verify it's not plaintext
					if matches!(action, PiiAction::Hash | PiiAction::Encrypt | PiiAction::Scrub) {
						if let serde_json::Value::String(s) = value {
							// Check that value is protected
							if action == PiiAction::Scrub && s != "[REDACTED]" {
								return Err(anyhow!("field '{}' should be scrubbed but contains: {}", key, s));
							}
							if action == PiiAction::Hash && !Self::is_hashed(s) {
								return Err(anyhow!("field '{}' should be hashed but is plaintext", key));
							}
							if action == PiiAction::Encrypt && !Self::is_encrypted(s) {
								return Err(anyhow!("field '{}' should be encrypted but is plaintext", key));
							}
						}
					}

					// Recurse into nested objects/arrays
					self.validate_no_plaintext_pii(value)?;
				}
			}
			serde_json::Value::Array(arr) => {
				for item in arr {
					self.validate_no_plaintext_pii(item)?;
				}
			}
			_ => {}
		}
		Ok(())
	}
}

/// One-time nonce sequence (uses a single nonce, then errors)
struct OneTimeNonce {
	nonce: Option<Nonce>,
}

impl OneTimeNonce {
	fn new(nonce: Nonce) -> Self {
		Self { nonce: Some(nonce) }
	}
}

impl NonceSequence for OneTimeNonce {
	fn advance(&mut self) -> Result<Nonce, Unspecified> {
		self.nonce.take().ok_or(Unspecified)
	}
}

// Helper modules for base64, hex, and timestamps
mod base64_helper {
	use base64::Engine;
	
	pub fn encode<T: AsRef<[u8]>>(input: T) -> String {
		base64::engine::general_purpose::STANDARD.encode(input)
	}

	pub fn decode<T: AsRef<[u8]>>(input: T) -> Result<Vec<u8>, base64::DecodeError> {
		base64::engine::general_purpose::STANDARD.decode(input)
	}
}

mod hex_helper {
	pub fn encode<T: AsRef<[u8]>>(data: T) -> String {
		data.as_ref()
			.iter()
			.map(|b| format!("{:02x}", b))
			.collect()
	}
}

mod time_helper {
	pub fn now_rfc3339() -> String {
		chrono::Utc::now().to_rfc3339()
	}
}

#[cfg(test)]
#[cfg(feature = "unit-tests")]
mod tests {
	use super::*;

	fn test_config() -> PiiPolicyConfig {
		let mut rules = HashMap::new();
		rules.insert("email".to_string(), PiiAction::Hash);
		rules.insert("ssn".to_string(), PiiAction::Encrypt);
		rules.insert("password".to_string(), PiiAction::Scrub);

		PiiPolicyConfig {
			rules,
			default_action: PiiAction::Passthrough,
		}
	}

	fn test_master_key() -> Vec<u8> {
		vec![0x42; 32] // 32-byte test key
	}

	#[test]
	fn test_policy_engine_creation() {
		let config = test_config();
		let key = test_master_key();
		let engine = PiiPolicyEngine::new(config, key, "test-key-1".to_string());
		assert!(engine.is_ok());
	}

	#[test]
	fn test_policy_engine_rejects_invalid_key() {
		let config = test_config();
		let key = vec![0x42; 16]; // Wrong size
		let engine = PiiPolicyEngine::new(config, key, "test-key-1".to_string());
		assert!(engine.is_err());
	}

	#[test]
	fn test_scrub_action() {
		let config = test_config();
		let engine = PiiPolicyEngine::new(config, test_master_key(), "test-key-1".to_string())
			.unwrap();
		
		let result = engine.apply_policy("password", "secret123").unwrap();
		assert_eq!(result, "[REDACTED]");
	}

	#[test]
	fn test_hash_action() {
		let config = test_config();
		let engine = PiiPolicyEngine::new(config, test_master_key(), "test-key-1".to_string())
			.unwrap();
		
		let result = engine.apply_policy("email", "user@example.com").unwrap();
		assert!(result.starts_with("sha256:"));
		assert_eq!(result.len(), 71); // "sha256:" + 64 hex chars
	}

	#[test]
	fn test_encrypt_decrypt_roundtrip() {
		let config = test_config();
		let engine = PiiPolicyEngine::new(config, test_master_key(), "test-key-1".to_string())
			.unwrap();

		let plaintext = "sensitive-data-12345";
		let envelope = engine.encrypt(plaintext).unwrap();

		assert_eq!(envelope.algorithm, "AES-256-GCM");
		assert_eq!(envelope.key_id, "test-key-1");
		assert!(!envelope.ciphertext.is_empty());
		assert!(!envelope.nonce.is_empty());

		// Decrypt
		let decrypted = engine
			.decrypt(&envelope, "test-actor", "testing")
			.unwrap();
		assert_eq!(decrypted, plaintext);
	}

	#[test]
	fn test_decrypt_with_wrong_key_id() {
		let config = test_config();
		let engine1 = PiiPolicyEngine::new(config.clone(), test_master_key(), "key-1".to_string())
			.unwrap();
		let engine2 = PiiPolicyEngine::new(config, test_master_key(), "key-2".to_string())
			.unwrap();

		let envelope = engine1.encrypt("secret").unwrap();
		let result = engine2.decrypt(&envelope, "actor", "reason");
		
		assert!(result.is_err());
		assert!(result.unwrap_err().to_string().contains("key ID mismatch"));
	}

	#[test]
	fn test_parse_master_key_hex() {
		let hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
		let key = PiiPolicyEngine::parse_master_key_hex(hex).unwrap();
		assert_eq!(key.len(), 32);
		assert_eq!(key[0], 0x01);
		assert_eq!(key[1], 0x23);
		assert_eq!(key[31], 0xef);
	}

	#[test]
	fn test_parse_master_key_hex_invalid_length() {
		let hex = "0123456789abcdef"; // Too short
		let result = PiiPolicyEngine::parse_master_key_hex(hex);
		assert!(result.is_err());
	}

	#[test]
	fn test_validate_no_plaintext_pii() {
		let mut rules = HashMap::new();
		rules.insert("email".to_string(), PiiAction::Hash);
		rules.insert("ssn".to_string(), PiiAction::Encrypt);

		let config = PiiPolicyConfig {
			rules,
			default_action: PiiAction::Passthrough,
		};

		let engine = PiiPolicyEngine::new(config, test_master_key(), "test-key-1".to_string())
			.unwrap();

		// Valid: hashed email
		let json = serde_json::json!({
			"email": "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
			"name": "John Doe"
		});
		assert!(engine.validate_no_plaintext_pii(&json).is_ok());

		// Invalid: plaintext email
		let json = serde_json::json!({
			"email": "user@example.com",
			"name": "John Doe"
		});
		assert!(engine.validate_no_plaintext_pii(&json).is_err());
	}

	#[test]
	fn test_is_encrypted() {
		assert!(PiiPolicyEngine::is_encrypted(r#"{"ciphertext":"abc","nonce":"xyz","key_id":"k1","algorithm":"AES-256-GCM"}"#));
		assert!(!PiiPolicyEngine::is_encrypted("plaintext"));
		assert!(!PiiPolicyEngine::is_encrypted("sha256:abcdef"));
	}

	#[test]
	fn test_is_hashed() {
		assert!(PiiPolicyEngine::is_hashed("sha256:abcdef1234567890"));
		assert!(!PiiPolicyEngine::is_hashed("plaintext"));
		assert!(!PiiPolicyEngine::is_hashed("md5:abcdef"));
	}
}
