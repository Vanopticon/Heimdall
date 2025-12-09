# M0T3 — Graph Schema & Postgres+AGE Client Wrapper

**Category:** Functional
**Status:** Complete
**Milestone:** 0 (Foundation)

## Summary

This feature implements the canonical graph schema for Heimdall's provenance-first data model and provides a lightweight Postgres+Apache AGE client wrapper with support for transactional upserts, batch operations, and row persistence.

## Deliverables

### SQL Schema (`sql/v1/001-create_graph.sql`)

- Defines core node types: Dump, Row, Sighting, Field, FieldValue, Entity, RowHash
- Creates the `heimdall_graph` in Apache AGE
- Establishes vertex labels (VLabels) for all canonical node types
- Provides foundation for provenance tracking and value deduplication

### Enhanced AGE Client (`src/age_client.rs`)

**New Methods:**

1. `persist_row()` - Persist a single row with its cells (sightings) into the graph
	- Creates Row and Sighting nodes
	- Links to canonical FieldValue nodes
	- Preserves provenance (Dump → Row → Sighting → FieldValue)

2. `increment_co_occurrence()` - Track when two values appear together
	- Creates or updates CO_OCCURS relationships
	- Uses deterministic ordering to avoid duplicates
	- Maintains count and last_seen timestamps

3. `persist_credential()` - Track credential pairs (email → password)
	- Creates or updates CREDENTIAL relationships
	- Increments observation count
	- Tracks last_seen timestamps

4. `apply_migration()` - Apply SQL migrations
	- Executes raw SQL to initialize schema
	- Supports idempotent migrations

### Integration Tests

Created comprehensive test suite (`tests/integration_schema_migration.rs`):

- `test_schema_migration` - Validates SQL schema application
- `test_persist_row_workflow` - Tests row persistence with cells
- `test_co_occurrence_tracking` - Validates co-occurrence counting
- `test_credential_relationship` - Tests credential pair tracking
- `test_multiple_rows_same_dump` - Validates multiple row persistence

### Documentation

- `sql/v1/README.md` - Complete schema documentation
	- Node type specifications
	- Relationship patterns
	- Example queries
	- Best practices
	- Migration instructions

## Implementation Details

### Provenance-First Design

The schema maintains full provenance by:

1. Preserving original structure (Dump → Row → Sighting)
2. Deduplicating values at the FieldValue level
3. Enabling reconstruction of original dumps
4. Supporting co-occurrence analysis

### Safety Features

All operations include:

- Input sanitization (alphanumeric + underscore for identifiers)
- JSON serialization for safe value embedding
- Parameterized SQL queries (graph name, Cypher query)
- Deterministic ordering for undirected relationships

### Batch Operations

The client supports:

- Single-statement row persistence (multiple MERGE operations)
- Batched entity merges (existing functionality enhanced)
- Fallback to per-item operations on batch failure

## Usage Examples

### Apply Schema Migration

```rust
let client = AgeClient::new(pool, "heimdall_graph");
let schema_sql = include_str!("../sql/v1/001-create_graph.sql");
client.apply_migration(schema_sql).await?;
```

### Persist a Row

```rust
let cells = vec![
    ("email".to_string(), "test@example.com".to_string(), 
     "email:test@example.com".to_string(), "test@example.com".to_string()),
    ("password".to_string(), "pass123".to_string(),
     "password:hashed_pass123".to_string(), "hashed_pass123".to_string()),
];

client.persist_row(
    "dump-001",           // dump_id
    0,                    // row_index
    Some("abc123hash"),   // row_hash
    &cells,
    "2024-01-15T10:30:00Z"
).await?;
```

### Track Co-occurrence

```rust
client.increment_co_occurrence(
    "email:alice@example.com",
    "password:hashed_alice_pass",
    "2024-01-15T10:30:00Z"
).await?;
```

### Track Credential Pair

```rust
client.persist_credential(
    "email:bob@example.com",
    "password:hashed_bob_pass",
    "2024-01-15T10:30:00Z"
).await?;
```

## Testing

All tests are gated behind the `integration-tests` feature and require Docker:

```bash
# Run integration tests
RUN_DOCKER_INTEGRATION_TESTS=1 cargo test --features integration-tests

# Run specific test
RUN_DOCKER_INTEGRATION_TESTS=1 cargo test test_persist_row_workflow --features integration-tests
```

## Acceptance Criteria

- [x] SQL migrations apply on local dev Postgres+AGE
- [x] `src/age_client.rs` exposes clear API for MERGE-style upserts
- [x] Integration tests confirm canonical nodes are created with correct provenance
- [x] Schema and usage examples are documented

## Related

- [DataModel.md](../DataModel.md) - Canonical data model specification
- [Implementation-Roadmap.md](../Implementation-Roadmap.md) - Milestone 0 requirements
- `sql/v1/001-create_graph.sql` - SQL schema definition
- `sql/v1/README.md` - Schema documentation
- `tests/integration_schema_migration.rs` - Integration tests

## Future Enhancements

1. Add query helpers for common graph traversals (reconstruct dump, find co-occurrences)
2. Implement pairwise co-occurrence in application code for efficiency
3. Add support for Entity composition from FieldValues
4. Implement row-hash based duplicate detection
5. Add property-based indices for production performance optimization
