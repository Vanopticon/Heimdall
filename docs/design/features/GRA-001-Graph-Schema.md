# GRA-001: Graph Schema & Postgres+AGE Client Wrapper

## Category

Functional

## Description

Define the canonical graph schema for Heimdall's data model and provide a lightweight Postgres+Apache AGE client wrapper to perform MERGE/upsert operations, transactional writes, and batch operations. The schema represents dumps, fields, field values, sightings, entities, and NPI categories as graph nodes with relationships for correlation and provenance tracking.

## Requirements

1. SQL schema defining:
   - Core vertex types: `sources`, `dumps`, `fields`, `field_data`, `rows`, `field_value`, `sightings`, `ip`, `NPI_Category`
   - Edge types for relationships: `HAS_SUBCATEGORY`, `HAS_MEMBER`, `ROUTES_TO`, `ROOT_CONTAINS`, `DUMP_HAS_FIELD`, `FIELD_HAS_DATA`, `FIELD_LINK`, `FIELD_HAS_NPI`, `FIELD_VALUE_HAS_NPI`, `DUMP_HAS_ROW`, `ROW_HAS_FIELD_VALUE`, `FIELD_VALUE_FOR_FIELD`, `SIGHTING_OF`
   - ROOT nodes for hierarchical organization
   - NPI category taxonomy with predefined categories

2. AGE Client Wrapper (`src/lib/server/ageClient.ts`) providing:
   - Initialization from environment variables (HMD_DATABASE_URL, PG* variables)
   - Schema registration from `sql/v1/*.sql`
   - Basic CRUD operations for vertices and edges
   - Domain-specific helpers: `createIp()`, `createDump()`, `createField()`, `createFieldValue()`
   - NPI linking: `linkFieldToNpi()`, `linkFieldValueToNpi()`
   - **Transaction support**: `withTransaction()` for atomic operations
   - **Batch operations**: `batchCreateVertices()`, `batchUpsertVertices()`, `batchCreateEdges()`
   - **Raw Cypher execution**: `executeCypher()`, `executeCypherBatch()`

3. Integration tests validating:
   - Schema migration (ROOT nodes, NPI categories)
   - Basic vertex and edge creation
   - Domain-specific operations
   - Transaction commit and rollback
   - Batch write operations
   - Idempotent MERGE behavior

4. Documentation:
   - Schema design in `sql/v1/001-create_graph.sql` with inline comments
   - Usage examples in this feature card
   - API reference for all client functions

## Acceptance Criteria

- SQL migrations apply successfully on Postgres+AGE
- `src/lib/server/ageClient.ts` exposes clear API for MERGE-style upserts
- Transaction support allows atomic multi-operation writes
- Batch operations reduce round-trips for bulk ingestion
- Integration tests confirm canonical nodes are created with correct provenance
- Tests validate transaction rollback on errors
- Tests verify idempotent behavior of MERGE operations
- All tests pass when database is available

## Implementation Notes

### Graph Schema Design

The schema follows a hierarchical model with ROOT nodes for major categories:

- **Dumps**: Represent data ingestion batches with metadata
- **Fields**: Define columns/attributes in dumps
- **Field Values**: Store actual data values with row keys for provenance
- **Sightings**: Track repeated occurrences of values across different contexts
- **NPI Categories**: Classify fields and values by sensitivity (PII, FINANCIAL, HEALTH, etc.)
- **Entities**: Represent infrastructure elements (IP addresses, domains, etc.)

### Transaction Support

The `withTransaction()` function provides automatic BEGIN/COMMIT/ROLLBACK:

```typescript
import { withTransaction } from '$lib/server/ageClient';

await withTransaction(async (conn) => {
	// Multiple operations in a single transaction
	await conn.query(sql1);
	await conn.query(sql2);
	// Auto-commits on success, rolls back on error
});
```

### Batch Operations

Efficiently create multiple nodes or edges:

```typescript
import { batchUpsertVertices, batchCreateEdges } from '$lib/server/ageClient';

// Batch upsert nodes (MERGE behavior)
await batchUpsertVertices('dumps', [
	{ match: { name: 'dump-1' }, set: { updated_at: new Date() } },
	{ match: { name: 'dump-2' }, set: { updated_at: new Date() } },
]);

// Batch create relationships
await batchCreateEdges('DUMP_HAS_FIELD', [
	{
		fromLabel: 'dumps',
		fromMatch: { name: 'dump-1' },
		toLabel: 'fields',
		toMatch: { name: 'field-a' },
	},
]);
```

### Usage Examples

#### Initialize the Client

```typescript
import { initAgeClient, disconnectAgeClient } from '$lib/server/ageClient';

// Initialize with environment config
await initAgeClient();

// Or override specific settings
await initAgeClient({
	host: 'localhost',
	port: 5432,
	database: 'heimdall_dev',
	graph: 'dumps_graph',
});

// Always disconnect when done
await disconnectAgeClient();
```

#### Create Entities

```typescript
import { createDump, createField, createFieldValue } from '$lib/server/ageClient';

// Create a data dump
await createDump('vendor-feed-2024-01', {
	source: 'acme-vendor',
	received_at: new Date().toISOString(),
});

// Add a field to the dump
await createField('vendor-feed-2024-01', 'email', {
	type: 'string',
	description: 'User email addresses',
});

// Store field values with provenance
await createFieldValue(
	'vendor-feed-2024-01',
	'email',
	'row-1',
	'user@example.com',
	{ sanitized: false }
);
```

#### Link to NPI Categories

```typescript
import { linkFieldToNpi, linkFieldValueToNpi } from '$lib/server/ageClient';

// Mark field as containing PII
await linkFieldToNpi('email', 'PII');

// Mark specific value as financial data
await linkFieldValueToNpi('card_number', 'row-42', 'FINANCIAL');
```

#### Track Infrastructure

```typescript
import { createIp, createRoute } from '$lib/server/ageClient';

// Create IP node linked to INFRASTRUCTURE category
await createIp('10.0.1.100', {
	description: 'Web server',
	asn: 'AS64512',
});

// Define routing relationship
await createRoute('10.0.1.100', '10.0.2.200');
```

### Test Configuration

Integration tests require a running Postgres+AGE instance. Set environment variables:

```bash
export PGHOST=localhost
export PGPORT=5432
export PGDATABASE=heimdall_test
export PGUSER=postgres
export PGPASSWORD=secret
export AGE_GRAPH=dumps_graph
```

To skip database tests (e.g., in CI without DB):

```bash
export SKIP_DB_TESTS=true
```

### Security Considerations

- All string values are escaped to prevent Cypher injection
- Use parameterized queries where possible
- Transaction support ensures consistency
- NPI classification enables proper data handling policies

## Related Files

- `sql/v1/000-prerequisites.sql` — Extension setup
- `sql/v1/001-create_graph.sql` — Graph schema definition
- `src/lib/server/ageClient.ts` — Client implementation
- `src/lib/server/ageClient.test.ts` — Integration tests
- `docs/design/DataModel.md` — Data model overview
- `docs/design/Graph.md` — Graph design patterns
- `docs/design/Implementation-Roadmap.md` — Milestone tracking
