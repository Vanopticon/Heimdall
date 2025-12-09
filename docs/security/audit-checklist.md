# Security Audit Checklist — Heimdall

## Purpose

This document defines a comprehensive security audit checklist for Heimdall, covering TLS enforcement, OIDC token validation, PII controls, and other security-critical requirements. Each item includes verification steps and remediation guidance.

## Audit Areas

### 1. TLS Configuration and Enforcement

#### 1.1 TLS Version Enforcement

**Requirement**: All network communication must use TLS 1.3 exclusively.

**Verification Steps**:

- Review `src/tls_utils.rs` `build_server_config_tls13()` to confirm protocol version restriction
- Run `tests/integration_tls.rs` to validate TLS configuration
- Scan server configuration to ensure no fallback to TLS 1.2 or earlier

**Current Status**: ✅ Implemented

**Evidence**:

- `src/tls_utils.rs` lines 128-132 enforce TLS 1.3 only via `.with_protocol_versions(&[&rustls::version::TLS13])`
- Test: `tests/integration_tls.rs`

**Remediation**: N/A — already enforced

#### 1.2 Self-Signed Certificate Rejection

**Requirement**: Server must reject self-signed certificates to prevent use of untrusted certificates.

**Verification Steps**:

- Confirm `build_server_config_tls13()` validates certificate chain
- Run test case that attempts to use self-signed certificate
- Verify server startup fails with appropriate error message

**Current Status**: ✅ Implemented

**Evidence**:

- `src/tls_utils.rs` lines 122-125 explicitly reject self-signed certificates
- Test: `tests/integration_tls.rs::integration_tls_rejects_self_signed_cert`

**Remediation**: N/A — already enforced

#### 1.3 Certificate Validation

**Requirement**: Certificates must be valid (not expired, proper chain of trust).

**Verification Steps**:

- Review certificate loading and parsing logic
- Confirm expiration checking via `is_cert_expired()` function
- Validate that expired certificates are rejected at startup

**Current Status**: ⚠️ Partially Implemented

**Evidence**:

- `src/tls_utils.rs` includes `is_cert_expired()` helper (lines 99-107)
- Function is available but not currently called during server startup

**Remediation**:

- Add expiration check in server initialization before starting TLS listener
- Add integration test for expired certificate rejection
- Document certificate rotation procedures

#### 1.4 Client Certificate Verification (Mutual TLS)

**Requirement**: For machine-to-machine communication, consider mutual TLS authentication.

**Verification Steps**:

- Review server configuration for client certificate requirements
- Confirm whether client certificates are required for specific endpoints
- Validate client certificate verification logic

**Current Status**: ❌ Not Implemented

**Evidence**:

- `src/tls_utils.rs` line 135 uses `.with_no_client_auth()` — client certificates not required

**Remediation**:

- Evaluate requirement for mutual TLS for sync agent peer-to-peer communication
- If required, implement client certificate verification for peer sync endpoints
- Add configuration option to enable/require client certificates
- Create tests for mutual TLS scenarios

### 2. OIDC/OAuth2 Token Validation

#### 2.1 OIDC Discovery and Configuration

**Requirement**: OIDC provider configuration must be loaded from discovery endpoint and validated.

**Verification Steps**:

- Verify `HMD_OAUTH_DISCOVERY_URL` is required and validated at startup
- Confirm discovery document is fetched and parsed
- Validate required OIDC endpoints are extracted (authorization, token, jwks_uri)

**Current Status**: ❌ Not Implemented

**Evidence**:

- Configuration mentions `HMD_OAUTH_DISCOVERY_URL` in `README.md` and `docs/design/Architecture.md`
- No implementation found in `src/` for OIDC discovery or token validation

**Remediation**:

- Implement OIDC discovery client to fetch provider configuration
- Add validation for discovery document structure
- Cache JWKS (JSON Web Key Set) for token signature verification
- Implement periodic JWKS refresh
- Add tests for discovery endpoint parsing and error handling

#### 2.2 JWT Signature Verification

**Requirement**: All bearer tokens must be validated using provider's public keys from JWKS.

