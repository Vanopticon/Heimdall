use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc::{self, Sender};
use tokio::time::Duration;

use crate::age_client::AgeRepo;
use crate::observability::MetricsRegistry;
use serde_json::Value;

/// A single persistence job: represents a normalized and sanitized record
/// that is safe to persist. Do NOT construct this from raw, unprocessed
/// data; use the normalization pipeline to produce instances of this type.
#[derive(Clone, Debug)]
pub struct PersistJob {
	/// Cypher label to use for the node (e.g., "FieldValue")
	pub label: String,
	/// Canonical key for the entity (used for MERGE)
	pub key: String,
	/// Properties to set on the node. These should be sanitized values
	/// (no raw PII). Prefer storing canonical values rather than original raw
	/// payloads.
	pub props: Value,
}

/// Sender side exported type
pub type PersistSender = Sender<PersistJob>;

/// Submit a job to the persistence sender while recording a simple metric.
/// This helper centralizes submission so metrics are accurate and callers
/// don't need to remember to increment counters.
pub fn submit_job(
	sender: &PersistSender,
	job: PersistJob,
	metrics: &Arc<MetricsRegistry>,
) -> Result<(), tokio::sync::mpsc::error::TrySendError<PersistJob>> {
	metrics.persist_jobs_submitted.inc();
	metrics.persist_queue_length.inc();
	sender.try_send(job)
}

/// Start a background batcher task that collects persistence jobs and
/// flushes them to the provided `repo` either when `batch_size` is
/// reached or when `flush_interval_ms` elapses. Returns the Sender which
/// can be used to submit `PersistJob`s.
///
/// This function spawns a detached task and returns immediately.
#[tracing::instrument(skip(repo, metrics))]
pub fn start_batcher(
	repo: Arc<dyn AgeRepo>,
	metrics: Arc<MetricsRegistry>,
	channel_capacity: usize,
	batch_size: usize,
	flush_interval_ms: u64,
) -> PersistSender {
	let (tx, mut rx) = mpsc::channel::<PersistJob>(channel_capacity);

	// Spawn the background worker
	tokio::spawn(async move {
		let mut buffer: Vec<PersistJob> = Vec::with_capacity(batch_size);
		let flush_interval = Duration::from_millis(flush_interval_ms);

		loop {
			tokio::select! {
				biased;
				maybe_job = rx.recv() => {
					match maybe_job {
						Some(job) => {
							metrics.persist_queue_length.dec();
							buffer.push(job);
							if buffer.len() >= batch_size {
								flush_buffer(&repo, &metrics, &mut buffer).await;
							}
						}
						None => {
							// Channel closed; flush remaining and exit
							if !buffer.is_empty() {
								flush_buffer(&repo, &metrics, &mut buffer).await;
							}
							break;
						}
					}
				}
				_ = tokio::time::sleep(flush_interval) => {
					if !buffer.is_empty() {
						flush_buffer(&repo, &metrics, &mut buffer).await;
					}
				}
			}
		}
	});

	tx
}

#[tracing::instrument(skip(repo, metrics, buffer), fields(batch_size = buffer.len()))]
async fn flush_buffer(
	repo: &Arc<dyn AgeRepo>,
	metrics: &Arc<MetricsRegistry>,
	buffer: &mut Vec<PersistJob>,
) {
	// Drain FIFO order
	let jobs: Vec<PersistJob> = buffer.drain(..).collect();
	if jobs.is_empty() {
		return;
	}

	// Attempt a single batched merge for improved throughput. Implementations
	// may fall back to individual merges when the batch fails.
	let tuples: Vec<(String, String, Value)> = jobs
		.iter()
		.map(|j| (j.label.clone(), j.key.clone(), j.props.clone()))
		.collect();

	// Measure batch latency and record metrics
	let start = Instant::now();
	let res = repo.merge_batch(&tuples).await;
	let elapsed_ms = start.elapsed().as_millis() as f64;
	metrics.persist_batch_flushes.inc();
	// Histogram expects milliseconds, as per metric name
	metrics.persist_batch_latency_ms.observe(elapsed_ms);

	if let Err(e) = res {
		metrics.persist_batch_failures.inc();
		eprintln!("persistence batch failed: {}", e);
		// If merge_batch returns an error, try per-item merges as a last resort
		for j in jobs {
			if let Err(e2) = repo.merge_entity(&j.label, &j.key, &j.props).await {
				metrics.persist_per_item_failures.inc();
				eprintln!("per-item persist failed for {}: {}", j.key, e2);
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn persist_job_creation() {
		let job = PersistJob {
			label: "TestLabel".to_string(),
			key: "test-key".to_string(),
			props: json!({"field": "value"}),
		};
		assert_eq!(job.label, "TestLabel");
		assert_eq!(job.key, "test-key");
	}

	#[test]
	fn metrics_text_format() {
		let metrics = metrics_text();
		assert!(metrics.contains("heimdall_persist_jobs_submitted_total"));
		assert!(metrics.contains("heimdall_persist_batch_flushes_total"));
		assert!(metrics.contains("heimdall_persist_batch_failures_total"));
		assert!(metrics.contains("heimdall_persist_per_item_failures_total"));
		assert!(metrics.contains("heimdall_persist_batch_flush_latency_ms_sum"));
		// Verify Prometheus format includes HELP and TYPE lines
		assert!(metrics.contains("# HELP"));
		assert!(metrics.contains("# TYPE"));
	}

	#[test]
	fn metrics_text_includes_counter_type() {
		let metrics = metrics_text();
		// Verify all metrics are marked as counters
		let type_count = metrics.matches("# TYPE").count();
		let counter_count = metrics.matches("counter").count();
		assert_eq!(type_count, counter_count);
	}

	#[tokio::test]
	async fn submit_job_increments_metric() {
		// Read initial value
		let initial = PERSIST_JOBS_SUBMITTED.load(Ordering::Relaxed);

		// Create a channel with capacity 10
		let (tx, _rx) = mpsc::channel::<PersistJob>(10);

		let job = PersistJob {
			label: "TestLabel".to_string(),
			key: "test-key".to_string(),
			props: json!({}),
		};

		// Submit job
		let result = submit_job(&tx, job);
		assert!(result.is_ok());

		// Verify metric incremented
		let after = PERSIST_JOBS_SUBMITTED.load(Ordering::Relaxed);
		assert_eq!(after, initial + 1);
	}

	#[tokio::test]
	async fn submit_job_fails_when_channel_full() {
		// Create a channel with capacity 1, fill it, then try to send another
		let (tx, _rx) = mpsc::channel::<PersistJob>(1);

		// Fill the channel
		let job1 = PersistJob {
			label: "TestLabel".to_string(),
			key: "test-key-1".to_string(),
			props: json!({}),
		};
		let result1 = submit_job(&tx, job1);
		assert!(result1.is_ok());

		// Try to send another job - should fail because channel is full
		let job2 = PersistJob {
			label: "TestLabel".to_string(),
			key: "test-key-2".to_string(),
			props: json!({}),
		};
		let result2 = submit_job(&tx, job2);
		assert!(result2.is_err());
	}
}
