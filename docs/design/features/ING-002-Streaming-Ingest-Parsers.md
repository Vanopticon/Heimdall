# Feature: ING-002 -> Streaming Ingest Endpoint & Parsers (CSV/NDJSON/Excel)

## Related Items

- Priority: High (Milestone 0)
- See: `docs/design/Implementation-Roadmap.md`, `docs/design/features/ING-001-Bulk-Dump-Normalization.md`

## Story

As a Full-Stack operator, I want to upload large telemetry dumps in various formats (CSV, NDJSON, Excel, compressed archives) so Heimdall can parse and normalize them without buffering entire files in memory.

## Overview

- Accept multipart form-data uploads with streaming processing to avoid memory spikes on large files.
- Support multiple input formats via streaming parser adapters: CSV, TSV, NDJSON, Excel (XLSX row streaming), and compressed archives (gzip, zip).
- Implement format detection/sniffing with user-provided hint overrides.
- Emit normalized record events for downstream processing without buffering entire datasets.

## Out of Scope

- UI upload form (server-side API only).
- Deep vendor-specific parsers beyond standard CSV/JSON/Excel formats.
- Real-time progress reporting during upload (basic completion status only).

## Implementation Steps

1. Add dependencies for Excel parsing (calamine crate) to Cargo.toml.
2. Create streaming parser adapters:
	- CSV/TSV streaming parser in `src/ingest/parsers/csv.rs`
	- NDJSON streaming parser (refactor existing code) in `src/ingest/parsers/ndjson.rs`
	- XLSX streaming parser in `src/ingest/parsers/xlsx.rs`
	- Compressed archive support (gzip, zip) in `src/ingest/parsers/compressed.rs`
3. Implement format detection logic in `src/ingest/format_detection.rs`:
	- Magic byte detection for binary formats
	- Heuristic analysis for text formats
	- User hint override mechanism
4. Implement streaming multipart upload endpoint in `src/ingest/handler.rs`:
	- Accept multipart/form-data with streaming
	- Detect format from content or user hints
	- Route to appropriate parser adapter
	- Apply backpressure and memory bounds
5. Add comprehensive tests:
	- Unit tests for each parser adapter
	- Format detection tests with sample files
	- Integration tests for multipart streaming endpoint
	- Memory boundary tests (100k+ line files)

## Acceptance Criteria

- Endpoint accepts multipart form-data uploads and streams content without buffering entire files.
- Format detection correctly identifies CSV, TSV, NDJSON, Excel (XLSX), gzip, and zip formats.
- Parser adapters emit normalized records incrementally for 100k+ line files.
- Memory usage remains bounded (no OOM) when processing large files under expected constraints.
- User-provided format hints override automatic detection when specified.
- Unit and integration tests pass for all supported formats and edge cases.
- Helpful error messages returned when format detection fails or parsing errors occur.

## Notes

- Streaming multipart parsing avoids the need to buffer entire uploads in memory.
- Excel (XLSX) files require row-by-row streaming to handle large spreadsheets.
- Format detection should be fast (sample first 64KB) and accurate.
- Parsers should handle malformed input gracefully with clear error messages.

## Security Considerations

- **Dependency Security**: Uses zip 2.3.0+ which addresses GHSA-9w5j-4mwv-2wj8 (path canonicalization vulnerability in archive extraction).
- **Memory Bounds**: All parsers implement streaming to prevent memory exhaustion attacks from large files.
- **Input Validation**: Format detection and parsing include size limits and validation to prevent malicious input.
- **XLSX/ZIP Limitation**: Current format detection treats XLSX files as generic ZIP archives. Users must provide "xlsx" format hint for Excel files to route to the correct parser.
