# Feature: SEC-001 -> PII Policy & Field-level Encryption

## Related Items

- See: `docs/design/Architecture.md`, `docs/design/Implementation-Roadmap.md`

## Story

As a security lead, I need Heimdall to never store PII in plaintext, using scrub or one-/two-way encryption.

## Overview

- PII policy engine: per-field rule (scrub / hash / encrypt).
- Two-way encrypted fields stored as ciphertext plus metadata (nonce, key-id).
- Master key provided via environment for envelope encryption; KMS/Vault planned later.
- Audit trail for any decrypt operations (actor, reason, timestamp).

## Out of Scope

- Full external KMS integration (planned for Milestone 5).

## Implementation Steps

1. Define policy config and storage schema for encrypted fields.
2. Implement envelope encryption helpers using a vetted Rust crypto crate (discuss selection before implementation).
3. Integrate into ingest pipeline.
4. Add tests ensuring plaintext never written to DB.

## Acceptance Criteria

- PII fields are not present in plaintext in backups of the graph store (Postgres+AGE); decryption is auditable and requires explicit key availability.
