# KMS/Vault Integration Plan for Envelope Encryption

## Executive Summary

This document outlines Heimdall's operational plan to replace environment-injected master keys with a Key Management Service (KMS) or HashiCorp Vault-backed envelope encryption approach. The plan addresses key rotation, access control, audit requirements, and migration strategies for existing encrypted data.

**Target Milestone**: Milestone 5 (Production Readiness)

**Priority**: High (Security & Compliance)

## Background

### Current State

Heimdall currently uses envelope encryption with environment-provided master keys (via `HMD_*` environment variables) for PII field encryption. While this approach is operationally simple and suitable for early development, it has significant limitations for production deployments:

- **Key Lifecycle**: No automated key rotation or versioning
- **Access Control**: Keys are static and shared across all instances
- **Audit Trail**: No centralized audit of key usage
- **Compromise Recovery**: No established procedures for key compromise scenarios
- **Compliance**: Limited ability to demonstrate key management controls for regulatory requirements

### Requirements

- Replace static environment keys with centrally-managed encryption keys
- Support key rotation without service disruption
- Maintain backward compatibility with existing encrypted data during migration
- Provide comprehensive audit trail for all key operations
- Enable emergency key revocation and re-encryption
- Support multi-region and high-availability deployments

## KMS Provider Evaluation

### 1. AWS Key Management Service (KMS)

**Architecture Overview**: AWS-managed HSM-backed key service with envelope encryption support, key versioning, and tight IAM integration.

**Pros**:

- Native integration with AWS ecosystem (IAM, CloudTrail, CloudWatch)
- FIPS 140-2 Level 2 validated (Level 3 for CloudHSM backend option)
- Automatic key rotation (annual for AWS-managed keys)
- Regional availability with multi-region key support
- Mature API and SDK support (via AWS SDK for Rust: `aws-sdk-kms`)
- CloudTrail provides comprehensive audit logs by default
- Fine-grained access control via IAM policies and key policies
- Pay-per-use pricing model (API calls + key storage)
- Supports envelope encryption pattern natively with `GenerateDataKey` API
- Key usage limits are high (shared 10,000 RPS per account/region, increased on request)

**Cons**:

- AWS-specific; vendor lock-in for key infrastructure
- Requires AWS account and network connectivity to AWS APIs
- Cross-region key usage incurs latency
- Cost can scale with API call volume
- Limited to AWS regions (geographic coverage gaps in some jurisdictions)
- Key material cannot be exported (by design, but limits portability)

**Integration Pattern**:

```rust
// Envelope encryption flow with AWS KMS
// 1. Generate data encryption key (DEK) via KMS
let dek_response = kms_client
    .generate_data_key()
    .key_id("arn:aws:kms:us-west-2:123456789012:key/abc123")
    .key_spec(DataKeySpec::Aes256)
    .send()
    .await?;

// 2. Encrypt plaintext with DEK (AES-256-GCM)
let encrypted_data = encrypt_with_dek(&dek_response.plaintext, &plaintext)?;

// 3. Store encrypted DEK with ciphertext
store_encrypted_field(EncryptedField {
    ciphertext: encrypted_data,
    encrypted_dek: dek_response.ciphertext_blob,
    key_id: "arn:aws:kms:...",
    algorithm: "AES-256-GCM",
});

// Decryption flow
// 1. Retrieve encrypted DEK from storage
// 2. Decrypt DEK via KMS
let dek = kms_client
    .decrypt()
    .ciphertext_blob(encrypted_dek)
    .send()
    .await?;

// 3. Decrypt data with DEK
let plaintext = decrypt_with_dek(&dek.plaintext, &ciphertext)?;
```

**Best For**: Deployments primarily on AWS infrastructure with need for strong compliance posture and minimal operational overhead.

### 2. Google Cloud Key Management (Cloud KMS)

**Architecture Overview**: Google-managed key service with HSM backing, integrated with Google Cloud IAM and audit logging.

**Pros**:

- Native GCP integration (IAM, Cloud Audit Logs, Cloud Monitoring)
- FIPS 140-2 Level 3 validated (HSM backend)
- Automatic key rotation support (90-day default, configurable)
- Global and regional key support with automatic replication
- Strong integration with Google services (GKE, Cloud Run, etc.)
- Rust SDK available (`google-cloudkms`)
- Comprehensive audit via Cloud Audit Logs
- Fine-grained IAM permissions
- Supports external key management (EKMS) for hybrid scenarios

**Cons**:

- GCP-specific; vendor lock-in
- Requires Google Cloud account and network connectivity
- API rate limits (1000 QPS per key by default, can request increase)
- Cost scales with operations and key versions
- Less mature Rust SDK compared to AWS
- Geographic coverage varies by region

