//! Test utilities for ingest module tests.
//!
//! This module provides shared test fixtures and utilities to reduce code duplication
//! in test code.

#![cfg(test)]

use std::sync::Arc;
use tokio::sync::mpsc;

/// A dummy repository implementation for testing that accepts all operations.
pub struct DummyRepo;

#[async_trait::async_trait]
impl crate::age_client::AgeRepo for DummyRepo {
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

/// Create a test AppState with a dummy repository and channel.
pub fn create_test_app_state() -> crate::state::AppState {
	let (tx, _rx) = mpsc::channel(16);
	let repo: Arc<dyn crate::age_client::AgeRepo> = Arc::new(DummyRepo);
	crate::state::AppState {
		repo,
		persist_sender: tx,
	}
}