**Verification Steps**:

- Confirm JWT signature verification using RS256 or other supported algorithms
- Validate token signature against JWKS public keys
- Verify key rotation handling

**Current Status**: ❌ Not Implemented

**Evidence**:

- No JWT validation logic found in codebase
- No dependency on JWT libraries (e.g., `jsonwebtoken`)

**Remediation**:

- Add `jsonwebtoken` crate dependency
- Implement middleware to extract and validate bearer tokens
- Create `src/auth/` module with token validation logic
- Add comprehensive tests for valid/invalid/expired tokens
- Test signature verification with multiple keys

#### 2.3 Token Claims Validation

**Requirement**: Validate required claims (`iss`, `aud`, `exp`, `iat`, `sub`) and reject invalid tokens.

**Verification Steps**:

- Confirm issuer (`iss`) matches expected OIDC provider
- Validate audience (`aud`) matches application client ID
- Check expiration (`exp`) and reject expired tokens
- Validate issued-at (`iat`) is reasonable
- Extract subject (`sub`) for audit logging

**Current Status**: ❌ Not Implemented

**Evidence**:

- No claims validation logic in codebase

**Remediation**:

- Implement claims validation in token validation middleware
- Configure expected issuer and audience from environment variables
- Add clock skew tolerance for `exp` and `iat` validation
- Add tests for each claim validation scenario
- Document required token claims in API documentation

#### 2.4 RBAC and Authorization

**Requirement**: Token claims must be used to enforce role-based access control (RBAC).

**Verification Steps**:

- Confirm roles/scopes are extracted from token claims
- Validate endpoint-level authorization checks
- Test that unauthorized requests are rejected with 403 status

**Current Status**: ❌ Not Implemented

**Evidence**:

- No authorization middleware found in `src/lib.rs` or handler code

**Remediation**:

- Design role/permission model based on OIDC claims (e.g., `roles`, `groups`, `scope`)
- Implement authorization middleware that checks claims against endpoint requirements
- Add role checks to sensitive endpoints (ingest, query, admin operations)
- Create tests for authorized and unauthorized access attempts
- Document permission model and claim mappings

#### 2.5 Token Introspection and Revocation

**Requirement**: Support token introspection to detect revoked tokens.

**Verification Steps**:

- Confirm whether token introspection endpoint is used
- Validate revocation checking for sensitive operations
- Test behavior with revoked tokens

**Current Status**: ❌ Not Implemented

**Evidence**:

- No token introspection logic found

**Remediation**:

- Evaluate need for active token introspection (vs. relying on expiration)
- If required, implement introspection endpoint calls
- Consider caching introspection results to avoid excessive calls
- Add configuration for introspection endpoint
- Test with revoked and active tokens

### 3. PII Protection and Field-Level Encryption

#### 3.1 PII Policy Enforcement

**Requirement**: PII fields must never be stored in plaintext; use scrub, one-way hash, or envelope encryption.

**Verification Steps**:

- Review PII policy configuration and implementation
- Confirm PII classification rules are applied during ingestion
- Test that plaintext PII is rejected or transformed before persistence

**Current Status**: ⚠️ Designed but Not Implemented

**Evidence**:

- PII policy described in `docs/design/Architecture.md` and `docs/design/features/SEC-001-PII-Policy-Field-Encryption.md`
- No implementation found in `src/ingest/` or `src/persist/`

**Remediation**:

- Implement PII policy engine in `src/pii/` or `src/policy/`
- Define field classification rules (e.g., email, SSN, credit card patterns)
- Add field transformation logic: scrub (mask), hash (SHA-256), encrypt (AES-GCM)
- Integrate PII policy into normalization pipeline before persistence
- Add tests verifying PII is never stored in plaintext
- Create field classification configuration file

#### 3.2 Envelope Encryption Implementation

**Requirement**: Two-way encrypted fields use envelope encryption with KMS-provided master key.

**Verification Steps**:

