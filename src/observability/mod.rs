pub mod logging;
pub mod metrics;
pub mod tracing_setup;

pub use logging::init_logging;
pub use metrics::{MetricsRegistry, init_metrics};
pub use tracing_setup::init_tracing;

use std::sync::Arc;

/// Global observability state
pub struct ObservabilityState {
	pub metrics: Arc<MetricsRegistry>,
}

impl ObservabilityState {
	pub fn new() -> Self {
		Self {
			metrics: Arc::new(MetricsRegistry::new()),
		}
	}
}

impl Default for ObservabilityState {
	fn default() -> Self {
		Self::new()
	}
}

/// Initialize all observability components
pub async fn init_observability() -> anyhow::Result<ObservabilityState> {
	// Initialize structured JSON logging
	init_logging()?;

	// Initialize Prometheus metrics registry
	let metrics = init_metrics()?;

	// Initialize OpenTelemetry tracing
	init_tracing().await?;

	tracing::info!(
		component = "observability",
		"Observability initialized: structured logging, metrics, and tracing enabled"
	);

	Ok(ObservabilityState { metrics })
}

#[cfg(feature = "unit-tests")]
mod tests {
	#[test]
	fn observability_state_creation() {
		let state = super::ObservabilityState::new();
		assert!(!state.metrics.encode().is_empty());
	}
}
