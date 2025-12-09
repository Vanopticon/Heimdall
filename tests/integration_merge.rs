use serde_json::json;
use vanopticon_heimdall::sync::{
	EntityVersion, MergeConfig, MergeResolver, MergeRule, MergeStrategy, VersionVector,
};

#[cfg(feature = "integration-tests")]
mod integration_tests {
	use super::*;

	/// Test deterministic conflict resolution with concurrent updates.
	#[test]
	fn test_concurrent_updates_lww() {
		let config = MergeConfig::new().with_default_strategy(MergeStrategy::LastWriterWins);
		let resolver = MergeResolver::new(config);

		// Two nodes update the same entity concurrently
		let node1_update = EntityVersion::new(
			"FieldValue",
			"email:user@example.com",
			json!({"value": "user@example.com", "first_seen": 1000, "category": "email"}),
			VersionVector::new("node1", 1500),
		);

		let node2_update = EntityVersion::new(
			"FieldValue",
			"email:user@example.com",
			json!({"value": "user@example.com", "first_seen": 1200, "category": "email_verified"}),
			VersionVector::new("node2", 1400),
		);

		// Resolve: node1 should win (timestamp 1500 > 1400)
		let merged = resolver.merge(&node1_update, &node2_update).unwrap();
		assert_eq!(merged.version.timestamp, 1500);
		assert_eq!(merged.version.origin, "node1");
		assert_eq!(merged.props["category"], "email");
	}

	/// Test sightings merge strategy with co-occurrence counts.
	#[test]
	fn test_sightings_merge_with_counts() {
		let rule = MergeRule::new("CoOccurrence", MergeStrategy::MergeSightings)
			.with_merge_fields(vec![
				"count".to_string(),
				"last_seen".to_string(),
				"first_seen".to_string(),
			])
			.with_lww_fields(vec!["context".to_string()]);

		let config = MergeConfig::new().add_rule(rule);
		let resolver = MergeResolver::new(config);

		// Node1 observes a co-occurrence 10 times
		let node1 = EntityVersion::new(
			"CoOccurrence",
			"email:user@example.com|password:hash123",
			json!({
				"count": 10,
				"first_seen": 1000,
				"last_seen": 1500,
				"context": "breach_A"
			}),
			VersionVector::new("node1", 1500),
		);

		// Node2 observes the same co-occurrence 5 times
		let node2 = EntityVersion::new(
			"CoOccurrence",
			"email:user@example.com|password:hash123",
			json!({
				"count": 5,
				"first_seen": 1100,
				"last_seen": 1700,
				"context": "breach_B"
			}),
			VersionVector::new("node2", 1700),
		);

		// Merge should sum counts, use newer last_seen, and older first_seen
		let merged = resolver.merge(&node1, &node2).unwrap();
		assert_eq!(merged.props["count"], 15); // 10 + 5
		assert_eq!(merged.props["last_seen"], 1700); // newer timestamp wins
		assert_eq!(merged.props["first_seen"], 1000); // older timestamp wins (first observation)
		assert_eq!(merged.props["context"], "breach_B"); // LWW field, node2 is newer
	}

	/// Test tombstone semantics for deletions.
	#[test]
	fn test_tombstone_deletion() {
		let config = MergeConfig::new().with_default_strategy(MergeStrategy::Tombstone);
		let resolver = MergeResolver::new(config);

		// Node1 has an active entity
		let node1 = EntityVersion::new(
			"FieldValue",
			"temp_data",
			json!({"value": "temporary", "active": true}),
			VersionVector::new("node1", 1000),
		);

		// Node2 deletes it (tombstone)
		let node2 = EntityVersion::new(
			"FieldValue",
			"temp_data",
			json!({"deleted_at": 1500}),
			VersionVector::new("node2", 1500),
		)
		.with_tombstone(true);

		// Tombstone should win
		let merged = resolver.merge(&node1, &node2).unwrap();
		assert!(merged.tombstone);
		assert_eq!(merged.version.timestamp, 1500);

		// Reverse merge should also preserve tombstone
		let merged_rev = resolver.merge(&node2, &node1).unwrap();
		assert!(merged_rev.tombstone);
		assert_eq!(merged_rev.version.timestamp, 1500);
	}

