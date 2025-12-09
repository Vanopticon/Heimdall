# Merge Rules and Conflict Resolution

This document describes the merge resolver and per-entity merge rules used in Heimdall's multi-instance synchronization.

## Overview

The merge resolver provides configurable conflict resolution strategies for reconciling entity updates across multiple Heimdall instances. When two instances modify the same entity concurrently, the merge resolver applies configured rules to produce a deterministic, consistent outcome.

## Core Concepts

### Version Vector

Each entity version includes a version vector with:

- **origin**: Instance ID (hostname or UUID) that created this version
- **timestamp**: Logical timestamp (Unix milliseconds) when the version was created
- **version**: Optional counter for strict ordering within the same timestamp

Version vectors enable conflict detection and provide deterministic ordering when timestamps are equal (tie-breaking by origin ID).

### Tombstones

Deleted entities are marked with a tombstone flag rather than being removed from the graph. This ensures deletions propagate correctly across instances and prevents deleted entities from reappearing after synchronization.

## Merge Strategies

### 1. Last Writer Wins (LWW)

**Strategy**: `LastWriterWins`

Selects the entity version with the most recent timestamp. If timestamps are equal, uses lexicographic comparison of origin IDs for deterministic tie-breaking.

**Use Cases**:

- Metadata fields (description, tags, labels)
- Configuration settings
- User preferences
- Any field where the latest value is authoritative

**Example**:

```rust
use vanopticon_heimdall::sync::{MergeConfig, MergeStrategy, MergeRule};

let config = MergeConfig::new()
    .with_default_strategy(MergeStrategy::LastWriterWins);
```

**Behavior**:

- Tombstones always win (deletions propagate)
- Newer timestamp wins
- Tie-break by origin ID (deterministic)

### 2. Merge Sightings

**Strategy**: `MergeSightings`

Combines observation counts and updates timestamps from both versions. Used for aggregating sighting data, co-occurrence counts, and other cumulative metrics.

**Use Cases**:

- Sighting counts (how many times a value was observed)
- Co-occurrence relationships (email+password pairs)
- Aggregated statistics
- Any metric that should be summed across instances

**Configuration**:

```rust
use vanopticon_heimdall::sync::{MergeConfig, MergeStrategy, MergeRule};

let rule = MergeRule::new("Sighting", MergeStrategy::MergeSightings)
    .with_merge_fields(vec![
        "count".to_string(),
        "last_seen".to_string(),
    ])
    .with_lww_fields(vec![
        "context".to_string(),
    ]);

let config = MergeConfig::new().add_rule(rule);
```

**Field Naming Conventions**:

The merge implementation uses substring matching to identify field types:

- Fields containing `"count"` are treated as counters (summed)
- Fields containing `"seen"` or `"timestamp"` are treated as timestamps
- Fields containing `"first"` within timestamps use older value
- Other timestamp fields use newer value

**Important**: Use clear, unambiguous field names to avoid false positives:

- ✅ Good: `count`, `sighting_count`, `last_seen`, `first_seen`
- ❌ Avoid: `account`, `unseen_data` (would trigger false matches)

**Behavior**:

- **Count fields**: Values are summed (e.g., `count: 10 + 5 = 15`)
- **Timestamp fields** (`last_seen`, `first_seen`, `timestamp`): Use the newer value
- **LWW fields**: Apply last-writer-wins logic using version timestamp
- Other fields: Preserved from local version

### 3. Tombstone

**Strategy**: `Tombstone`

Specialized strategy for handling entity deletions. Tombstones always take precedence over active entities.

**Use Cases**:

- Entity deletion
- Soft deletes with audit trail
- Preventing resurrection of deleted data

**Example**:

```rust
use vanopticon_heimdall::sync::{MergeConfig, MergeStrategy, EntityVersion, VersionVector};
use serde_json::json;

let config = MergeConfig::new()
    .with_default_strategy(MergeStrategy::Tombstone);

// Mark an entity as deleted
let deleted = EntityVersion::new(
    "FieldValue",
    "temp_key",
    json!({"deleted_at": 1500}),
    VersionVector::new("node1", 1500),
)
.with_tombstone(true);
```

**Behavior**:

- If either version is a tombstone, result is a tombstone
- Between two tombstones, use the newer version
- If neither is a tombstone, fall back to LWW

## Per-Entity Configuration

Different entity types can use different merge strategies:

```rust
use vanopticon_heimdall::sync::{MergeConfig, MergeStrategy, MergeRule};

let config = MergeConfig::new()
    // Default for entities without specific rules
    .with_default_strategy(MergeStrategy::LastWriterWins)
    // Sightings use merge strategy
    .add_rule(
        MergeRule::new("Sighting", MergeStrategy::MergeSightings)
            .with_merge_fields(vec!["count".to_string(), "last_seen".to_string()])
    )
    // Co-occurrence uses merge with hybrid fields
    .add_rule(
        MergeRule::new("CoOccurrence", MergeStrategy::MergeSightings)
            .with_merge_fields(vec!["count".to_string(), "last_seen".to_string()])
            .with_lww_fields(vec!["context".to_string()])
    )
    // FieldValue uses LWW
    .add_rule(
        MergeRule::new("FieldValue", MergeStrategy::LastWriterWins)
    );
```

## Common Patterns

