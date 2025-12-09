# Migration and Backfill Plan for Canonical Key / Row Hash Versioning

**Document Version**: 1.0  
**Date**: 2025-12-09  
**Status**: Design Document  
**Related Issue**: ARCH-001.5 (Part of ARCH-001)

## Executive Summary

This document outlines the strategy for handling future changes to canonical key generation and row-hash semantics in Heimdall's graph data model. As the system evolves, changes to how we generate unique identifiers for nodes (particularly `row_key`, `field_value` composite keys, and entity canonical keys) may be necessary to improve deduplication, support new data sources, or fix semantic issues. This plan ensures we can migrate existing data safely without breaking queries or losing provenance.

## Current State Analysis

### Existing Key Generation Patterns

Based on `src/lib/server/ageClient.ts` and `sql/v1/001-create_graph.sql`:

1. **Row Keys** (`rows` vertex):
   - Current: `row_key` property (string or number)
   - Used to group field values from the same logical row in a dump
   - Example: `{dump: 'breach_2024', row_key: '12345'}`

2. **Field Value Keys** (`field_value` vertex):
   - Current: Composite uniqueness on `{value, field, row_key, dump}`
   - MERGE statement ensures deduplication within a dump/field/row combination
   - Example: `{value: 'user@example.com', field: 'email', row_key: '12345', dump: 'breach_2024'}`

3. **Sightings** (cross-reference detection):
   - Matches field values with identical `value` property across different contexts
   - No explicit versioning; relies on exact string matching

### Potential Change Scenarios

1. **Row Key Versioning**: Need to change how row keys are computed (e.g., add checksum, change delimiter)
2. **Field Value Normalization**: Change how values are normalized before storage (e.g., lowercase emails, trim whitespace)
3. **Hash Algorithm Updates**: Switch from implicit string keys to explicit hash-based keys (e.g., SHA-256)
4. **Schema Extension**: Add new properties to uniqueness constraints

## Strategy Overview

### Versioning Approach

Implement **canonical key versioning** using property-level version tags:

```cypher
// Version 1 (current implicit)
MERGE (fv:field_value {value: 'user@example.com', field: 'email', row_key: '12345', dump: 'breach_2024'})

// Version 2 (explicit versioning)
MERGE (fv:field_value {
	value: 'user@example.com',
	field: 'email',
	row_key: '12345',
	dump: 'breach_2024',
	canonical_key_version: 'v2',
	canonical_key: 'sha256:abcd1234...'
})
```

### Key Principles

1. **Backward Compatibility**: Old queries continue to work during migration
2. **Incremental Migration**: Process data in batches with resume capability
3. **Dual-Write Period**: New data written with both old and new keys during transition
4. **Validation First**: Dry-run validation on snapshot before production
5. **Rollback Capability**: Maintain ability to revert if issues detected

## Migration Strategies

### Strategy 1: Shadow Property Migration (Recommended)

**Use Case**: Add new key format while preserving existing keys

**Approach**:
1. Add new versioned properties alongside existing ones
2. Update ingestion code to write both formats
3. Incrementally backfill historical data
4. Gradually migrate queries to use new format
5. Eventually deprecate old properties

**Pros**:
- Zero downtime
- Rollback-friendly
- Can validate new format against old
- Gradual query migration

**Cons**:
- Temporarily increased storage (typically <5% for key properties)
- Requires coordinated application updates

**Timeline**: 4-6 weeks for full migration

### Strategy 2: In-Place Transformation

**Use Case**: Semantic change that doesn't require old format

**Approach**:
1. Snapshot database
2. Run transformation job with progress tracking
3. Update affected nodes with new key properties
4. Validate referential integrity
5. Update application code simultaneously

**Pros**:
- Clean final state
- No storage overhead
- Simpler application logic post-migration

**Cons**:
- Requires coordinated downtime or complex read-write coordination
- More complex rollback process

**Timeline**: 1-2 weeks with brief maintenance window

### Strategy 3: Node Duplication with Edge Reconciliation

**Use Case**: Incompatible key changes requiring full node recreation

**Approach**:
1. Create new nodes with updated key semantics
2. Copy edges from old nodes to new nodes
3. Mark old nodes as deprecated (`_deprecated: true`)
4. Update queries to exclude deprecated nodes
5. Eventually delete deprecated nodes

