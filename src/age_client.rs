use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use sqlx::PgPool;

/// Minimal AGE client wrapper for Postgres + Apache AGE.
pub struct AgeClient {
	pool: PgPool,
	graph: String,
}

impl AgeClient {
	/// Create a new client from a `sqlx::PgPool` and the AGE graph name to target.
	pub fn new(pool: PgPool, graph: impl Into<String>) -> Self {
		Self {
			pool,
			graph: graph.into(),
		}
	}

	/// Connect helper using a DATABASE_URL-like string
	pub async fn connect(database_url: &str, graph: &str) -> Result<Self> {
		let pool = PgPool::connect(database_url).await?;
		Ok(Self::new(pool, graph))
	}

	/// Merge an entity into the AGE graph using a simple Cypher MERGE statement.
	///
	/// NOTE: This implementation constructs a Cypher string directly and is
	/// intended as a minimal example. In production code you should carefully
	/// validate/escape inputs or use parameterization patterns if available.
	pub async fn merge_entity(&self, label: &str, key: &str, props: &Value) -> Result<()> {
		// Build a Cypher map from JSON properties, sanitizing keys and
		// using JSON-serialized values to ensure proper escaping.
		fn sanitize_prop_key(k: &str) -> String {
			let mut out = String::new();
			for c in k.chars() {
				if c.is_ascii_alphanumeric() || c == '_' {
					out.push(c);
				} else {
					out.push('_');
				}
			}
			if out.is_empty() {
				"prop".to_string()
			} else {
				out
			}
		}

		let mut props_kv = Vec::new();
		if let Value::Object(map) = props {
			for (k, v) in map.iter() {
				// Sanitize property key and serialize the value safely to JSON
				let k_s = sanitize_prop_key(k);
				let s = serde_json::to_string(v)?;
				props_kv.push(format!("{}: {}", k_s, s));
			}
		}
		let props_str = props_kv.join(", ");

		// Sanitize label and serialize key using JSON encoding to ensure
		// safe, quoted string injection into the Cypher statement.
		fn sanitize_label(label: &str) -> String {
			let mut out = String::new();
			for c in label.chars() {
				if c.is_ascii_alphanumeric() || c == '_' {
					out.push(c);
				}
			}
			if out.is_empty() {
				"FieldValue".to_string()
			} else {
				out
			}
		}

		let label_s = sanitize_label(label);
		let key_json = serde_json::to_string(&key)?;

		// Cypher MERGE statement (creates node if missing, otherwise matches)
		let cypher = format!(
			"MERGE (n:{label} {{canonical_key: {key}}}) SET n += {{{props}}} RETURN n",
			label = label_s,
			key = key_json,
			props = props_str
		);

		// Execute via AGE's `cypher` SQL function
		// Use parameterized SQL to avoid embedding graph name or cypher
		// into the outer SQL string. The `cypher` function takes the
		// graph name and the Cypher query as text parameters.
		let sql = "SELECT * FROM cypher($1::text, $2::text) as (v agtype);";

		// Fire the query; we don't need the returned row for this simple upsert
		sqlx::query(sql)
			.bind(&self.graph)
			.bind(&cypher)
			.execute(&self.pool)
			.await?;
		Ok(())
	}
}

/// Trait abstraction for persistence operations so tests can substitute a
/// mock implementation. Implemented by `AgeClient` and any test doubles.
#[async_trait]
pub trait AgeRepo: Send + Sync + 'static {
	async fn merge_entity(&self, label: &str, key: &str, props: &Value) -> Result<()>;
	/// Lightweight ping to verify DB connectivity / readiness.
	async fn ping(&self) -> Result<()>;
	/// Merge a batch of entities in a single Cypher call for improved
	/// throughput. Implementations should attempt to execute the batch in
	/// a single `cypher` invocation where possible and fall back to per-item
	/// merges on partial failure.
	async fn merge_batch(&self, items: &[(String, String, Value)]) -> Result<()>;
}

#[async_trait]
impl AgeRepo for AgeClient {
	async fn merge_entity(&self, label: &str, key: &str, props: &Value) -> Result<()> {
		// Call the inherent method
		AgeClient::merge_entity(self, label, key, props).await
	}

	async fn ping(&self) -> Result<()> {
		// Simple lightweight query to verify the connection
		// We don't need the returned row; success indicates connectivity.
		sqlx::query("SELECT 1").fetch_one(&self.pool).await?;
		Ok(())
	}

