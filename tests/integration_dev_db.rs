mod common;

use serde_json::json;

#[tokio::test]
async fn integration_dev_db_merge() {
	// This integration test is gated behind an env var to avoid running Docker in CI by default.
	if !common::check_docker_enabled() {
		return;
	}

	// Start the dev DB (use default options)
	vanopticon_heimdall::devops::start_dev_db()
		.await
		.expect("start db");

	// Wait for Postgres to accept connections
	let pool = common::wait_for_postgres("postgres://heimdall:heimdall@127.0.0.1:5432/heimdall", 30)
		.await
		.expect("connect to postgres");

	// Use AgeClient to perform a merge
	let client = vanopticon_heimdall::age_client::AgeClient::new(pool.clone(), "heimdall_graph");

	client
		.merge_entity("TestNode", "integration-key-1", &json!({"foo": "bar"}))
		.await
		.expect("merge_entity succeeded");

	// Tear down the compose stack
	vanopticon_heimdall::devops::stop_dev_db()
		.await
		.expect("stop db");
}
