use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::sync::mpsc::{self, Sender};
use tokio::time::Duration;

use crate::age_client::AgeRepo;
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

// Simple in-process metrics exposed via the /metrics endpoint. We avoid
// adding a heavy dependency (Prometheus client) for now and expose a
// minimal Prometheus-compatible text format from the application.
static PERSIST_JOBS_SUBMITTED: AtomicU64 = AtomicU64::new(0);
static PERSIST_BATCH_FLUSHES: AtomicU64 = AtomicU64::new(0);
static PERSIST_BATCH_FAILURES: AtomicU64 = AtomicU64::new(0);
static PERSIST_PER_ITEM_FAILURES: AtomicU64 = AtomicU64::new(0);
static PERSIST_BATCH_LATENCY_MS_SUM: AtomicU64 = AtomicU64::new(0);

/// Submit a job to the persistence sender while recording a simple metric.
/// This helper centralizes submission so metrics are accurate and callers
/// don't need to remember to increment counters.
pub fn submit_job(
	sender: &PersistSender,
	job: PersistJob,
) -> Result<(), tokio::sync::mpsc::error::TrySendError<PersistJob>> {
	PERSIST_JOBS_SUBMITTED.fetch_add(1, Ordering::Relaxed);
	sender.try_send(job)
}

/// Return a small Prometheus-compatible metrics payload describing persistence
/// queue and batcher activity.
pub fn metrics_text() -> String {
	let mut out = String::new();
	out.push_str("# HELP heimdall_persist_jobs_submitted_total Total persist jobs submitted\n");
	out.push_str("# TYPE heimdall_persist_jobs_submitted_total counter\n");
	out.push_str(&format!(
		"heimdall_persist_jobs_submitted_total {}\n",
		PERSIST_JOBS_SUBMITTED.load(Ordering::Relaxed)
	));

	out.push_str("# HELP heimdall_persist_batch_flushes_total Number of batch flushes\n");
	out.push_str("# TYPE heimdall_persist_batch_flushes_total counter\n");
	out.push_str(&format!(
		"heimdall_persist_batch_flushes_total {}\n",
		PERSIST_BATCH_FLUSHES.load(Ordering::Relaxed)
	));

	out.push_str("# HELP heimdall_persist_batch_failures_total Number of batch failures\n");
	out.push_str("# TYPE heimdall_persist_batch_failures_total counter\n");
	out.push_str(&format!(
		"heimdall_persist_batch_failures_total {}\n",
		PERSIST_BATCH_FAILURES.load(Ordering::Relaxed)
	));

	out.push_str("# HELP heimdall_persist_per_item_failures_total Per-item persistence failures\n");
	out.push_str("# TYPE heimdall_persist_per_item_failures_total counter\n");
	out.push_str(&format!(
		"heimdall_persist_per_item_failures_total {}\n",
		PERSIST_PER_ITEM_FAILURES.load(Ordering::Relaxed)
	));

	out.push_str(
		"# HELP heimdall_persist_batch_flush_latency_ms_sum Cumulative batch flush latency in ms\n",
	);
	out.push_str("# TYPE heimdall_persist_batch_flush_latency_ms_sum counter\n");
	out.push_str(&format!(
		"heimdall_persist_batch_flush_latency_ms_sum {}\n",
		PERSIST_BATCH_LATENCY_MS_SUM.load(Ordering::Relaxed)
	));

	out
}

/// Start a background batcher task that collects persistence jobs and
/// flushes them to the provided `repo` either when `batch_size` is
/// reached or when `flush_interval_ms` elapses. Returns the Sender which
/// can be used to submit `PersistJob`s.
///
/// This function spawns a detached task and returns immediately.
pub fn start_batcher(
	repo: Arc<dyn AgeRepo>,
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
							buffer.push(job);
							if buffer.len() >= batch_size {
								flush_buffer(&repo, &mut buffer).await;
							}
						}
						None => {
							// Channel closed; flush remaining and exit
							if !buffer.is_empty() {
								flush_buffer(&repo, &mut buffer).await;
							}
							break;
						}
					}
				}
				_ = tokio::time::sleep(flush_interval) => {
					if !buffer.is_empty() {
						flush_buffer(&repo, &mut buffer).await;
					}
				}
			}
		}
	});

	tx
}

async fn flush_buffer(repo: &Arc<dyn AgeRepo>, buffer: &mut Vec<PersistJob>) {
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

	// Measure batch latency and record simple metrics
	let start = Instant::now();
	let res = repo.merge_batch(&tuples).await;
	let elapsed_ms = start.elapsed().as_millis() as u64;
	PERSIST_BATCH_FLUSHES.fetch_add(1, Ordering::Relaxed);
	PERSIST_BATCH_LATENCY_MS_SUM.fetch_add(elapsed_ms, Ordering::Relaxed);

	if let Err(e) = res {
		PERSIST_BATCH_FAILURES.fetch_add(1, Ordering::Relaxed);
		eprintln!("persistence batch failed: {}", e);
		// If merge_batch returns an error, try per-item merges as a last resort
		for j in jobs {
			if let Err(e2) = repo.merge_entity(&j.label, &j.key, &j.props).await {
				PERSIST_PER_ITEM_FAILURES.fetch_add(1, Ordering::Relaxed);
				eprintln!("per-item persist failed for {}: {}", j.key, e2);
			}
		}
	}
}
