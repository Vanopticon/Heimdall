use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Merge strategy for resolving conflicts between entity versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
	/// Last Writer Wins: use the version with the latest timestamp.
	LastWriterWins,
	/// Merge sightings: combine observation counts and update last_seen.
	MergeSightings,
	/// Tombstone: mark entity as deleted and preserve deletion metadata.
	Tombstone,
}

impl Default for MergeStrategy {
	fn default() -> Self {
		MergeStrategy::LastWriterWins
	}
}

/// Configuration for entity-specific merge rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeRule {
	/// Entity type (e.g., "FieldValue", "Sighting", "Row").
	pub entity_type: String,
	/// Merge strategy to apply.
	pub strategy: MergeStrategy,
	/// Optional: Fields that should always be merged (for sightings strategy).
	#[serde(default)]
	pub merge_fields: Vec<String>,
	/// Optional: Fields that should use LWW even in merge strategy.
	#[serde(default)]
	pub lww_fields: Vec<String>,
}

impl MergeRule {
	/// Create a new merge rule for an entity type with a strategy.
	pub fn new(entity_type: impl Into<String>, strategy: MergeStrategy) -> Self {
		Self {
			entity_type: entity_type.into(),
			strategy,
			merge_fields: Vec::new(),
			lww_fields: Vec::new(),
		}
	}

	/// Add fields that should be merged (for sightings strategy).
	pub fn with_merge_fields(mut self, fields: Vec<String>) -> Self {
		self.merge_fields = fields;
		self
	}

	/// Add fields that should use LWW (for hybrid strategies).
	pub fn with_lww_fields(mut self, fields: Vec<String>) -> Self {
		self.lww_fields = fields;
		self
	}
}

/// Configuration for merge resolver with per-entity rules.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MergeConfig {
	/// Map of entity type to merge rule.
	#[serde(default)]
	pub rules: HashMap<String, MergeRule>,
	/// Default strategy when no rule is specified.
	#[serde(default)]
	pub default_strategy: MergeStrategy,
}

impl MergeConfig {
	/// Create a new merge configuration.
	pub fn new() -> Self {
		Self::default()
	}

	/// Add a merge rule for an entity type.
	pub fn add_rule(mut self, rule: MergeRule) -> Self {
		self.rules.insert(rule.entity_type.clone(), rule);
		self
	}

	/// Set the default merge strategy.
	pub fn with_default_strategy(mut self, strategy: MergeStrategy) -> Self {
		self.default_strategy = strategy;
		self
	}

	/// Get the merge rule for an entity type, falling back to default.
	pub fn get_rule(&self, entity_type: &str) -> MergeRule {
		self.rules
			.get(entity_type)
			.cloned()
			.unwrap_or_else(|| MergeRule::new(entity_type, self.default_strategy.clone()))
	}
}

/// Version vector representing the causal history of an entity.
/// Simplified implementation using a timestamp and origin ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionVector {
	/// Origin instance ID (e.g., hostname or UUID).
	pub origin: String,
	/// Logical timestamp (e.g., Unix timestamp in milliseconds).
	pub timestamp: u64,
	/// Optional: version counter for strict ordering.
	#[serde(default)]
	pub version: u64,
}

impl VersionVector {
	/// Create a new version vector.
	pub fn new(origin: impl Into<String>, timestamp: u64) -> Self {
		Self {
			origin: origin.into(),
			timestamp,
			version: 0,
		}
	}

	/// Compare two version vectors. Returns true if self is newer.
	pub fn is_newer_than(&self, other: &VersionVector) -> bool {
		if self.timestamp != other.timestamp {
			self.timestamp > other.timestamp
		} else {
			// Tie-break using origin ID for deterministic ordering
			self.origin > other.origin
		}
	}
}

/// Entity version with metadata for merge resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityVersion {
	/// Entity type (label).
	pub entity_type: String,
	/// Canonical key for the entity.
	pub key: String,
	/// Entity properties.
	pub props: Value,
	/// Version vector for conflict detection.
	pub version: VersionVector,
	/// Tombstone flag: true if entity is deleted.
	#[serde(default)]
	pub tombstone: bool,
}

impl EntityVersion {
	/// Create a new entity version.
	pub fn new(
		entity_type: impl Into<String>,
		key: impl Into<String>,
		props: Value,
		version: VersionVector,
	) -> Self {
		Self {
			entity_type: entity_type.into(),
			key: key.into(),
			props,
			version,
			tombstone: false,
		}
	}

	/// Mark entity as deleted (tombstone).
	pub fn with_tombstone(mut self, tombstone: bool) -> Self {
		self.tombstone = tombstone;
		self
	}
}

