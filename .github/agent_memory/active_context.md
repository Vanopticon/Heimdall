# Active Context

- Repository: `Vanopticon/Heimdall`
- Branch: `v1.0.0`
- Date (snapshot): 2025-12-06
- Current work: create/update long-term memory (LTM) files summarizing repo state.

Recent Svelte-specific notes:

- Frontend uses **Svelte 5 (runes mode)**. Code patterns include `$props()`, `$state()`, `$effect()`, and other runes rather than Svelte 4 `export let`/`$:` patterns.
- A recent change updated `src/routes/+error.svelte` to use `$props()` for props and `$effect` to update `page.data.title` reactively (see commit in this session).
- Migration guidance: Svelte 4/context7 docs are present in the repo history but the active codebase targets Svelte 5 runes; prefer Svelte 5 docs when resolving reactivity/props issues.

Recent notable files read for context:

- `README.md` — high-level project overview and quick start.
- `docs/design/Architecture.md`, `docs/design/DataModel.md` — architecture diagram and graph model.
- `server/server.ts` — Node/Express server, OIDC, TLS, sessions, CSP and security hardening.
- `src/lib/server/ageClient.ts` — AGE / Postgres graph client wrapper; core graph operations.
- `src/routes/api/upload/+server.ts` — file upload API used for ingesting dumps.

Open items / assumptions:

- No `.github/agent_memory` existed prior to this session; created as part of LTM maintenance.
- `HMD_*` env vars are required to run the server; see `server/server.ts` requiredEnv list.
- Tests and CI not inspected as part of this quick snapshot — verify in follow-up if required.

Next actions (planned): finalize `system_patterns.md`, `tech_stack.md`, `progress_tracker.md`, and `handoff.md`.