**Integration Pattern**: Similar to AWS KMS with `encrypt` and `decrypt` API calls, using Cloud KMS's envelope encryption with generated data keys.

**Best For**: Deployments on Google Cloud Platform with strong HSM requirements and global distribution needs.

### 3. Azure Key Vault

**Architecture Overview**: Microsoft Azure key management service with HSM support, integrated with Azure Active Directory.

**Pros**:

- Native Azure integration (AAD, Monitor, Security Center)
- FIPS 140-2 Level 2/3 validated (premium SKU with HSM)
- Managed key rotation with versioning
- Multi-region replication (Premium tier)
- REST API and SDK support (Azure SDK for Rust: `azure_security_keyvault`)
- Azure Monitor for audit logs
- RBAC via Azure AD
- Supports Azure Policy for compliance enforcement
- Network isolation via Private Link

**Cons**:

- Azure-specific; vendor lock-in
- Requires Azure subscription
- Premium tier required for HSM and replication
- API rate limits (varies by tier, typically 2000 requests/10s)
- Rust SDK is less mature than Azure SDKs for other languages
- Regional availability constraints

**Integration Pattern**: Use Key Vault cryptographic operations API for envelope encryption, similar pattern to AWS/GCP.

**Best For**: Deployments on Azure with enterprise Active Directory integration requirements.

### 4. HashiCorp Vault

**Architecture Overview**: Self-hosted or Vault Enterprise (HCP) secret and key management platform with flexible backends and authentication methods.

**Pros**:

- **Vendor-neutral**: Can run on any infrastructure (AWS, GCP, Azure, on-premises, Kubernetes)
- **Flexible authentication**: Kubernetes, AWS IAM, GCP IAM, Azure AD, LDAP, AppRole, etc.
- **Transit secrets engine**: Dedicated encryption-as-a-service with key versioning and rotation
- **Key derivation**: Can derive encryption keys from a master key with context/nonce
- **Comprehensive audit**: Audit log for all operations (file, syslog, socket)
- **Dynamic secrets**: Can generate short-lived credentials for databases, clouds, etc.
- **Policy-based access control**: Fine-grained HCL policies per path
- **No vendor lock-in**: Open source core, enterprise features available
- **Rust client**: `vaultrs` crate with good API coverage
- **On-premises option**: Full control over key material and infrastructure
- **Cost**: Open-source version is free; Enterprise pricing for advanced features

**Cons**:

- **Operational overhead**: Requires self-hosting, HA setup, backup/restore procedures
- **Infrastructure complexity**: Need to manage Vault cluster (consensus, storage backend, unsealing)
- **Performance**: Network hop to Vault cluster adds latency vs cloud-native KMS
- **Scaling**: High-throughput scenarios require performance tuning and potentially Vault Enterprise
- **No HSM by default**: HSM auto-unseal and entropy augmentation require Enterprise license
- **Team expertise**: Requires operational knowledge of Vault architecture and best practices
- **Managed option**: HCP Vault available but adds cost and some vendor dependency

**Integration Pattern**:

```rust
// Envelope encryption with Vault Transit Engine
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};
use vaultrs::transit;

// Initialize Vault client
let client = VaultClient::new(
    VaultClientSettingsBuilder::default()
        .address("https://vault.internal:8200")
        .token(std::env::var("VAULT_TOKEN")?)
        .build()?
)?;

// Encrypt data (Vault generates DEK internally)
let encrypted = transit::encrypt(
    &client,
    "transit",           // mount point
    "heimdall-pii",      // key name
    plaintext_base64,
).await?;

// Store ciphertext (includes version info)
// Format: vault:v1:base64-ciphertext
store_encrypted_field(EncryptedField {
    ciphertext: encrypted.ciphertext,
    key_name: "heimdall-pii",
    key_version: 1,
});

// Decrypt
let decrypted = transit::decrypt(
    &client,
    "transit",
    "heimdall-pii",
    ciphertext,
).await?;
```

**Alternative Pattern** (Vault generates data key, application encrypts):

```rust
// Generate data key
let dek = transit::generate_data_key(
    &client,
    "transit",
    "heimdall-master",
    transit::KeyType::Aes256Gcm96,
).await?;

// Encrypt locally with plaintext DEK
let encrypted_data = encrypt_aes_gcm(&dek.plaintext, &data)?;

// Store encrypted DEK (returned by Vault) with ciphertext
store_encrypted_field(EncryptedField {
    ciphertext: encrypted_data,
    encrypted_dek: dek.ciphertext,
    key_name: "heimdall-master",
});
```

