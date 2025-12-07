use anyhow::Result;
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
		// Build a Cypher map from JSON properties
		let mut props_kv = Vec::new();
		if let Value::Object(map) = props {
			for (k, v) in map.iter() {
				// Serialize each value to JSON and then embed into the Cypher literal
				let s = serde_json::to_string(v)?;
				// Example: propKey: "value" or propNum: 123
				props_kv.push(format!("{}: {}", k, s));
			}
		}
		let props_str = props_kv.join(", ");

		// Cypher MERGE statement (creates node if missing, otherwise matches)
		let cypher = format!(
			"MERGE (n:{label} {{canonical_key: \"{key}\"}}) SET n += {{{props}}} RETURN n",
			label = label,
			key = key.replace('\"', "\\\""),
			props = props_str
		);

		// Execute via AGE's `cypher` SQL function
		let sql = format!(
			"SELECT * FROM cypher($${}$$, $$ {} $$) as (v agtype);",
			self.graph, cypher
		);

		// Fire the query; we don't need the returned row for this simple upsert
		sqlx::query(&sql).execute(&self.pool).await?;
		Ok(())
	}
}

#[cfg(feature = "integration-tests")]
mod tests {
	use super::*;
	use serde_json::json;

	// Note: This test is a compile-time smoke test only and does not connect to a DB.
	#[tokio::test]
	async fn client_smoke() {
		// connecting to a real DB is outside unit test scope here; just ensure types work
		let url = "postgres://heimdall:heimdall@localhost/heimdall";
		let _ = AgeClient::connect(url, "heimdall_graph").await;
	}
}
