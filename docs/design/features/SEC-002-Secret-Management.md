# SEC-002: Secret Management Integration & Key Rotation Playbook

## Category

Non-Functional

## Description

Operational guidance and initial integration points for secret management and key rotation for envelope encryption and service credentials. Provides comprehensive documentation for managing secrets across various backends and runbooks for rotating cryptographic keys.

## Requirements

1. Create operational documentation in `docs/ops/`:
	- `secrets.md` — Comprehensive guide to secret management with backend-specific configurations
	- `key-rotation.md` — Detailed playbook for rotating keys and credentials

2. Implement KMS integration module in `src/lib/server/kms/`:
	- Define `KMSProvider` interface for pluggable KMS backends
	- Implement local KMS provider for development/testing
	- Provide stub implementations for AWS KMS, GCP KMS, and HashiCorp Vault
	- Support envelope encryption pattern (DEK/KEK)
	- Include encryption context for additional authenticated data

3. Documentation must cover:
	- Recommended secret backends (Vault, AWS Secrets Manager, GCP Secret Manager, Kubernetes Secrets)
	- Sample configurations and environment variables
	- TLS certificate management and rotation
	- OAuth/OIDC credential rotation
	- Cookie secret rotation with multi-key support
	- Database credential rotation (manual and automated)
	- Data encryption key (DEK) and key encryption key (KEK) rotation

4. Key rotation procedures must include:
	- Planned rotation schedules
	- Emergency rotation procedures
	- Pre-rotation checklists
	- Step-by-step rotation procedures for each secret type
	- Verification steps
	- Rollback procedures
	- Automation guidance

## Acceptance Criteria

- `docs/ops/secrets.md` created with recommended backends and sample configurations
- `docs/ops/key-rotation.md` created with rotation procedures and verification steps
- Runbook covers both emergency and planned rotation scenarios
- `src/lib/server/kms/` module created with `KMSProvider` interface
- Local KMS provider fully implemented and functional
- Stub implementations provided for AWS KMS, GCP KMS, and Vault with implementation guides
- KMS module includes README with usage examples
- Documentation reviewed and accepted by repository owners
- Feature card created and linked in Implementation Roadmap

## Implementation Notes

### Secret Management (`docs/ops/secrets.md`)

- Document environment variable naming conventions (HMD_* prefix)
- Provide complete configuration examples for each backend
- Include security best practices and troubleshooting guides
- Cover migration from environment variables to secret managers
- Document envelope encryption for field-level data protection

### Key Rotation Playbook (`docs/ops/key-rotation.md`)

- Separate procedures for each credential type (TLS, OAuth, database, cookie, KEK)
- Include both manual and automated rotation approaches
- Provide verification steps and rollback procedures
- Include example commands for common operations
- Cover emergency rotation scenarios

### KMS Integration (`src/lib/server/kms/`)

- Interface supports encrypt, decrypt, generateDataKey, and healthCheck operations
- Local provider uses AES-256-GCM with proper IV and authentication tag handling
- Encryption context support for additional authenticated data (AAD)
- Clear separation between KEK (in KMS) and DEK (generated per-field)
- Comprehensive implementation guides in stub providers

## Related Files

- `src/lib/server/config/index.ts` — Configuration module for reading secrets
- `server/server.ts` — Current secret usage (TLS, OAuth, cookie secrets)
- `src/hooks.server.ts` — Cookie decryption implementation
- `docs/design/Implementation-Roadmap.md` — Milestone tracking

## Implementation Roadmap Reference

This feature is part of **Milestone 5: Security Hardening & Operations**.

Task 5.1: Secret management integration & key rotation playbook

## Testing

While this is primarily a documentation and infrastructure feature:

1. Local KMS provider should be tested for correctness:
	- Test encrypt/decrypt round-trip
	- Test generateDataKey produces valid keys
	- Test encryption context enforcement
	- Test healthCheck functionality

2. Documentation should be reviewed for:
	- Accuracy of commands and configuration examples
	- Completeness of procedures
	- Clarity for operators and security teams

3. No automated tests required for documentation files

## Security Considerations

- Never commit secrets to version control
- Use separate secrets per environment (dev, staging, prod)
- Implement least-privilege access for secret backends
- Enable audit logging for all secret access
- Rotate secrets regularly per documented procedures
- Use encryption context to bind encrypted data to specific usage
- Securely erase plaintext DEKs after use

## Future Enhancements

- Implement AWS KMS provider (requires @aws-sdk/client-kms)
- Implement GCP KMS provider (requires @google-cloud/kms)
- Implement Vault provider (requires node-vault)
- Add automated key rotation scripts and Kubernetes CronJobs
- Integrate KMS module with field-level encryption in ingestion pipeline
- Add metrics and monitoring for KMS operations
- Implement DEK caching for performance optimization
