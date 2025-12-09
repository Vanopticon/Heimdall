# Key Rotation Playbook

## Overview

This document provides operational procedures for rotating cryptographic keys and credentials in Heimdall deployments. Regular key rotation is a security best practice that limits the impact of compromised credentials and satisfies compliance requirements.

## Rotation Types

### Planned Rotation

Scheduled rotation as part of regular security maintenance:

- **Frequency**: Quarterly for most secrets, more frequently for high-sensitivity environments
- **Timing**: During maintenance windows with reduced traffic
- **Notification**: Advance notice to operations and security teams

### Emergency Rotation

Immediate rotation in response to a security incident:

- **Trigger**: Suspected or confirmed compromise of credentials
- **Timing**: As soon as possible, potentially outside maintenance windows
- **Notification**: Immediate notification to incident response team

## Pre-Rotation Checklist

Before rotating any secret:

- [ ] Verify backup and rollback procedures are in place
- [ ] Document current secret identifiers (versions, key IDs)
- [ ] Identify all services and components using the secret
- [ ] Schedule or obtain approval for the rotation window
- [ ] Notify relevant teams (operations, security, application owners)
- [ ] Prepare new secrets in advance where possible
- [ ] Test rotation procedure in non-production environment
- [ ] Have rollback plan ready

## Rotation Procedures

### TLS Certificate Rotation

**Impact**: Low if using zero-downtime deployment; brief connection interruptions if restarting services

**Procedure**:

1. **Generate new certificate**:

	```bash
	# Using Let's Encrypt/ACME
	certbot certonly --standalone -d heimdall.example.com

	# Or using openssl for internal CA
	openssl req -new -newkey rsa:4096 -nodes \
	  -keyout new-tls.key -out new-tls.csr \
	  -subj "/CN=heimdall.example.com"
	# Submit CSR to CA and receive certificate
	```

2. **Store new certificate in secret backend**:

	```bash
	# Vault
	vault kv put secret/heimdall/tls \
	  key=@new-tls.key \
	  cert=@new-tls.crt

	# AWS Secrets Manager
	aws secretsmanager update-secret \
	  --secret-id heimdall/tls-key \
	  --secret-string file://new-tls.key

	aws secretsmanager update-secret \
	  --secret-id heimdall/tls-cert \
	  --secret-string file://new-tls.crt

	# Kubernetes
	kubectl create secret tls heimdall-tls-new \
	  --key=new-tls.key --cert=new-tls.crt
	```

3. **Update deployment configuration** (if not using dynamic secret loading):

	```bash
	# Update Kubernetes deployment to use new secret
	kubectl set volume deployment/heimdall \
	  --add --name=tls --type=secret \
	  --secret-name=heimdall-tls-new \
	  --mount-path=/etc/heimdall/tls \
	  --overwrite
	```

4. **Perform rolling update**:

	```bash
	# Kubernetes rolling update
	kubectl rollout restart deployment/heimdall
	kubectl rollout status deployment/heimdall

	# Docker Compose
	docker-compose up -d --no-deps --build heimdall

	# Systemd service
	systemctl reload heimdall
	```

5. **Verify new certificate**:

	```bash
	# Check certificate expiry and details
	echo | openssl s_client -connect heimdall.example.com:443 -servername heimdall.example.com 2>/dev/null | openssl x509 -noout -dates -subject

	# Verify HTTPS endpoint
	curl -v https://heimdall.example.com/health
	```

6. **Remove old certificate** (after verification period):

	```bash
	# Vault
	vault kv metadata delete secret/heimdall/tls

	# AWS Secrets Manager - delete old version
	# (AWS keeps version history, no action needed)

	# Kubernetes
	kubectl delete secret heimdall-tls-old
	```

**Rollback**:

If issues arise, revert to the previous certificate:

```bash
kubectl rollout undo deployment/heimdall
# or restore previous secret version from backup
```

### OAuth/OIDC Client Secret Rotation

**Impact**: Medium — may require application restart; can be zero-downtime with multi-secret support

**Procedure**:

1. **Create new client secret in OAuth provider**:

	- Log into OAuth/OIDC provider admin console
	- Navigate to client configuration
	- Generate additional client secret (do not delete old secret yet)
	- Note new secret value

2. **Store new secret in secret backend**:

	```bash
	# Vault
	vault kv put secret/heimdall/oauth-h2m \
	  client_id="<client_id>" \
	  client_secret="<new_secret>"

	# AWS Secrets Manager
	aws secretsmanager update-secret \
	  --secret-id heimdall/oauth-h2m \
	  --secret-string '{"client_id":"<id>","client_secret":"<new_secret>"}'

	# Kubernetes
	kubectl create secret generic heimdall-oauth-new \
	  --from-literal=oauth-h2m-secret="<new_secret>" \
	  --dry-run=client -o yaml | kubectl apply -f -
	```

3. **Deploy updated secret to application**:

	```bash
	# Restart application to load new secret
	kubectl rollout restart deployment/heimdall
	kubectl rollout status deployment/heimdall
	```

4. **Verify OAuth flow**:

	```bash
	# Test H2M flow (interactive login)
	# - Navigate to https://heimdall.example.com/login
	# - Complete OAuth flow
	# - Verify successful authentication

	# Test M2M flow (machine-to-machine)
	curl -X POST https://auth.example.com/oauth/token \
	  -H "Content-Type: application/x-www-form-urlencoded" \
	  -d "grant_type=client_credentials" \
	  -d "client_id=<m2m_client_id>" \
	  -d "client_secret=<new_m2m_secret>" \
	  -d "scope=heimdall:api"
	```

5. **Delete old client secret** (after verification period of 24-48 hours):

	- Log into OAuth provider admin console
	- Remove old client secret
	- Monitor for any authentication failures

**Rollback**:

If authentication fails, revert to old client secret:

```bash
# Re-enable old secret in OAuth provider
# Revert secret in secret backend
kubectl rollout undo deployment/heimdall
```

### Cookie Secret Rotation

**Impact**: High — invalidates all active sessions; users must re-authenticate

**Procedure**:

1. **Generate new cookie secret**:

	```bash
	# Generate 32-byte random secret
	NEW_SECRET=$(openssl rand -hex 32)
	echo "New cookie secret: $NEW_SECRET"
	```

2. **Store new secret in secret backend**:

	```bash
	# Vault
	vault kv put secret/heimdall/cookie-secret value="$NEW_SECRET"

	# AWS Secrets Manager
	aws secretsmanager update-secret \
	  --secret-id heimdall/cookie-secret \
	  --secret-string "$NEW_SECRET"

	# Kubernetes
	kubectl create secret generic heimdall-cookie-new \
	  --from-literal=cookie-secret="$NEW_SECRET" \
	  --dry-run=client -o yaml | kubectl apply -f -
	```

3. **Notify users of pending session invalidation**:

	- Send notification that users will need to re-authenticate
	- Schedule rotation during low-traffic period if possible

4. **Deploy new cookie secret**:

	```bash
	# Rolling update will gradually invalidate sessions
	kubectl rollout restart deployment/heimdall
	kubectl rollout status deployment/heimdall
	```

5. **Verify new sessions use new secret**:

	```bash
	# Login and verify successful session creation
	# Check server logs for decryption errors
	kubectl logs -f deployment/heimdall | grep -i "cookie\|session"
	```

**Multi-Key Support** (for zero-downtime rotation):

To implement zero-downtime cookie rotation, support multiple active cookie secrets:

1. Maintain array of valid cookie secrets: `[old_secret, new_secret]`
2. Encrypt new cookies with `new_secret`
3. Attempt decryption with each secret in the array
4. After verification period (e.g., session max age), remove `old_secret` from array

**Rollback**:

```bash
# Restore old cookie secret
kubectl rollout undo deployment/heimdall
```

### Database Credentials Rotation

**Impact**: High — may cause connection failures if not done properly

**Procedure** (for manual rotation):

