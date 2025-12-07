use serde_json::json;
use std::env;
use tokio::time::{Duration, sleep};

#[tokio::test]
async fn integration_dev_db_merge() {
	// This integration test is gated behind an env var to avoid running Docker in CI by default.
	if env::var("RUN_DOCKER_INTEGRATION_TESTS").is_err() {
		eprintln!("Skipping Docker integration test; set RUN_DOCKER_INTEGRATION_TESTS=1 to enable");
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
