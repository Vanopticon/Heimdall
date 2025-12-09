# Implementation Roadmap

This document tracks the implementation milestones for Heimdall.

## Milestone 0: Project Scaffolding & Core Infrastructure

**Goal**: Establish project foundation, configuration management, and CI/CD infrastructure.

### Task 0.1: Configuration Module (Environment-First)

**Status**: Complete

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

### Task 0.3: Graph Schema & Postgres+AGE Client Wrapper

**Status**: In Progress

**Description**: Define the canonical graph schema and provide a lightweight Postgres+AGE client wrapper with support for transactional upserts, batch operations, and provenance tracking.

**Deliverables**:

- SQL schema under `sql/v1/` implementing core nodes (dumps, fields, field_value, sightings, entities, NPI categories)
- Enhanced `src/lib/server/ageClient.ts` with transaction support and batch operations
- Integration tests verifying schema migrations and upsert workflows
- Documentation for schema design and usage examples

**Feature Card**: [GRA-001-Graph-Schema](features/GRA-001-Graph-Schema.md)

**Acceptance Criteria**:

- SQL migrations apply on local dev Postgres+AGE
- `src/lib/server/ageClient.ts` exposes clear API for MERGE-style upserts
- Transaction support allows atomic operations
- Batch operations reduce round-trips for bulk writes
- Integration tests confirm canonical nodes are created with correct provenance
- Tests validate transaction rollback on errors

### Future Tasks

Additional tasks for Milestone 0 will be added as planning progresses.

## Milestone 1: Core Ingestion Pipeline

_To be defined_

## Milestone 2: Graph Operations & Enrichment

_To be defined_

## Milestone 3: API & UI Components

_To be defined_
