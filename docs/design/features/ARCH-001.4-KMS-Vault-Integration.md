# Feature: ARCH-001.4 -> KMS/Vault Integration for Envelope Encryption

## Related Items

- Parent Issue: ARCH-001 (Vanopticon/Heimdall#31)
- See: `docs/design/KMS-Vault-Plan.md`, `docs/design/Architecture.md`, `docs/design/Implementation-Roadmap.md`
- Related: `docs/design/features/SEC-001-PII-Policy-Field-Encryption.md`

## Story

As a security lead, I need to replace environment-injected master keys with a centrally-managed KMS or HashiCorp Vault solution to enable key rotation, comprehensive audit, and meet compliance requirements for production deployments.

## Overview

This feature implements a complete Key Management Service (KMS) integration for Heimdall's envelope encryption system. The solution replaces static environment variables with dynamic key management, automated rotation, and comprehensive audit capabilities.

### Key Components

- **Provider Selection**: HashiCorp Vault (primary) with cloud KMS alternatives (AWS KMS, GCP Cloud KMS, Azure Key Vault)
- **Envelope Encryption**: Data encryption keys (DEK) generated/encrypted by master encryption key (MEK) in KMS
- **Key Rotation**: Automated 90-day rotation with background re-encryption
- **Migration Strategy**: Backward-compatible hybrid mode during transition from env-keys to KMS
- **Audit Trail**: Comprehensive logging of all encryption, decryption, and key lifecycle events

## Implementation Approach

### Phase 1: Infrastructure (Milestone 5a)

1. Deploy Vault cluster or configure cloud KMS
2. Create master encryption key
3. Configure authentication (AppRole, Kubernetes auth, or cloud IAM)
4. Test connectivity and basic operations

### Phase 2: Code Integration (Milestone 5b)

1. Implement `KeyManagementService` trait abstraction
2. Add Vault and cloud KMS client implementations
3. Integrate envelope encryption into PII policy engine
4. Support hybrid mode (legacy + KMS) for backward compatibility
5. Add comprehensive unit and integration tests

### Phase 3: Migration (Milestone 5c)

1. Deploy hybrid-mode application
2. Implement and run re-encryption worker
3. Migrate all existing encrypted data to KMS
4. Validate migration completion
5. Switch to KMS-only mode

### Phase 4: Hardening (Milestone 5d)

1. Implement automated key rotation
2. Create operational runbooks
3. Configure audit logging and monitoring
4. Conduct tabletop exercises for emergency scenarios

## Out of Scope

- Multi-KMS provider support in single deployment (single provider per deployment)
- Hardware Security Module (HSM) integration (relies on KMS provider's HSM backend)
- Client-side key caching (all operations go through KMS for strongest audit trail)

## Acceptance Criteria

1. **KMS Integration**: Application can encrypt/decrypt PII using Vault or cloud KMS instead of env-keys
2. **Backward Compatibility**: Can decrypt existing env-key encrypted data during migration
3. **Migration Complete**: All encrypted data re-encrypted with KMS; env-keys removed from config
4. **Key Rotation**: Automated 90-day key rotation with re-encryption working in test environment
5. **Audit Trail**: All encryption/decryption operations logged with actor, timestamp, and context
6. **Emergency Response**: Runbook for key compromise validated via tabletop exercise
7. **Documentation**: Comprehensive design document, runbooks, and updated architecture docs complete
8. **Compliance**: Meets PCI DSS 3.5/3.6, GDPR Article 32, SOC 2 key management controls

## Security Considerations

- Master keys never leave KMS/Vault HSM
- Least-privilege access via role-based policies (encrypt, decrypt, admin)
- Encryption metadata includes key version for rotation support
- DEKs zeroed from memory immediately after use (`secrecy` + `zeroize` crates)
- Comprehensive audit log for compliance and incident response

## Testing Requirements

- Unit tests for KMS clients and envelope encryption helpers
- Integration tests with Vault/KMS in Docker or cloud test environment
- Migration test: encrypt with env-key, migrate to KMS, decrypt successfully
- Key rotation test: rotate key, re-encrypt data, verify old and new keys work
- Failure scenarios: KMS unavailable, authentication failure, rate limiting
- Performance tests: encrypt/decrypt throughput, latency measurements

## Documentation Requirements

- Design document: `docs/design/KMS-Vault-Plan.md` (comprehensive provider evaluation, architecture, migration plan)
- Runbooks: Routine rotation, emergency compromise, migration procedures
- Configuration guide: KMS setup for different providers and deployment scenarios
- Update: `docs/design/Architecture.md`, `docs/design/Implementation-Roadmap.md`
- API documentation: `KeyManagementService` trait and implementations

## Operational Runbooks

See `docs/design/KMS-Vault-Plan.md` for detailed runbooks:

1. **Routine Key Rotation**: Every 90 days, automated or manual trigger
2. **Emergency Key Compromise Response**: P1 incident, <4 hour rotation SLA
3. **Re-encryption After Migration**: One-time migration from env-keys to KMS

## Dependencies

- Rust crates: `vaultrs` (Vault client), `aws-sdk-kms`, `google-cloudkms`, or `azure_security_keyvault`
- Crypto crates: `aes-gcm` or `xchacha20poly1305`, `secrecy`, `zeroize`
- Infrastructure: Vault cluster or cloud KMS access
- Configuration: Authentication credentials (Vault token, cloud IAM roles, service accounts)

## Rollback Plan

- Hybrid mode allows reverting to env-key encryption if KMS issues arise
- Keep legacy decryption code path during transition period (30 days post-migration)
- Database backups before migration
- Configuration flag to switch modes: `HMD_ENCRYPTION_MODE=legacy|kms|hybrid`

## Metrics & Monitoring

- `heimdall_crypto_encrypt_total{provider, key_version}`: Total encryption operations
- `heimdall_crypto_decrypt_total{provider, key_version}`: Total decryption operations
- `heimdall_crypto_operation_duration_seconds{operation, provider}`: Operation latency
- `heimdall_crypto_errors_total{operation, provider, error_type}`: Error counts
- `heimdall_reencryption_progress{status}`: Re-encryption job progress (total, completed, failed)

## Success Criteria

- Zero PII fields encrypted with env-keys in production
- All key operations auditable with actor, timestamp, and reason
- Key rotation completes without service disruption
- Compliance artifacts available for audit
- Operational team trained on runbooks and confident in procedures

---

**Status**: Design Complete (Implementation Pending)
**Priority**: High (Security & Compliance)
**Estimated Effort**: 3 sprints (Milestones 5a-5d)
**Target Release**: v1.0.0
