# Design Choices — Heimdall (finalized)

This file captures the final, accepted crate choices and the denied alternatives. Use this as the source when adding dependencies or implementing modules.

Accepted stacks (approved by team):

- HTTP runtime and server
    + `tokio` (already present)
    + `axum` (already present)

- HTTP client
    + `reqwest` — async, tokio-native, TLS support, easy to use for enrichment and sync push/pull.

- DB
    + PostgreSQL + Apache AGE + `pgvector` — recommended for graph+vector workloads. Use a local Docker container for development and integration tests (see `docker-compose.yml` and `docker/postgres-age/`).

- Serialization/Parsing
    + `serde`, `serde_json` (already present)
    + `csv` crate for streaming CSV parsing (add when implementing CSV adapter)

- PII / Envelope encryption
    + RustCrypto AEAD crates (recommend `xchacha20poly1305` or `aes-gcm`), plus `secrecy` + `zeroize` for secret handling.

- JWT / OIDC
    + `jsonwebtoken` for token verification
    + `openidconnect` for discovery and advanced OIDC flows

- Observability & Tracing
    + `opentelemetry` and related crates for traces/OTLP if required
    + NOTE: `tracing` and `tracing-subscriber` are intentionally _not_ used for log output because the project already uses `fern` for structured logging.

- Testing & Mocks
    + `wiremock` for HTTP provider mocks
    + `tokio::test` for async tests
    + Use Docker Compose to spin up ephemeral Postgres+AGE containers for integration tests (see `docker-compose.yml`).

- Secrets
    + `secrecy` + `zeroize`

- Circuit Breaker
    + Circuit breaker approved; implement an in-process circuit-breaker (small state machine) or adopt a lightweight community crate if available.

Denied / Not-in-use (per team):

- `tracing`/`tracing-subscriber` for primary logs (FERN in use instead).
- `governor` rate-limiter (not approved).
- `backoff` / `retry` (tower already in use for middleware/retries).
- `tonic` / gRPC — not approved (HTTPS/JSON for sync transport).

Notes & next steps

- Add `reqwest`, `jsonwebtoken`, `openidconnect`, `xchacha20poly1305` (or preferred AEAD), `secrecy`, `zeroize`, `opentelemetry` and `wiremock` to `Cargo.toml` as the next step.
- Implement small `src/lib/crypto` wrapper around chosen AEAD + envelope encryption pattern.
-- Use Docker-based Postgres+AGE for integration tests; `testcontainers` or `docker-compose` are acceptable for CI/local runs.

Example Cargo additions (no versions pinned here; pick compatible versions with existing crates):

```
reqwest = { features = ["json", "rustls-tls" ] }
xchacha20poly1305 = "*"
secrecy = "*"
zeroize = "*"
jsonwebtoken = "*"
openidconnect = "*"
opentelemetry = "*"
opentelemetry-otlp = "*"
wiremock = "*" # dev-dependency
```

If you want, I can add a minimal `Cargo.toml` snippet with exact versions compatible with your current `tokio`/`axum` stack and open a PR locally (not pushing to remote per your preference).
