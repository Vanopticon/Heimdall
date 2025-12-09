# Implementation Roadmap — Heimdall

This roadmap breaks the work into milestones and suggested sprint-sized deliverables. Each milestone includes acceptance criteria and test expectations.

## Milestone 0 — Foundation

Timeline: 1–2 sprints

Deliverables:

- Project scaffolding and core configuration module (env-first).
- Streaming ingest API endpoint with minimal CSV/JSON/NDJSON parser.
- Graph schema for core entities: `dump`, `field`, `field_value`, `sighting`, `entity` and provenance metadata implemented in PostgreSQL+Apache AGE (graph schema) and `pgvector` for vector embeddings.
- Unit tests for parser and normalizers.

Acceptance:

- Able to ingest a small NDJSON dump and observe canonical nodes in the Postgres+AGE graph.

## Milestone 1 — Normalization & PII

Timeline: 1–2 sprints

Deliverables:

- Canonicalizers for IP, domain, and common hash formats.
- PII policy engine and envelope encryption implementation (env-injected master key).
- Tests ensuring PII is never stored plaintext.

Acceptance:

- Upload a test dump containing PII; verify stored values are hashed/encrypted or scrubbed.

## Milestone 2 — Enrichment Framework

Timeline: 1–2 sprints

Deliverables:

- Enricher worker framework and adapter interface.
- Provider configuration schema (rate-limits, credentials).
- Sample adapters (geoip, ASN).
- Integration tests with mocked providers.

Acceptance:

- Enrichment nodes/edges appear and are linked with source records.

## Milestone 3 — Sync & Multi-instance

Timeline: 2–3 sprints

Deliverables:

- Append-only change-log and per-record version vectors.
- Sync agent (push/pull) with TLS 1.3 and OIDC-backed control plane authentication.
- Merge resolver with configurable rules per entity type.
- Tests simulating partition and reconciliation.

Acceptance:

- Two Heimdall instances reconcile changes after simulated network partitions.

## Milestone 4 — Observability & Harden

Timeline: 1–2 sprints

Deliverables:

- Structured logging, Prometheus metrics, and OpenTelemetry traces.
- Expanded test coverage (overflow, malformed input, cert errors).
- Operational docs: backup/restore, TLS bootstrapping, key rotation playbook.

Acceptance:

- Metrics and traces available; e2e tests pass.

## Milestone 5 — KMS/Vault Integration & Production Prep

Timeline: 3 sprints

### Milestone 5a — KMS Infrastructure Setup

Timeline: 1 sprint

Deliverables:

- Deploy HashiCorp Vault cluster (or configure cloud KMS provider: AWS KMS, GCP Cloud KMS, or Azure Key Vault).
- Create master encryption key (`heimdall-pii-v1`).
- Configure authentication backend (AppRole for VMs, Kubernetes auth for containers, or cloud IAM).
- Set up Vault policies for application roles (encrypt/decrypt) and admin roles (key rotation).
- Test connectivity and basic encrypt/decrypt operations.
- Document Vault/KMS deployment and configuration.

Owner: DevOps Lead

Acceptance:

- Vault/KMS operational and accessible from Heimdall dev environment.
- Health checks passing; authentication configured.
- Can successfully encrypt and decrypt test data via Vault/KMS.

### Milestone 5b — Envelope Encryption Code Integration

Timeline: 1–2 sprints

Deliverables:

- Add KMS client dependencies to `Cargo.toml` (`vaultrs` for Vault, `aws-sdk-kms`, `google-cloudkms`, or `azure_security_keyvault`).
- Implement `KeyManagementService` trait in `src/crypto/kms.rs` with Vault and cloud KMS implementations.
- Implement envelope encryption helpers in `src/crypto/envelope.rs` (using `aes-gcm` or `xchacha20poly1305`).
- Update `src/config/mod.rs` to include KMS configuration (provider, endpoint, key ID, auth method, encryption mode).
- Integrate KMS encryption into PII policy engine (`src/pii/` or ingest pipeline).
- Add backward-compatible decryption supporting both legacy (env-key) and KMS formats.
- Implement encryption context tracking for audit (field name, request ID, actor).
- Add unit tests for KMS clients and envelope encryption.
- Add integration tests encrypting/decrypting via Vault/KMS.

