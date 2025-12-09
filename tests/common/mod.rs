/// Common test utilities and helpers for integration tests.
use std::env;
use tokio::time::{Duration, sleep};

/// Check if Docker integration tests are enabled via environment variable.
/// Returns true if RUN_DOCKER_INTEGRATION_TESTS is set.
pub fn is_docker_test_enabled() -> bool {
	env::var("RUN_DOCKER_INTEGRATION_TESTS").is_ok()
}

/// Skip the test with a message if Docker integration tests are not enabled.
/// Call this at the start of integration tests that require Docker.
/// Returns true if the test should proceed, false if it should be skipped.
pub fn check_docker_enabled() -> bool {
	if !is_docker_test_enabled() {
		eprintln!("Skipping Docker integration test; set RUN_DOCKER_INTEGRATION_TESTS=1 to enable");
		return false;
	}
	true
}

/// Wait for Postgres to accept connections with a maximum retry count.
/// Returns the connection pool on success, or an error if retries exhausted.
pub async fn wait_for_postgres(
	connection_string: &str,
	max_retries: u32,
) -> Result<sqlx::PgPool, String> {
	let mut attempts = 0;
	loop {
		match sqlx::PgPool::connect(connection_string).await {
			Ok(pool) => return Ok(pool),
			Err(e) => {
				attempts += 1;
				if attempts >= max_retries {
					return Err(format!(
						"Postgres did not become ready after {} attempts: {}",
						max_retries, e
					));
				}
				sleep(Duration::from_secs(1)).await;
			}
		}
	}
}

/// Escape a string value for use in Cypher queries to prevent injection.
/// This is a simple escaping function for test purposes.
/// In production code, use parameterized queries instead.
#[allow(dead_code)]
pub fn escape_cypher_string(value: &str) -> String {
	// Escape double quotes and backslashes
	value.replace('\\', "\\\\").replace('"', "\\\"")
}