1. **Create new database user with same privileges**:

	```sql
	-- Connect to database as admin
	CREATE USER heimdall_new WITH PASSWORD 'new_secure_password';
	GRANT ALL PRIVILEGES ON DATABASE heimdall TO heimdall_new;
	GRANT USAGE ON SCHEMA heimdall_graph TO heimdall_new;
	GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA heimdall_graph TO heimdall_new;
	GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA heimdall_graph TO heimdall_new;
	```

2. **Update connection string in secret backend**:

	```bash
	# New connection string
	NEW_DB_URL="postgresql://heimdall_new:new_secure_password@db.example.com:5432/heimdall"

	# Vault
	vault kv put secret/heimdall/database url="$NEW_DB_URL"

	# AWS Secrets Manager
	aws secretsmanager update-secret \
	  --secret-id heimdall/database-url \
	  --secret-string "$NEW_DB_URL"

	# Kubernetes
	kubectl create secret generic heimdall-db-new \
	  --from-literal=database-url="$NEW_DB_URL" \
	  --dry-run=client -o yaml | kubectl apply -f -
	```

3. **Deploy updated credentials**:

	```bash
	kubectl rollout restart deployment/heimdall
	kubectl rollout status deployment/heimdall
	```

4. **Verify database connectivity**:

	```bash
	# Check application logs for connection errors
	kubectl logs deployment/heimdall | grep -i "database\|connection"

	# Test database operations
	curl https://heimdall.example.com/api/health
	```

5. **Remove old database user** (after verification period):

	```sql
	-- Terminate existing connections from old user
	SELECT pg_terminate_backend(pid)
	FROM pg_stat_activity
	WHERE usename = 'heimdall';

	-- Drop old user
	DROP USER heimdall;
	```

**Automated Rotation** (AWS RDS/GCP Cloud SQL):

For managed databases with automatic rotation support:

```bash
# AWS RDS - enable automatic rotation (30 days)
aws secretsmanager rotate-secret \
  --secret-id heimdall/database-credentials \
  --rotation-lambda-arn arn:aws:lambda:region:account:function:SecretsManagerRotation \
  --rotation-rules AutomaticallyAfterDays=30
```

**Rollback**:

```bash
# Revert to old database credentials
kubectl rollout undo deployment/heimdall
# or restore old secret version
```

### Data Encryption Key (DEK) Rotation

**Impact**: Medium — existing encrypted data remains readable; new data uses new keys

**Procedure**:

1. **Generate new Key Encryption Key (KEK) in KMS**:

	```bash
	# AWS KMS
	aws kms create-key \
	  --description "Heimdall DEK encryption key v2" \
	  --key-usage ENCRYPT_DECRYPT

	# GCP KMS
	gcloud kms keys create heimdall-kek-v2 \
	  --location global \
	  --keyring heimdall \
	  --purpose encryption

	# Vault Transit
	vault write transit/keys/heimdall-dek-v2 type=aes256-gcm96
	```

2. **Update KMS configuration**:

	```bash
	# Update HMD_KMS_KEY_ID to new key
	kubectl set env deployment/heimdall \
	  HMD_KMS_KEY_ID=arn:aws:kms:region:account:key/new-key-id
	```

3. **Deploy new configuration**:

	```bash
	kubectl rollout restart deployment/heimdall
	```

4. **Verify new encryptions use new KEK**:

	- Insert test data
	- Verify encrypted DEK metadata references new KEK

5. **Re-encrypt existing data** (background process):

	```bash
	# Run re-encryption job (implement as needed)
	kubectl apply -f k8s/jobs/reencrypt-data.yaml
	kubectl logs -f job/reencrypt-data
	```

**Note**: Re-encryption is optional but recommended. Existing data remains decryptable using the old KEK. Implement re-encryption as a background job that:

- Reads encrypted data + encrypted DEK
- Decrypts DEK using old KEK
- Decrypts data using DEK
- Encrypts DEK using new KEK
- Updates database with new encrypted DEK

6. **Disable old KEK** (after all data is re-encrypted):

	```bash
	# AWS KMS
	aws kms disable-key --key-id <old-key-id>

	# GCP KMS
	gcloud kms keys versions disable <version> \
	  --key heimdall-kek-v1 \
	  --keyring heimdall \
	  --location global

	# Vault Transit
	vault write transit/keys/heimdall-dek-v1/config deletion_allowed=true
	vault delete transit/keys/heimdall-dek-v1
	```