- Confirm envelope encryption implementation using vetted crypto library
- Validate data encryption keys (DEKs) are generated per-record
- Verify master key is sourced from environment or KMS
- Check encrypted field format includes: IV/nonce, auth tag, ciphertext

**Current Status**: ❌ Not Implemented

**Evidence**:

- Envelope encryption mentioned in design docs but not implemented
- No crypto dependencies for field-level encryption in `Cargo.toml`

**Remediation**:

- Add `ring` or `aes-gcm` crate for authenticated encryption
- Implement envelope encryption module:
  - Generate random DEK per record/batch
  - Encrypt data with DEK using AES-256-GCM
  - Encrypt DEK with master key
  - Store encrypted DEK + IV + tag + ciphertext
- Add master key loading from `HMD_MASTER_KEY` environment variable
- Implement KMS integration for production (Phase 2)
- Add comprehensive encryption/decryption tests
- Test key rotation scenarios

#### 3.3 No Plaintext PII in Database Backups

**Requirement**: Database backups must not contain plaintext PII.

**Verification Steps**:

- Create test database with encrypted PII fields
- Perform `pg_dump` backup
- Parse backup SQL and verify PII fields are encrypted
- Confirm no plaintext sensitive data in dump

**Current Status**: ⚠️ Testable Once Encryption Implemented

**Evidence**:

- PII encryption not yet implemented, so backups would contain plaintext if present

**Remediation**:

- Implement PII encryption first (see 3.1 and 3.2)
- Create test: `tests/security_pii_backup_protection.rs`
  - Insert test records with PII
  - Execute `pg_dump` via test
  - Parse dump SQL
  - Assert PII fields are ciphertext, not plaintext
  - Verify no plaintext patterns (SSN, email, etc.)
- Add to CI test suite
- Document backup encryption procedures

#### 3.4 Audit Logging for Decrypt Operations

**Requirement**: All PII decryption operations must be logged with actor, reason, timestamp.

**Verification Steps**:

- Review audit logging implementation
- Confirm decrypt operations emit structured log entries
- Validate logs include: actor ID (OIDC `sub`), operation, timestamp, request ID, field accessed
- Test that audit logs are tamper-evident

**Current Status**: ⚠️ Infrastructure Exists, Decrypt Not Implemented

**Evidence**:

- Structured logging mentioned in `docs/design/Architecture.md`
- No decrypt operations exist yet to audit

**Remediation**:

- Implement audit logging module: `src/audit/mod.rs`
- Define audit event schema for decrypt operations:
  - `event_type`: "pii_decrypt"
  - `actor`: OIDC subject or service account ID
  - `timestamp`: ISO 8601
  - `request_id`: correlation ID
  - `field`: encrypted field identifier
  - `reason`: optional justification or access reason
- Integrate audit logging into decrypt function
- Emit structured JSON logs to stdout
- Add test: verify decrypt operation creates audit log entry
- Consider write audit logs to append-only store (S3, audit DB)

### 4. Secure Configuration Management

#### 4.1 Secrets Management

**Requirement**: Secrets must not be stored in source code, configuration files, or logs.

**Verification Steps**:

- Review codebase for hardcoded secrets (grep for common patterns)
- Confirm environment variable usage for secrets
- Validate secrets are not logged (use log sanitization/redaction)
- Check `.gitignore` includes `.env` and other secret files

**Current Status**: ✅ Mostly Implemented

**Evidence**:

- Secrets loaded from environment variables (see `README.md`)
- `.gitignore` includes `.env`
- Configuration module uses `HMD_*` prefixed variables

**Remediation**:

- Add log sanitization to redact sensitive values (tokens, keys, passwords)
- Review logging statements to ensure no secrets are logged
- Add pre-commit hook to detect potential secrets in code
- Document secrets management best practices
- Consider secrets scanning tool (e.g., `trufflehog`, `git-secrets`)

#### 4.2 Configuration Validation

**Requirement**: Configuration must be validated at startup; invalid configuration should prevent server start.

**Verification Steps**:

- Review configuration loading in `src/config/mod.rs`
- Confirm required variables cause startup failure if missing
- Validate format checking (URLs, file paths, numeric ranges)
- Test server startup with invalid configuration