### Pattern 1: Aggregating Observations

For entities that accumulate sightings across instances:

```rust
MergeRule::new("Sighting", MergeStrategy::MergeSightings)
    .with_merge_fields(vec![
        "count".to_string(),
        "first_seen".to_string(),
        "last_seen".to_string(),
    ])
```

**Result**: Counts are summed, timestamps are updated to newest.

### Pattern 2: Metadata with LWW

For entities where the latest value is authoritative:

```rust
MergeRule::new("Entity", MergeStrategy::LastWriterWins)
```

**Result**: Entire entity is replaced by the version with the newer timestamp.

### Pattern 3: Hybrid (Merge + LWW)

For entities that combine aggregated and LWW fields:

```rust
MergeRule::new("CoOccurrence", MergeStrategy::MergeSightings)
    .with_merge_fields(vec!["count".to_string(), "last_seen".to_string()])
    .with_lww_fields(vec!["breach_source".to_string(), "severity".to_string()])
```

**Result**: Counts are summed, metadata uses LWW based on version timestamp.

## Determinism Guarantees

The merge resolver provides deterministic outcomes:

1. **Timestamp ordering**: Versions are ordered by timestamp first
2. **Origin tie-breaking**: When timestamps are equal, origin IDs provide deterministic ordering
3. **Tombstone precedence**: Deletions always win over active entities
4. **Commutative merges**: `merge(A, B)` produces the same logical outcome as `merge(B, A)`

## Usage Example

```rust
use vanopticon_heimdall::sync::{
    EntityVersion, MergeConfig, MergeResolver, MergeRule, MergeStrategy, VersionVector,
};
use serde_json::json;

// Configure merge rules
let config = MergeConfig::new()
    .add_rule(
        MergeRule::new("Sighting", MergeStrategy::MergeSightings)
            .with_merge_fields(vec!["count".to_string(), "last_seen".to_string()])
    )
    .with_default_strategy(MergeStrategy::LastWriterWins);

let resolver = MergeResolver::new(config);

// Two concurrent updates to the same sighting
let node1 = EntityVersion::new(
    "Sighting",
    "ip:192.168.1.1",
    json!({"count": 100, "last_seen": 2000}),
    VersionVector::new("node1", 2000),
);

let node2 = EntityVersion::new(
    "Sighting",
    "ip:192.168.1.1",
    json!({"count": 50, "last_seen": 2100}),
    VersionVector::new("node2", 2100),
);

// Merge produces deterministic result
let merged = resolver.merge(&node1, &node2).unwrap();
assert_eq!(merged.props["count"], 150); // 100 + 50
assert_eq!(merged.props["last_seen"], 2100); // newer timestamp
```

## Testing Conflict Resolution

The merge resolver includes comprehensive tests covering:

- **Concurrent updates**: LWW with timestamp ordering
- **Sightings merge**: Count aggregation and timestamp updates
- **Tombstone semantics**: Deletion propagation
- **Partition reconciliation**: Multi-node recovery
- **Version vector tie-breaking**: Deterministic ordering
- **Three-way merges**: Sequential merge operations

Run tests with:

```bash
cargo test --features unit-tests --lib sync
cargo test --features integration-tests --test integration_merge
```

## Migration Guidance

### Existing Installations

For existing Heimdall instances, add merge configuration:

1. **Define rules**: Create a `MergeConfig` with rules for each entity type
2. **Test locally**: Run integration tests to verify behavior
3. **Deploy incrementally**: Roll out to one instance, verify synchronization
4. **Monitor metrics**: Watch for merge conflicts and resolution patterns

### Default Configuration

If no configuration is provided, the default behavior is:

- **Default strategy**: `LastWriterWins`
- **No per-entity rules**: All entities use LWW
- **Tombstones**: Always respected regardless of strategy

### Adding New Entity Types

When adding a new entity type:

1. Decide the merge strategy based on semantics:
	 - Metadata/config → `LastWriterWins`
	 - Observations/counts → `MergeSightings`
	 - Deletions → `Tombstone`
2. Add a rule to the configuration
3. Add tests covering conflict scenarios
4. Document the merge behavior

## Limitations and Future Improvements

### Current Limitations

1. **Field Type Detection**: Uses substring matching (`contains()`) which can cause false positives
	 - **Mitigation**: Use clear, unambiguous field names
	 - **Future**: Explicit field type configuration or type tags

2. **Timestamp Format**: Assumes timestamps are u64 values
	 - **Mitigation**: Use Unix milliseconds (u64) for all timestamps
	 - **Future**: Support multiple timestamp formats (ISO8601, f64, etc.)

3. **Version Vector Simplicity**: Uses single timestamp + origin ID
	 - **Current**: Sufficient for deterministic ordering
	 - **Future**: Full vector clocks for complex causality

### Planned Enhancements

- **Field Type Configuration**: Explicit field classification instead of substring matching
- **Custom Merge Functions**: Per-field custom merge logic
- **Conflict Callbacks**: Application-defined conflict resolution hooks
- **Version Vector Evolution**: Full CRDT support if needed

## See Also

- `src/sync/merge.rs` — Implementation
- `tests/integration_merge.rs` — Integration tests
- `docs/design/features/SYNC-001-Multi-Heimdall-Synchronization.md` — Feature specification
- `docs/design/DataModel.md` — Entity model
