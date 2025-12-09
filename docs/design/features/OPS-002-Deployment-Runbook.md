# OPS-002 — Deployment Runbook for Linux Hosts

## Category

non-functional

## Description

Comprehensive deployment runbook for production deployment of Heimdall on Linux hosts, including Postgres+AGE provisioning options, backup strategies, TLS configuration, and operational procedures.

## Implementation Steps

1. Draft `docs/ops/deployment-runbook.md` with step-by-step deployment instructions
2. Provide Postgres+AGE provisioning and backup guidance (sidecar vs managed)
3. Include TLS bootstrap and certificate rotation notes
4. Review and validate with repository owners

## Acceptance Criteria

- [ ] Runbook provides actionable steps to deploy Heimdall on Linux hosts
- [ ] Both sidecar and managed database deployment options are documented
- [ ] Backup and restore procedures are clearly defined
- [ ] TLS certificate setup and rotation procedures are documented
- [ ] Systemd service configuration is provided
- [ ] Monitoring and troubleshooting guidance is included
- [ ] Security considerations are addressed
- [ ] Production checklist is provided
- [ ] Reviewed and approved by repository owners

## Related Documents

- `docs/design/Architecture.md` — System architecture and components
- `docs/POSTGRES_AGE_SETUP.md` — Local development database setup
- `docs/design/features/CFG-001-Config-Module.md` — Configuration requirements
- `docs/design/Implementation-Roadmap.md` — Milestone 5 deliverables

## Deliverables

- [x] `docs/ops/deployment-runbook.md` — Comprehensive deployment runbook
- [x] `docs/design/features/OPS-002-Deployment-Runbook.feature.json` — Machine-readable feature spec
- [x] `docs/design/features/OPS-002-Deployment-Runbook.md` — Feature card documentation

## Status

✅ **Complete** — All deliverables have been implemented and documented.

## Notes

The runbook covers:

- System requirements and prerequisites
- Two deployment options: Docker sidecar and managed database
- Detailed TLS configuration and certificate rotation procedures
- Database backup strategies (logical, physical, and volume backups)
- Upgrade strategies for PostgreSQL and Heimdall
- Systemd service configuration for production
- Monitoring, observability, and troubleshooting guidance
- Security considerations and production checklist

The runbook is ready for review and validation by repository owners.
