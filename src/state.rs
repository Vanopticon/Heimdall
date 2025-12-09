use std::sync::Arc;

use crate::age_client::AgeRepo;
use crate::observability::MetricsRegistry;

/// Application state passed to handlers via Axum's `State` extractor.
///
/// Holds a shared `AgeRepo` and a sender to the persistence batcher so
/// handlers can enqueue records without blocking on DB round-trips.
#[derive(Clone)]
pub struct AppState {
	pub repo: Arc<dyn AgeRepo>,
	pub persist_sender: tokio::sync::mpsc::Sender<crate::persist::PersistJob>,
}