**Best For**: Multi-cloud deployments, on-premises requirements, or organizations requiring vendor-neutral key management with flexibility.

### Recommendation

**Primary Recommendation: HashiCorp Vault** (with cloud KMS as alternative)

**Rationale**:

1. **Vendor Neutrality**: Heimdall is designed as a multi-cloud, potentially on-premises system. Vault provides maximum flexibility without cloud provider lock-in.

2. **Operational Fit**: The architecture document already references "KMS/Vault planned later," indicating Vault was considered from the start. Vault's policy model and audit capabilities align well with Heimdall's security-first design.

3. **Integration Flexibility**: Vault's authentication backends (Kubernetes service accounts, cloud IAM, AppRole) provide flexibility for different deployment scenarios.

4. **Cost Model**: For moderate to high throughput, self-hosted Vault (especially with HCP Vault for managed option) can be more cost-effective than per-API-call cloud KMS pricing.

5. **Audit & Compliance**: Vault's audit logging is comprehensive and can be directed to Heimdall's existing observability stack.

**Fallback Strategy**: For AWS-heavy deployments, AWS KMS can be used as a simpler alternative with lower operational overhead. Implement an abstraction layer (`KeyManagementService` trait) to allow switching between Vault and cloud KMS providers.

## Envelope Encryption Architecture

### Key Hierarchy

```
                    ┌─────────────────────────┐
                    │  Master Encryption Key  │
                    │   (MEK) in Vault/KMS    │
                    │   Never leaves KMS      │
                    └───────────┬─────────────┘
                                │
                    ┌───────────▼─────────────┐
                    │ Data Encryption Keys    │
                    │ (DEK) per field/record  │
                    │ Encrypted by MEK        │
                    └───────────┬─────────────┘
                                │
                    ┌───────────▼─────────────┐
                    │   Encrypted Plaintext   │
                    │  (ciphertext + metadata)│
                    └─────────────────────────┘
```

### Key Identifiers and Versioning

**Key Naming Convention**:

- **Vault**: `heimdall-pii-v{version}` (e.g., `heimdall-pii-v1`, `heimdall-pii-v2`)
- **AWS KMS**: Use key aliases (`alias/heimdall-pii`) with automatic versioning
- **Cloud KMS**: `projects/{project}/locations/{location}/keyRings/heimdall/cryptoKeys/pii` with version tracking

**Encrypted Field Storage Schema**:

```json
{
	"ciphertext": "base64-encoded-encrypted-data",
	"encrypted_dek": "base64-encoded-encrypted-data-key",
	"key_identifier": "heimdall-pii-v1",
	"key_version": 1,
	"algorithm": "AES-256-GCM",
	"encryption_timestamp": "2025-12-09T00:00:00Z"
}
```

**Storage in Apache AGE**:

Store encrypted fields as JSON properties on `FieldValue` or `Sighting` nodes:

```cypher
MERGE (fv:FieldValue {canonical_key: $key})
SET fv.encrypted_data = $encrypted_json,
    fv.is_encrypted = true,
    fv.key_version = $key_version
```

### Encryption Flow

1. **Ingest**: When PII field detected by policy engine
2. **Key Request**: Request data encryption key (DEK) from Vault/KMS
	- For Vault Transit: Vault generates and stores DEK internally, returns ciphertext
	- For cloud KMS: Service generates DEK, returns both plaintext and encrypted DEK
3. **Local Encryption**: Encrypt plaintext with DEK using AES-256-GCM
4. **Store**: Persist ciphertext + encrypted DEK + metadata to graph store
5. **Cleanup**: Zero and drop plaintext DEK from memory (use `secrecy` + `zeroize` crates)

### Decryption Flow

1. **Retrieve**: Fetch encrypted field from graph store
2. **Key Request**: Decrypt DEK using Vault/KMS
	- Submit encrypted DEK or ciphertext to service
	- Receive plaintext DEK
3. **Local Decryption**: Decrypt ciphertext with DEK
4. **Audit**: Log decryption event (actor, field identifier, timestamp, reason)
5. **Cleanup**: Zero and drop DEK from memory

### Key Rotation Strategy

**Rotation Cadence**:

- **Automatic Rotation**: Every 90 days (configurable)
- **Manual Rotation**: On-demand via admin API/CLI
- **Emergency Rotation**: Immediate on suspected compromise

**Rotation Procedure** (Zero-Downtime):

1. **Create New Key Version**:
	- Vault: `vault write transit/keys/heimdall-pii/rotate`
	- AWS KMS: Automatic with `EnableKeyRotation`
	- GCP KMS: `gcloud kms keys versions create`

