# Feature: API-001 -> Configurable External API Caller & Enrichment Framework

## Related Items

- See: `docs/design/Architecture.md`, `docs/design/Implementation-Roadmap.md`

## Story

As an operator, I want to configure external enrichment providers and their credentials so Heimdall can augment records.

## Overview

- Worker pool that reads pending items and runs enrichment tasks.
- Provider adapter interface with retry/backoff, rate-limiting and circuit-breaker.
- Results stored as separate enrichment nodes with provenance metadata.

## Out of Scope

- Paid-provider account setup UI or billing integrations.

## Implementation Steps

1. Define adapter interface and provider config format.
2. Implement scheduler and worker pool.
3. Add rate-limiter, backoff, and circuit-breaker.
4. Build sample adapters (ASN, geoip).
5. Add integration tests using mocked providers.

## Acceptance Criteria

- Enrichment results are linked to source records and respect per-provider rate-limits and retries.
