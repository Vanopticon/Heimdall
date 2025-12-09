mod common;

use serde_json::json;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

/// Integration test demonstrating sync partition logic.
/// This test validates that data partitioned by source can be tracked
/// and synchronized between multiple Heimdall instances (mocked as separate graphs).
#[tokio::test]
async fn e2e_sync_partition_basic() {
	// Gate the test
	if !common::check_docker_enabled() {
		return;
	}

	// Start the dev DB
	vanopticon_heimdall::devops::start_dev_db()
		.await
		.expect("start db");

	// Wait for Postgres to accept connections
	let pool = common::wait_for_postgres("postgres://heimdall:heimdall@127.0.0.1:5432/heimdall", 30)
		.await
		.expect("connect to postgres");

	// Create two separate graphs to simulate two Heimdall instances
	// In production, these would be separate database instances or separate AGE graphs
	let graph_a = "heimdall_graph"; // default graph (already created)
	let graph_b = "heimdall_graph_b"; // second instance

	// Create second graph
	let create_graph_sql = "SELECT ag_catalog.create_graph($1::text);";
	sqlx::query(create_graph_sql)
		.bind(graph_b)
		.execute(&pool)
		.await
		.expect("create graph B");

	// Build clients for both graphs
	let client_a = vanopticon_heimdall::age_client::AgeClient::new(pool.clone(), graph_a);
	let client_b = vanopticon_heimdall::age_client::AgeClient::new(pool.clone(), graph_b);

	let repo_a: Arc<dyn vanopticon_heimdall::age_client::AgeRepo> = Arc::new(client_a);
	let repo_b: Arc<dyn vanopticon_heimdall::age_client::AgeRepo> = Arc::new(client_b);

	// Start batchers for both instances
	let sender_a = vanopticon_heimdall::persist::start_batcher(repo_a.clone(), 1024, 1, 100);
	let sender_b = vanopticon_heimdall::persist::start_batcher(repo_b.clone(), 1024, 1, 100);

	// Scenario: Instance A receives data from source "sensor_1"
	// Instance B receives data from source "sensor_2"
	// Both instances should be able to sync and merge data

	// Instance A: Ingest data from sensor_1
	let job_a1 = vanopticon_heimdall::persist::PersistJob {
		label: "Sighting".to_string(),
		key: "sighting_sensor1_001".to_string(),
		props: json!({
			"canonical_key": "sighting_sensor1_001",
			"source": "sensor_1",
			"instance": "heimdall_a",
			"ip_address": "10.0.1.100",
			"timestamp": "2024-01-01T10:00:00Z",
			"partition_key": "sensor_1",
		}),
	};
	vanopticon_heimdall::persist::submit_job(&sender_a, job_a1)
		.expect("submit job to instance A");

	let job_a2 = vanopticon_heimdall::persist::PersistJob {
		label: "Sighting".to_string(),
		key: "sighting_sensor1_002".to_string(),
		props: json!({
			"canonical_key": "sighting_sensor1_002",
			"source": "sensor_1",
			"instance": "heimdall_a",
			"ip_address": "10.0.1.101",
			"timestamp": "2024-01-01T10:01:00Z",
			"partition_key": "sensor_1",
		}),
	};
	vanopticon_heimdall::persist::submit_job(&sender_a, job_a2)
		.expect("submit second job to instance A");

	// Instance B: Ingest data from sensor_2
	let job_b1 = vanopticon_heimdall::persist::PersistJob {
		label: "Sighting".to_string(),
		key: "sighting_sensor2_001".to_string(),
		props: json!({
			"canonical_key": "sighting_sensor2_001",
			"source": "sensor_2",
			"instance": "heimdall_b",
			"ip_address": "10.0.2.100",
			"timestamp": "2024-01-01T10:00:00Z",
			"partition_key": "sensor_2",
		}),
	};
	vanopticon_heimdall::persist::submit_job(&sender_b, job_b1)
		.expect("submit job to instance B");

	let job_b2 = vanopticon_heimdall::persist::PersistJob {
		label: "Sighting".to_string(),
		key: "sighting_sensor2_002".to_string(),
		props: json!({
			"canonical_key": "sighting_sensor2_002",
			"source": "sensor_2",
			"instance": "heimdall_b",
			"ip_address": "10.0.2.101",
			"timestamp": "2024-01-01T10:01:00Z",
			"partition_key": "sensor_2",
		}),
	};
	vanopticon_heimdall::persist::submit_job(&sender_b, job_b2)
		.expect("submit second job to instance B");

	// Allow time for batchers to flush
	sleep(Duration::from_millis(500)).await;

	// Verify: Each instance should have its own partition data
	let sql = "SELECT * FROM cypher($1::text, $2::text) as (v agtype);";

	// Verify instance A has sensor_1 data
	let cypher_a = "MATCH (n:Sighting {partition_key: \"sensor_1\"}) RETURN n";
	let rows_a = sqlx::query(sql)
		.bind(graph_a)
		.bind(cypher_a)
		.fetch_all(&pool)
		.await
		.expect("query instance A");
	assert_eq!(rows_a.len(), 2, "instance A should have 2 sensor_1 sightings");

	// Verify instance B has sensor_2 data
	let cypher_b = "MATCH (n:Sighting {partition_key: \"sensor_2\"}) RETURN n";
	let rows_b = sqlx::query(sql)
		.bind(graph_b)
		.bind(cypher_b)
		.fetch_all(&pool)
		.await
		.expect("query instance B");
	assert_eq!(rows_b.len(), 2, "instance B should have 2 sensor_2 sightings");

	// Simulate sync: Copy instance B's data to instance A
	// In a real sync scenario, this would be done via API or message queue
	let sync_job_b1 = vanopticon_heimdall::persist::PersistJob {
		label: "Sighting".to_string(),
		key: "sighting_sensor2_001".to_string(),
		props: json!({
			"canonical_key": "sighting_sensor2_001",
			"source": "sensor_2",
			"instance": "heimdall_b",
			"ip_address": "10.0.2.100",
			"timestamp": "2024-01-01T10:00:00Z",
			"partition_key": "sensor_2",
			"synced_from": "heimdall_b",
			"synced_at": "2024-01-01T10:05:00Z",
		}),
	};
	vanopticon_heimdall::persist::submit_job(&sender_a, sync_job_b1)
		.expect("sync job from B to A");

	let sync_job_b2 = vanopticon_heimdall::persist::PersistJob {
		label: "Sighting".to_string(),
		key: "sighting_sensor2_002".to_string(),
		props: json!({
			"canonical_key": "sighting_sensor2_002",
			"source": "sensor_2",
			"instance": "heimdall_b",
			"ip_address": "10.0.2.101",
			"timestamp": "2024-01-01T10:01:00Z",
			"partition_key": "sensor_2",
			"synced_from": "heimdall_b",
			"synced_at": "2024-01-01T10:05:00Z",
		}),
	};
	vanopticon_heimdall::persist::submit_job(&sender_a, sync_job_b2)
		.expect("sync second job from B to A");

	sleep(Duration::from_millis(500)).await;

	// Verify: Instance A now has both partitions
	let cypher_all = "MATCH (n:Sighting) RETURN n";
	let rows_all = sqlx::query(sql)
		.bind(graph_a)
		.bind(cypher_all)
		.fetch_all(&pool)
		.await
		.expect("query all sightings in instance A");
	assert_eq!(
		rows_all.len(),
		4,
		"instance A should have 4 total sightings after sync"
	);

	// Verify: Instance A has both partition keys
	let cypher_a_sensor1 = "MATCH (n:Sighting {partition_key: \"sensor_1\"}) RETURN n";
	let rows_a_sensor1 = sqlx::query(sql)
		.bind(graph_a)
		.bind(cypher_a_sensor1)
		.fetch_all(&pool)
		.await
		.expect("query sensor_1 in instance A");
	assert_eq!(rows_a_sensor1.len(), 2, "instance A should have 2 sensor_1 sightings");

	let cypher_a_sensor2 = "MATCH (n:Sighting {partition_key: \"sensor_2\"}) RETURN n";
	let rows_a_sensor2 = sqlx::query(sql)
		.bind(graph_a)
		.bind(cypher_a_sensor2)
		.fetch_all(&pool)
		.await
		.expect("query sensor_2 in instance A");
	assert_eq!(
		rows_a_sensor2.len(),
		2,
		"instance A should have 2 sensor_2 sightings after sync"
	);

	// Cleanup: Drop the second graph
	let drop_graph_sql = "SELECT ag_catalog.drop_graph($1::text, true);";
	sqlx::query(drop_graph_sql)
		.bind(graph_b)
		.execute(&pool)
		.await
		.expect("drop graph B");

	// Tear down DB
	vanopticon_heimdall::devops::stop_dev_db()
		.await
		.expect("stop db");
}