	/// Test partition scenario: nodes reconcile after network split.
	#[test]
	fn test_partition_reconciliation() {
		let sighting_rule = MergeRule::new("Sighting", MergeStrategy::MergeSightings)
			.with_merge_fields(vec!["count".to_string(), "last_seen".to_string()]);

		let config = MergeConfig::new()
			.add_rule(sighting_rule)
			.with_default_strategy(MergeStrategy::LastWriterWins);

		let resolver = MergeResolver::new(config);

		// During partition, node1 collects 100 sightings
		let node1_state = EntityVersion::new(
			"Sighting",
			"ip:192.168.1.1",
			json!({"count": 100, "last_seen": 2000, "location": "datacenter_A"}),
			VersionVector::new("node1", 2000),
		);

		// During partition, node2 collects 50 sightings
		let node2_state = EntityVersion::new(
			"Sighting",
			"ip:192.168.1.1",
			json!({"count": 50, "last_seen": 2100, "location": "datacenter_B"}),
			VersionVector::new("node2", 2100),
		);

		// After partition heals, merge states
		let merged = resolver.merge(&node1_state, &node2_state).unwrap();

		// Counts should be summed
		assert_eq!(merged.props["count"], 150);
		// Last_seen should be the newer timestamp
		assert_eq!(merged.props["last_seen"], 2100);
		// Location is not in merge_fields or lww_fields, so it's preserved from
		// the local version (node1) per merge_sightings implementation
		assert_eq!(merged.props["location"], "datacenter_A");
	}

	/// Test version vector tie-breaking with deterministic ordering.
	#[test]
	fn test_version_vector_tiebreak() {
		let config = MergeConfig::new().with_default_strategy(MergeStrategy::LastWriterWins);
		let resolver = MergeResolver::new(config);

		// Two updates with the same timestamp but different origins
		let node_a = EntityVersion::new(
			"FieldValue",
			"test_key",
			json!({"value": "from_A"}),
			VersionVector::new("node_a", 1000),
		);

		let node_z = EntityVersion::new(
			"FieldValue",
			"test_key",
			json!({"value": "from_Z"}),
			VersionVector::new("node_z", 1000),
		);

		// node_z should win due to lexicographic ordering ("node_z" > "node_a")
		let merged = resolver.merge(&node_a, &node_z).unwrap();
		assert_eq!(merged.version.origin, "node_z");
		assert_eq!(merged.props["value"], "from_Z");

		// Reverse should also be deterministic
		let merged_rev = resolver.merge(&node_z, &node_a).unwrap();
		assert_eq!(merged_rev.version.origin, "node_z");
		assert_eq!(merged_rev.props["value"], "from_Z");
	}

	/// Test multiple sequential merges (3-way).
	#[test]
	fn test_three_way_merge() {
		let rule = MergeRule::new("Sighting", MergeStrategy::MergeSightings)
			.with_merge_fields(vec!["count".to_string(), "last_seen".to_string()]);

		let config = MergeConfig::new().add_rule(rule);
		let resolver = MergeResolver::new(config);

		let node1 = EntityVersion::new(
			"Sighting",
			"domain:example.com",
			json!({"count": 10, "last_seen": 1000}),
			VersionVector::new("node1", 1000),
		);

		let node2 = EntityVersion::new(
			"Sighting",
			"domain:example.com",
			json!({"count": 20, "last_seen": 1100}),
			VersionVector::new("node2", 1100),
		);

		let node3 = EntityVersion::new(
			"Sighting",
			"domain:example.com",
			json!({"count": 30, "last_seen": 1200}),
			VersionVector::new("node3", 1200),
		);

		// Merge 1 and 2
		let merged_12 = resolver.merge(&node1, &node2).unwrap();
		assert_eq!(merged_12.props["count"], 30); // 10 + 20
		assert_eq!(merged_12.props["last_seen"], 1100);

		// Merge result with 3
		let merged_123 = resolver.merge(&merged_12, &node3).unwrap();
		assert_eq!(merged_123.props["count"], 60); // 30 + 30
		assert_eq!(merged_123.props["last_seen"], 1200);
	}

	/// Test tombstone vs tombstone (both deleted).
	#[test]
	fn test_tombstone_vs_tombstone() {
		let config = MergeConfig::new().with_default_strategy(MergeStrategy::Tombstone);
		let resolver = MergeResolver::new(config);

		let node1_tombstone = EntityVersion::new(
			"FieldValue",
			"deleted_key",
			json!({"deleted_at": 1000}),
			VersionVector::new("node1", 1000),
		)
		.with_tombstone(true);

		let node2_tombstone = EntityVersion::new(
			"FieldValue",
			"deleted_key",
			json!({"deleted_at": 1500}),
			VersionVector::new("node2", 1500),
		)
		.with_tombstone(true);

		// Both are tombstones: use newer version
		let merged = resolver.merge(&node1_tombstone, &node2_tombstone).unwrap();
		assert!(merged.tombstone);
		assert_eq!(merged.version.timestamp, 1500);
		assert_eq!(merged.props["deleted_at"], 1500);
	}
}
