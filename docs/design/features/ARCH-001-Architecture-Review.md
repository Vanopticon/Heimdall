# ARCH-001 — Architecture Review

Summary
-------

This feature card captures the results of an architecture review performed against the current codebase and design docs. It includes a system diagram, a mapping from components to repository paths, short-term actionable recommendations, and acceptance criteria to guide implementation work.

Decisions
---------

- Keep PostgreSQL + Apache AGE + `pgvector` as the canonical store for graph + vector embeddings.
- Implement persistence helpers (`persist_row` / `persist_dump`) to centralize MERGE/upsert logic and co-occurrence batching.
- Harden the `AgeClient` to avoid direct Cypher string composition; use validated parameterization/preamble patterns.

Immediate Next Steps
--------------------

1. Implement `src/persistence.rs` with `persist_row` and batched co-occurrence upserts.
2. Refactor `src/age_client.rs::merge_entity` to a safer, parameterized implementation.
3. Add integration tests that run ingest → persist → query flows against the `docker/` dev DB.
4. Draft a KMS/Vault integration plan for envelope encryption and add it to the roadmap.

Acceptance Criteria
-------------------

- The architecture doc includes a mermaid diagram and component mapping.
- The feature JSON `ARCH-001-Architecture-Review.feature.json` exists and describes steps and acceptance criteria.
- The roadmap contains tasks and test expectations for the recommended implementation work.
- A memory entry `/memories/Feature-ARCH-Architecture-Review.md` documents decisions and owners.

Security & Observability Notes
------------------------------

- Parameterize or validate queries to avoid injection risks when interacting with AGE/Cypher.
- Ensure logs include correlation ids (request id, dump id) for traceability across ingest and enrichment flows.

Migration Plan (high level)
--------------------------

If canonical key or row-hash semantics change, provide a backfill/migration plan that can re-ingest historical dumps or run a transformation job to reconcile nodes.
