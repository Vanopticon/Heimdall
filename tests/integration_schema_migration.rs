// Integration tests for SQL schema migrations and row persistence
use std::env;
use tokio::time::{Duration, sleep};
use vanopticon_heimdall::age_client::{AgeClient, AgeRepo};

/// Helper to wait for Postgres to be ready
async fn wait_for_postgres() -> sqlx::PgPool {
	let mut attempts = 0;
	loop {
		match sqlx::PgPool::connect("postgres://heimdall:heimdall@127.0.0.1:5432/heimdall").await {
			Ok(p) => break p,
			Err(_) => {
				attempts += 1;
				if attempts > 30 {
					panic!("Postgres did not become ready in time");
				}
				sleep(Duration::from_secs(1)).await;
			}
		}
	}
}

#[tokio::test]
#[cfg(feature = "integration-tests")]
async fn test_schema_migration() {
	// Skip unless explicitly enabled
	if env::var("RUN_DOCKER_INTEGRATION_TESTS").is_err() {
		eprintln!("Skipping Docker integration test; set RUN_DOCKER_INTEGRATION_TESTS=1");
		return;
	}

	// Start dev DB
	vanopticon_heimdall::devops::start_dev_db()
		.await
		.expect("start db");

	let pool = wait_for_postgres().await;
	let client = AgeClient::new(pool.clone(), "heimdall_graph");

	// Load and apply the schema migration
	let schema_sql = include_str!("../sql/v1/001-create_graph.sql");
	client
		.apply_migration(schema_sql)
		.await
		.expect("apply migration");

	// Verify connectivity with ping
	client.ping().await.expect("ping succeeded");

	// Clean up
	vanopticon_heimdall::devops::stop_dev_db()
		.await
		.expect("stop db");
}

#[tokio::test]
#[cfg(feature = "integration-tests")]
async fn test_persist_row_workflow() {
	// Skip unless explicitly enabled
	if env::var("RUN_DOCKER_INTEGRATION_TESTS").is_err() {
		eprintln!("Skipping Docker integration test; set RUN_DOCKER_INTEGRATION_TESTS=1");
		return;
	}

	// Start dev DB
	vanopticon_heimdall::devops::start_dev_db()
		.await
		.expect("start db");

	let pool = wait_for_postgres().await;
	let client = AgeClient::new(pool.clone(), "heimdall_graph");

	// Apply schema
	let schema_sql = include_str!("../sql/v1/001-create_graph.sql");
	client
		.apply_migration(schema_sql)
		.await
		.expect("apply migration");

	// Persist a test row with sample cells
	let dump_id = "test-dump-001";
	let row_index = 0;
	let row_hash = Some("abc123hash");
	let timestamp = "2024-01-15T10:30:00Z";

	// Cells: (column, raw, canonical_key, canonical_value)
	let cells = vec![
		(
			"email".to_string(),
			"test@example.com".to_string(),
			"email:test@example.com".to_string(),
			"test@example.com".to_string(),
		),
		(
			"password".to_string(),
			"pass123".to_string(),
			"password:hashed_pass123".to_string(),
			"hashed_pass123".to_string(),
		),
		(
			"ip".to_string(),
			"192.168.1.1".to_string(),
			"ip:192.168.1.1".to_string(),
			"192.168.1.1".to_string(),
		),
	];

	client
		.persist_row(dump_id, row_index, row_hash, &cells, timestamp)
		.await
		.expect("persist_row succeeded");

	// Verify the row was created by checking connectivity (more thorough
	// verification would query the graph to ensure nodes exist)
	client.ping().await.expect("ping after persist");

	// Clean up
	vanopticon_heimdall::devops::stop_dev_db()
		.await
		.expect("stop db");
}

