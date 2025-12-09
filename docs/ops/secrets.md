# Secret Management

## Overview

Heimdall requires careful management of cryptographic secrets, API credentials, and encryption keys to maintain the security and integrity of the system. This document outlines recommended approaches, supported backends, and sample configurations for secret management in Heimdall deployments.

## Secret Types

Heimdall uses several types of secrets:

1. **TLS Certificates & Keys** — Server TLS private key and certificate for HTTPS endpoints
2. **OAuth/OIDC Credentials** — Client IDs and secrets for authentication flows (H2M and M2M)
3. **Cookie Encryption Secret** — AES-256-GCM key for encrypting session cookies
4. **Database Credentials** — Connection credentials for the graph database (Postgres+AGE)
5. **Data Encryption Keys (DEKs)** — Keys for field-level encryption of PII/sensitive data
6. **Key Encryption Keys (KEKs)** — Master keys for envelope encryption of DEKs

## Recommended Secret Backends

### HashiCorp Vault

**Recommended for**: Production deployments with existing Vault infrastructure, multi-tenant environments, compliance requirements

**Features**:

- Dynamic secret generation and rotation
- Audit logging and access control policies
- Transit secrets engine for encryption-as-a-service
- PKI secrets engine for certificate management
- Kubernetes integration via ServiceAccount tokens

**Sample Configuration**:

```json
{
	"secret_backend": "vault",
	"vault": {
		"address": "https://vault.example.com:8200",
		"namespace": "heimdall",
		"auth_method": "kubernetes",
		"role": "heimdall-app",
		"mount_path": "secret",
		"kv_version": 2
	}
}
```

**Environment Variables**:

```bash
export HMD_SECRET_BACKEND=vault
export VAULT_ADDR=https://vault.example.com:8200
export VAULT_NAMESPACE=heimdall
export VAULT_ROLE=heimdall-app
# For Kubernetes auth, service account token is auto-detected
# For token auth:
export VAULT_TOKEN=hvs.CAES...
```

**Accessing Secrets**:

- TLS certs: `secret/data/heimdall/tls`
- OAuth creds: `secret/data/heimdall/oauth`
- Cookie secret: `secret/data/heimdall/cookie-secret`
- Database credentials: `secret/data/heimdall/database`

### AWS Secrets Manager

**Recommended for**: AWS-hosted deployments, organizations using AWS KMS

**Features**:

- Automatic secret rotation for RDS credentials
- Integration with AWS KMS for encryption
- Fine-grained IAM access control
- Cross-region replication
- Versioning and rollback support

**Sample Configuration**:

```json
{
	"secret_backend": "aws_secrets_manager",
	"aws": {
		"region": "us-west-2",
		"secret_prefix": "heimdall/",
		"kms_key_id": "arn:aws:kms:us-west-2:123456789012:key/abc-def-ghi"
	}
}
```

**Environment Variables**:

```bash
export HMD_SECRET_BACKEND=aws_secrets_manager
export AWS_REGION=us-west-2
export HMD_SECRET_PREFIX=heimdall/
# AWS credentials via IAM role (recommended) or:
export AWS_ACCESS_KEY_ID=AKIA...
export AWS_SECRET_ACCESS_KEY=...
```

**Secret Naming Convention**:

- `heimdall/tls-key` — TLS private key (PEM format)
- `heimdall/tls-cert` — TLS certificate chain
- `heimdall/oauth-h2m` — H2M OAuth credentials (JSON: `{"client_id": "...", "client_secret": "..."}`)
- `heimdall/oauth-m2m` — M2M OAuth credentials
- `heimdall/cookie-secret` — Cookie encryption secret (base64-encoded 32 bytes)
- `heimdall/database-url` — Database connection string

### Google Cloud Secret Manager

**Recommended for**: GCP-hosted deployments, organizations using Google Cloud KMS

**Features**:

- Automatic replication across regions
- Integration with Cloud KMS
- IAM-based access control
- Secret versioning and automatic rotation
- Audit logging via Cloud Audit Logs

**Sample Configuration**:

```json
{
	"secret_backend": "gcp_secret_manager",
	"gcp": {
		"project_id": "my-project-123",
		"secret_prefix": "heimdall-",
		"kms_key_name": "projects/my-project-123/locations/global/keyRings/heimdall/cryptoKeys/envelope-key"
	}
}
```

**Environment Variables**:

```bash
export HMD_SECRET_BACKEND=gcp_secret_manager
export GCP_PROJECT_ID=my-project-123
export HMD_SECRET_PREFIX=heimdall-
# GCP credentials via service account (recommended) or:
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account-key.json
```

**Secret Naming Convention**:

- `heimdall-tls-key` — TLS private key
- `heimdall-tls-cert` — TLS certificate
- `heimdall-oauth-h2m` — H2M OAuth credentials (JSON)
- `heimdall-oauth-m2m` — M2M OAuth credentials (JSON)
- `heimdall-cookie-secret` — Cookie secret
- `heimdall-database-url` — Database connection string

