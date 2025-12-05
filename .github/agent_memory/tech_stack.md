# Tech Stack

- Language: `TypeScript` (target ES2022)
- Frontend: `SvelteKit` (adapter-node), `svelte` and SvelteKit tooling
- Server: `Node.js` + `Express` wrapper (custom `server/server.ts`) for TLS and OIDC
- Package manager / tooling: `pnpm`, `vite`, `nodemon`, `tsx` for dev
- Graph DB: `PostgreSQL` + `AGE` (accessed via `age-schema-client`)
- DB client libraries: `pg`, `pgvector`
- Auth: `openid-client` for OIDC
- Security & middleware: `helmet`, `cors`, `express-session`, `express-rate-limit`, `compression`, `morgan`
- Build/test: `vitest`, `vite`, `prettier` and `svelte-check`

Runtime / deployment notes:

- `server/` expects TLS key/cert paths via environment (`VOH_TLS_KEY`, `VOH_TLS_CERT`) and host/port via `VOH_HOST`/`VOH_PORT`.
- DB connection supports `VOH_DATABASE_URL` or standard `PG*` env vars.
