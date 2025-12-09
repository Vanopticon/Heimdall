/// Example demonstrating the merge resolver for multi-instance synchronization.
///
/// This example shows how to configure and use the merge resolver to reconcile
/// conflicting entity updates across multiple Heimdall instances.
///
/// Run with: cargo run --example merge_resolver_demo
use serde_json::json;
use vanopticon_heimdall::sync::{
	EntityVersion, MergeConfig, MergeResolver, MergeRule, MergeStrategy, VersionVector,
};

fn main() {
	println!("=== Heimdall Merge Resolver Demo ===\n");

	// Example 1: Last Writer Wins (LWW)
	println!("Example 1: Last Writer Wins Strategy");
	println!("-------------------------------------");
	demo_lww();
	println!();

	// Example 2: Merge Sightings (Aggregation)
	println!("Example 2: Merge Sightings Strategy");
	println!("------------------------------------");
	demo_merge_sightings();
	println!();

	// Example 3: Tombstone (Deletion)
	println!("Example 3: Tombstone Strategy");
	println!("-----------------------------");
	demo_tombstone();
	println!();

	// Example 4: Partition Reconciliation
	println!("Example 4: Partition Reconciliation");
	println!("------------------------------------");
	demo_partition_reconciliation();
	println!();

	println!("=== Demo Complete ===");
}

fn demo_lww() {
	let config = MergeConfig::new().with_default_strategy(MergeStrategy::LastWriterWins);
	let resolver = MergeResolver::new(config);

	// Node1 updates an email entity
	let node1 = EntityVersion::new(
		"FieldValue",
		"email:user@example.com",
		json!({
			"value": "user@example.com",
			"category": "email",
			"verified": false,
		}),
		VersionVector::new("heimdall-node1", 1700000000000_u64), // Earlier timestamp
	);

	// Node2 updates the same entity with verification
	let node2 = EntityVersion::new(
		"FieldValue",
		"email:user@example.com",
		json!({
			"value": "user@example.com",
			"category": "email",
			"verified": true,
		}),
		VersionVector::new("heimdall-node2", 1700000001000_u64), // Later timestamp
	);

	println!("Node1 version (timestamp: {}):", node1.version.timestamp);
	println!("  verified: {}", node1.props["verified"]);

	println!("Node2 version (timestamp: {}):", node2.version.timestamp);
	println!("  verified: {}", node2.props["verified"]);

	let merged = resolver.merge(&node1, &node2).unwrap();
	println!("\nMerged result (LWW):");
	println!("  origin: {}", merged.version.origin);
	println!("  timestamp: {}", merged.version.timestamp);
	println!("  verified: {}", merged.props["verified"]);
	println!("  → Node2 wins (newer timestamp)");
}

fn demo_merge_sightings() {
	let rule = MergeRule::new("Sighting", MergeStrategy::MergeSightings)
		.with_merge_fields(vec!["count".to_string(), "last_seen".to_string()]);

	let config = MergeConfig::new().add_rule(rule);
	let resolver = MergeResolver::new(config);

	// Node1 observes an IP 100 times
	let node1 = EntityVersion::new(
		"Sighting",
		"ip:192.168.1.1",
		json!({
			"count": 100,
			"last_seen": 1700000000000_u64,
		}),
		VersionVector::new("heimdall-node1", 1700000000000_u64),
	);

	// Node2 observes the same IP 50 times
	let node2 = EntityVersion::new(
		"Sighting",
		"ip:192.168.1.1",
		json!({
			"count": 50,
			"last_seen": 1700000001000_u64,
		}),
		VersionVector::new("heimdall-node2", 1700000001000_u64),
	);

	println!("Node1 observations:");
	println!("  count: {}", node1.props["count"]);
	println!("  last_seen: {}", node1.props["last_seen"]);

	println!("Node2 observations:");
	println!("  count: {}", node2.props["count"]);
	println!("  last_seen: {}", node2.props["last_seen"]);

	let merged = resolver.merge(&node1, &node2).unwrap();
	println!("\nMerged result (Sightings):");
	println!("  count: {} (100 + 50)", merged.props["count"]);
	println!(
		"  last_seen: {} (newer)",
		merged.props["last_seen"]
	);
	println!("  → Counts aggregated, timestamps updated");
}

fn demo_tombstone() {
	let config = MergeConfig::new().with_default_strategy(MergeStrategy::Tombstone);
	let resolver = MergeResolver::new(config);

	// Node1 has an active entity
	let node1 = EntityVersion::new(
		"FieldValue",
		"temp_token",
		json!({
			"value": "temp_access_token",
			"active": true,
		}),
		VersionVector::new("heimdall-node1", 1700000000000_u64),
	);

	// Node2 deletes it (tombstone)
	let node2 = EntityVersion::new(
		"FieldValue",
		"temp_token",
		json!({
			"deleted_at": 1700000001000_u64,
		}),
		VersionVector::new("heimdall-node2", 1700000001000_u64),
	)
	.with_tombstone(true);

	println!("Node1 state:");
	println!("  active: {}", node1.props["active"]);
	println!("  tombstone: {}", node1.tombstone);

	println!("Node2 state:");
	println!("  tombstone: {}", node2.tombstone);
	println!("  deleted_at: {}", node2.props["deleted_at"]);

	let merged = resolver.merge(&node1, &node2).unwrap();
	println!("\nMerged result (Tombstone):");
	println!("  tombstone: {}", merged.tombstone);
	println!("  origin: {}", merged.version.origin);
	println!("  → Tombstone wins (deletion propagates)");
}

fn demo_partition_reconciliation() {
	let sighting_rule = MergeRule::new("Sighting", MergeStrategy::MergeSightings)
		.with_merge_fields(vec!["count".to_string(), "last_seen".to_string()]);

	let config = MergeConfig::new()
		.add_rule(sighting_rule)
		.with_default_strategy(MergeStrategy::LastWriterWins);

	let resolver = MergeResolver::new(config);

	println!("Scenario: Network partition between datacenter_A and datacenter_B");
	println!();

	// During partition, datacenter_A collects data
	let datacenter_a = EntityVersion::new(
		"Sighting",
		"domain:malicious.example",
		json!({
			"count": 250,
			"last_seen": 1700000005000_u64,
			"datacenter": "A",
		}),
		VersionVector::new("heimdall-datacenter-a", 1700000005000_u64),
	);

	// During partition, datacenter_B collects data
	let datacenter_b = EntityVersion::new(
		"Sighting",
		"domain:malicious.example",
		json!({
			"count": 175,
			"last_seen": 1700000006000_u64,
			"datacenter": "B",
		}),
		VersionVector::new("heimdall-datacenter-b", 1700000006000_u64),
	);

	println!("Datacenter A (during partition):");
	println!("  count: {}", datacenter_a.props["count"]);
	println!("  last_seen: {}", datacenter_a.props["last_seen"]);

	println!("Datacenter B (during partition):");
	println!("  count: {}", datacenter_b.props["count"]);
	println!("  last_seen: {}", datacenter_b.props["last_seen"]);

	// Partition heals: synchronize
	let merged = resolver
		.merge(&datacenter_a, &datacenter_b)
		.unwrap();

	println!("\nAfter partition heals:");
	println!("  count: {} (250 + 175)", merged.props["count"]);
	println!(
		"  last_seen: {} (newer)",
		merged.props["last_seen"]
	);
	println!("  → Both datacenters' observations are preserved");
	println!("  → Deterministic outcome regardless of sync order");
}