### Kubernetes Secrets (Development/Testing)

**Recommended for**: Development, testing, and small-scale Kubernetes deployments

**Features**:

- Native Kubernetes resource
- Simple to configure via `kubectl`
- Can be mounted as files or environment variables
- Base64-encoded by default (not encrypted at rest without additional configuration)

**Important**: For production use, enable encryption at rest via KMS plugin or use external secrets operator with Vault/Cloud provider.

**Sample Configuration**:

Create secrets via `kubectl`:

```bash
kubectl create secret generic heimdall-secrets \
  --from-file=tls.key=/path/to/tls.key \
  --from-file=tls.crt=/path/to/tls.crt \
  --from-literal=oauth-h2m-id="client_id" \
  --from-literal=oauth-h2m-secret="client_secret" \
  --from-literal=oauth-m2m-id="m2m_client_id" \
  --from-literal=oauth-m2m-secret="m2m_client_secret" \
  --from-literal=cookie-secret="$(openssl rand -base64 32)" \
  --from-literal=database-url="postgresql://user:pass@host:5432/heimdall"
```

**Deployment YAML**:

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: heimdall
spec:
  containers:
    - name: heimdall
      image: heimdall:latest
      env:
        - name: HMD_TLS_KEY
          value: /etc/heimdall/tls/tls.key
        - name: HMD_TLS_CERT
          value: /etc/heimdall/tls/tls.crt
        - name: HMD_OAUTH_H2M_ID
          valueFrom:
            secretKeyRef:
              name: heimdall-secrets
              key: oauth-h2m-id
        - name: HMD_OAUTH_H2M_SECRET
          valueFrom:
            secretKeyRef:
              name: heimdall-secrets
              key: oauth-h2m-secret
        - name: HMD_OAUTH_M2M_ID
          valueFrom:
            secretKeyRef:
              name: heimdall-secrets
              key: oauth-m2m-id
        - name: HMD_OAUTH_M2M_SECRET
          valueFrom:
            secretKeyRef:
              name: heimdall-secrets
              key: oauth-m2m-secret
        - name: HMD_COOKIE_SECRET
          valueFrom:
            secretKeyRef:
              name: heimdall-secrets
              key: cookie-secret
        - name: HMD_DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: heimdall-secrets
              key: database-url
      volumeMounts:
        - name: tls
          mountPath: /etc/heimdall/tls
          readOnly: true
  volumes:
    - name: tls
      secret:
        secretName: heimdall-secrets
        items:
          - key: tls.key
            path: tls.key
          - key: tls.crt
            path: tls.crt