**Pros**:
- Clean separation of old and new
- No data loss risk
- Easy rollback (just remove new nodes)

**Cons**:
- Highest storage overhead (2x temporarily)
- Complex edge reconciliation
- Query updates required

**Timeline**: 6-8 weeks for full migration

## Backfill Strategy

### Incremental Batch Processing

Process nodes in batches with progress tracking to enable resumption:

```typescript
interface MigrationState {
	version: string;           // Target version (e.g., 'v2')
	startTime: string;         // ISO timestamp
	endTime?: string;          // ISO timestamp when complete
	totalNodes: number;        // Total nodes to migrate
	processedNodes: number;    // Nodes processed so far
	batchSize: number;         // Nodes per batch
	lastProcessedId: string;   // Resume point
	errors: number;            // Error count
	status: 'running' | 'paused' | 'completed' | 'failed';
}
```

### Backfill Job Pseudocode

```typescript
async function backfillCanonicalKeys(
	targetVersion: string,
	batchSize: number = 1000,
	resumeFromId?: string
): Promise<MigrationState> {
	const state = await loadOrCreateMigrationState(targetVersion, resumeFromId);
	
	while (state.status === 'running') {
		// Fetch batch of nodes to process
		const batch = await fetchNextBatch({
			nodeLabel: 'field_value',
			batchSize,
			afterId: state.lastProcessedId,
			filter: {
				// Only process nodes without the new version
				canonical_key_version: null
			}
		});
		
		if (batch.length === 0) {
			state.status = 'completed';
			state.endTime = new Date().toISOString();
			break;
		}
		
		// Transform each node
		for (const node of batch) {
			try {
				const newKey = computeCanonicalKey(node, targetVersion);
				
				await updateNode({
					id: node.id,
					properties: {
						canonical_key_version: targetVersion,
						canonical_key: newKey,
						// Keep original properties for dual-read support
						...preserveOriginalProperties(node)
					}
				});
				
				state.processedNodes++;
				state.lastProcessedId = node.id;
			} catch (error) {
				state.errors++;
				await logMigrationError({
					nodeId: node.id,
					error: error.message,
					timestamp: new Date().toISOString()
				});
				
				// Fail fast if error rate too high
				if (state.errors > state.processedNodes * 0.01) { // 1% threshold
					state.status = 'failed';
					throw new Error('Migration error rate exceeded threshold');
				}
			}
		}
		
		// Checkpoint progress
		await saveMigrationState(state);
		
		// Rate limiting: pause between batches
		await sleep(100);
	}
	
	return state;
}

function computeCanonicalKey(node: any, version: string): string {
	switch (version) {
		case 'v2':
			// Example: SHA-256 hash of normalized composite key
			const normalized = [
				node.dump,
				node.field,
				node.row_key,
				normalizeValue(node.value)
			].join('::');
			return `sha256:${sha256(normalized)}`;
		
		default:
			throw new Error(`Unknown version: ${version}`);
	}
}

function normalizeValue(value: string): string {
	// Apply version-specific normalization
	return value.trim().toLowerCase();
}
```

### Progress Tracking Table

Store migration state in a dedicated tracking table:

```sql
CREATE TABLE IF NOT EXISTS heimdall_migrations (
	id SERIAL PRIMARY KEY,
	version VARCHAR(50) NOT NULL,
	node_label VARCHAR(100) NOT NULL,
	start_time TIMESTAMPTZ NOT NULL,
	end_time TIMESTAMPTZ,
	total_nodes INTEGER,
	processed_nodes INTEGER DEFAULT 0,
	batch_size INTEGER DEFAULT 1000,
	last_processed_id TEXT,
	errors INTEGER DEFAULT 0,
	status VARCHAR(20) DEFAULT 'running',
	metadata JSONB,
	UNIQUE(version, node_label)
);
```

## Compatibility Strategy

### Dual-Read Pattern

During migration, support reading both old and new key formats:

```typescript
async function findFieldValue(criteria: {
	value: string,
	field: string,
	rowKey: string,
	dump: string
}): Promise<any> {
	const c = getAgeClient();
	
	// Try new format first (more efficient if available)
	const canonicalKey = computeCanonicalKey(criteria, 'v2');
	let result = await c.connection.query(`
		SELECT * FROM cypher('${c.graph}', $$
			MATCH (fv:field_value {canonical_key: '${canonicalKey}'})
			RETURN fv
		$$) as (fv agtype);
	`);
	
	if (result.rows.length > 0) {
		return result.rows[0];
	}
	
	// Fallback to old format
	result = await c.connection.query(`
		SELECT * FROM cypher('${c.graph}', $$
			MATCH (fv:field_value {
				value: '${criteria.value}',
				field: '${criteria.field}',
				row_key: '${criteria.rowKey}',
				dump: '${criteria.dump}'
			})
			WHERE fv.canonical_key_version IS NULL
			RETURN fv
		$$) as (fv agtype);
	`);
	
	return result.rows[0] || null;
}
```

### Staged Rollout Phases

**Phase 1: Preparation (Week 1-2)**
- Add migration tracking table
- Implement backfill script
- Create monitoring dashboard
- Test on development snapshot

**Phase 2: Shadow Write (Week 3-4)**
- Deploy code that writes both old and new formats
- Begin incremental backfill of historical data
- Monitor error rates and performance

**Phase 3: Dual Read (Week 5-6)**
- Update queries to try new format first, fallback to old
- Complete backfill process
- Validate 100% coverage

**Phase 4: Deprecation (Week 7-8)**
- Remove fallback logic from queries
- Mark old properties as deprecated
- Plan cleanup of old properties

**Phase 5: Cleanup (Week 9+)**
- Remove old key properties from schema
- Update documentation
- Archive migration code

## Validation and Testing

### Pre-Migration Validation

Create snapshot of production data and validate transformation:

```typescript
async function validateMigration(
	sampleSize: number = 10000
): Promise<ValidationReport> {
	const report = {
		totalSampled: 0,
		matchingKeys: 0,
		mismatchedKeys: 0,
		errors: [],
		sampleMismatches: []
	};
	
	// Sample random nodes
	const sample = await fetchRandomSample('field_value', sampleSize);
	
	for (const node of sample) {
		report.totalSampled++;
		
		try {
			// Compute what the new key would be
			const newKey = computeCanonicalKey(node, 'v2');
			
			// Verify the new key can uniquely identify the node
			const found = await queryByCanonicalKey(newKey);
			
			if (found.length === 1 && found[0].id === node.id) {
				report.matchingKeys++;
			} else if (found.length === 0) {
				// This is expected - new key doesn't exist yet
				report.matchingKeys++;
			} else {
				// Collision detected!
				report.mismatchedKeys++;
				report.sampleMismatches.push({
					originalId: node.id,
					newKey,
					collisions: found.map(n => n.id)
				});
			}
		} catch (error) {
			report.errors.push({
				nodeId: node.id,
				error: error.message
			});
		}
	}
	
	return report;
}
```

### Smoke Tests

**Test 1: Key Uniqueness**
- Verify no canonical key collisions in migrated data
- Check that each old node maps to exactly one new key

**Test 2: Referential Integrity**
- Verify all edges still point to valid nodes
- Check that sightings still connect correct field values

**Test 3: Query Equivalence**
- Run critical queries against both old and new formats
- Verify result sets are identical

**Test 4: Performance Baseline**
- Measure query performance before and after
- Ensure no significant regression (target: <10% variance)

**Test 5: Round-Trip Consistency**
- Ingest test data → query → verify returned matches ingested
- Validate with both old and new key formats

### Automated Test Script

```typescript
interface SmokeTestResults {
	keyUniqueness: { passed: boolean, details: string };
	referentialIntegrity: { passed: boolean, details: string };
	queryEquivalence: { passed: boolean, details: string };
	performanceBaseline: { passed: boolean, details: string };
	roundTripConsistency: { passed: boolean, details: string };
}

async function runSmokeTests(): Promise<SmokeTestResults> {
	const results: SmokeTestResults = {
		keyUniqueness: await testKeyUniqueness(),
		referentialIntegrity: await testReferentialIntegrity(),
		queryEquivalence: await testQueryEquivalence(),
		performanceBaseline: await testPerformanceBaseline(),
		roundTripConsistency: await testRoundTripConsistency()
	};
	
	const allPassed = Object.values(results).every(r => r.passed);
	
	if (!allPassed) {
		throw new Error('Smoke tests failed. Migration cannot proceed.');
	}
	
	return results;
}
```