2. **Dual-Key Period**:
	- Old key version (`v1`) remains active for decryption
	- New key version (`v2`) used for all new encryptions
	- Duration: Until all data re-encrypted or key retirement policy met

3. **Re-encryption** (Background Job):
	- Query graph for all encrypted fields with old key version
	- Decrypt with old key, re-encrypt with new key
	- Update `key_version` metadata
	- Track progress via metrics/dashboard

4. **Retire Old Key**:
	- After re-encryption complete or retention policy met
	- Mark key version as retired (decrypt-only, no new encryptions)
	- After retirement grace period, disable old key version

**Configuration Example**:

```json
{
	"key_rotation": {
		"enabled": true,
		"cadence_days": 90,
		"auto_reencrypt": true,
		"reencrypt_batch_size": 1000,
		"reencrypt_interval_seconds": 60,
		"grace_period_days": 30
	}
}
```

## Access Control Model

### Vault Policy Example

```hcl
# Policy for Heimdall application instances
path "transit/encrypt/heimdall-pii" {
	capabilities = ["update"]
}

path "transit/decrypt/heimdall-pii" {
	capabilities = ["update"]
}

path "transit/keys/heimdall-pii" {
	capabilities = ["read"]
}

# Deny direct access to key material
path "transit/export/*" {
	capabilities = ["deny"]
}

# Policy for enrichment workers (read-only decrypt)
path "transit/decrypt/heimdall-pii" {
	capabilities = ["update"]
}

# Admin policy for key rotation
path "transit/keys/heimdall-pii/rotate" {
	capabilities = ["update"]
}

path "transit/keys/heimdall-pii/config" {
	capabilities = ["read", "update"]
}
```

### AWS KMS Policy Example

```json
{
	"Version": "2012-10-17",
	"Statement": [
		{
			"Sid": "AllowHeimdallEncryptDecrypt",
			"Effect": "Allow",
			"Principal": {
				"AWS": "arn:aws:iam::123456789012:role/heimdall-app-role"
			},
			"Action": [
				"kms:Encrypt",
				"kms:Decrypt",
				"kms:GenerateDataKey",
				"kms:DescribeKey"
			],
			"Resource": "*"
		},
		{
			"Sid": "AllowAdminKeyRotation",
			"Effect": "Allow",
			"Principal": {
				"AWS": "arn:aws:iam::123456789012:role/heimdall-admin-role"
			},
			"Action": [
				"kms:EnableKeyRotation",
				"kms:CreateKey",
				"kms:CreateAlias"
			],
			"Resource": "*"
		}
	]
}
```

### Role-Based Access

| Role | Permissions | Use Case |
|------|-------------|----------|
| **heimdall-app** | encrypt, decrypt, read key metadata | Normal application operations |
| **heimdall-worker** | decrypt only | Enrichment workers (read-only access to encrypted data) |
| **heimdall-admin** | rotate keys, update policies, audit log access | Key lifecycle management |
| **heimdall-audit** | read audit logs only | Compliance and security review |

### Authentication Methods

**Vault**:

- **Kubernetes Auth**: For containerized deployments (service account tokens)
- **AppRole**: For VM-based deployments (role ID + secret ID)
- **Cloud IAM Auth**: For cloud-hosted Vault (AWS IAM, GCP IAM, Azure AD)

**AWS KMS**:

- **IAM Roles**: EC2 instance profiles, ECS task roles, EKS pod identities
- **STS Assume Role**: For cross-account access

**GCP Cloud KMS**:

- **Service Accounts**: Workload Identity for GKE, Compute Engine service accounts

## Threat Model

### Threats Mitigated

1. **Plaintext Key Exposure** (High):
	- **Threat**: Master keys in environment variables exposed via process listing, logs, or configuration dumps
	- **Mitigation**: Keys never leave KMS/Vault; only encrypted DEKs stored with data

2. **Insider Threat** (High):
	- **Threat**: Malicious insider with database access reads PII
	- **Mitigation**: Encrypted data requires both database and KMS/Vault access; audit trail for all decryption

3. **Key Rotation Gap** (Medium):
	- **Threat**: Compromised key not rotated; extended exposure window
	- **Mitigation**: Automated 90-day rotation; emergency rotation procedures

4. **Privilege Escalation** (Medium):
	- **Threat**: Attacker gains application credentials, accesses KMS
	- **Mitigation**: Least-privilege IAM/policies; network segmentation; audit alerts

5. **Database Backup Exposure** (High):
	- **Threat**: Stolen database backup contains usable PII
	- **Mitigation**: All PII encrypted with keys not present in backup; keys separate in KMS

### Residual Risks