/// Merge resolver for reconciling conflicting entity updates.
pub struct MergeResolver {
	config: MergeConfig,
}

impl MergeResolver {
	/// Create a new merge resolver with the given configuration.
	pub fn new(config: MergeConfig) -> Self {
		Self { config }
	}

	/// Merge two entity versions according to configured rules.
	/// Returns the merged entity version.
	pub fn merge(
		&self,
		local: &EntityVersion,
		remote: &EntityVersion,
	) -> Result<EntityVersion> {
		// Validate that entities have the same type and key
		if local.entity_type != remote.entity_type {
			return Err(anyhow!(
				"cannot merge entities of different types: {} vs {}",
				local.entity_type,
				remote.entity_type
			));
		}
		if local.key != remote.key {
			return Err(anyhow!(
				"cannot merge entities with different keys: {} vs {}",
				local.key,
				remote.key
			));
		}

		// Get merge rule for entity type
		let rule = self.config.get_rule(&local.entity_type);

		// Apply merge strategy
		match rule.strategy {
			MergeStrategy::LastWriterWins => self.merge_lww(local, remote),
			MergeStrategy::MergeSightings => self.merge_sightings(local, remote, &rule),
			MergeStrategy::Tombstone => self.merge_tombstone(local, remote),
		}
	}

	/// Last Writer Wins: select the version with the latest timestamp.
	fn merge_lww(&self, local: &EntityVersion, remote: &EntityVersion) -> Result<EntityVersion> {
		// Tombstones always win
		if remote.tombstone {
			return Ok(remote.clone());
		}
		if local.tombstone {
			return Ok(local.clone());
		}

		// Select newer version
		if remote.version.is_newer_than(&local.version) {
			Ok(remote.clone())
		} else {
			Ok(local.clone())
		}
	}

	/// Merge sightings: combine observation counts and update timestamps.
	fn merge_sightings(
		&self,
		local: &EntityVersion,
		remote: &EntityVersion,
		rule: &MergeRule,
	) -> Result<EntityVersion> {
		// Tombstones always win
		if remote.tombstone || local.tombstone {
			return self.merge_tombstone(local, remote);
		}

		// Start with local props
		let mut merged_props = local.props.clone();
		let remote_props = &remote.props;

		// Merge counters and timestamps
		if let Value::Object(merged_map) = &mut merged_props {
			if let Value::Object(remote_map) = remote_props {
				// Merge count fields (sum)
				for field in &rule.merge_fields {
					if field.contains("count") {
						let local_count = merged_map
							.get(field)
							.and_then(|v| v.as_u64())
							.unwrap_or(0);
						let remote_count =
							remote_map.get(field).and_then(|v| v.as_u64()).unwrap_or(0);
						merged_map
							.insert(field.clone(), Value::Number((local_count + remote_count).into()));
					}
				}

				// Handle timestamp fields
				for field in &rule.merge_fields {
					if field.contains("seen") || field.contains("timestamp") {
						if let Some(remote_val) = remote_map.get(field) {
							if let Some(local_val) = merged_map.get(field) {
								if let (Some(local_ts), Some(remote_ts)) =
									(local_val.as_u64(), remote_val.as_u64())
								{
									// For first_seen, keep the older timestamp
									if field.contains("first") {
										if remote_ts < local_ts {
											merged_map.insert(field.clone(), remote_val.clone());
										}
									} else {
										// For last_seen and other timestamps, keep the newer
										if remote_ts > local_ts {
											merged_map.insert(field.clone(), remote_val.clone());
										}
									}
								}
							} else {
								merged_map.insert(field.clone(), remote_val.clone());
							}
						}
					}
				}

				// LWW for other specified fields
				for field in &rule.lww_fields {
					if remote.version.is_newer_than(&local.version) {
						if let Some(val) = remote_map.get(field) {
							merged_map.insert(field.clone(), val.clone());
						}
					}
				}
			}
		}

		// Use the newer version vector
		let version = if remote.version.is_newer_than(&local.version) {
			remote.version.clone()
		} else {
			local.version.clone()
		};

		Ok(EntityVersion {
			entity_type: local.entity_type.clone(),
			key: local.key.clone(),
			props: merged_props,
			version,
			tombstone: false,
		})
	}

	/// Tombstone merge: preserve deletion markers.
	fn merge_tombstone(
		&self,
		local: &EntityVersion,
		remote: &EntityVersion,
	) -> Result<EntityVersion> {
		// If either is a tombstone, the result is a tombstone with newer version
		if remote.tombstone && !local.tombstone {
			return Ok(remote.clone());
		}
		if local.tombstone && !remote.tombstone {
			return Ok(local.clone());
		}
		if remote.tombstone && local.tombstone {
			// Both tombstones: use newer version
			return if remote.version.is_newer_than(&local.version) {
				Ok(remote.clone())
			} else {
				Ok(local.clone())
			};
		}

		// Neither is tombstone: use LWW
		self.merge_lww(local, remote)
	}
}