**Current Status**: ⚠️ Partially Implemented

**Evidence**:

- Configuration module exists at `src/config/mod.rs`
- Some validation present but comprehensive validation needed

**Remediation**:

- Add comprehensive configuration validation:
  - Required field checks
  - Format validation (URL schemes, file existence)
  - Range checking for numeric values
  - Mutual exclusivity checks
- Return clear error messages for configuration issues
- Add startup tests with invalid configurations
- Document all configuration options and validation rules

### 5. Database Security

#### 5.1 SQL Injection Prevention

**Requirement**: All database queries must use parameterized queries or strict input validation.

**Verification Steps**:

- Review AGE client query construction
- Confirm no string concatenation of user input into queries
- Validate input sanitization for Cypher queries
- Test with malicious input patterns

**Current Status**: ⚠️ Needs Hardening

**Evidence**:

- `src/age_client.rs` constructs Cypher queries with some validation
- Comment in `docs/design/Architecture.md` (line 106) notes need to harden AGE client

**Remediation**:

- Review all query construction in `src/age_client.rs`
- Implement parameterized query support or strict input validation:
  - Whitelist allowed characters in identifiers
  - JSON-serialize string values
  - Validate numeric values
  - Escape special characters
- Add SQL injection test suite with OWASP test vectors
- Consider prepared statement support if available
- Document secure query construction patterns

#### 5.2 Database Access Control

**Requirement**: Database user must have minimum required privileges.

**Verification Steps**:

- Review database user permissions
- Confirm user cannot access system tables unnecessarily
- Validate row-level security policies if applicable
- Test unauthorized database operations

**Current Status**: ⚠️ Deployment Dependent

**Evidence**:

- No explicit user privilege management in codebase
- Deployment documentation needed

**Remediation**:

- Document required database permissions
- Create least-privilege database user for application
- Add DDL scripts for user/role creation
- Implement row-level security policies if needed
- Add deployment checklist for database security
- Test with restricted database user

#### 5.3 Connection Security

**Requirement**: Database connections must use TLS encryption.

**Verification Steps**:

- Review database connection string configuration
- Confirm TLS/SSL mode is enforced
- Validate certificate verification
- Test connection without TLS fails

**Current Status**: ⚠️ Configuration Dependent

**Evidence**:

- `Cargo.toml` uses `sqlx` with `runtime-tokio-native-tls`
- TLS enforcement depends on connection string configuration

**Remediation**:

- Enforce `sslmode=require` or `sslmode=verify-full` in connection string
- Add validation that rejects non-TLS database connections
- Document database TLS configuration requirements
- Add test for TLS connection enforcement
- Consider mutual TLS for database connections

### 6. Observability and Incident Response

#### 6.1 Security Event Logging

**Requirement**: Security-relevant events must be logged in structured format.

**Verification Steps**:

- Review log entries for authentication failures
- Confirm authorization denials are logged
- Validate suspicious activity detection (rate limiting, repeated failures)
- Check logs include correlation IDs for tracing

**Current Status**: ⚠️ Partially Implemented

**Evidence**:

- Structured JSON logging mentioned in design
- Specific security event logging not verified

**Remediation**:

- Define security event taxonomy:
  - Authentication failures
  - Authorization denials
  - Invalid tokens
  - Suspicious patterns
  - Configuration errors
  - PII access/decrypt events
- Implement security event logging
- Add correlation IDs to all requests
- Test log output format and content
- Document log analysis procedures

#### 6.2 Metrics and Monitoring

**Requirement**: Security metrics must be exposed for monitoring and alerting.

**Verification Steps**:

- Review Prometheus metrics endpoint
- Confirm security metrics are tracked:
  - Authentication success/failure rates
  - Authorization denials
  - TLS handshake errors
  - Token validation failures
- Validate metrics can trigger alerts

**Current Status**: ⚠️ Partially Implemented

**Evidence**:

- Prometheus metrics mentioned in design
- Security-specific metrics need definition

**Remediation**:

