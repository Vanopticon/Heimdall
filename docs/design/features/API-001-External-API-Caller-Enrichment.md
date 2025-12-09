# API-001: External API Caller Enrichment

## Category

Functional

## Description

Implement an enrichment framework that provides adapter trait, provider configuration, scheduler, and worker pool to process enrichment tasks. The framework enables asynchronous enrichment of ingested data with external metadata (ASN lookups, geolocation, etc.) while respecting rate limits and providing resilience through retry logic and circuit breakers.

## Requirements

1. **Adapter Interface**: Define a common interface/trait for enrichment adapters that can fetch data from external APIs
   - Standard method signatures for enrichment operations
   - Support for both synchronous and asynchronous enrichment
   - Type-safe input/output contracts

2. **Provider Configuration**: Support per-provider configuration including:
   - API endpoint URLs and authentication credentials
   - Rate limiting parameters (requests per second, burst limits)
   - Retry policies (max attempts, backoff strategy)
   - Circuit breaker thresholds
   - Concurrency limits

3. **Worker Pool**: Implement a worker pool that:
   - Manages concurrent enrichment tasks
   - Respects per-provider concurrency limits
   - Handles task queuing and distribution
   - Provides graceful shutdown and error handling

4. **Scheduler**: Implement a scheduler that:
   - Picks pending enrichment items from the graph
   - Dispatches tasks to appropriate workers based on enrichment type
   - Tracks enrichment status and provenance
   - Handles failures and retries

5. **Resilience Features**:
   - Rate limiting with token bucket algorithm
   - Exponential backoff for retries
   - Circuit breaker to prevent cascading failures
   - Timeout handling for slow providers

6. **Sample Adapters**: Provide example implementations:
   - ASN lookup adapter
   - GeoIP lookup adapter

## Acceptance Criteria

- Enrichment results stored as distinct nodes with provenance and linked to source records
- Worker pool respects per-provider configuration (rate limits, concurrency)
- Circuit breaker prevents repeated calls to failing providers
- Retry logic with exponential backoff handles transient failures
- Integration tests with mocked providers validate adapter behavior
- Unit tests cover rate limiter, circuit breaker, and worker pool logic

## Implementation Notes

- Use TypeScript interfaces for adapter trait
- Store provider configurations in a type-safe manner
- Implement rate limiting using token bucket algorithm
- Use Promise-based async patterns for worker pool
- Store enrichment results as graph nodes with `ENRICHED_BY` edges to source data
- Include timestamps and provenance metadata on enrichment nodes

## Related Files

- `src/lib/server/enrich/` — Enrichment module implementation
- `src/lib/server/ageClient.ts` — Graph client for storing enrichment results
- `sql/v1/001-create_graph.sql` — Schema for enrichment nodes and edges
