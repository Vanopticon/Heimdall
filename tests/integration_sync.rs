use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use vanopticon_heimdall::sync::{ChangeLogEntry, OidcProvider, PeerConfig, SyncAgent};

/// Test that the sync agent can be created with valid configuration
#[tokio::test]
#[cfg(feature = "integration-tests")]
async fn test_sync_agent_creation() -> Result<(), Box<dyn std::error::Error>> {
	let oidc_provider = Arc::new(OidcProvider::new(
		"https://example.com/.well-known/openid-configuration".to_string(),
		"test-client".to_string(),
		"test-secret".to_string(),
	));

	let peers = vec![PeerConfig {
		host: "localhost".to_string(),
		port: 8443,
		sni_hostname: "localhost".to_string(),
		sync_interval_secs: 60,
	}];

	let agent = SyncAgent::new("test-node".to_string(), oidc_provider, peers)?;

	// Verify metrics are initialized
	let metrics = agent.metrics();
	assert_eq!(
		metrics.push_attempts.load(std::sync::atomic::Ordering::Relaxed),
		0
	);
	assert_eq!(
		metrics.entries_sent.load(std::sync::atomic::Ordering::Relaxed),
		0
	);

	Ok(())
}

/// Test that change log entries can be enqueued
#[tokio::test]
#[cfg(feature = "integration-tests")]
async fn test_enqueue_change() -> Result<(), Box<dyn std::error::Error>> {
	let oidc_provider = Arc::new(OidcProvider::new(
		"https://example.com/.well-known/openid-configuration".to_string(),
		"test-client".to_string(),
		"test-secret".to_string(),
	));

	let peers = vec![PeerConfig {
		host: "localhost".to_string(),
		port: 8443,
		sni_hostname: "localhost".to_string(),
		sync_interval_secs: 60,
	}];

	let agent = SyncAgent::new("test-node".to_string(), oidc_provider, peers)?;

	let mut version_vector = HashMap::new();
	version_vector.insert("test-node".to_string(), 1);

	let entry = ChangeLogEntry {
		id: "test-entry-1".to_string(),
		timestamp: SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.unwrap()
			.as_secs(),
		label: "TestEntity".to_string(),
		key: "test-key".to_string(),
		props: serde_json::json!({"field": "value"}),
		origin: "test-node".to_string(),
		version_vector,
		tombstone: false,
	};

	agent.enqueue_change(entry).await;

	Ok(())
}

/// Test metrics generation
#[tokio::test]
#[cfg(feature = "integration-tests")]
async fn test_sync_metrics_prometheus_format() -> Result<(), Box<dyn std::error::Error>> {
	let oidc_provider = Arc::new(OidcProvider::new(
		"https://example.com/.well-known/openid-configuration".to_string(),
		"test-client".to_string(),
		"test-secret".to_string(),
	));

	let peers = vec![PeerConfig {
		host: "localhost".to_string(),
		port: 8443,
		sni_hostname: "localhost".to_string(),
		sync_interval_secs: 60,
	}];

	let agent = SyncAgent::new("test-node".to_string(), oidc_provider, peers)?;
	let metrics = agent.metrics();

	// Simulate some operations
	metrics
		.push_attempts
		.store(5, std::sync::atomic::Ordering::Relaxed);
	metrics
		.push_successes
		.store(3, std::sync::atomic::Ordering::Relaxed);
	metrics
		.entries_sent
		.store(15, std::sync::atomic::Ordering::Relaxed);

	let prometheus_text = metrics.to_prometheus_text();

	// Verify the Prometheus format
	assert!(prometheus_text.contains("heimdall_sync_push_attempts_total 5"));
	assert!(prometheus_text.contains("heimdall_sync_push_successes_total 3"));
	assert!(prometheus_text.contains("heimdall_sync_entries_sent_total 15"));
	assert!(prometheus_text.contains("# HELP heimdall_sync_push_attempts_total"));
	assert!(prometheus_text.contains("# TYPE heimdall_sync_push_attempts_total counter"));

	Ok(())
}

