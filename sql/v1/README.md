# Heimdall Graph Schema v1

This directory contains SQL migration scripts for the Heimdall canonical graph schema.

## Overview

The schema implements a provenance-first data model for ingestion, normalization, and correlation of telemetry data. All data is stored in PostgreSQL with Apache AGE (graph database) and pgvector extensions.

## Core Node Types

### Dump

Represents a single uploaded file or ingestion run.

**Properties:**

- `id` (string, unique) - Unique identifier for the dump
- `received_at` (timestamp) - When the dump was ingested
- `uploader` (string, optional) - Who uploaded the dump
- `format` (string, optional) - Original file format (NDJSON, CSV, etc.)
- `fingerprint` (string, optional) - Hash of the dump content

### Row

Represents a single row within a Dump, preserving original structure.

**Properties:**

- `dump_id` (string) - Parent dump identifier
- `index` (integer) - Zero-based row number within the dump
- `row_hash` (string, optional) - Canonical hash for duplicate detection

**Relationships:**

- `(Dump)-[:HAS_ROW]->(Row)` - Links row to its parent dump

### Sighting

Represents a single observed cell/field value occurrence in a Row.

**Properties:**

- `id` (string, optional) - Unique sighting identifier
- `column` (string) - Field name for this cell
- `raw` (string) - Original raw value as observed
- `timestamp` (timestamp) - When this value was observed
- `position` (integer, optional) - Column position in the row

**Relationships:**

- `(Row)-[:HAS_SIGHTING]->(Sighting)` - Links sighting to its row
- `(Sighting)-[:OBSERVED_VALUE]->(FieldValue)` - Links to canonical value

### Field

Represents a canonical field name (e.g., "email", "ip", "domain").

**Properties:**

- `name` (string, unique) - Canonical field name
- `type` (string, optional) - Data type classification
- `sensitive` (boolean, optional) - Whether this is a sensitive field

**Relationships:**

- `(FieldValue)-[:VALUE_OF]->(Field)` - Links canonical values to field types

### FieldValue

Represents a canonicalized value instance with stable identity for deduplication.

**Properties:**

- `canonical_key` (string, unique) - Stable identifier for this value
- `value` (string) - Canonicalized/normalized value
- `created_at` (timestamp) - When first observed
- `hash` (string, optional) - Hash of the value (for PII)

**Relationships:**

- `(FieldValue)-[:VALUE_OF]->(Field)` - Links to field type
- `(FieldValue)-[:CO_OCCURS]->(FieldValue)` - Co-occurrence tracking
- `(FieldValue)-[:CREDENTIAL]->(FieldValue)` - Credential pair tracking

### Entity

Represents a higher-level deduplicated object (e.g., host, user) composed from FieldValues.

**Properties:**

- `id` (string, unique) - Entity identifier
- `type` (string) - Entity type (host, user, etc.)
- `confidence` (float, optional) - Confidence score for deduplication

**Relationships:**

- `(Entity)-[:HAS_VALUE]->(FieldValue)` - Composes entity from values

### RowHash

Optional node type for efficient row-level duplicate detection.

**Properties:**

- `hash` (string, unique) - Canonical row hash

**Relationships:**

- `(Dump)-[:HAS_ROWHASH]->(RowHash)` - Links dumps to row hashes

## Relationship Types

### Co-occurrence: CO_OCCURS

Tracks when two canonical values appear together in the same row.

**Properties:**

- `count` (integer) - Number of times observed together
- `last_seen` (timestamp) - Most recent co-occurrence

**Pattern:**

```cypher
(FieldValue)-[:CO_OCCURS]-(FieldValue)
```

**Note:** Use deterministic ordering (a.canonical_key < b.canonical_key) to avoid duplicates.

### Credentials: CREDENTIAL

Tracks credential pairs (e.g., email -> password).

**Properties:**

- `count` (integer) - Number of times this pair was observed
- `last_seen` (timestamp) - Most recent observation
- `samples` (array, optional) - Sample occurrences

**Pattern:**

```cypher
(email:FieldValue)-[:CREDENTIAL]->(password:FieldValue)
```

## Migration Files

### 001-create_graph.sql

Creates the `heimdall_graph` in Apache AGE and establishes the core node labels.

**What it does:**

1. Ensures AGE extension is installed
2. Creates the `heimdall_graph` if it doesn't exist
3. Establishes vertex labels (VLabels) for core node types
4. Creates bootstrap nodes to initialize the schema
5. Cleans up bootstrap nodes after schema initialization

**Usage:**

```sql
-- Apply via psql
\i sql/v1/001-create_graph.sql

-- Or via application code
client.apply_migration(include_str!("sql/v1/001-create_graph.sql")).await?;
```

## Indices

Apache AGE automatically creates internal indices on node IDs. For production deployments with large graphs, consider adding property-based indices:

```sql
-- Example: Index on FieldValue.canonical_key for fast lookups
CREATE INDEX IF NOT EXISTS idx_fieldvalue_canonical_key 
  ON heimdall_graph."FieldValue" 
  USING btree ((properties->>'canonical_key'));

-- Example: Index on Dump.id
CREATE INDEX IF NOT EXISTS idx_dump_id 
  ON heimdall_graph."Dump" 
  USING btree ((properties->>'id'));
```

Consult the Apache AGE documentation for index strategies specific to your query patterns.

## Example Queries

### Reconstruct a Dump as a table

```cypher
MATCH (d:Dump {id: $dump_id})-[:HAS_ROW]->(r:Row)
OPTIONAL MATCH (r)-[:HAS_SIGHTING]->(s:Sighting)-[:OBSERVED_VALUE]->(fv:FieldValue)
RETURN r.index AS row_index, 
       collect({column: s.column, raw: s.raw, canonical: fv.canonical_key}) AS cells
ORDER BY r.index
```

### Find co-occurring values

```cypher
MATCH (a:FieldValue {canonical_key: $key_a})-[co:CO_OCCURS]-(b:FieldValue)
RETURN b.canonical_key, co.count, co.last_seen
ORDER BY co.count DESC
```

### Track credential variations

```cypher
MATCH (email:FieldValue {canonical_key: $email_key})-[c:CREDENTIAL]->(pwd:FieldValue)
RETURN pwd.canonical_key, c.count, c.last_seen
ORDER BY c.count DESC
```

## Best Practices

1. **Idempotent Operations**: Use MERGE instead of CREATE for nodes that should be unique
2. **Deterministic Ordering**: For undirected relationships, use canonical ordering
3. **Batch Operations**: Group multiple operations in single Cypher statements
4. **Sanitization**: All identifiers and values are sanitized/JSON-serialized by the client
5. **Provenance**: Always link sightings back to Dump and Row for auditability

## Version History

- v1 (2024) - Initial schema with provenance-first design

## References

- [DataModel.md](../../docs/design/DataModel.md) - Detailed data model specification
- [Apache AGE Documentation](https://age.apache.org/)
- [PostgreSQL Documentation](https://www.postgresql.org/docs/)