- Define security metrics:
  - `auth_attempts_total{result="success|failure"}`
  - `authz_denials_total{endpoint, reason}`
  - `tls_errors_total{type}`
  - `token_validation_errors_total{reason}`
  - `pii_decrypt_operations_total{actor}`
- Implement metrics collection in auth middleware
- Add metrics endpoint tests
- Document alerting thresholds
- Create sample Prometheus alert rules

#### 6.3 Incident Response Readiness

**Requirement**: Incident response procedures must be documented and tested.

**Verification Steps**:

- Review incident response documentation
- Confirm contact information and escalation paths
- Validate breach notification procedures
- Test incident response drills

**Current Status**: ❌ Not Implemented

**Evidence**:

- No incident response documentation found

**Remediation**:

- Create `docs/security/incident-response.md`
- Document:
  - Security contact information
  - Escalation procedures
  - Breach notification requirements
  - Log collection and preservation
  - Forensics procedures
  - Recovery procedures
- Schedule annual incident response drill
- Review SECURITY.md for vulnerability reporting

### 7. Sync Agent Security

#### 7.1 Peer Authentication

**Requirement**: Sync agent peer-to-peer communication must use mutual TLS with OIDC tokens.

**Verification Steps**:

- Review sync agent authentication implementation
- Confirm mutual TLS is used for peer connections
- Validate OIDC tokens are exchanged and verified
- Test unauthorized peer rejection

**Current Status**: ❌ Not Implemented

**Evidence**:

- Sync agent mentioned in design but not implemented
- No `src/sync/` module found

**Remediation**:

- Design sync agent authentication architecture
- Implement mutual TLS for peer connections
- Add OIDC token exchange for peer authorization
- Create peer identity verification logic
- Add comprehensive sync security tests
- Document peer onboarding procedures

#### 7.2 Change Log Integrity

**Requirement**: Change log entries must be tamper-evident and authenticated.

**Verification Steps**:

- Review change log format and storage
- Confirm entries include cryptographic signatures or MACs
- Validate integrity checking on replication
- Test detection of tampered entries

**Current Status**: ❌ Not Implemented

**Evidence**:

- Change log replication mentioned in design
- No implementation found

**Remediation**:

- Design change log integrity mechanism:
  - HMAC signatures per entry
  - Hash chain for ordering
  - Actor signatures for non-repudiation
- Implement integrity verification
- Add tests for tamper detection
- Document change log format and verification

### 8. Build and Deployment Security

#### 8.1 Dependency Scanning

**Requirement**: Dependencies must be scanned for known vulnerabilities.

**Verification Steps**:

- Run `cargo audit` to check for vulnerable dependencies
- Review Dependabot or similar automated scanning
- Confirm vulnerable dependencies are updated promptly
- Check CI pipeline includes security scanning

**Current Status**: ⚠️ Manual Process

**Evidence**:

- No automated security scanning in CI found

**Remediation**:

- Add `cargo-audit` to CI pipeline
- Configure Dependabot for Rust dependencies
- Define SLA for security update application
- Add security scanning to PR checks
- Document vulnerability management process

#### 8.2 Container Security (if applicable)

**Requirement**: Container images must use minimal base images and non-root users.

**Verification Steps**:

- Review Dockerfiles for base image selection
- Confirm application runs as non-root user
- Validate no unnecessary packages installed
- Scan images with vulnerability scanner

**Current Status**: ⚠️ Dev Only

**Evidence**:

- `docker/postgres-age/Dockerfile` exists for dev database
- No production application Dockerfile found

**Remediation**:

- Create production Dockerfile:
  - Use distroless or minimal base image
  - Multi-stage build for smaller image
  - Run as non-root user
  - Copy only required binaries
- Add container scanning to CI
- Document container security best practices
- Create security-hardened image variants

#### 8.3 Secure Defaults

**Requirement**: Default configuration must be secure; insecure options require explicit opt-in.

**Verification Steps**:

- Review default configuration values
- Confirm secure defaults (TLS required, auth enabled, strict validation)
- Validate no debug/development features in production builds
- Test default configuration security posture

