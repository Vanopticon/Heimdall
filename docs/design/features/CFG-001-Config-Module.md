# CFG-001: Configuration Module (Environment-First)

## Category

Functional

## Description

Implement a centralized, environment-first configuration module that provides runtime configuration for the Heimdall application. The module consolidates environment variable reading, validation, and optional configuration file support into a single, testable interface.

## Requirements

1. Create a `src/lib/server/config/` module that exposes:
   - `Settings` interface/type defining all configuration options
   - `load()` function that reads `HMD_*` environment variables
   - Optional configuration file support
   - Validation and error handling for required settings

2. Configuration should support:
   - Host and port settings (`HMD_HOST`, `HMD_PORT`)
   - TLS certificate paths (`HMD_TLS_KEY`, `HMD_TLS_CERT`)
   - OAuth/OIDC settings (`HMD_OAUTH_*`)
   - Database connection (`HMD_DATABASE_URL` or PG* variables)
   - Session and cookie secrets
   - All other runtime configuration needs

3. Export the module from `src/lib/server/index.ts` for easy imports

4. Add unit tests that exercise `load()` with environment overrides

## Acceptance Criteria

- `pnpm build` succeeds
- Unit tests exercise `load()` with env overrides and pass
- `src/lib/server/config` is documented in README
- Configuration module is referenced in `docs/design/Implementation-Roadmap.md`
- CI workflow runs unit tests

## Implementation Notes

- Follow existing `HMD_*` environment variable naming convention
- Provide sensible defaults where appropriate
- Validate required settings and fail fast with clear error messages
- Support both environment variables and optional config file
- Keep configuration separate from application logic

## Related Files

- `server/server.ts` — Current configuration logic (to be refactored)
- `README.md` — Documentation
- `docs/design/Implementation-Roadmap.md` — Milestone tracking
