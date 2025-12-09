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
