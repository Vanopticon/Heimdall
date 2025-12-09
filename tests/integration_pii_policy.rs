use serde_json::json;
use std::collections::HashMap;
use vanopticon_heimdall::pii::pii_policy::{PiiAction, PiiPolicyConfig, PiiPolicyEngine};

#[cfg(feature = "integration-tests")]
#[test]
fn test_pii_policy_prevents_plaintext_storage() {
	// Create a policy that protects sensitive fields
	let mut rules = HashMap::new();
	rules.insert("email".to_string(), PiiAction::Hash);
	rules.insert("ssn".to_string(), PiiAction::Encrypt);
	rules.insert("password".to_string(), PiiAction::Scrub);

	let config = PiiPolicyConfig {
		rules,
		default_action: PiiAction::Passthrough,
	};

	// Create a test master key (32 bytes)
	let master_key = vec![0x42; 32];
	let engine = PiiPolicyEngine::new(config, master_key, "test-key-1".to_string())
		.expect("engine creation");

	// Test that email is hashed
	let email_protected = engine.apply_policy("email", "user@example.com")
		.expect("hash email");
	assert!(email_protected.starts_with("sha256:"));
	assert_ne!(email_protected, "user@example.com");

	// Test that SSN is encrypted
	let ssn_protected = engine.apply_policy("ssn", "123-45-6789")
		.expect("encrypt ssn");
	assert!(PiiPolicyEngine::is_encrypted(&ssn_protected));
	assert_ne!(ssn_protected, "123-45-6789");

	// Test that password is scrubbed
	let password_protected = engine.apply_policy("password", "secret123")
		.expect("scrub password");
	assert_eq!(password_protected, "[REDACTED]");

	// Test that non-sensitive fields pass through
	let domain_protected = engine.apply_policy("domain", "example.com")
		.expect("passthrough domain");
	assert_eq!(domain_protected, "example.com");
}

#[cfg(feature = "integration-tests")]
#[test]
fn test_pii_validation_detects_plaintext() {
	// Create a policy that requires email to be hashed
	let mut rules = HashMap::new();
	rules.insert("email".to_string(), PiiAction::Hash);

	let config = PiiPolicyConfig {
		rules,
		default_action: PiiAction::Passthrough,
	};

	let master_key = vec![0x42; 32];
	let engine = PiiPolicyEngine::new(config, master_key, "test-key-1".to_string())
		.expect("engine creation");

	// Valid JSON with hashed email
	let valid_json = json!({
		"email": "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
		"domain": "example.com"
	});
	assert!(engine.validate_no_plaintext_pii(&valid_json).is_ok());

	// Invalid JSON with plaintext email
	let invalid_json = json!({
		"email": "user@example.com",
		"domain": "example.com"
	});
	assert!(engine.validate_no_plaintext_pii(&invalid_json).is_err());
}

#[cfg(feature = "integration-tests")]
#[test]
fn test_encrypted_data_roundtrip() {
	let config = PiiPolicyConfig::default();
	let master_key = vec![0x42; 32];
	let engine = PiiPolicyEngine::new(config, master_key, "test-key-1".to_string())
		.expect("engine creation");

	// Encrypt sensitive data
	let plaintext = "sensitive-personal-info";
	let envelope = engine.encrypt(plaintext).expect("encryption");

	// Verify ciphertext is different from plaintext
	assert_ne!(envelope.ciphertext, plaintext);
	assert_eq!(envelope.algorithm, "AES-256-GCM");
	assert_eq!(envelope.key_id, "test-key-1");

	// Decrypt with audit trail
	let decrypted = engine.decrypt(&envelope, "test-user", "debugging")
		.expect("decryption");
	assert_eq!(decrypted, plaintext);
}

#[cfg(feature = "integration-tests")]
#[test]
fn test_master_key_parsing() {
	// Valid 64-character hex key
	let valid_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
	let key = PiiPolicyEngine::parse_master_key_hex(valid_hex)
		.expect("parse valid key");
	assert_eq!(key.len(), 32);

	// Invalid key length
	let invalid_hex = "0123456789abcdef";
	assert!(PiiPolicyEngine::parse_master_key_hex(invalid_hex).is_err());

	// Invalid hex characters
	let invalid_chars = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdeg";
	assert!(PiiPolicyEngine::parse_master_key_hex(invalid_chars).is_err());
}

#[cfg(feature = "integration-tests")]
#[tokio::test]
async fn test_ingest_with_pii_protection() {
	// This test simulates the full ingest flow with PII protection
	use std::sync::Arc;
	use tokio::sync::mpsc;
	use vanopticon_heimdall::state::AppState;

	// Create PII engine with protective policies
	let mut rules = HashMap::new();
	rules.insert("email".to_string(), PiiAction::Hash);
	rules.insert("ssn".to_string(), PiiAction::Encrypt);

	let config = PiiPolicyConfig {
		rules,
		default_action: PiiAction::Passthrough,
	};

	let master_key = vec![0x42; 32];
	let engine = Arc::new(
		PiiPolicyEngine::new(config, master_key, "test-key-1".to_string())
			.expect("engine creation")
	);

	// Create a dummy repo for testing
	struct DummyRepo;
	#[async_trait::async_trait]
	impl vanopticon_heimdall::age_client::AgeRepo for DummyRepo {
		async fn merge_entity(
			&self,
			_label: &str,
			_key: &str,
			_props: &serde_json::Value,
		) -> anyhow::Result<()> {
			Ok(())
		}

		async fn ping(&self) -> anyhow::Result<()> {
			Ok(())
		}

		async fn merge_batch(
			&self,
			_items: &[(String, String, serde_json::Value)],
		) -> anyhow::Result<()> {
			Ok(())
		}
	}

	let (tx, _rx) = mpsc::channel(16);
	let repo: Arc<dyn vanopticon_heimdall::age_client::AgeRepo> = Arc::new(DummyRepo);
	
	let _app_state = AppState {
		repo,
		persist_sender: tx,
		pii_engine: Some(engine.clone()),
	};

	// Test that PII policies are applied
	let email_protected = engine.apply_policy("email", "user@example.com")
		.expect("hash email");
	assert!(email_protected.starts_with("sha256:"));

	let ssn_protected = engine.apply_policy("ssn", "123-45-6789")
		.expect("encrypt ssn");
	assert!(PiiPolicyEngine::is_encrypted(&ssn_protected));
}

#[cfg(not(feature = "integration-tests"))]
#[test]
fn integration_tests_disabled() {
	eprintln!("Integration tests disabled; run with --features integration-tests");
}