**Current Status**: ⚠️ Needs Review

**Evidence**:

- Configuration defaults not fully documented

**Remediation**:

- Document all default configuration values
- Ensure secure defaults:
  - TLS required (no plaintext fallback)
  - Authentication required (no bypass)
  - Strict validation enabled
  - Rate limiting enabled
  - Audit logging enabled
- Add configuration security checklist
- Test default configuration against security requirements

## Summary of Findings

### Current Security Posture

**Strengths**:

- TLS 1.3 enforcement implemented ✅
- Self-signed certificate rejection implemented ✅
- Environment-based configuration for secrets ✅
- Structured logging architecture ✅

**Critical Gaps** (Priority 1 - Block Production):

- ❌ OIDC/OAuth2 token validation not implemented
- ❌ PII field-level encryption not implemented
- ❌ No authentication/authorization on API endpoints
- ❌ SQL injection hardening needed in AGE client

**Important Gaps** (Priority 2 - Address Before Production):

- ⚠️ Certificate expiration checking not enforced at startup
- ⚠️ Audit logging for sensitive operations incomplete
- ⚠️ No security event metrics/monitoring
- ⚠️ Configuration validation incomplete

**Enhancement Opportunities** (Priority 3 - Post-Production):

- 🔄 Mutual TLS for machine-to-machine communication
- 🔄 Token introspection and revocation
- 🔄 Incident response procedures
- 🔄 Automated dependency scanning in CI
- 🔄 Sync agent security (when implemented)

### Recommended Remediation Order

1. **Authentication & Authorization** (Weeks 1-2)
   - Implement OIDC token validation
   - Add authentication middleware
   - Implement RBAC checks
   - Create comprehensive auth tests

2. **PII Protection** (Weeks 3-4)
   - Implement PII policy engine
   - Add envelope encryption
   - Create backup protection tests
   - Implement decrypt audit logging

3. **Security Hardening** (Week 5)
   - Harden AGE client against injection
   - Add certificate expiration checks
   - Implement configuration validation
   - Add security event logging

4. **Monitoring & Operations** (Week 6)
   - Add security metrics
   - Create monitoring dashboards
   - Document incident response
   - Implement automated scanning

## Compliance Considerations

### Data Protection Regulations

- **GDPR**: PII handling, right to erasure, audit logging
- **CCPA**: Data access, deletion, disclosure tracking
- **HIPAA** (if applicable): PHI protection, access controls, audit trails

### Security Standards

- **OWASP Top 10**: Injection, auth, sensitive data exposure, access control
- **NIST Cybersecurity Framework**: Identify, Protect, Detect, Respond, Recover
- **SOC 2 Type II**: Access controls, encryption, logging, monitoring

## Appendix: Testing Checklist

### Manual Testing

- [ ] TLS 1.3 connection with `openssl s_client`
- [ ] Self-signed certificate rejection
- [ ] Invalid token rejection
- [ ] Expired token rejection
- [ ] RBAC enforcement for each endpoint
- [ ] PII field encryption verification
- [ ] Database backup PII check
- [ ] Audit log completeness
- [ ] Configuration validation errors
- [ ] SQL injection attempts

### Automated Testing

- [ ] `tests/integration_tls.rs` — TLS enforcement
- [ ] `tests/security_auth.rs` — Authentication/authorization (to be created)
- [ ] `tests/security_pii_backup_protection.rs` — PII in backups (to be created)
- [ ] `tests/security_audit_logging.rs` — Decrypt audit logs (to be created)
- [ ] `tests/security_injection.rs` — SQL injection vectors (to be created)
- [ ] `cargo audit` — Dependency vulnerabilities
- [ ] Static analysis (clippy with security lints)

## Maintenance

This checklist should be reviewed and updated:

- Quarterly for general updates
- After significant feature additions
- Following security incidents
- When regulatory requirements change
- During annual security audits

**Last Updated**: 2025-12-09
**Next Review**: 2026-03-09
**Owner**: Security Team / Engineering Lead
