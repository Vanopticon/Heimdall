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

	/// Persist a single row with its cells (sightings) into the graph.
	///
	/// This creates Row + Sighting nodes and links them to canonical FieldValue nodes.
	/// The row structure is preserved for provenance while deduplicating values.
	///
	/// # Arguments
	/// * `dump_id` - Unique identifier for the parent Dump
	/// * `row_index` - Zero-based row number in the dump
	/// * `row_hash` - Optional canonical hash of the row for duplicate detection
	/// * `cells` - Vector of (column_name, raw_value, canonical_key, canonical_value) tuples
	/// * `timestamp` - ISO8601 timestamp string for the sighting
	pub async fn persist_row(
		&self,
		dump_id: &str,
		row_index: i64,
		row_hash: Option<&str>,
		cells: &[(String, String, String, String)],
		timestamp: &str,
	) -> Result<()> {
		// Build Cypher statements for row and sighting creation
		let dump_id_json = serde_json::to_string(dump_id)?;
		let timestamp_json = serde_json::to_string(timestamp)?;

		// Start building the Cypher script
		let mut cypher = format!(
			"MERGE (d:Dump {{id: {}}}) ON CREATE SET d.received_at = {}",
			dump_id_json, timestamp_json
		);

		// Create row node with optional row_hash
		if let Some(hash) = row_hash {
			let hash_json = serde_json::to_string(hash)?;
			cypher.push_str(&format!(
				"\nCREATE (r:Row {{dump_id: {}, index: {}, row_hash: {}}}))",
				dump_id_json, row_index, hash_json
			));
		} else {
			cypher.push_str(&format!(
				"\nCREATE (r:Row {{dump_id: {}, index: {}}})",
				dump_id_json, row_index
			));
		}
		cypher.push_str("\nCREATE (d)-[:HAS_ROW]->(r)");

		// Process each cell
		for (i, (column, raw, canonical_key, canonical_value)) in cells.iter().enumerate() {
			let column_json = serde_json::to_string(column)?;
			let raw_json = serde_json::to_string(raw)?;
			let canonical_key_json = serde_json::to_string(canonical_key)?;
			let canonical_value_json = serde_json::to_string(canonical_value)?;

			// Use unique variable names for each cell
			let fv_var = format!("fv{}", i);
			let f_var = format!("f{}", i);
			let s_var = format!("s{}", i);

			cypher.push_str(&format!(
				"\nMERGE ({}:FieldValue {{canonical_key: {}}}) ON CREATE SET {}.value = {}, {}.created_at = {}",
				fv_var, canonical_key_json, fv_var, canonical_value_json, fv_var, timestamp_json
			));
			cypher.push_str(&format!(
				"\nMERGE ({}:Field {{name: {}}})",
				f_var, column_json
			));
			cypher.push_str(&format!("\nMERGE ({})-[:VALUE_OF]->({})", fv_var, f_var));
			cypher.push_str(&format!(
				"\nCREATE ({}:Sighting {{column: {}, raw: {}, timestamp: {}}})",
				s_var, column_json, raw_json, timestamp_json
			));
			cypher.push_str(&format!("\nCREATE (r)-[:HAS_SIGHTING]->({})", s_var));
			cypher.push_str(&format!(
				"\nCREATE ({})-[:OBSERVED_VALUE]->({})",
				s_var, fv_var
			));
		}

		cypher.push_str("\nRETURN r");

		// Execute the Cypher script
		let sql = "SELECT * FROM cypher($1::text, $2::text) as (v agtype);";
		sqlx::query(sql)
			.bind(&self.graph)
			.bind(&cypher)
			.execute(&self.pool)
			.await?;

		Ok(())
	}

	/// Increment co-occurrence count between two canonical values.
	///
	/// Creates or updates a CO_OCCURS relationship between two FieldValue nodes.
	/// Uses deterministic ordering (a_key < b_key) to avoid duplicate edges.
	pub async fn increment_co_occurrence(
		&self,
		a_key: &str,
		b_key: &str,
		timestamp: &str,
	) -> Result<()> {
		// Ensure deterministic ordering
		let (first, second) = if a_key < b_key {
			(a_key, b_key)
		} else {
			(b_key, a_key)
		};

		let first_json = serde_json::to_string(first)?;
		let second_json = serde_json::to_string(second)?;
		let timestamp_json = serde_json::to_string(timestamp)?;

		let cypher = format!(
			"MERGE (a:FieldValue {{canonical_key: {}}}) \
			 MERGE (b:FieldValue {{canonical_key: {}}}) \
			 MERGE (a)-[co:CO_OCCURS]-(b) \
			 SET co.count = coalesce(co.count, 0) + 1, co.last_seen = {} \
			 RETURN co",
			first_json, second_json, timestamp_json
		);

		let sql = "SELECT * FROM cypher($1::text, $2::text) as (v agtype);";
		sqlx::query(sql)
			.bind(&self.graph)
			.bind(&cypher)
			.execute(&self.pool)
			.await?;

		Ok(())
	}

	/// Persist a credential relationship (e.g., email -> password).
	///
	/// Creates or updates a CREDENTIAL edge with count and last_seen tracking.
	pub async fn persist_credential(
		&self,
		from_key: &str,
		to_key: &str,
		timestamp: &str,
	) -> Result<()> {
		let from_json = serde_json::to_string(from_key)?;
		let to_json = serde_json::to_string(to_key)?;
		let timestamp_json = serde_json::to_string(timestamp)?;

		let cypher = format!(
			"MERGE (a:FieldValue {{canonical_key: {}}}) \
			 MERGE (b:FieldValue {{canonical_key: {}}}) \
			 MERGE (a)-[c:CREDENTIAL]->(b) \
			 SET c.count = coalesce(c.count, 0) + 1, c.last_seen = {} \
			 RETURN c",
			from_json, to_json, timestamp_json
		);

		let sql = "SELECT * FROM cypher($1::text, $2::text) as (v agtype);";
		sqlx::query(sql)
			.bind(&self.graph)
			.bind(&cypher)
			.execute(&self.pool)
			.await?;

		Ok(())
	}

	/// Apply SQL migrations from a file to set up the graph schema.
	///
	/// This executes raw SQL statements (including Cypher via AGE functions)
	/// to initialize the graph structure, indices, and labels.
	///
	/// **Important Limitations:**
	/// - Executes the entire SQL content as a single statement batch
	/// - Does not parse individual statements or handle complex transaction boundaries
	/// - Suitable for initial schema setup and idempotent migration scripts
	/// - For production use with multiple migrations, consider using a dedicated
	///   migration tool like `sqlx-cli` or `refinery` that supports proper
	///   versioning, rollback, and statement-by-statement execution
	///
	/// **Safety:**
	/// - Only execute trusted SQL content (typically embedded via `include_str!`)
	/// - Never pass user-provided content to this method
	/// - Ensure migrations are idempotent (use CREATE IF NOT EXISTS, MERGE, etc.)
	pub async fn apply_migration(&self, sql_content: &str) -> Result<()> {
		// Execute the SQL content directly as a batch.
		// This works for simple DO blocks and CREATE IF NOT EXISTS statements
		// but does not handle complex multi-statement scripts with dependencies.
		sqlx::query(sql_content).execute(&self.pool).await?;
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
	/// Persist a single row with its cells into the graph.
	async fn persist_row(
		&self,
		dump_id: &str,
		row_index: i64,
		row_hash: Option<&str>,
		cells: &[(String, String, String, String)],
		timestamp: &str,
	) -> Result<()>;
	/// Increment co-occurrence count between two canonical values.
	async fn increment_co_occurrence(
		&self,
		a_key: &str,
		b_key: &str,
		timestamp: &str,
	) -> Result<()>;
	/// Persist a credential relationship (e.g., email -> password).
	async fn persist_credential(&self, from_key: &str, to_key: &str, timestamp: &str)
	-> Result<()>;
	/// Apply SQL migrations to set up the graph schema.
	async fn apply_migration(&self, sql_content: &str) -> Result<()>;
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

	async fn persist_row(
		&self,
		dump_id: &str,
		row_index: i64,
		row_hash: Option<&str>,
		cells: &[(String, String, String, String)],
		timestamp: &str,
	) -> Result<()> {
		AgeClient::persist_row(self, dump_id, row_index, row_hash, cells, timestamp).await
	}

	async fn increment_co_occurrence(
		&self,
		a_key: &str,
		b_key: &str,
		timestamp: &str,
	) -> Result<()> {
		AgeClient::increment_co_occurrence(self, a_key, b_key, timestamp).await
	}

	async fn persist_credential(
		&self,
		from_key: &str,
		to_key: &str,
		timestamp: &str,
	) -> Result<()> {
		AgeClient::persist_credential(self, from_key, to_key, timestamp).await
	}

	async fn apply_migration(&self, sql_content: &str) -> Result<()> {
		AgeClient::apply_migration(self, sql_content).await
	}
}

// NOTE: Global repo helpers were intentionally removed in favor of
// injecting the shared `Arc<dyn AgeRepo>` into application state. Use
// `crate::state::AppState` (or pass an Arc<dyn AgeRepo>) when handlers
// need persistence.

#[cfg(feature = "integration-tests")]
mod tests {
	use super::AgeClient;

	// Note: This test is a compile-time smoke test only and does not connect to a DB.
	#[tokio::test]
	async fn client_smoke() {
		// connecting to a real DB is outside unit test scope here; just ensure types work
		let url = "postgres://heimdall:heimdall@localhost/heimdall";
		let _ = AgeClient::connect(url, "heimdall_graph").await;
	}
}
