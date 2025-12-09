use vanopticon_heimdall::observability::{MetricsRegistry, init_metrics};

#[cfg(feature = "unit-tests")]
#[test]
fn test_metrics_registry_creation() {
	let registry = MetricsRegistry::new();
	let output = registry.encode();

	// Verify metrics are present in the output
	assert!(output.contains("heimdall_ingest_requests_total"));
	assert!(output.contains("heimdall_ingest_records_total"));
	assert!(output.contains("heimdall_persist_jobs_submitted_total"));
	assert!(output.contains("heimdall_persist_batch_flushes_total"));
	assert!(output.contains("heimdall_sync_lag_seconds"));
	assert!(output.contains("heimdall_enrichment_requests_total"));
}

#[cfg(feature = "unit-tests")]
#[test]
fn test_metrics_increment() {
	let registry = MetricsRegistry::new();

	// Increment some counters
	registry.ingest_requests_total.inc();
	registry.ingest_records_total.inc_by(5);
	registry.persist_jobs_submitted.inc_by(10);

	let output = registry.encode();

	// The output should reflect the incremented values
	assert!(output.contains("heimdall_ingest_requests_total 1"));
	assert!(output.contains("heimdall_ingest_records_total 5"));
	assert!(output.contains("heimdall_persist_jobs_submitted_total 10"));
}

#[cfg(feature = "unit-tests")]
#[test]
fn test_metrics_gauge_operations() {
	let registry = MetricsRegistry::new();

	// Set and modify gauge values
	registry.persist_queue_length.set(50);
	registry.sync_lag_seconds.set(1.5);

	let output = registry.encode();

	// Gauges should show their set values
	assert!(output.contains("heimdall_persist_queue_length 50"));
	assert!(output.contains("heimdall_sync_lag_seconds 1.5"));

	// Increment and decrement
	registry.persist_queue_length.inc();
	registry.persist_queue_length.dec();

	let output2 = registry.encode();
	assert!(output2.contains("heimdall_persist_queue_length 50"));
}

#[cfg(feature = "unit-tests")]
#[test]
fn test_metrics_histogram_observations() {
	let registry = MetricsRegistry::new();

	// Observe some values
	registry.ingest_duration_seconds.observe(0.1);
	registry.ingest_duration_seconds.observe(0.5);
	registry.ingest_duration_seconds.observe(1.2);

	let output = registry.encode();

	// Histograms generate multiple series (buckets, sum, count)
	assert!(output.contains("heimdall_ingest_duration_seconds_bucket"));
	assert!(output.contains("heimdall_ingest_duration_seconds_sum"));
	assert!(output.contains("heimdall_ingest_duration_seconds_count"));
}

#[cfg(feature = "unit-tests")]
#[test]
fn test_init_metrics() {
	let result = init_metrics();
	assert!(result.is_ok());
	let metrics = result.unwrap();
	assert!(!metrics.encode().is_empty());
}

#[cfg(feature = "integration-tests")]
#[tokio::test]
async fn test_metrics_in_persist_workflow() {
	use std::sync::Arc;
	use vanopticon_heimdall::persist::{PersistJob, submit_job, start_batcher};
	use serde_json::json;

	// Create a dummy repo
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

	let repo: Arc<dyn vanopticon_heimdall::age_client::AgeRepo> = Arc::new(DummyRepo);
	let metrics = Arc::new(MetricsRegistry::new());

	// Start batcher
	let sender = start_batcher(repo.clone(), metrics.clone(), 100, 10, 100);

	// Submit some jobs
	for i in 0..5 {
		let job = PersistJob {
			label: "TestNode".to_string(),
			key: format!("key_{}", i),
			props: json!({ "test": true }),
		};
		let _ = submit_job(&sender, job, &metrics);
	}

	// Give the batcher time to process
	tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

	// Check metrics
	let output = metrics.encode();
	assert!(output.contains("heimdall_persist_jobs_submitted_total 5"));
	assert!(output.contains("heimdall_persist_batch_flushes_total"));
}

#[cfg(feature = "integration-tests")]
#[tokio::test]
async fn test_tracing_spans_in_handler() {
	use std::sync::Arc;
	use axum::{body::Body, extract::State, http::Request};
	use vanopticon_heimdall::ingest::ndjson_upload;
	use vanopticon_heimdall::state::AppState;

	// Setup dummy repo and metrics
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

	let repo: Arc<dyn vanopticon_heimdall::age_client::AgeRepo> = Arc::new(DummyRepo);
	let metrics = Arc::new(MetricsRegistry::new());
	let (tx, _rx) = tokio::sync::mpsc::channel(16);

	let app_state = AppState {
		repo,
		persist_sender: tx,
		metrics: metrics.clone(),
	};

	// Create a test request
	let payload = r#"{"field_type":"domain","value":"example.com"}"#;
	let req = Request::builder()
		.method("POST")
		.uri("/ingest/ndjson")
		.body(Body::from(payload.to_string()))
		.unwrap();

	// Call the handler (tracing instrumentation should be applied)
	let _resp = ndjson_upload(State(app_state), req).await;

	// Verify metrics were updated
	let output = metrics.encode();
	assert!(output.contains("heimdall_ingest_requests_total 1"));
}
