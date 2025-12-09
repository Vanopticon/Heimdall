use prometheus::{
	Counter, Gauge, Histogram, HistogramOpts, IntCounter, IntGauge, Opts, Registry, TextEncoder,
};
use std::sync::Arc;

/// Central registry for all Prometheus metrics
pub struct MetricsRegistry {
	registry: Registry,

	// Ingest metrics
	pub ingest_requests_total: IntCounter,
	pub ingest_records_total: IntCounter,
	pub ingest_errors_total: IntCounter,
	pub ingest_bytes_total: Counter,
	pub ingest_duration_seconds: Histogram,

	// Persistence metrics
	pub persist_jobs_submitted: IntCounter,
	pub persist_batch_flushes: IntCounter,
	pub persist_batch_failures: IntCounter,
	pub persist_per_item_failures: IntCounter,
	pub persist_queue_length: IntGauge,
	pub persist_batch_latency_ms: Histogram,

	// Sync metrics (for future multi-Heimdall sync)
	pub sync_lag_seconds: Gauge,
	pub sync_operations_total: IntCounter,
	pub sync_errors_total: IntCounter,

	// Enrichment metrics
	pub enrichment_requests_total: IntCounter,
	pub enrichment_failures_total: IntCounter,
	pub enrichment_duration_seconds: Histogram,
}

