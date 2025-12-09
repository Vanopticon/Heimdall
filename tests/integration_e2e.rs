mod common;

use axum::body::Body;
use axum::http::Request;
use axum::response::IntoResponse;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

#[tokio::test]
async fn e2e_ndjson_upload_persists() {
	// This integration test is gated by an env var to avoid running Docker in CI
	if !common::check_docker_enabled() {
		return;
	}

	// Start the dev DB
	vanopticon_heimdall::devops::start_dev_db()
		.await
		.expect("start db");

	// Wait for Postgres to accept connections
	let pool = common::wait_for_postgres("postgres://heimdall:heimdall@127.0.0.1:5432/heimdall", 30)
		.await
		.expect("connect to postgres");

	// Build an AgeClient and shared repo
	let client = vanopticon_heimdall::age_client::AgeClient::new(pool.clone(), "heimdall_graph");
	let repo: std::sync::Arc<dyn vanopticon_heimdall::age_client::AgeRepo> = Arc::new(client);

	// Start a batcher that flushes immediately (batch_size=1)
	let sender = vanopticon_heimdall::persist::start_batcher(repo.clone(), 1024, 1, 100);

	let app_state = vanopticon_heimdall::state::AppState {
		repo: repo.clone(),
		persist_sender: sender.clone(),
	};

	// Build NDJSON payload and call handler directly
	let payload = r#"{"field_type":"domain","value":"Example.COM"}
"#;
	let req = Request::builder()
		.method("POST")
		.uri("/ingest/ndjson")
		.body(Body::from(payload.to_string()))
		.unwrap();

	// Call the handler directly with the app state
	let resp =
		vanopticon_heimdall::ingest::ndjson_upload(axum::extract::State(app_state.clone()), req)
			.await
			.into_response();
	assert_eq!(resp.status(), axum::http::StatusCode::OK);

	// Allow a short time for the batcher to flush
	sleep(Duration::from_millis(500)).await;

	// Verify DB contains the merged node (canonical: example.com)
	let sql = "SELECT * FROM cypher($1::text, $2::text) as (v agtype);";
	let cypher = format!(
		"MATCH (n:FieldValue {{canonical_key: \"{}\"}}) RETURN n LIMIT 1",
		"example.com"
	);
	let row = sqlx::query(sql)
		.bind("heimdall_graph")
		.bind(&cypher)
		.fetch_optional(&pool)
		.await
		.expect("query");

	assert!(row.is_some(), "expected a merged node in the DB");

	// Tear down DB
	vanopticon_heimdall::devops::stop_dev_db()
		.await
		.expect("stop db");
}
