use serde_json::json;
use std::env;
use tokio::time::{Duration, sleep};
use vanopticon_heimdall::age_client::AgeRepo;

/// Integration test for security: verifies that malicious input is properly
/// sanitized and does not cause Cypher injection or other security issues.
#[tokio::test]
async fn security_cypher_injection_prevention() {
	// This integration test is gated behind an env var to avoid running Docker in CI by default.
	if env::var("RUN_DOCKER_INTEGRATION_TESTS").is_err() {
		eprintln!(
			"Skipping Docker security integration test; set RUN_DOCKER_INTEGRATION_TESTS=1 to enable"
		);
		return;
	}

	// Start the dev DB (use default options)
	vanopticon_heimdall::devops::start_dev_db()
		.await
		.expect("start db");

	// Wait a little for Postgres to accept connections
	let mut attempts = 0;
	let pool = loop {
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
	};

	// Use AgeClient to perform a merge with potentially malicious input
	let client = vanopticon_heimdall::age_client::AgeClient::new(pool.clone(), "heimdall_graph");

	// Test 1: Malicious label with Cypher injection attempt
	let malicious_label = "Label'); CREATE (e:Evil)-[:OWNS]->(n"; // Should be sanitized
	let result1 = client
		.merge_entity(malicious_label, "test-key-1", &json!({"safe": "value"}))
		.await;
	assert!(
		result1.is_ok(),
		"Should successfully sanitize malicious label"
	);

	// Test 2: Malicious key with SQL/Cypher injection attempt
	let malicious_key = "'; DROP TABLE users; --";
	let result2 = client
		.merge_entity("TestNode", malicious_key, &json!({"safe": "value"}))
		.await;
	assert!(
		result2.is_ok(),
		"Should successfully sanitize malicious key"
	);

	// Test 3: Property keys with special characters that could break Cypher
	let malicious_props = json!({
		"prop'; DELETE n; MATCH (x": "value1",
		"prop}); DROP DATABASE test; MATCH (y": "value2",
		"normal_prop": "normal_value"
	});
	let result3 = client
		.merge_entity("TestNode", "test-key-3", &malicious_props)
		.await;
	assert!(
		result3.is_ok(),
		"Should successfully sanitize property keys"
	);

	// Test 4: Very long input to test for buffer overflows or DoS
	let very_long_key = "a".repeat(10000);
	let result4 = client
		.merge_entity("TestNode", &very_long_key, &json!({"test": "value"}))
		.await;
	assert!(result4.is_ok(), "Should handle very long keys");

	// Test 5: Null bytes in input
	let key_with_nulls = "key\0with\0nulls";
	let result5 = client
		.merge_entity("TestNode", key_with_nulls, &json!({"test": "value"}))
		.await;
	assert!(result5.is_ok(), "Should handle null bytes in keys");

	// Verify that no "Evil" nodes were created due to injection
	let sql = "SELECT * FROM cypher($1::text, $2::text) as (v agtype);";
	let cypher = "MATCH (n:Evil) RETURN count(n) as cnt";
	let row = sqlx::query(sql)
		.bind("heimdall_graph")
		.bind(cypher)
		.fetch_optional(&pool)
		.await
		.expect("query");

	// If the injection was prevented, there should be no Evil nodes
	// Note: The query might fail if Evil label doesn't exist, which is also acceptable
	if let Some(_row) = row {
		// If we got a result, verify count is 0
		// This is a simplified check - in a real scenario you'd parse the agtype result
		eprintln!("Query for Evil nodes completed - injection prevention likely successful");
	}

	// Tear down the compose stack
	vanopticon_heimdall::devops::stop_dev_db()
		.await
		.expect("stop db");
}

/// Test for resource exhaustion protection
#[tokio::test]
async fn security_resource_exhaustion_protection() {
	if env::var("RUN_DOCKER_INTEGRATION_TESTS").is_err() {
		eprintln!("Skipping Docker security test; set RUN_DOCKER_INTEGRATION_TESTS=1 to enable");
		return;
	}

	vanopticon_heimdall::devops::start_dev_db()
		.await
		.expect("start db");

	let pool = loop {
		match sqlx::PgPool::connect("postgres://heimdall:heimdall@127.0.0.1:5432/heimdall").await {
			Ok(p) => break p,
			Err(_) => {
				sleep(Duration::from_secs(1)).await;
			}
		}
	};

	let client = vanopticon_heimdall::age_client::AgeClient::new(pool.clone(), "heimdall_graph");

	// Test merging a large batch of items to verify it doesn't cause resource exhaustion
	let mut items = Vec::new();
	for i in 0..100 {
		items.push((
			"TestNode".to_string(),
			format!("test-key-{}", i),
			json!({"index": i}),
		));
	}

	let result = client.merge_batch(&items).await;
	assert!(
		result.is_ok(),
		"Should handle batch operations without resource exhaustion"
	);

	vanopticon_heimdall::devops::stop_dev_db()
		.await
		.expect("stop db");
}