1. **KMS/Vault Compromise** (Low but Critical):
	- **Risk**: If KMS/Vault fully compromised, all encrypted data at risk
	- **Mitigation**: HSM-backed keys (Vault Enterprise, cloud KMS Premium tiers); strict network isolation; comprehensive monitoring

2. **Memory Scraping** (Low):
	- **Risk**: Attacker with host access dumps process memory, extracts plaintext DEK
	- **Mitigation**: Use `secrecy` and `zeroize` crates; limit DEK lifetime in memory; consider hardware-based memory encryption (SGX, SEV)

3. **Supply Chain** (Low):
	- **Risk**: Compromised dependency exfiltrates keys or plaintext
	- **Mitigation**: Dependency auditing; minimal dependencies; regular security scans

4. **Side-Channel Attacks** (Very Low):
	- **Risk**: Timing or cache attacks extract key material
	- **Mitigation**: Use constant-time cryptographic implementations; HSM operations

## Migration Strategy

### Phase 1: Infrastructure Setup (Sprint 1)

**Deliverables**:

- Deploy Vault cluster (or configure cloud KMS)
- Create `heimdall-pii-v1` master key
- Configure authentication (AppRole, Kubernetes, or IAM)
- Test connectivity from dev environment

**Acceptance Criteria**:

- Vault/KMS accessible from Heimdall instances
- Authentication working (can obtain token/credentials)
- Encrypt/decrypt test succeeds

### Phase 2: Code Integration (Sprint 1-2)

**Deliverables**:

- Add Vault/KMS client crates to `Cargo.toml` (`vaultrs` or `aws-sdk-kms`)
- Implement `KeyManagementService` trait in `src/crypto/kms.rs`:

```rust
#[async_trait]
pub trait KeyManagementService: Send + Sync {
    async fn encrypt(&self, plaintext: &[u8], context: &EncryptionContext) -> Result<EncryptedData>;
    async fn decrypt(&self, encrypted: &EncryptedData, context: &EncryptionContext) -> Result<Vec<u8>>;
    async fn generate_data_key(&self, key_id: &str) -> Result<DataKey>;
    async fn rotate_key(&self, key_id: &str) -> Result<()>;
}

pub struct VaultKMS { /* ... */ }
pub struct AwsKMS { /* ... */ }
```

- Implement envelope encryption in `src/crypto/envelope.rs`
- Update PII policy engine to use KMS for encryption
- Add configuration for KMS endpoint, credentials, key identifiers

**Acceptance Criteria**:

- Unit tests pass for KMS client
- Integration test encrypts/decrypts via Vault/KMS
- PII fields encrypted using new path

### Phase 3: Backward Compatibility (Sprint 2)

**Deliverables**:

- Add `encryption_version` field to encrypted field metadata
- Implement decryption path that handles both old (env-key) and new (KMS) formats
- Migration flag in config: `HMD_ENCRYPTION_MODE=legacy|kms|hybrid`

**Decryption Logic**:

```rust
match field.encryption_version {
    EncryptionVersion::Legacy => decrypt_with_env_key(&field.ciphertext)?,
    EncryptionVersion::KmsV1 => kms.decrypt(&field.encrypted_data).await?,
}
```

**Acceptance Criteria**:

- Application can decrypt legacy env-key encrypted data
- Application can decrypt new KMS-encrypted data
- Mixed mode works in single instance

### Phase 4: Re-encryption Job (Sprint 2-3)

**Deliverables**:

- Implement re-encryption worker: `src/crypto/reencrypt.rs`
- Query graph for all encrypted fields with legacy encryption
- Decrypt with env key, re-encrypt with KMS, update metadata
- Progress tracking and metrics (Prometheus counters)
- Admin API endpoint to trigger/monitor re-encryption

**Re-encryption Algorithm**:

```rust
async fn reencrypt_batch(
    repo: &Arc<dyn AgeRepo>,
    kms: &Arc<dyn KeyManagementService>,
    env_key: &[u8],
    batch_size: usize,
) -> Result<usize> {
    // Query for legacy encrypted fields
    let fields = repo.query_legacy_encrypted_fields(batch_size).await?;
    
    for field in fields {
        // Decrypt with env key
        let plaintext = decrypt_with_env_key(&field.ciphertext, env_key)?;
        
        // Encrypt with KMS
        let encrypted = kms.encrypt(&plaintext, &field.context).await?;
        
        // Update graph
        repo.update_encrypted_field(&field.canonical_key, &encrypted).await?;
        
        // Increment metrics
        REENCRYPTED_FIELDS.inc();
    }
    
    Ok(fields.len())
}
```

**Acceptance Criteria**:

- Re-encryption worker processes legacy fields
- Metrics show progress
- Post-migration query confirms all fields use KMS encryption
- Can rollback to legacy mode if needed (keep env key during transition)