/// Integration test demonstrating partition deduplication during sync.
/// This validates that duplicate entities are properly handled when syncing
/// between instances.
#[tokio::test]
async fn e2e_sync_partition_deduplication() {
	// Gate the test
	if !common::check_docker_enabled() {
		return;
	}

	// Start the dev DB
	vanopticon_heimdall::devops::start_dev_db()
		.await
		.expect("start db");

	// Wait for Postgres to accept connections
	let pool = common::wait_for_postgres("postgres://heimdall:heimdall@127.0.0.1:5432/heimdall", 30)
		.await
		.expect("connect to postgres");

	// Use default graph
	let graph_name = "heimdall_graph";
	let client = vanopticon_heimdall::age_client::AgeClient::new(pool.clone(), graph_name);
	let repo: Arc<dyn vanopticon_heimdall::age_client::AgeRepo> = Arc::new(client);
	let sender = vanopticon_heimdall::persist::start_batcher(repo.clone(), 1024, 1, 100);

	// Scenario: Same entity appears in multiple partitions
	// The sync process should deduplicate based on canonical_key

	let shared_ip = "192.168.1.100";

	// Partition 1: Entity from sensor_1
	let job1 = vanopticon_heimdall::persist::PersistJob {
		label: "IPAddress".to_string(),
		key: shared_ip.to_string(),
		props: json!({
			"canonical_key": shared_ip,
			"field_type": "ip",
			"first_seen": "2024-01-01T10:00:00Z",
			"partition_key": "sensor_1",
			"seen_count": 1,
		}),
	};
	vanopticon_heimdall::persist::submit_job(&sender, job1)
		.expect("submit first partition job");

	sleep(Duration::from_millis(200)).await;

	// Partition 2: Same entity from sensor_2 (simulating sync from another instance)
	// MERGE should update the existing node rather than creating a duplicate
	let job2 = vanopticon_heimdall::persist::PersistJob {
		label: "IPAddress".to_string(),
		key: shared_ip.to_string(),
		props: json!({
			"canonical_key": shared_ip,
			"field_type": "ip",
			"last_seen": "2024-01-01T11:00:00Z",
			"partition_key": "sensor_2",
			"seen_count": 2,
		}),
	};
	vanopticon_heimdall::persist::submit_job(&sender, job2)
		.expect("submit second partition job");

	sleep(Duration::from_millis(500)).await;

	// Verify: Only one node should exist for the shared IP
	let sql = "SELECT * FROM cypher($1::text, $2::text) as (v agtype);";
	let escaped_ip = common::escape_cypher_string(shared_ip);
	let cypher = format!(
		"MATCH (n:IPAddress {{canonical_key: \"{}\"}}) RETURN n",
		escaped_ip
	);
	let rows = sqlx::query(sql)
		.bind(graph_name)
		.bind(&cypher)
		.fetch_all(&pool)
		.await
		.expect("query IP nodes");

	assert_eq!(
		rows.len(),
		1,
		"should have exactly 1 node for the shared IP (deduplicated)"
	);

	// The node should have properties from the most recent merge
	// (AGE MERGE will update properties on subsequent merges)

	// Tear down DB
	vanopticon_heimdall::devops::stop_dev_db()
		.await
		.expect("stop db");
}
