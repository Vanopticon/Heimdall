# ARCH-001.5: Migration and Backfill Plan for Canonical Key / Row Hash Versioning

## Category

Design / Operations

## Description

Create a comprehensive migration and backfill plan to handle future changes to canonical key or row-hash semantics in Heimdall's graph data model. The plan provides safe approaches for re-ingesting or transforming existing nodes and establishes a compatibility strategy to avoid breaking queries during schema evolution.

## Requirements

1. **Migration Plan Document**: Create `docs/design/Migration-Backfill-Plan.md` that outlines:
   - Current state analysis of key generation patterns
   - Multiple migration strategies with trade-offs
   - Canonical key version tagging approach
   - Incremental backfill job design with resume capability
   - Compatibility strategy for dual-read during migration

2. **Transformation Scripts/Pseudocode**: Provide implementation examples including:
   - Backfill job with batch processing and progress tracking
   - Canonical key computation functions
   - Dual-read query patterns for backward compatibility
   - Migration state management

3. **Validation Plan**: Include smoke tests and validation procedures:
   - Pre-migration validation on database snapshot
   - Key uniqueness checks
   - Referential integrity verification
   - Query equivalence tests
   - Performance baseline comparison

4. **Staged Rollout Strategy**: Document phased deployment approach:
   - Shadow write phase (new format alongside old)
   - Dual-read transition
   - Deprecation timeline
   - Cleanup procedures

5. **Rollback Procedures**: Define rollback steps for each migration phase

## Acceptance Criteria

- [ ] Migration plan document exists at `docs/design/Migration-Backfill-Plan.md`
- [ ] Document describes at least 3 migration strategies with trade-offs
- [ ] Canonical key versioning strategy defined (e.g., `canonical_key:v1`, `canonical_key:v2`)
- [ ] Backfill pseudocode shows incremental, resumable processing
- [ ] Smoke tests and validation procedures documented
- [ ] Staged rollout phases clearly defined with timelines
- [ ] Rollback procedures documented for each phase
- [ ] Feature card created in `docs/design/features/`
- [ ] Implementation Roadmap updated to reference this work

## Implementation Notes

### Current Key Patterns Identified

From `src/lib/server/ageClient.ts`:

1. **Row Keys**: `{dump, row_key}` composite
2. **Field Values**: `{value, field, row_key, dump}` composite uniqueness
3. **Sightings**: Exact string matching on `value` property

### Versioning Strategy

Recommended: **Shadow Property Migration**

- Add `canonical_key_version` and `canonical_key` properties
- Maintain backward compatibility during transition
- Enable gradual query migration
- Minimal storage overhead (<5%)

### Key Technical Decisions

1. **Version Tagging**: Use `canonical_key_version: 'v2'` property
2. **Key Format**: SHA-256 hash of normalized composite key (`sha256:abc123...`)
3. **Batch Size**: 1000 nodes per batch (tunable based on performance)
4. **Error Threshold**: 1% error rate triggers migration pause
5. **Progress Tracking**: Dedicated `heimdall_migrations` table

### Transformation Example

```typescript
// Version 1 (current)
{value: 'user@example.com', field: 'email', row_key: '12345', dump: 'breach_2024'}

// Version 2 (with versioning)
{
  value: 'user@example.com',
  field: 'email',
  row_key: '12345',
  dump: 'breach_2024',
  canonical_key_version: 'v2',
  canonical_key: 'sha256:abcd1234...'
}
```

## Related Files

- `docs/design/Migration-Backfill-Plan.md` — Main migration document (created)
- `src/lib/server/ageClient.ts` — Current key generation code
- `sql/v1/001-create_graph.sql` — Graph schema definition
- `docs/design/DataModel.md` — Data model documentation
- `docs/design/Implementation-Roadmap.md` — Project milestones

## Related Issues

- Part of ARCH-001 (#31) - Architecture Review
- Addresses task: "Create a migration/backfill plan for changing canonical_key/row_hash semantics"

## Smoke Tests Plan

1. **Key Uniqueness Test**: Verify no canonical key collisions
2. **Referential Integrity Test**: Verify all edges remain valid
3. **Query Equivalence Test**: Compare results from old vs new format
4. **Performance Baseline Test**: Ensure <10% query latency variance
5. **Round-Trip Test**: Ingest → query → verify consistency

## Monitoring Requirements

- Migration progress tracking (nodes/hour, ETA)
- Error rate monitoring (pause at >1%)
- Query latency metrics (p50, p95, p99)
- Key collision detection (critical alert)
- Database resource utilization

## Estimated Timeline

- **Phase 1 (Preparation)**: 1-2 weeks
- **Phase 2 (Shadow Write)**: 2 weeks
- **Phase 3 (Dual Read)**: 2 weeks
- **Phase 4 (Deprecation)**: 2 weeks
- **Phase 5 (Cleanup)**: 1+ weeks

**Total**: 8-9 weeks for full migration lifecycle

## Next Steps

1. Architecture review of migration plan
2. Security review for data handling
3. Operations review for deployment strategy
4. Implement backfill script (future task)
5. Create monitoring dashboards (future task)
6. Execute dry-run on development environment (future task)