### Phase 5: Cleanup and Enforcement (Sprint 3)

**Deliverables**:

- Remove legacy encryption code path
- Remove env key from configuration
- Set `HMD_ENCRYPTION_MODE=kms` (enforce KMS only)
- Update documentation and runbooks

**Acceptance Criteria**:

- No legacy encryption code remains
- All encrypted data uses KMS
- Env key no longer required for operation

### Rollback Plan

**Scenario**: KMS integration has critical bug; need to revert to env-key encryption

**Procedure**:

1. Set `HMD_ENCRYPTION_MODE=hybrid` (allows both modes)
2. Deploy previous application version (with legacy support)
3. For new writes, temporarily use env-key encryption
4. Investigate and fix KMS integration issue
5. Re-deploy fixed version
6. Resume migration to KMS

**Data Consistency**: During hybrid mode, track encryption mode per field. Ensure decrypt logic handles both paths.

## Audit and Compliance

### Audit Log Requirements

**Events to Log**:

1. **Encryption Operations**:
	- Field identifier (canonical_key or surrogate)
	- Key version used
	- Actor (OIDC sub or service account)
	- Timestamp
	- Request ID (correlation)

2. **Decryption Operations**:
	- Field identifier
	- Actor
	- Reason (enrichment, query, admin access)
	- Timestamp
	- Request ID

3. **Key Lifecycle Events**:
	- Key creation
	- Key rotation
	- Key retirement
	- Key deletion
	- Actor
	- Timestamp

4. **Access Denied Events**:
	- Attempted operation
	- Actor
	- Reason for denial
	- Timestamp

**Log Format** (JSON structured logs):

```json
{
	"timestamp": "2025-12-09T01:23:45.678Z",
	"event_type": "crypto.decrypt",
	"actor": "oidc:user@example.com",
	"field_identifier": "email:hash:abc123",
	"key_version": 1,
	"key_identifier": "heimdall-pii-v1",
	"request_id": "req-xyz-789",
	"reason": "enrichment:geoip",
	"source_ip": "10.0.1.42",
	"result": "success"
}
```

### Vault Audit Backend Configuration

```hcl
vault audit enable file file_path=/var/log/vault/audit.log
```

Forward Vault audit logs to centralized logging (syslog, Splunk, ELK, etc.).

### AWS KMS CloudTrail

Enable CloudTrail for KMS API calls; filter for `Encrypt`, `Decrypt`, `GenerateDataKey` events. Export to S3 or CloudWatch Logs for analysis.

### Compliance Artifacts

**PCI DSS**:

- Requirement 3.5: Key management procedures documented (this document)
- Requirement 3.6: Key rotation (90-day cadence)
- Requirement 10: Audit trail for all key access

**GDPR**:

- Article 32: Technical measures for pseudonymization and encryption
- Audit trail for data subject access requests (decryption events)

**SOC 2 Type II**:

- Control for key lifecycle management
- Evidence: key rotation logs, audit trail, access control policies

## Operational Runbooks

### Runbook: Routine Key Rotation

**Frequency**: Every 90 days (automated)

**Procedure**:

1. **Trigger Rotation**:
	```bash
	# Vault
	vault write transit/keys/heimdall-pii/rotate
	
	# AWS KMS (automatic with EnableKeyRotation=true)
	# No manual action required
	```

2. **Verify New Key Version**:
	```bash
	# Vault
	vault read transit/keys/heimdall-pii
	
	# AWS KMS
	aws kms describe-key --key-id alias/heimdall-pii
	```

3. **Monitor Re-encryption**:
	- Check Prometheus metrics: `heimdall_reencryption_progress`
	- Query graph for count of fields with old key version
	- Verify re-encryption job is running and progressing

4. **Validate**:
	- Decrypt sample field with new key version
	- Verify audit logs show key usage

5. **Document**:
	- Record rotation date and new key version in change log
	- Update key version in configuration management

### Runbook: Emergency Key Compromise Response

**Trigger**: Suspected or confirmed key compromise (intrusion, leaked credentials, audit anomaly)

**Severity**: P1 (Critical)

**Procedure**:

1. **Immediate Actions** (within 1 hour):
	- Alert security team and incident commander
	- Isolate affected systems (network segmentation, disable service accounts)
	- Revoke compromised credentials (Vault tokens, IAM keys, service accounts)
	- Enable emergency monitoring (increased logging, alerting)

