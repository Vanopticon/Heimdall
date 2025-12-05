# Active Context

- Repository: `Vanopticon/Heimdall`
- Branch: `main`
- Date (snapshot): 2025-12-05
- Current work: create/update long-term memory (LTM) files summarizing repo state.

Recent notable files read for context:

- `README.md` — high-level project overview and quick start.
- `docs/design/Architecture.md`, `docs/design/DataModel.md` — architecture diagram and graph model.
- `server/server.ts` — Node/Express server, OIDC, TLS, sessions, CSP and security hardening.
- `src/lib/server/ageClient.ts` — AGE / Postgres graph client wrapper; core graph operations.
- `src/routes/api/upload/+server.ts` — file upload API used for ingesting dumps.

Open items / assumptions:

- No `.github/agent_memory` existed prior to this session; created as part of LTM maintenance.
- `VOH_*` env vars are required to run the server; see `server/server.ts` requiredEnv list.
- Tests and CI not inspected as part of this quick snapshot — verify in follow-up if required.

Next actions (planned): finalize `system_patterns.md`, `tech_stack.md`, `progress_tracker.md`, and `handoff.md`.