### Key Encryption Key (KEK) Rotation

See Data Encryption Key rotation above — KEK rotation involves generating a new master key in the KMS and optionally re-encrypting all DEKs with the new KEK.

## Verification Steps

After any rotation, perform the following verification:

### Health Check

```bash
# Verify application is running
kubectl get pods -l app=heimdall
curl https://heimdall.example.com/health
```

### Authentication Test

```bash
# Test H2M OAuth flow
# 1. Navigate to https://heimdall.example.com/login
# 2. Complete OAuth flow
# 3. Verify successful authentication and session creation

# Test M2M OAuth flow
curl -X POST https://auth.example.com/oauth/token \
  -d grant_type=client_credentials \
  -d client_id=<m2m_id> \
  -d client_secret=<new_m2m_secret> \
  -d scope=heimdall:api
```

### Database Connectivity

```bash
# Verify database operations
curl https://heimdall.example.com/api/status

# Check database connection pool
kubectl logs deployment/heimdall | grep -i "database\|pool"
```

### TLS Certificate

```bash
# Verify certificate chain and expiry
echo | openssl s_client -connect heimdall.example.com:443 \
  -servername heimdall.example.com 2>/dev/null | openssl x509 -noout -text

# Verify TLS version and cipher suites
nmap --script ssl-enum-ciphers -p 443 heimdall.example.com
```

### Encryption/Decryption

```bash
# Insert test encrypted field
curl -X POST https://heimdall.example.com/api/test-encrypt \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"data": "test sensitive data"}'

# Retrieve and verify decryption
curl -X GET https://heimdall.example.com/api/test-decrypt/<id> \
  -H "Authorization: Bearer <token>"
```

### Monitoring and Logs

```bash
# Check for errors in application logs
kubectl logs deployment/heimdall --tail=100 | grep -i "error\|failed\|exception"

# Monitor authentication failures
kubectl logs deployment/heimdall | grep -i "auth.*fail\|unauthorized"

# Check KMS usage
aws cloudtrail lookup-events \
  --lookup-attributes AttributeKey=ResourceType,AttributeValue=AWS::KMS::Key \
  --max-results 50
```

## Rollback Procedures

If rotation causes issues:

### Immediate Rollback

1. **Restore previous secret version**:

	```bash
	# Vault - restore previous version
	vault kv rollback -version=<previous-version> secret/heimdall/<secret-path>

	# AWS Secrets Manager - restore previous version
	aws secretsmanager update-secret-version-stage \
	  --secret-id heimdall/<secret-name> \
	  --version-stage AWSCURRENT \
	  --remove-from-version-id <current-version-id> \
	  --move-to-version-id <previous-version-id>

	# Kubernetes - rollback deployment
	kubectl rollout undo deployment/heimdall
	```

2. **Verify rollback**:

	```bash
	kubectl rollout status deployment/heimdall
	# Perform verification steps (see above)
	```

3. **Investigate root cause**:

	- Check application logs for errors
	- Verify secret format and content
	- Check permissions and access policies
	- Test secret retrieval manually

### Post-Rollback Actions

- Document the issue and root cause
- Update rotation procedures if needed
- Test rotation in non-production environment
- Schedule new rotation attempt

## Emergency Rotation

In case of suspected or confirmed secret compromise:

### Immediate Actions

1. **Assess scope of compromise**:

	- Which secret(s) are compromised?
	- When did the compromise occur?
	- What systems have access to the secret?
	- What is the potential impact?

2. **Initiate incident response**:

	- Notify security team
	- Open incident ticket
	- Begin audit log review

3. **Rotate compromised secret immediately**:

	- Follow rotation procedure for affected secret
	- Do not wait for maintenance window
	- Accept potential service disruption if necessary

4. **Revoke compromised credentials**:

	- Disable old secret in secret backend
	- Revoke OAuth client secrets
	- Disable old TLS certificates in CA
	- Block old database users