2. **Key Rotation** (within 4 hours):
	```bash
	# Vault: Create new key, disable old
	vault write transit/keys/heimdall-pii-v2/rotate
	vault write transit/keys/heimdall-pii/config min_decryption_version=2
	
	# AWS KMS: Schedule key deletion for old key, create new
	aws kms schedule-key-deletion --key-id <old-key-id> --pending-window-in-days 7
	aws kms create-key --description "Heimdall PII (emergency rotation)"
	```

3. **Re-encryption** (within 24 hours):
	- Trigger emergency re-encryption job with high priority
	- Increase batch size and parallelism
	- Monitor progress continuously

4. **Verification** (within 48 hours):
	- Audit all decryption events during compromise window
	- Identify potentially accessed data
	- Validate all data re-encrypted with new key
	- Test decryption with new key

5. **Notification**:
	- If PII potentially accessed, follow data breach notification procedures
	- Notify stakeholders and compliance team
	- Document incident and lessons learned

6. **Retire Old Key**:
	- After re-encryption complete, disable old key for all operations
	- Vault: `vault write transit/keys/heimdall-pii/config deletion_allowed=true`
	- AWS KMS: Complete key deletion after 7-30 day window

### Runbook: Re-encryption After Migration

**Purpose**: Migrate all legacy env-key encrypted data to KMS encryption

**Duration**: 1-7 days (depending on data volume)

**Procedure**:

1. **Pre-flight Checks**:
	- Verify KMS/Vault connectivity
	- Confirm authentication working
	- Test encrypt/decrypt on sample data
	- Backup database (full backup before migration)

2. **Start Re-encryption Job**:
	```bash
	# Via admin API
	curl -X POST https://heimdall.internal/admin/crypto/reencrypt \
	  -H "Authorization: Bearer $ADMIN_TOKEN" \
	  -d '{"batch_size": 1000, "interval_seconds": 60}'
	```

3. **Monitor Progress**:
	```bash
	# Prometheus query
	heimdall_reencryption_progress{status="completed"} / heimdall_reencryption_progress{status="total"}
	
	# Or via admin API
	curl https://heimdall.internal/admin/crypto/reencrypt/status
	```

4. **Validate**:
	- Query random sample of fields
	- Verify `encryption_version` is `kms_v1`
	- Test decryption via application

5. **Switch to KMS-Only Mode**:
	```bash
	# Update config
	export HMD_ENCRYPTION_MODE=kms
	# Restart application instances (rolling restart)
	```

6. **Cleanup**:
	- Remove env key from configuration
	- Update documentation
	- Archive legacy encryption code (do not delete immediately; keep for 30 days)

## Integration Points

### `src/config/mod.rs`

Add KMS configuration fields:

```rust
pub struct Settings {
    // ... existing fields ...
    
    // KMS configuration
    pub kms_provider: KmsProvider,  // vault | aws | gcp | azure
    pub kms_endpoint: String,       // Vault URL or KMS endpoint
    pub kms_key_id: String,         // Key identifier
    pub kms_auth_method: String,    // approle | kubernetes | iam
    pub encryption_mode: EncryptionMode, // legacy | kms | hybrid
}

pub enum KmsProvider {
    Vault,
    AwsKms,
    GcpKms,
    AzureKeyVault,
}

pub enum EncryptionMode {
    Legacy,  // env-key only (deprecated)
    Kms,     // KMS only (target state)
    Hybrid,  // Support both (migration period)
}
```

### `src/persist/mod.rs`

Update `PersistJob` to include encryption context:

```rust
pub struct PersistJob {
    pub label: String,
    pub key: String,
    pub props: Value,
    pub encryption_context: Option<EncryptionContext>, // For audit trail
}

pub struct EncryptionContext {
    pub field_name: String,
    pub request_id: String,
    pub actor: String,
}
```

### Worker Processes (Enrichment)

Enrichment workers need decrypt-only access:

- Configure with read-only KMS credentials
- Vault policy: `decrypt` permission only
- AWS IAM: `kms:Decrypt` action only
- Log all decryption events with reason="enrichment:{provider}"

## Implementation Milestones

Update `docs/design/Implementation-Roadmap.md`:

### Milestone 5a — KMS Infrastructure (Sprint 1)

**Deliverables**:

- Deploy Vault cluster or configure cloud KMS
- Create master encryption key
- Configure authentication (AppRole/IAM)
- Test connectivity and basic encrypt/decrypt

**Owner**: DevOps Lead

**Acceptance**: Vault/KMS operational; health checks passing; can encrypt/decrypt test data

### Milestone 5b — Code Integration (Sprint 1-2)

**Deliverables**:

- Implement `KeyManagementService` trait and Vault/KMS clients
- Integrate envelope encryption into PII policy engine
- Add backward-compatible decryption for legacy and KMS formats
- Unit and integration tests