```

### Environment Variables (Development Only)

**Recommended for**: Local development only — **NOT for production**

For local development, secrets can be loaded directly from environment variables or a `.env` file (never commit `.env` to version control):

```bash
# .env (add to .gitignore)
HMD_TLS_KEY=/path/to/dev-tls.key
HMD_TLS_CERT=/path/to/dev-tls.crt
HMD_OAUTH_DISCOVERY_URL=https://auth.example.com/.well-known/openid-configuration
HMD_OAUTH_H2M_ID=dev_h2m_client_id
HMD_OAUTH_H2M_SECRET=dev_h2m_client_secret
HMD_OAUTH_M2M_ID=dev_m2m_client_id
HMD_OAUTH_M2M_SECRET=dev_m2m_client_secret
HMD_COOKIE_SECRET=$(openssl rand -hex 32)
HMD_DATABASE_URL=postgresql://heimdall:devpass@localhost:5432/heimdall_dev
```

Load via `dotenv`:

```bash
pnpm install dotenv
node -r dotenv/config server/server.js
```

## Envelope Encryption for Field-Level Data

Heimdall uses envelope encryption to protect sensitive field-level data (PII, credentials, etc.) in the graph database:

1. **Data Encryption Keys (DEKs)** — AES-256-GCM keys generated per-field or per-batch
2. **Key Encryption Keys (KEKs)** — Master keys stored in KMS that encrypt DEKs
3. **Encrypted DEKs** — Stored alongside encrypted data in the database

**Flow**:

1. Generate a random DEK for encrypting a field
2. Encrypt the field data with the DEK using AES-256-GCM
3. Encrypt the DEK using the KEK from the KMS
4. Store encrypted field data + encrypted DEK + metadata in the database
5. To decrypt: retrieve encrypted DEK, decrypt with KEK from KMS, decrypt field data with DEK

**KMS Integration Points**:

See `src/lib/server/kms/` for pluggable KMS integration examples. The interface supports:

- `encrypt(plaintext: Buffer, context?: Record<string, string>): Promise<Buffer>` — Encrypt a DEK
- `decrypt(ciphertext: Buffer, context?: Record<string, string>): Promise<Buffer>` — Decrypt a DEK
- `generateDataKey(): Promise<{ plaintext: Buffer; encrypted: Buffer }>` — Generate and return both plaintext and encrypted DEK

## Best Practices

### General

1. **Never commit secrets to version control** — Use `.gitignore` for `.env`, credential files, and key material
2. **Use different secrets per environment** — Dev, staging, and production must have separate credentials
3. **Rotate secrets regularly** — See [Key Rotation](./key-rotation.md) for procedures
4. **Apply least-privilege access** — Grant only necessary permissions to each service/user
5. **Enable audit logging** — Track secret access and modifications
6. **Encrypt secrets at rest** — Use KMS or secret manager encryption features
7. **Use short-lived credentials** — Prefer dynamic secrets with automatic expiration

### TLS Certificates

1. **Use automated certificate management** — ACME/Let's Encrypt for public endpoints, cert-manager for Kubernetes
2. **Rotate certificates before expiration** — Set up monitoring and alerts for certificate expiry
3. **Use separate certificates per environment** — Avoid sharing certificates between dev/staging/prod
4. **Protect private keys** — Store in secret manager, never in config files or environment variables if avoidable

### OAuth/OIDC Credentials

1. **Register separate OAuth clients per environment** — Dev, staging, and prod should use different client IDs
2. **Use client secret rotation** — Many identity providers support multiple active secrets for zero-downtime rotation
3. **Restrict redirect URIs** — Configure only necessary redirect URIs in your OAuth provider
4. **Enable MFA for OIDC admin accounts** — Protect the accounts that manage OAuth client configurations

### Database Credentials

1. **Use connection pooling** — Limit the number of database connections
2. **Enable TLS for database connections** — Encrypt traffic between Heimdall and the database
3. **Rotate database passwords regularly** — Use automated rotation via AWS RDS/Cloud SQL or secret managers
4. **Use IAM authentication where possible** — AWS RDS IAM auth, GCP Cloud SQL IAM auth

### Cookie Secrets

1. **Generate strong random secrets** — Use at least 32 bytes of cryptographically secure random data
2. **Rotate cookie secrets** — See [Key Rotation](./key-rotation.md) for procedures and multi-key support
3. **Do not reuse secrets across services** — Each application should have its own cookie secret

## Migration from Environment Variables to Secret Manager

To migrate an existing deployment from environment variables to a secret manager:

1. **Document current secrets** — List all `HMD_*` secrets currently in use
2. **Create secrets in the secret manager** — Use appropriate naming conventions
3. **Update deployment configuration** — Configure `HMD_SECRET_BACKEND` and backend-specific settings
4. **Test in non-production environment** — Verify secret retrieval and application startup
5. **Deploy to production** — Update production deployment with secret manager configuration
6. **Remove old environment variables** — Clean up old secret definitions after verifying the migration
7. **Enable audit logging** — Monitor secret access patterns

## Troubleshooting

### Secret Not Found

**Symptoms**: Application fails to start with error "Missing required environment variable" or "Secret not found"

**Resolution**:

1. Verify secret exists in the configured backend
2. Check secret naming matches configuration (prefix, path, key name)
3. Verify authentication credentials for the secret backend
4. Check IAM/access policies allow read access to the secret

### Permission Denied

**Symptoms**: Application fails with "Access denied" or "Forbidden" errors when accessing secrets

**Resolution**:

1. Verify IAM role/service account has necessary permissions
2. For Vault: Check policy allows read on the secret path
3. For AWS: Verify IAM policy includes `secretsmanager:GetSecretValue`
4. For GCP: Verify service account has `secretmanager.versions.access` permission
5. Check namespaces/projects are correctly configured

### TLS Certificate Errors

**Symptoms**: HTTPS server fails to start or clients report certificate errors

**Resolution**:

1. Verify TLS certificate and key paths are correct
2. Check file permissions (readable by the application user)
3. Verify certificate format (PEM expected)
4. Ensure certificate and key match (use `openssl` to verify)
5. Check certificate is not expired
6. Verify certificate chain is complete

### Database Connection Failures

**Symptoms**: Application fails to connect to database with authentication errors

**Resolution**:

1. Verify database credentials are correct
2. Check database URL format matches expected format
3. Test database connection from the application host using `psql` or similar tools
4. Verify database firewall rules allow connections from Heimdall
5. Check if database requires TLS and configure connection string accordingly

## References

- [Key Rotation Playbook](./key-rotation.md)
- [HashiCorp Vault Documentation](https://www.vaultproject.io/docs)
- [AWS Secrets Manager Documentation](https://docs.aws.amazon.com/secretsmanager/)
- [Google Cloud Secret Manager Documentation](https://cloud.google.com/secret-manager/docs)
- [Kubernetes Secrets Documentation](https://kubernetes.io/docs/concepts/configuration/secret/)