5. **Review audit logs**:

	```bash
	# Vault audit logs
	vault audit list
	vault read sys/audit/<audit-device>/logs

	# AWS CloudTrail
	aws cloudtrail lookup-events \
	  --lookup-attributes AttributeKey=ResourceName,AttributeValue=<secret-name> \
	  --start-time <compromise-time>

	# Kubernetes audit logs
	kubectl logs -n kube-system kube-apiserver | grep "secret.*heimdall"
	```

6. **Monitor for unauthorized access**:

	- Watch for failed authentication attempts with old credentials
	- Check for API calls using compromised tokens
	- Review database access logs for suspicious activity

### Post-Emergency Actions

- Document incident timeline and actions taken
- Perform root cause analysis
- Update security procedures
- Implement additional monitoring/alerting
- Review access controls and least-privilege policies

## Automation

Consider automating key rotation where possible:

### Scripted Rotation

Create rotation scripts for each secret type:

```bash
#!/bin/bash
# rotate-tls-cert.sh

set -e

# Generate new certificate
certbot certonly --standalone -d heimdall.example.com

# Update secret in Vault
vault kv put secret/heimdall/tls \
  key=@/etc/letsencrypt/live/heimdall.example.com/privkey.pem \
  cert=@/etc/letsencrypt/live/heimdall.example.com/fullchain.pem

# Trigger rolling update
kubectl rollout restart deployment/heimdall
kubectl rollout status deployment/heimdall

# Verify
./verify-heimdall.sh
```

### Scheduled Rotation

Use cron or Kubernetes CronJob for scheduled rotation:

```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: heimdall-cert-rotation
spec:
  schedule: "0 2 1 */3 *" # Quarterly at 2 AM on 1st day of month
  jobTemplate:
    spec:
      template:
        spec:
          containers:
            - name: rotate-cert
              image: heimdall-ops:latest
              command: ["/scripts/rotate-tls-cert.sh"]
              env:
                - name: VAULT_ADDR
                  value: "https://vault.example.com"
                - name: VAULT_TOKEN
                  valueFrom:
                    secretKeyRef:
                      name: vault-token
                      key: token
          restartPolicy: OnFailure
```

### Automatic Secret Rotation

Use secret manager built-in rotation (AWS Secrets Manager, GCP Secret Manager):

- Configure automatic rotation policies
- Implement rotation Lambda/Cloud Function
- Test rotation in non-production environment

## Compliance and Audit

### Documentation Requirements

- Maintain rotation schedule and history
- Document all manual rotation procedures performed
- Record verification steps and results
- Keep audit logs for compliance period (typically 1-7 years)

### Rotation Schedule Template

| Secret Type                | Last Rotated | Next Rotation | Owner       | Status  |
| -------------------------- | ------------ | ------------- | ----------- | ------- |
| TLS Certificate            | 2024-12-01   | 2025-03-01    | Ops Team    | Active  |
| OAuth H2M Client Secret    | 2024-11-15   | 2025-02-15    | Security    | Active  |
| OAuth M2M Client Secret    | 2024-11-15   | 2025-02-15    | Security    | Active  |
| Cookie Secret              | 2024-10-01   | 2025-01-01    | Ops Team    | Active  |
| Database Password          | 2024-09-01   | 2024-12-01    | DBA         | Pending |
| KEK (Envelope Encryption)  | 2024-08-01   | 2025-08-01    | Security    | Active  |

### Audit Trail

Maintain audit trail for all rotation activities:

- Timestamp of rotation
- Person/system performing rotation
- Secret type and identifier
- Previous and new key IDs/versions
- Verification results
- Issues encountered and resolution

## References

- [Secret Management Guide](./secrets.md)
- [HashiCorp Vault Key Rotation](https://learn.hashicorp.com/tutorials/vault/eaas-transit)
- [AWS Secrets Manager Rotation](https://docs.aws.amazon.com/secretsmanager/latest/userguide/rotating-secrets.html)
- [Google Cloud Secret Manager](https://cloud.google.com/secret-manager/docs/creating-and-accessing-secrets)
- [NIST Key Management Guidelines](https://csrc.nist.gov/publications/detail/sp/800-57-part-1/rev-5/final)