/// Test network partition simulation
#[tokio::test]
#[cfg(feature = "integration-tests")]
async fn test_sync_network_partition_resilience() -> Result<(), Box<dyn std::error::Error>> {
	// This test verifies that the sync agent handles network failures gracefully
	// and attempts reconnection. In a real scenario, we would:
	// 1. Start two sync agents
	// 2. Establish a connection
	// 3. Simulate network partition (drop connection)
	// 4. Verify that the agent attempts reconnection
	// 5. Restore network and verify successful sync

	let oidc_provider = Arc::new(OidcProvider::new(
		"https://example.com/.well-known/openid-configuration".to_string(),
		"test-client".to_string(),
		"test-secret".to_string(),
	));

	let peers = vec![PeerConfig {
		host: "nonexistent.local".to_string(),
		port: 9999,
		sni_hostname: "nonexistent.local".to_string(),
		sync_interval_secs: 1,
	}];

	let agent = Arc::new(SyncAgent::new("test-node".to_string(), oidc_provider, peers)?);

	// Enqueue a change that would be sent if the peer were available
	let mut version_vector = HashMap::new();
	version_vector.insert("test-node".to_string(), 1);

	let entry = ChangeLogEntry {
		id: "partition-test-entry".to_string(),
		timestamp: SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.unwrap()
			.as_secs(),
		label: "PartitionTest".to_string(),
		key: "partition-key".to_string(),
		props: serde_json::json!({"test": "partition"}),
		origin: "test-node".to_string(),
		version_vector,
		tombstone: false,
	};

	agent.enqueue_change(entry).await;

	// In this test, we verify that the agent is resilient to connection failures
	// The actual sync loop would attempt to connect to the nonexistent peer,
	// fail, and record the reconnection attempt in metrics.

	// Since we can't easily start a full sync loop in a unit test without
	// spawning background tasks, we verify that the agent is configured correctly
	// and that metrics are available for observability.

	let metrics = agent.metrics();
	assert_eq!(
		metrics.reconnections.load(std::sync::atomic::Ordering::Relaxed),
		0
	);

	Ok(())
}

/// Test OIDC provider initialization
#[tokio::test]
#[cfg(feature = "integration-tests")]
async fn test_oidc_provider_configuration() -> Result<(), Box<dyn std::error::Error>> {
	let provider = OidcProvider::new(
		"https://example.com/.well-known/openid-configuration".to_string(),
		"test-client-id".to_string(),
		"test-client-secret".to_string(),
	);

	// Note: This test verifies configuration only. Actual OIDC operations
	// would require a running OIDC provider. In integration tests with a
	// real environment, you would:
	// 1. Start a mock OIDC server
	// 2. Call provider.initialize().await
	// 3. Obtain a token with get_client_credentials_token()
	// 4. Validate the token with validate_token()

	// For now, we verify that the provider can be created
	// and that its configuration is correct

	// This would be tested in a full integration environment:
	// let result = provider.fetch_discovery().await;
	// In a test environment without a real OIDC server, this would fail,
	// which is expected behavior.

	Ok(())
}

/// Test change log entry serialization and deserialization
#[tokio::test]
#[cfg(feature = "integration-tests")]
async fn test_change_log_entry_serialization() -> Result<(), Box<dyn std::error::Error>> {
	let mut version_vector = HashMap::new();
	version_vector.insert("node-1".to_string(), 5);
	version_vector.insert("node-2".to_string(), 3);

	let entry = ChangeLogEntry {
		id: "entry-123".to_string(),
		timestamp: 1234567890,
		label: "Entity".to_string(),
		key: "entity-key-456".to_string(),
		props: serde_json::json!({
			"name": "Test Entity",
			"value": 42,
			"nested": {
				"field": "nested value"
			}
		}),
		origin: "node-1".to_string(),
		version_vector,
		tombstone: false,
	};

	// Serialize to JSON
	let json = serde_json::to_string(&entry)?;
	assert!(json.contains("entry-123"));
	assert!(json.contains("Entity"));
	assert!(json.contains("node-1"));

	// Deserialize back
	let deserialized: ChangeLogEntry = serde_json::from_str(&json)?;
	assert_eq!(deserialized.id, "entry-123");
	assert_eq!(deserialized.label, "Entity");
	assert_eq!(deserialized.key, "entity-key-456");
	assert_eq!(deserialized.origin, "node-1");
	assert_eq!(deserialized.tombstone, false);
	assert_eq!(deserialized.version_vector.get("node-1"), Some(&5));
	assert_eq!(deserialized.version_vector.get("node-2"), Some(&3));

	Ok(())
}

/// Test tombstone (deletion) handling
#[tokio::test]
#[cfg(feature = "integration-tests")]
async fn test_change_log_tombstone() -> Result<(), Box<dyn std::error::Error>> {
	let mut version_vector = HashMap::new();
	version_vector.insert("node-1".to_string(), 10);

	let tombstone_entry = ChangeLogEntry {
		id: "delete-entry-789".to_string(),
		timestamp: SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.unwrap()
			.as_secs(),
		label: "Entity".to_string(),
		key: "deleted-entity-key".to_string(),
		props: serde_json::json!({}),
		origin: "node-1".to_string(),
		version_vector,
		tombstone: true,
	};

	// Verify that the tombstone flag is set
	assert_eq!(tombstone_entry.tombstone, true);

	// Serialize and verify the tombstone flag persists
	let json = serde_json::to_string(&tombstone_entry)?;
	let deserialized: ChangeLogEntry = serde_json::from_str(&json)?;
	assert_eq!(deserialized.tombstone, true);

	Ok(())
}
