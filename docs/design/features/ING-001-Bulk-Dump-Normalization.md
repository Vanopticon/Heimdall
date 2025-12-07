# Feature: ING-001 -> Bulk Dump Normalization & Idempotent Ingest

## Related Items

	+ Priority: Top
	+ See: `docs/design/Architecture.md`, `docs/design/Implementation-Roadmap.md`

## Story

As a Full-Stack operator, I want to upload large telemetry dumps so Heimdall normalizes and stores canonical graph entities without duplicating work.

## Overview

	+ Stream large files (multipart/form-data) and parse them incrementally to avoid memory spikes.
	+ Support multiple input types via parser adapters (CSV, TSV, Excel, JSON, NDJSON, vendor-specific).
	+ Normalize values (IP, domain, hash), generate canonical keys, and apply PII policy before persistence.
	+ Writes are idempotent: canonical key → idempotent merge-style write into the Postgres+AGE graph with provenance.

## Out of Scope

	+ UI upload form. Vendor-specific deep parsers beyond the initial adapters.

## Detailed Ingest Phases

These phases describe the end-to-end processing pipeline for a bulk dump upload.

	+ **1) Pre-ingest Validation & Format Detection**
		+ Detect container-level metadata (multipart boundaries, content-type) and upload hints.
		+ Sample the first N KB of the payload to sniff format: CSV/TSV (delimiter heuristics), Excel (XLS/XLSX magic bytes), NDJSON/JSON (line-delimited JSON detection), compressed archives (gzip, zip), or vendor-specific wrappers.
		+ If a format cannot be confidently detected, fall back to user-supplied hints or reject with an actionable error message.
		+ Extract or validate optional schema hints (field name mappings, declared encodings) provided via metadata.

	+ **2) Staging & Streaming Parsing**
		+ Accept uploads as a streaming multipart form to avoid buffering whole files in memory.
		+ Use format-specific streaming parsers (CSV/TSV tokenizers, XLSX row stream, NDJSON line reader) that emit record events.
		+ Persist a lightweight `dump` metadata record immediately (uploader, timestamp, declared format, sample stats) to track provenance.
		+ Apply backpressure and bounded buffering to maintain memory bounds.

	+ **3) Scrub & Normalize Phase (record-by-record)**
		+ Field mapping: map incoming fields to canonical field names using hints and a configurable mapping table.
		+ PII policy application: for each field check the configured PII policy and apply the chosen action (redact, one-way hash, format-preserving transform, or allow). Policies may depend on provider, field, and deployment settings.
		+ Normalizers: canonicalize values for known types with deterministic algorithms:
			+ IP addresses: canonical IPv4/IPv6 forms; CIDR handling.
			+ Domains: lowercasing, IDNA normalization, strip trailing dots.
			+ Hashes: normalize hex case and algorithm length checks.
			+ Emails: local-part handling per policy (hash or redact), domain canonicalization.
			+ Timestamps: parse to UTC and store ISO-8601 canonical form.
		+ Canonical key generation: compute a stable canonical key for each canonical `field_value` (deterministic hash using stable salt/versioning) to enable idempotent merges.
		+ Local deduplication within the dump: optionally dedupe identical canonical keys before persistence to reduce writes.

	+ **4) Enrichment (optional, async-safe)**
		+ Enrich normalized values with non-PII context: ASN for IPs, geolocation, known blocklist flags, or vendor metadata. Enrichments either attach as separate nodes or as transient attributes, preserving original value provenance.
		+ Enrichment can be deferred and applied after initial merge to keep ingest latency predictable.

	+ **5) Cross-Dump Correlation & Idempotent Persistence**
		+ Use canonical keys to MERGE (or upsert) `entity` / `field_value` nodes in Postgres+AGE. Implement batched, transactional operations to reduce lock contention.
		+ Create a `dump` node and `sighting` relationships that link the dump, the `field_value`, and any `entity` nodes; store provenance details (original raw snippet, position, uploader ID, ingestion run id) in the sighting.
		+ Ensure operations are idempotent: re-processing the same dump (or overlapping dumps) should not create duplicate canonical nodes or duplicate sightings for identical provenance.

	+ **6) Post-ingest Checks, Metrics & Notification**
		+ Run post-ingest validation to assert no plaintext-prohibited PII persisted (sweep of newly-created nodes for policy violations).
		+ Emit metrics: record count, unique canonical values created, memory/CPU footprints, ingest duration, and any enrichment latencies.
		+ Provide ingest report back to uploader: accepted rows, dropped rows, normalization errors, and a provenance summary.

## Implementation Steps

	+ 1. Define canonical schema for `dump`, `field`, `field_value`, `sighting`, and `entity`.
	+ 2. Implement streaming upload endpoint with incremental parsing and **explicit format-detection**.
			+ Add parsing adapters for CSV, TSV, Excel (XLSX row stream), JSON/NDJSON, and compressed archives.
			+ Implement format-sniffing logic and user-hint overrides.
	+ 3. Implement staging and record-streaming infrastructure with backpressure and bounded buffers.
	+ 4. Implement normalizers, canonical-key generation, and local deduplication.
	+ 5. Integrate the PII policy enforcement module (redact/hash/transform actions) and add a pre/post persistence validation sweep.
	+ 6. Implement enrichment hooks that can run synchronously or asynchronously and preserve provenance.
	+ 7. Implement idempotent merge operations for the Postgres+AGE graph (MERGE-style Cypher or upserts) with batching and retries.
	+ 8. Add unit + integration tests for format detection, streaming parsing, normalization, PII enforcement, and idempotent writes (including malformed and large inputs).
	+ 9. Add observability: ingest metrics, memory footprints, and a human-readable ingest report.

## Acceptance Criteria

	+ Detect and correctly classify the upload format for CSV, TSV, Excel, NDJSON, and common compressed archives; provide meaningful errors when detection fails.
	+ Ingest a 100k-line NDJSON (or equivalent CSV) file without exceeding expected memory bounds and produce canonical nodes in Postgres+AGE.
	+ PII policy enforced: no plaintext PII persists for fields marked as protected; transformed (hashed/encoded) values are present where expected.
	+ Idempotent behavior: re-ingesting the same dump or overlapping dumps does not create duplicate canonical nodes or duplicate sightings for the same provenance.
	+ Produce an ingest report summarizing accepted/dropped rows, normalization errors, and metrics (ingest duration, memory).

## Notes

	+ Canonical-key stability is critical: changes to canonical-key algorithms must be versioned and migration strategies documented.
	+ Enrichment that adds non-PII context should not leak PII; any enrichment that could reveal PII must be gated by policy checks.