**Owner**: Security Lead + Backend Developer

**Acceptance**: CI tests pass; can encrypt with KMS and decrypt both legacy and KMS data

### Milestone 5c — Migration (Sprint 2-3)

**Deliverables**:

- Deploy hybrid-mode application (supports both legacy and KMS)
- Run re-encryption job to migrate all existing encrypted data
- Validate all data migrated
- Switch to KMS-only mode

**Owner**: Backend Developer + DBA

**Acceptance**: All encrypted fields use KMS; legacy mode disabled; application stable

### Milestone 5d — Hardening & Docs (Sprint 3)

**Deliverables**:

- Comprehensive audit logging
- Key rotation automation (cron job or Kubernetes CronJob)
- Operational runbooks
- Update documentation (this document, README, deployment guides)

**Owner**: Tech Writer + Security Lead

**Acceptance**: Runbooks validated via tabletop exercise; docs reviewed and approved

## Cost Estimation

### HashiCorp Vault

**Self-Hosted (Open Source)**:

- Infrastructure: 3-node cluster (HA) = ~$300-500/month (cloud VMs)
- Storage backend (Consul or integrated storage) = ~$100/month
- Operational overhead: 20-40 hours/month (initial setup + maintenance)
- **Total**: ~$400-600/month + labor

**HCP Vault (Managed)**:

- Starter: ~$1.00/hour = ~$720/month (includes HA, auto-backups)
- Standard: ~$2.00/hour = ~$1440/month (includes audit log streaming, namespaces)
- **Total**: $720-1440/month (no operational overhead for Vault itself)

### AWS KMS

- Key storage: $1/month per key
- API requests:
	- First 20,000 requests/month: Free
	- Next 980,000 requests: $0.03 per 10,000 = ~$2.94
	- Assuming 100k encryptions/day = 3M/month = ~$9/month
- **Total**: ~$10-50/month (scales with usage)

### GCP Cloud KMS

- Key storage: $0.06/month per key version
- Operations: $0.03 per 10,000 operations
- Assuming 100k ops/day = 3M/month = ~$9/month
- **Total**: ~$10-50/month

### Recommendation

- **Low to moderate throughput** (<50k operations/day): AWS/GCP KMS (simplest, lowest operational cost)
- **High throughput or multi-cloud**: Self-hosted Vault or HCP Vault (more cost-effective at scale, no per-operation cost)
- **Enterprise with compliance needs**: HCP Vault (managed, comprehensive audit, HSM support with Enterprise)

## Security Summary

**Key Security Controls**:

1. **Encryption at Rest**: All PII encrypted with KMS-backed keys
2. **Key Separation**: Master keys never leave KMS/Vault HSM
3. **Least Privilege**: Role-based access (encrypt, decrypt, admin)
4. **Audit Trail**: All key operations logged with actor, timestamp, reason
5. **Key Rotation**: Automated 90-day rotation with re-encryption
6. **Emergency Response**: Runbook for key compromise (<4 hour key rotation SLA)
7. **Compliance**: Meets PCI DSS, GDPR, SOC 2 requirements for key management

**Risk Reduction**:

- **Before** (env-key): Static key, no rotation, no audit, exposed in config
- **After** (KMS): Dynamic keys, automated rotation, comprehensive audit, centralized management

## Next Steps

1. **Review and Approval**: Stakeholder review of this design document
2. **Provider Selection**: Confirm HashiCorp Vault vs cloud KMS based on deployment environment
3. **Budget Approval**: Secure budget for KMS infrastructure (Vault hosting or cloud KMS costs)
4. **Milestone Planning**: Add Milestone 5a-5d tasks to project tracker with owners
5. **Implementation**: Begin Milestone 5a (infrastructure setup)

## References

- [HashiCorp Vault Transit Secrets Engine](https://developer.hashicorp.com/vault/docs/secrets/transit)
- [AWS KMS Best Practices](https://docs.aws.amazon.com/kms/latest/developerguide/best-practices.html)
- [Google Cloud KMS Envelope Encryption](https://cloud.google.com/kms/docs/envelope-encryption)
- [NIST SP 800-57: Key Management Recommendations](https://csrc.nist.gov/publications/detail/sp/800-57-part-1/rev-5/final)
- [OWASP Cryptographic Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Cryptographic_Storage_Cheat_Sheet.html)
- Feature Card: `docs/design/features/SEC-001-PII-Policy-Field-Encryption.md`
- Architecture: `docs/design/Architecture.md`

---

**Document Version**: 1.0
**Last Updated**: 2025-12-09
**Owner**: Security Lead
**Reviewers**: Architecture Team, DevOps Lead
**Status**: Draft for Review
