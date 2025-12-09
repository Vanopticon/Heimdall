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

### Task 0.2: Migration and Backfill Plan for Canonical Keys

**Status**: Completed

**Description**: Design a comprehensive migration and backfill plan to handle future changes to canonical key or row-hash semantics in the graph data model.

**Deliverables**:

- `docs/design/Migration-Backfill-Plan.md` — Comprehensive migration strategy document
- Feature card documenting acceptance criteria and implementation approach
- Migration strategies with trade-offs (Shadow Property, In-Place, Node Duplication)
- Backfill pseudocode with batch processing and resume capability
- Validation and smoke test procedures
- Staged rollout plan with timelines
- Rollback procedures for each phase

**Feature Card**: [ARCH-001.5-Migration-Backfill-Plan](features/ARCH-001.5-Migration-Backfill-Plan.md)

**Acceptance Criteria**:

- Migration plan document exists with multiple strategies documented
- Canonical key versioning strategy defined
- Backfill job design shows incremental, resumable processing
- Smoke tests and validation procedures included
- Staged rollout phases defined
- Rollback procedures documented

### Future Tasks

Additional tasks for Milestone 0 will be added as planning progresses.

## Milestone 1: Core Ingestion Pipeline

_To be defined_

## Milestone 2: Graph Operations & Enrichment

_To be defined_

## Milestone 3: API & UI Components

_To be defined_