## Rollback Procedures

### Scenario 1: Rollback During Shadow Write

**Trigger**: High error rate or performance degradation detected

**Steps**:
1. Stop backfill job (set status to 'paused')
2. Deploy application code without new key writes
3. Optionally: Remove `canonical_key` and `canonical_key_version` properties from migrated nodes
4. No data loss; safe rollback

### Scenario 2: Rollback During Dual Read

**Trigger**: Query inconsistencies or application errors

**Steps**:
1. Deploy application code that only reads old format
2. Continue backfill if error was application-side only
3. Or: Pause backfill and remove new properties if data issue

### Scenario 3: Rollback After Deprecation

**Trigger**: Critical issue discovered post-migration

**Steps**:
1. If old properties still exist: Deploy code to read from them
2. If old properties deleted: Restore from backup and re-run previous version
3. Most complex scenario - avoid by thorough validation before cleanup phase

## Monitoring and Observability

### Key Metrics

1. **Migration Progress**:
   - Nodes processed per hour
   - Estimated time to completion
   - Error rate (errors per 1000 nodes)

2. **System Health**:
   - Query latency (p50, p95, p99)
   - Database CPU and memory usage
   - Connection pool saturation

3. **Data Quality**:
   - Canonical key collision count
   - Orphaned edge count
   - Duplicate node detection

### Alerting Thresholds

- Error rate > 1%: Pause migration, investigate
- Query latency increase > 50%: Review indexing
- Key collisions detected: CRITICAL - halt migration

## Cost and Resource Estimation

### Storage Impact

- **Shadow Property Migration**: +3-5% during migration, +0% after cleanup
- **Node Duplication**: +100% during migration, +0% after cleanup

### Performance Impact

- **Backfill CPU**: 10-20% sustained during migration (rate-limited)
- **Query Overhead**: <5% during dual-read phase, 0% after

### Engineering Time

- **Implementation**: 2-3 engineer-weeks
- **Testing**: 1 engineer-week
- **Deployment & Monitoring**: 1 engineer-week (spread over 8 weeks)

## Appendix A: Example Transformations

### Example 1: Row Key Versioning

**Before (v1 - implicit)**:
```cypher
MERGE (r:rows {dump: 'breach_2024', row_key: '12345'})
```

**After (v2 - explicit hash)**:
```cypher
MERGE (r:rows {
	dump: 'breach_2024',
	row_key: '12345',
	canonical_key_version: 'v2',
	canonical_key: 'sha256:abc123...'
})
```

### Example 2: Field Value Normalization

**Before (v1)**:
```cypher
MERGE (fv:field_value {
	value: 'User@Example.COM  ',
	field: 'email',
	row_key: '12345',
	dump: 'breach_2024'
})
```

**After (v2 - normalized value)**:
```cypher
MERGE (fv:field_value {
	value: 'user@example.com',  // normalized
	value_raw: 'User@Example.COM  ',  // preserved for provenance
	field: 'email',
	row_key: '12345',
	dump: 'breach_2024',
	canonical_key_version: 'v2',
	canonical_key: 'sha256:def456...'
})
```

## Appendix B: References

- **AGE Documentation**: [Apache AGE GitHub](https://github.com/apache/age)
- **Graph Migration Patterns**: [Neo4j Migration Guide](https://neo4j.com/docs/migration-guide/)
- **Heimdall Architecture**: `docs/design/Architecture.md`
- **Heimdall Data Model**: `docs/design/DataModel.md`
- **Related Issue**: ARCH-001 - Architecture Review (#31)

## Document History

| Version | Date       | Author  | Changes                    |
|---------|------------|---------|----------------------------|
| 1.0     | 2025-12-09 | Copilot | Initial migration plan     |

## Approval and Sign-Off

- [ ] Architecture Review: _Pending_
- [ ] Security Review: _Pending_
- [ ] Operations Review: _Pending_
- [ ] Engineering Lead: _Pending_

---

**Last Updated**: 2025-12-09  
**Document Owner**: Engineering Team  
**Next Review Date**: TBD (after architecture review)
