# Implementation Roadmap

This document tracks the implementation milestones for Heimdall.

## Milestone 0: Project Scaffolding & Core Infrastructure

**Goal**: Establish project foundation, configuration management, and CI/CD infrastructure.

### Task 0.1: Configuration Module (Environment-First)

**Status**: In Progress

**Description**: Implement a centralized configuration module that reads from `HMD_*` environment variables and optional configuration files.

**Deliverables**:

- `src/lib/server/config/` module exposing `Settings` and `load()` helper
- Unit tests for configuration loading with environment overrides
- CI workflow for running tests
- Documentation in README and feature cards

**Feature Card**: [CFG-001-Config-Module](features/CFG-001-Config-Module.md)

**Acceptance Criteria**:

- `pnpm build` succeeds
- Unit tests pass
- Configuration module documented in README
- CI workflow runs successfully

### Future Tasks

Additional tasks for Milestone 0 will be added as planning progresses.

## Milestone 1: Core Ingestion Pipeline

_To be defined_

## Milestone 2: Graph Operations & Enrichment

_To be defined_

## Milestone 3: API & UI Components

_To be defined_

## Milestone 5: Security Hardening & Operations

**Goal**: Establish operational procedures and infrastructure for secret management, key rotation, and production security hardening.

### Task 5.1: Secret Management Integration & Key Rotation Playbook

**Status**: Complete

**Description**: Create operational guidance and initial integration points for secret management and key rotation for envelope encryption and service credentials.

**Deliverables**:

- `docs/ops/key-rotation.md` — Detailed playbook for rotating keys and credentials with verification steps
- `docs/ops/secrets.md` — Comprehensive guide to secret management with backend-specific configurations
- `src/lib/server/kms/` — KMS integration module with pluggable provider interface
- Example implementations for local, AWS KMS, GCP KMS, and HashiCorp Vault providers

**Feature Card**: [SEC-002-Secret-Management](features/SEC-002-Secret-Management.md)

**Acceptance Criteria**:

- Runbook covers emergency rotation and planned rotation with verification steps
- Documentation includes sample configurations for Vault, AWS Secrets Manager, GCP Secret Manager, and Kubernetes Secrets
- KMS module provides interface for envelope encryption (DEK/KEK pattern)
- Local KMS provider fully implemented for development/testing
- Stub implementations provided for production KMS providers with implementation guides
- Docs reviewed and accepted by repository owners

### Future Tasks

Additional tasks for Milestone 5 will be added as security requirements evolve.