impl MetricsRegistry {
	pub fn new() -> Self {
		let registry = Registry::new();

		// Ingest metrics
		let ingest_requests_total = IntCounter::with_opts(
			Opts::new(
				"heimdall_ingest_requests_total",
				"Total number of ingest requests received",
			)
			.namespace("heimdall"),
		)
		.unwrap();

		let ingest_records_total = IntCounter::with_opts(
			Opts::new(
				"heimdall_ingest_records_total",
				"Total number of records ingested",
			)
			.namespace("heimdall"),
		)
		.unwrap();

		let ingest_errors_total = IntCounter::with_opts(
			Opts::new(
				"heimdall_ingest_errors_total",
				"Total number of ingest errors",
			)
			.namespace("heimdall"),
		)
		.unwrap();

		let ingest_bytes_total = Counter::with_opts(
			Opts::new(
				"heimdall_ingest_bytes_total",
				"Total bytes ingested",
			)
			.namespace("heimdall"),
		)
		.unwrap();

		let ingest_duration_seconds = Histogram::with_opts(
			HistogramOpts::new(
				"heimdall_ingest_duration_seconds",
				"Duration of ingest operations in seconds",
			)
			.namespace("heimdall")
			.buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]),
		)
		.unwrap();

		// Persistence metrics
		let persist_jobs_submitted = IntCounter::with_opts(
			Opts::new(
				"heimdall_persist_jobs_submitted_total",
				"Total persist jobs submitted",
			)
			.namespace("heimdall"),
		)
		.unwrap();

		let persist_batch_flushes = IntCounter::with_opts(
			Opts::new(
				"heimdall_persist_batch_flushes_total",
				"Number of batch flushes",
			)
			.namespace("heimdall"),
		)
		.unwrap();

		let persist_batch_failures = IntCounter::with_opts(
			Opts::new(
				"heimdall_persist_batch_failures_total",
				"Number of batch failures",
			)
			.namespace("heimdall"),
		)
		.unwrap();

		let persist_per_item_failures = IntCounter::with_opts(
			Opts::new(
				"heimdall_persist_per_item_failures_total",
				"Per-item persistence failures",
			)
			.namespace("heimdall"),
		)
		.unwrap();

		let persist_queue_length = IntGauge::with_opts(
			Opts::new(
				"heimdall_persist_queue_length",
				"Current length of persistence queue",
			)
			.namespace("heimdall"),
		)
		.unwrap();

		let persist_batch_latency_ms = Histogram::with_opts(
			HistogramOpts::new(
				"heimdall_persist_batch_latency_ms",
				"Batch flush latency in milliseconds",
			)
			.namespace("heimdall")
			.buckets(vec![1.0, 5.0, 10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0]),
		)
		.unwrap();

		// Sync metrics
		let sync_lag_seconds = Gauge::with_opts(
			Opts::new(
				"heimdall_sync_lag_seconds",
				"Synchronization lag in seconds",
			)
			.namespace("heimdall"),
		)
		.unwrap();

		let sync_operations_total = IntCounter::with_opts(
			Opts::new(
				"heimdall_sync_operations_total",
				"Total sync operations",
			)
			.namespace("heimdall"),
		)
		.unwrap();

		let sync_errors_total = IntCounter::with_opts(
			Opts::new(
				"heimdall_sync_errors_total",
				"Total sync errors",
			)
			.namespace("heimdall"),
		)
		.unwrap();

		// Enrichment metrics
		let enrichment_requests_total = IntCounter::with_opts(
			Opts::new(
				"heimdall_enrichment_requests_total",
				"Total enrichment requests",
			)
			.namespace("heimdall"),
		)
		.unwrap();

		let enrichment_failures_total = IntCounter::with_opts(
			Opts::new(
				"heimdall_enrichment_failures_total",
				"Total enrichment failures",
			)
			.namespace("heimdall"),
		)
		.unwrap();

		let enrichment_duration_seconds = Histogram::with_opts(
			HistogramOpts::new(
				"heimdall_enrichment_duration_seconds",
				"Duration of enrichment operations in seconds",
			)
			.namespace("heimdall")
			.buckets(vec![0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]),
		)
		.unwrap();

		// Register all metrics
		registry
			.register(Box::new(ingest_requests_total.clone()))
			.unwrap();
		registry
			.register(Box::new(ingest_records_total.clone()))
			.unwrap();
		registry
			.register(Box::new(ingest_errors_total.clone()))
			.unwrap();
		registry
			.register(Box::new(ingest_bytes_total.clone()))
			.unwrap();
		registry
			.register(Box::new(ingest_duration_seconds.clone()))
			.unwrap();
		registry
			.register(Box::new(persist_jobs_submitted.clone()))
			.unwrap();
		registry
			.register(Box::new(persist_batch_flushes.clone()))
			.unwrap();
		registry
			.register(Box::new(persist_batch_failures.clone()))
			.unwrap();
		registry
			.register(Box::new(persist_per_item_failures.clone()))
			.unwrap();
		registry
			.register(Box::new(persist_queue_length.clone()))
			.unwrap();
		registry
			.register(Box::new(persist_batch_latency_ms.clone()))
			.unwrap();
		registry
			.register(Box::new(sync_lag_seconds.clone()))
			.unwrap();
		registry
			.register(Box::new(sync_operations_total.clone()))
			.unwrap();
		registry
			.register(Box::new(sync_errors_total.clone()))
			.unwrap();
		registry
			.register(Box::new(enrichment_requests_total.clone()))
			.unwrap();
		registry
			.register(Box::new(enrichment_failures_total.clone()))
			.unwrap();
		registry
			.register(Box::new(enrichment_duration_seconds.clone()))
			.unwrap();

		Self {
			registry,
			ingest_requests_total,
			ingest_records_total,
			ingest_errors_total,
			ingest_bytes_total,
			ingest_duration_seconds,
			persist_jobs_submitted,
			persist_batch_flushes,
			persist_batch_failures,
			persist_per_item_failures,
			persist_queue_length,
			persist_batch_latency_ms,
			sync_lag_seconds,
			sync_operations_total,
			sync_errors_total,
			enrichment_requests_total,
			enrichment_failures_total,
			enrichment_duration_seconds,
		}
	}

	/// Encode metrics in Prometheus text format
	pub fn encode(&self) -> String {
		let encoder = TextEncoder::new();
		let metric_families = self.registry.gather();
		match encoder.encode_to_string(&metric_families) {
			Ok(s) => s,
			Err(e) => {
				eprintln!("Failed to encode metrics: {}", e);
				String::new()
			}
		}
	}
}

impl Default for MetricsRegistry {
	fn default() -> Self {
		Self::new()
	}
}

/// Initialize the global metrics registry
pub fn init_metrics() -> anyhow::Result<Arc<MetricsRegistry>> {
	Ok(Arc::new(MetricsRegistry::new()))
}

#[cfg(feature = "unit-tests")]
mod tests {
	#[test]
	fn metrics_registry_creation() {
		let registry = super::MetricsRegistry::new();
		assert!(!registry.encode().is_empty());
	}

	#[test]
	fn metrics_increment() {
		let registry = super::MetricsRegistry::new();
		registry.ingest_requests_total.inc();
		registry.ingest_records_total.inc_by(10);
		assert!(!registry.encode().is_empty());
	}
}