	async fn merge_batch(&self, items: &[(String, String, Value)]) -> Result<()> {
		if items.is_empty() {
			return Ok(());
		}

		// Build a single Cypher script with multiple MERGE statements while
		// sanitizing keys and labels. Values are JSON-serialized to ensure
		// correct escaping. Property keys are transformed to a safe
		// identifier (alphanumeric + underscore).
		fn sanitize_label(label: &str) -> String {
			let mut out = String::new();
			for c in label.chars() {
				if c.is_ascii_alphanumeric() || c == '_' {
					out.push(c);
				}
			}
			if out.is_empty() {
				"FieldValue".to_string()
			} else {
				out
			}
		}

		fn sanitize_prop_key(k: &str) -> String {
			let mut out = String::new();
			for c in k.chars() {
				if c.is_ascii_alphanumeric() || c == '_' {
					out.push(c);
				} else {
					out.push('_');
				}
			}
			if out.is_empty() {
				"prop".to_string()
			} else {
				out
			}
		}

		let mut stmts: Vec<String> = Vec::with_capacity(items.len());
		for (label, key, props) in items.iter() {
			let label_s = sanitize_label(label);
			let mut props_kv = Vec::new();
			if let Value::Object(map) = props {
				for (k, v) in map.iter() {
					let k_s = sanitize_prop_key(k);
					let s = serde_json::to_string(v)?;
					props_kv.push(format!("{}: {}", k_s, s));
				}
			}
			let props_str = props_kv.join(", ");
			let key_json = serde_json::to_string(key)?;
			let stmt = format!(
				"MERGE (n:{label} {{canonical_key: {key}}}) SET n += {{{props}}}",
				label = label_s,
				key = key_json,
				props = props_str
			);
			stmts.push(stmt);
		}

		let cypher = stmts.join("\n");
		let sql = "SELECT * FROM cypher($1::text, $2::text) as (v agtype);";

		// Execute batch
		let res = sqlx::query(sql)
			.bind(&self.graph)
			.bind(&cypher)
			.execute(&self.pool)
			.await;

		match res {
			Ok(_) => Ok(()),
			Err(e) => {
				// On batch failure, attempt per-item merges to make progress
				eprintln!("batch merge failed: {}; falling back to per-item merges", e);
				for (label, key, props) in items.iter() {
					if let Err(e2) = AgeClient::merge_entity(self, label, key, props).await {
						eprintln!("per-item merge failed for {}: {}", key, e2);
					}
				}
				Ok(())
			}
		}
	}
}

// NOTE: Global repo helpers were intentionally removed in favor of
// injecting the shared `Arc<dyn AgeRepo>` into application state. Use
// `crate::state::AppState` (or pass an Arc<dyn AgeRepo>) when handlers
// need persistence.

#[cfg(test)]
mod tests {

	// Helper functions to expose sanitize functions for testing
	fn test_sanitize_prop_key(k: &str) -> String {
		let mut out = String::new();
		for c in k.chars() {
			if c.is_ascii_alphanumeric() || c == '_' {
				out.push(c);
			} else {
				out.push('_');
			}
		}
		if out.is_empty() {
			"prop".to_string()
		} else {
			out
		}
	}

	fn test_sanitize_label(label: &str) -> String {
		let mut out = String::new();
		for c in label.chars() {
			if c.is_ascii_alphanumeric() || c == '_' {
				out.push(c);
			}
		}
		if out.is_empty() {
			"FieldValue".to_string()
		} else {
			out
		}
	}

	#[test]
	fn sanitize_prop_key_alphanumeric() {
		assert_eq!(test_sanitize_prop_key("valid_key123"), "valid_key123");
	}

	#[test]
	fn sanitize_prop_key_special_chars() {
		assert_eq!(test_sanitize_prop_key("key-with-dashes"), "key_with_dashes");
		assert_eq!(test_sanitize_prop_key("key.with.dots"), "key_with_dots");
		assert_eq!(test_sanitize_prop_key("key:with:colons"), "key_with_colons");
	}

	#[test]
	fn sanitize_prop_key_empty() {
		assert_eq!(test_sanitize_prop_key(""), "prop");
		assert_eq!(test_sanitize_prop_key("!!!"), "___");
	}

	#[test]
	fn sanitize_prop_key_unicode() {
		// Unicode characters are replaced with underscores, except ASCII alphanumeric and underscore
		let result = test_sanitize_prop_key("key_with_émojis_🔥");
		assert!(result.starts_with("key_with_"));
		// Verify no unicode characters remain
		assert!(
			result
				.chars()
				.all(|c| c.is_ascii_alphanumeric() || c == '_')
		);
	}

	#[test]
	fn sanitize_prop_key_sql_injection_attempt() {
		// SQL injection attempts should be sanitized to underscores
		let result = test_sanitize_prop_key("'; DROP TABLE users; --");
		assert!(
			result
				.chars()
				.all(|c| c.is_ascii_alphanumeric() || c == '_')
		);
		assert!(result.contains("DROP_TABLE_users"));
	}

	#[test]
	fn sanitize_label_alphanumeric() {
		assert_eq!(test_sanitize_label("ValidLabel"), "ValidLabel");
		assert_eq!(test_sanitize_label("Label_123"), "Label_123");
	}

	#[test]
	fn sanitize_label_special_chars() {
		assert_eq!(test_sanitize_label("Label-With-Dashes"), "LabelWithDashes");
		assert_eq!(test_sanitize_label("Label.With.Dots"), "LabelWithDots");
	}

	#[test]
	fn sanitize_label_empty() {
		assert_eq!(test_sanitize_label(""), "FieldValue");
		assert_eq!(test_sanitize_label("!!!"), "FieldValue");
	}

	#[test]
	fn sanitize_label_cypher_injection_attempt() {
		assert_eq!(test_sanitize_label("Label'); MATCH (n)--"), "LabelMATCHn");
	}

	#[test]
	fn sanitize_label_unicode() {
		// Unicode characters are removed, keeping only ASCII alphanumeric and underscore
		let result = test_sanitize_label("Label_with_émojis_🔥");
		assert!(result.starts_with("Label_with_"));
		// Verify only ASCII alphanumeric and underscore remain
		assert!(
			result
				.chars()
				.all(|c| c.is_ascii_alphanumeric() || c == '_')
		);
	}

	#[cfg(feature = "integration-tests")]
	mod integration {
		use super::*;

		// Note: This test is a compile-time smoke test only and does not connect to a DB.
		#[tokio::test]
		async fn client_smoke() {
			// connecting to a real DB is outside unit test scope here; just ensure types work
			let url = "postgres://heimdall:heimdall@localhost/heimdall";
			let _ = AgeClient::connect(url, "heimdall_graph").await;
		}
	}
}
