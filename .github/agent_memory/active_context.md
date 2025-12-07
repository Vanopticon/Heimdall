```markdown
# Active Context

- Current focus: Implement streaming, memory-efficient ingest handlers for NDJSON and bulk dumps in `src/ingest/handler.rs`.
- Short-term plan:
  1. Implement chunked write for `bulk_dump_upload` (peek buffer + temp file) and add unit tests that simulate chunked requests.
  2. Once stable, convert `ndjson_upload` to line-by-line streaming normalization (use the same approach for consistency).
  3. Wire routes into the dev HTTP server and add an integration test posting a small NDJSON payload and asserting normalization/persistence.

- Constraints: Keep changes compatible with current `axum` version and avoid introducing heavy new dependencies unless necessary. Ensure tests remain fast by gating integration tests behind feature flags.

```
