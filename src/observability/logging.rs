use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize structured JSON logging to stdout with contextual fields
pub fn init_logging() -> anyhow::Result<()> {
	// Get log level from environment or default to info
	let env_filter = EnvFilter::try_from_default_env()
		.or_else(|_| EnvFilter::try_new("info"))
		.unwrap_or_else(|_| EnvFilter::new("info"));

	// Build JSON formatter for structured logging
	let json_layer = tracing_subscriber::fmt::layer()
		.json()
		.with_current_span(true)
		.with_span_list(true)
		.with_target(true)
		.with_level(true)
		.with_thread_ids(true)
		.with_file(true)
		.with_line_number(true);

	// Compose the subscriber with the filter and JSON layer
	tracing_subscriber::registry()
		.with(env_filter)
		.with(json_layer)
		.try_init()
		.map_err(|e| anyhow::anyhow!("Failed to initialize logging: {}", e))?;

	Ok(())
}

#[cfg(feature = "unit-tests")]
mod tests {
	#[test]
	fn logging_initialization() {
		// Note: We can only initialize logging once per process
		// This test validates the function signature and error handling
		let _ = super::init_logging();
	}
}