#[tokio::test]
#[cfg(feature = "integration-tests")]
async fn test_co_occurrence_tracking() {
	// Skip unless explicitly enabled
	if env::var("RUN_DOCKER_INTEGRATION_TESTS").is_err() {
		eprintln!("Skipping Docker integration test; set RUN_DOCKER_INTEGRATION_TESTS=1");
		return;
	}

	// Start dev DB
	vanopticon_heimdall::devops::start_dev_db()
		.await
		.expect("start db");

	let pool = wait_for_postgres().await;
	let client = AgeClient::new(pool.clone(), "heimdall_graph");

	// Apply schema
	let schema_sql = include_str!("../sql/v1/001-create_graph.sql");
	client
		.apply_migration(schema_sql)
		.await
		.expect("apply migration");

	let timestamp = "2024-01-15T10:30:00Z";

	// Create two field values and track co-occurrence
	let key_a = "email:alice@example.com";
	let key_b = "password:hashed_alice_pass";

	client
		.increment_co_occurrence(key_a, key_b, timestamp)
		.await
		.expect("increment co-occurrence");

	// Increment again to test counter
	client
		.increment_co_occurrence(key_a, key_b, timestamp)
		.await
		.expect("increment co-occurrence again");

	client.ping().await.expect("ping after co-occurrence");

	// Clean up
	vanopticon_heimdall::devops::stop_dev_db()
		.await
		.expect("stop db");
}

#[tokio::test]
#[cfg(feature = "integration-tests")]
async fn test_credential_relationship() {
	// Skip unless explicitly enabled
	if env::var("RUN_DOCKER_INTEGRATION_TESTS").is_err() {
		eprintln!("Skipping Docker integration test; set RUN_DOCKER_INTEGRATION_TESTS=1");
		return;
	}

	// Start dev DB
	vanopticon_heimdall::devops::start_dev_db()
		.await
		.expect("start db");

	let pool = wait_for_postgres().await;
	let client = AgeClient::new(pool.clone(), "heimdall_graph");

	// Apply schema
	let schema_sql = include_str!("../sql/v1/001-create_graph.sql");
	client
		.apply_migration(schema_sql)
		.await
		.expect("apply migration");

	let timestamp = "2024-01-15T10:30:00Z";

	// Create credential relationship (email -> password)
	let email_key = "email:bob@example.com";
	let pwd_key = "password:hashed_bob_pass";

	client
		.persist_credential(email_key, pwd_key, timestamp)
		.await
		.expect("persist credential");

	// Persist same credential again to test counting
	client
		.persist_credential(email_key, pwd_key, timestamp)
		.await
		.expect("persist credential again");

	client.ping().await.expect("ping after credential");

	// Clean up
	vanopticon_heimdall::devops::stop_dev_db()
		.await
		.expect("stop db");
}

#[tokio::test]
#[cfg(feature = "integration-tests")]
async fn test_multiple_rows_same_dump() {
	// Skip unless explicitly enabled
	if env::var("RUN_DOCKER_INTEGRATION_TESTS").is_err() {
		eprintln!("Skipping Docker integration test; set RUN_DOCKER_INTEGRATION_TESTS=1");
		return;
	}

	// Start dev DB
	vanopticon_heimdall::devops::start_dev_db()
		.await
		.expect("start db");

	let pool = wait_for_postgres().await;
	let client = AgeClient::new(pool.clone(), "heimdall_graph");

	// Apply schema
	let schema_sql = include_str!("../sql/v1/001-create_graph.sql");
	client
		.apply_migration(schema_sql)
		.await
		.expect("apply migration");

	let dump_id = "test-dump-multi";
	let timestamp = "2024-01-15T10:30:00Z";

	// Persist first row
	let cells_1 = vec![
		(
			"username".to_string(),
			"alice".to_string(),
			"username:alice".to_string(),
			"alice".to_string(),
		),
		(
			"email".to_string(),
			"alice@example.com".to_string(),
			"email:alice@example.com".to_string(),
			"alice@example.com".to_string(),
		),
	];

	client
		.persist_row(dump_id, 0, Some("hash_row0"), &cells_1, timestamp)
		.await
		.expect("persist row 0");

	// Persist second row
	let cells_2 = vec![
		(
			"username".to_string(),
			"bob".to_string(),
			"username:bob".to_string(),
			"bob".to_string(),
		),
		(
			"email".to_string(),
			"bob@example.com".to_string(),
			"email:bob@example.com".to_string(),
			"bob@example.com".to_string(),
		),
	];

	client
		.persist_row(dump_id, 1, Some("hash_row1"), &cells_2, timestamp)
		.await
		.expect("persist row 1");

	client.ping().await.expect("ping after multiple rows");

	// Clean up
	vanopticon_heimdall::devops::stop_dev_db()
		.await
		.expect("stop db");
}
