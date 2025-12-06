# Tech Stack

- Language: `TypeScript` (target ES2022)
- Frontend: `SvelteKit` (adapter-node), **Svelte 5 (runes mode)** and SvelteKit tooling
   	+ Uses runes such as `$state`, `$props`, `$derived`, `$effect` instead of Svelte 4 `export let` and `$:` reactive statements.
- Server: `Node.js` + `Express` wrapper (custom `server/server.ts`) for TLS and OIDC
- Package manager / tooling: `pnpm`, `vite`, `nodemon`, `tsx` for dev
- Graph DB: `PostgreSQL` + `AGE` (accessed via `age-schema-client`)
- DB client libraries: `pg`, `pgvector`
- Auth: `openid-client` for OIDC
- Security & middleware: `helmet`, `cors`, `express-session`, `express-rate-limit`, `compression`, `morgan`
- Build/test: `vitest`, `vite`, `prettier` and `svelte-check`

Runtime / deployment notes:

- `server/` expects TLS key/cert paths via environment (`HMD_TLS_KEY`, `HMD_TLS_CERT`) and host/port via `HMD_HOST`/`HMD_PORT`.
- DB connection supports `HMD_DATABASE_URL` or standard `PG*` env vars.
