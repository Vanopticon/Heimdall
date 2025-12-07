# Feature: SYNC-001 -> Multi-Heimdall Continuous Synchronization

## Related Items

- See: `docs/design/Architecture.md`, `docs/design/Implementation-Roadmap.md`

## Story

As an operator, I want multiple Heimdall instances to synchronize continuously so data is available geographically and resilient.

## Overview

- Maintain an append-only change log for writes.
- Replicate change-logs to peers over TLS 1.3 with OIDC-secured control plane.
- Track per-record version vectors for conflict detection.
- Provide configurable merge rules per entity type (merge sightings, LWW metadata, tombstones for deletes).

## Out of Scope

- Full CRDT implementation — postponed until requirements demand it.

## Implementation Steps

1. Design change-log schema and per-record metadata (version vector, origin, tombstone).
2. Implement change-log writer and local storage.
3. Implement sync agent (push/pull) with TLS and OIDC auth.
4. Implement merge resolver and per-type rules.
5. Add tests for partitions and reconciliation.

## Acceptance Criteria

- Two Heimdall nodes reconcile differences after a simulated partition with consistent final state according to merge rules.