Owner: Security Lead, Backend Developer

Acceptance:

- CI tests pass for KMS integration.
- Application can encrypt PII fields using KMS.
- Application can decrypt both legacy (env-key) and KMS-encrypted fields (hybrid mode).
- Audit logs capture encryption/decryption events with actor and context.

### Milestone 5c — Data Migration & Re-encryption

Timeline: 1–2 sprints

Deliverables:

- Deploy application in hybrid mode (supports both legacy and KMS encryption).
- Implement re-encryption worker in `src/crypto/reencrypt.rs`:
	+ Query graph for encrypted fields with legacy encryption version.
	+ Decrypt with env-key, re-encrypt with KMS.
	+ Update field metadata (`encryption_version`, `key_identifier`).
	+ Track progress via Prometheus metrics.
- Add admin API endpoint to trigger and monitor re-encryption (`/admin/crypto/reencrypt`).
- Execute re-encryption job against staging/pre-production environment.
- Validate all encrypted data migrated to KMS.
- Switch to KMS-only mode (`HMD_ENCRYPTION_MODE=kms`).
- Remove env-key from configuration.

Owner: Backend Developer, DBA

Acceptance:

- All encrypted fields in graph store use KMS encryption.
- Legacy encryption code path removed or disabled.
- Application operates stably in KMS-only mode.
- Metrics confirm 100% migration completion.

### Milestone 5d — Key Rotation & Operational Hardening

Timeline: 1 sprint

Deliverables:

- Implement automated key rotation (90-day cadence via cron job or Kubernetes CronJob).
- Create operational runbooks (see `docs/design/KMS-Vault-Plan.md`):
	+ Routine key rotation procedure.
	+ Emergency key compromise response.
	+ Re-encryption after migration.
- Configure Vault audit logging or CloudTrail for KMS (depending on provider).
- Set up monitoring and alerting for KMS operations (latency, errors, rate limits).
- Add audit log export to centralized logging system.
- Document KMS integration in README and deployment guides.
- Conduct tabletop exercise for key compromise scenario.
- Update `SECURITY.md` with key management details.

Owner: Tech Writer, Security Lead, DevOps Lead

Acceptance:

- Key rotation automation tested and operational.
- Runbooks validated via tabletop exercise.
- Comprehensive audit logs available for all key operations.
- Documentation reviewed and approved by stakeholders.
- Compliance artifacts prepared (PCI DSS, GDPR, SOC 2).

### Milestone 5e — Production Deployment

Timeline: 0.5 sprint

Deliverables:

- Deployment runbook for Linux hosts (use Postgres+AGE sidecar or managed Postgres with AGE installed).
- Production KMS/Vault deployment with HA configuration.
- Production key creation and policy configuration.
- Blue-green or rolling deployment of KMS-integrated application.

Acceptance:

- Production deployment successful with zero downtime.
- All PII encryption using KMS.
- Monitoring and alerting operational.

## Cross-Cutting Requirements

- Testing: Unit + integration + e2e tests for all new features.
- Security: PII controls, TLS 1.3 enforcement, OIDC token validation.
- Observability: JSON logs, Prometheus metrics, traces.
- 100% Rust implementation; all CLI/output to `stdout`/`stderr`.

## Architecture Review — ARCH-001

Timeline: 0.5 sprint

Deliverables:

- Add system diagram and component mapping to `docs/design/Architecture.md`.
- Create `docs/design/features/ARCH-001-Architecture-Review.feature.json` and accompanying markdown summary.
- Define `persist_row` / `persist_dump` API contract and a plan to implement `src/persistence.rs`.
- Refactor `src/age_client.rs::merge_entity` to a safe parameterized approach.
- Add integration tests for ingest → persistence → query flows using the `docker/` dev DB.

Acceptance:

- Architecture doc updated and reviewed.
- Feature JSON + markdown exist and map to implementation tasks.
- Integration tests execute against local dev DB in CI or developer environment.