#[cfg(feature = "unit-tests")]
mod tests {
	use serde_json::json;

	use super::*;

	#[test]
	fn test_version_vector_comparison() {
		let v1 = VersionVector::new("node1", 1000);
		let v2 = VersionVector::new("node2", 2000);
		let v3 = VersionVector::new("node1", 2000);

		assert!(!v1.is_newer_than(&v2));
		assert!(v2.is_newer_than(&v1));
		assert!(v3.is_newer_than(&v1));

		// Tie-break by origin
		let v4 = VersionVector::new("node1", 2000);
		let v5 = VersionVector::new("node2", 2000);
		assert!(v5.is_newer_than(&v4)); // "node2" > "node1"
	}

	#[test]
	fn test_lww_merge() {
		let config = MergeConfig::new().with_default_strategy(MergeStrategy::LastWriterWins);
		let resolver = MergeResolver::new(config);

		let local = EntityVersion::new(
			"FieldValue",
			"test_key",
			json!({"value": "old"}),
			VersionVector::new("node1", 1000),
		);

		let remote = EntityVersion::new(
			"FieldValue",
			"test_key",
			json!({"value": "new"}),
			VersionVector::new("node2", 2000),
		);

		let merged = resolver.merge(&local, &remote).unwrap();
		assert_eq!(merged.props["value"], "new");
		assert_eq!(merged.version.timestamp, 2000);
	}

	#[test]
	fn test_merge_sightings() {
		let rule = MergeRule::new("Sighting", MergeStrategy::MergeSightings)
			.with_merge_fields(vec!["count".to_string(), "last_seen".to_string()]);
		let config = MergeConfig::new().add_rule(rule);
		let resolver = MergeResolver::new(config);

		let local = EntityVersion::new(
			"Sighting",
			"test_key",
			json!({"count": 5, "last_seen": 1000}),
			VersionVector::new("node1", 1000),
		);

		let remote = EntityVersion::new(
			"Sighting",
			"test_key",
			json!({"count": 3, "last_seen": 2000}),
			VersionVector::new("node2", 2000),
		);

		let merged = resolver.merge(&local, &remote).unwrap();
		assert_eq!(merged.props["count"], 8); // 5 + 3
		assert_eq!(merged.props["last_seen"], 2000); // newer timestamp
	}

	#[test]
	fn test_tombstone_wins() {
		let config = MergeConfig::new().with_default_strategy(MergeStrategy::Tombstone);
		let resolver = MergeResolver::new(config);

		let local = EntityVersion::new(
			"FieldValue",
			"test_key",
			json!({"value": "exists"}),
			VersionVector::new("node1", 1000),
		);

		let remote = EntityVersion::new(
			"FieldValue",
			"test_key",
			json!({}),
			VersionVector::new("node2", 2000),
		)
		.with_tombstone(true);

		let merged = resolver.merge(&local, &remote).unwrap();
		assert!(merged.tombstone);
		assert_eq!(merged.version.timestamp, 2000);
	}

	#[test]
	fn test_merge_config_default_rule() {
		let config = MergeConfig::new().with_default_strategy(MergeStrategy::MergeSightings);

		let rule = config.get_rule("UnknownType");
		assert_eq!(rule.strategy, MergeStrategy::MergeSightings);
		assert_eq!(rule.entity_type, "UnknownType");
	}

	#[test]
	fn test_merge_different_types_error() {
		let config = MergeConfig::new();
		let resolver = MergeResolver::new(config);

		let local = EntityVersion::new(
			"FieldValue",
			"key1",
			json!({}),
			VersionVector::new("node1", 1000),
		);

		let remote =
			EntityVersion::new("Sighting", "key1", json!({}), VersionVector::new("node2", 2000));

		let result = resolver.merge(&local, &remote);
		assert!(result.is_err());
		assert!(result
			.unwrap_err()
			.to_string()
			.contains("different types"));
	}

	#[test]
	fn test_merge_different_keys_error() {
		let config = MergeConfig::new();
		let resolver = MergeResolver::new(config);

		let local = EntityVersion::new(
			"FieldValue",
			"key1",
			json!({}),
			VersionVector::new("node1", 1000),
		);

		let remote = EntityVersion::new(
			"FieldValue",
			"key2",
			json!({}),
			VersionVector::new("node2", 2000),
		);

		let result = resolver.merge(&local, &remote);
		assert!(result.is_err());
		assert!(result
			.unwrap_err()
			.to_string()
			.contains("different keys"));
	}
}
