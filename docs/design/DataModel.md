# Data Model — Canonical Graph (high level)

This document extends the canonical graph model with structural row/cell semantics and co-occurrence modeling required to
reconstruct dumps as tables and to count when values are observed together (co-occurrence). It is intentionally provenance-first
— every observed cell is recorded with its source `Dump` and `Row` context so the original structure can be reconstructed.

Core concepts

- `Dump` — a single uploaded file/ingestion run. Key properties: `id`, `uploader`, `received_at`, `format`, optional `fingerprint`/`summary`.
- `Row` — a row in a `Dump`. Key properties: `dump_id`, `index` (row number, starting at 0), optional `row_hash` (canonical hash of the row's canonical values).
- `Sighting` (a.k.a. Cell) — a single observed cell in a `Row` (field value occurrence). Properties: `id`, `column` (canonical field name), `raw`, `timestamp`, and optionally `position`.
- `Field` — canonical field name (e.g. `email`, `password`, `ip`, `domain`).
- `FieldValue` — canonicalized value instance (normalized form) with stable `canonical_key` used for idempotent MERGE operations.
- `Entity` — higher-level, deduplicated object (e.g., an aggregated host or persona) composed from `FieldValue`s.
- `RowHash` — a lightweight node representing the canonical hash of a row (optional alternative to storing `row_hash` on `Row`).

Auxiliary / enrichment nodes

- `IP`, `ASN`, `Geo` — enrichment nodes linked from `FieldValue` or `Entity` nodes.
- `NPI_Category` — classification node for privacy-sensitive categories; linked from `FieldValue` or `Sighting` metadata.

Primary relationships and their intent

- `(Dump)-[:HAS_ROW]->(Row)` — retains the ordered structure of the original dump.
- `(Row)-[:HAS_SIGHTING]->(Sighting)` — row contains zero-or-more cell sightings.
- `(Sighting)-[:OBSERVED_VALUE]->(FieldValue)` — the canonical value associated with this sighting (many sightings can point to the same canonical value).
- `(FieldValue)-[:VALUE_OF]->(Field)` — ties a canonical value to a field type.
- `(Entity)-[:HAS_VALUE]->(FieldValue)` — composes an entity from canonical values.

Co-occurrence & credential relationships

- `(FieldValue)-[:CO_OCCURS {count, last_seen}]->(FieldValue)` — aggregated, undirected (or canonical-directed) relationship that counts how many times two canonical values are observed together in the same `Row` (co-occurrence). Maintain a deterministic ordering (e.g. `a.canonical_key < b.canonical_key`) when creating these relationships so they are unique.
- `(FieldValue)-[:CREDENTIAL {count, last_seen, samples}]->(FieldValue)` — typed relationship for sensitive field-pairs such as (`email` -> `password`). Incremented when a particular email/password pair is seen in the same row.

Why separate `Row` and `Sighting`?

This design keeps the canonical `FieldValue` node de-duplicated (global identity for a normalized value) while preserving the exact structural occurrence (`Row` and `Sighting`) so you can:

- Reconstruct the original dump (iterate `Dump` → `Row` ordered by `index` → `Sighting` → `raw` / `FieldValue`).
- Compute co-occurrence counts by iterating the `Sighting`s within each `Row` and incrementing pairwise counts on `FieldValue` edges.
- Compute row-level fingerprints (`row_hash`) deterministically from canonical keys so you can find exact or near-duplicates of entire rows or dumps.

Persistence patterns (example Cypher pseudocode)

Persist a single row and its cells (safe to run inside a transaction):

```cypher
-- Inputs: $dump_id, $row_index, $row_hash, $cells = [ {column, raw, canonical_key}, ... ]

MERGE (d:Dump {id: $dump_id})
ON CREATE SET d.received_at = $now

CREATE (r:Row {dump_id: $dump_id, index: $row_index, row_hash: $row_hash})
CREATE (d)-[:HAS_ROW]->(r)

UNWIND $cells AS cell
 MERGE (fv:FieldValue {canonical_key: cell.canonical_key})
	 ON CREATE SET fv.value = cell.canonical_value, fv.created_at = $now
 MERGE (f:Field {name: cell.column})
 MERGE (fv)-[:VALUE_OF]->(f)
 CREATE (s:Sighting {column: cell.column, raw: cell.raw, timestamp: $now})
 CREATE (r)-[:HAS_SIGHTING]->(s)
 CREATE (s)-[:OBSERVED_VALUE]->(fv)

-- After creating all sightings for the row, increment co-occurrence counts for every pair in this row.
-- Implementation note: perform pairwise increments in application code or a stored procedure for efficiency.
```

Increment / upsert co-occurrence between two canonical values (use deterministic ordering to avoid duplicates):

```cypher
-- Inputs: $a_key, $b_key, $now
MERGE (a:FieldValue {canonical_key: $a_key})
MERGE (b:FieldValue {canonical_key: $b_key})
MERGE (a)-[co:CO_OCCURS]-(b)
SET co.count = coalesce(co.count, 0) + 1,
		co.last_seen = $now
```

Credential edge (email -> password) upsert pattern:

```cypher
MERGE (email:FieldValue {canonical_key: $email_key})
MERGE (pwd:FieldValue {canonical_key: $pwd_key})
MERGE (email)-[c:CREDENTIAL]->(pwd)
SET c.count = coalesce(c.count,0) + 1,
		c.last_seen = $now
```

Reconstructing a `Dump` as a table (rows ordered by index):

```cypher
MATCH (d:Dump {id: $dump_id})-[:HAS_ROW]->(r:Row)
OPTIONAL MATCH (r)-[:HAS_SIGHTING]->(s:Sighting)-[:OBSERVED_VALUE]->(fv:FieldValue)
RETURN r.index AS row_index, collect({column: s.column, raw: s.raw, canonical: fv.canonical_key}) AS cells
ORDER BY r.index
```

Detecting repeated or similar dumps (row-hash / coverage approach)

1. Compute canonical `row_hash` for each row when ingesting (sort the set of canonical keys for the row and hash the concatenation with a stable salt/version).
2. Record the row-hashes for the `Dump` (option A: keep on `Row.row_hash`; Option B: `MERGE (rh:RowHash {hash: $row_hash}) MERGE (d)-[:HAS_ROWHASH]->(rh)`).
3. To find previous dumps that share rows with this ingress, run:

```cypher
-- Inputs: $row_hashes (list), $total_rows (int)
MATCH (rh:RowHash)<-[:HAS_ROWHASH]-(d:Dump)
WHERE rh.hash IN $row_hashes
WITH d, collect(DISTINCT rh.hash) AS matched_hashes
RETURN d.id, size(matched_hashes) AS matched_rows
ORDER BY matched_rows DESC
```

Use `matched_rows / $total_rows` to compute coverage and determine how much of the dump has been seen before. For an exact-duplicate detection you need matched_rows == $total_rows and the dump you compare against has the same number of unique row hashes.

Novelty and variation metrics

- **Row novelty**: fraction of row hashes in this `Dump` that are not present in any prior `RowHash` node.
- **Value novelty**: fraction of `FieldValue` canonical keys in this `Dump` that are new (no node previously existed with that `canonical_key`).
- **Variations per email**: number of distinct password `FieldValue` nodes linked by `CREDENTIAL` from the email `FieldValue`. Query example:

```cypher
MATCH (email:FieldValue {canonical_key: $email_key})-[c:CREDENTIAL]->(pwd:FieldValue)
RETURN count(DISTINCT pwd) AS password_variations, collect({pwd: pwd.canonical_key, count: c.count}) AS samples
```

Operational considerations / performance

-- Incrementing pairwise co-occurrence counts can be expensive for wide rows (O(n^2) pairs). Consider:
	+ limiting pairwise edges to configured field-pairs (e.g., only between credential-related columns),
	+ batching co-occurrence increments and upserting them in bulk, or
	+ using a sharded/approximate counting store for very large streams and materializing final counts periodically.

- Use deterministic ordering when creating undirected pair relationships (e.g., only create `(a)-[:CO_OCCURS]->(b)` when `a.canonical_key < b.canonical_key`).
- Indexes: create indexes on `:FieldValue(canonical_key)`, `:Dump(id)`, and `:Row(dump_id, index)`; consider an index on `:RowHash(hash)` if using `RowHash` nodes.

Next steps for implementation

1. Add a `persist_row` helper that consumes a row (list of canonicalized cell objects) and: creates `Row` + `Sighting`s + connects to `FieldValue`s, computes `row_hash`, and optionally `MERGE`s `RowHash` nodes.
2. After row persistence, efficiently increment co-occurrence relationships for pairs of `FieldValue` canonical keys observed in the row (respecting configured pair restrictions).
3. Add `CREDENTIAL` updates for configured field pairs (e.g., `email` + `password`) during row persistence.
4. Add integration tests: ingestion → persisted structure → reconstruct dump; co-occurrence counts increment correctly; novelty metrics computed as expected.

---

This document provides the canonical shape and pragmatic Cypher patterns to implement the structural and co-occurrence features you requested. If you want, I can now:

- implement `persist_row` / `persist_dump` helpers in `src/age_client.rs` and/or a new `src/persistence` module,
- add integration tests that run against the dev Postgres+AGE stack,
- or produce a branch/PR with the implementation changes.

"""
