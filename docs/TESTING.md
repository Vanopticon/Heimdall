# Running Tests (grouped by feature)

This repository groups tests by Cargo features so you can run small, fast sets by default and enable heavier groups when needed.

Default (fast) tests

- `cargo test` runs the default (fast) test group, which includes `unit-tests`.

Feature groups

- `unit-tests` (default): small, fast unit and compile-time smoke tests.
- `ingest-tests`: tests for ingestion/normalization code (NDJSON/CSV handlers).
- `devops-tests`: tests that touch development tooling (docker compose detection, etc.).
- `integration-tests`: slower tests intended for environments with external services (database clients, etc.).
- `all-tests`: enables all the above groups.

How to run particular groups

- Run unit tests (default):

```bash
cargo test
```

- Run ingest tests only:

```bash
cargo test --features ingest-tests
```

- Run devops tests only:

```bash
cargo test --features devops-tests
```

- Run integration tests only:

```bash
cargo test --features integration-tests
```

- Run everything (all groups):

```bash
cargo test --features all-tests
```

Notes

- Grouping tests this way allows CI to select which groups to run (e.g., run `ingest-tests` on PRs that touch ingestion code) and keeps local runs fast by default.
- If you need to enable any of these feature groups together, use a comma-separated list: `cargo test --features "ingest-tests,devops-tests"`.
- The features are declared in `Cargo.toml` under the `[features]` section.

Feature note

- By default Cargo enables the crate's default features when running with `--features`. Our default feature set includes `unit-tests`. If you want to run a specific group _without_ the default `unit-tests`, use `--no-default-features`, for example:

```bash
cargo test --no-default-features --features ingest-tests
```

## Integration and End-to-End Tests

### Overview

Integration and e2e tests validate complete workflows against an ephemeral Postgres+AGE database. These tests are gated by the `RUN_DOCKER_INTEGRATION_TESTS` environment variable to avoid running Docker containers in environments where Docker is unavailable or undesired.

### Test Structure

Integration tests are located in the `tests/` directory:

- `integration_dev_db.rs`: Basic database connectivity and AGE client operations
- `integration_e2e.rs`: End-to-end workflows (ingest → persist → verify)
- `integration_tls.rs`: TLS configuration validation

### Running Integration Tests Locally

**Prerequisites:**

- Docker and `docker-compose` (or `docker compose` v2) installed and running
- Network access to pull the Postgres+AGE image

**Step 1: Enable integration tests**

Set the environment variable to enable Docker-based tests:

```bash
export RUN_DOCKER_INTEGRATION_TESTS=1
```

**Step 2: Run the tests**

Run all integration tests:

```bash
cargo test --features integration-tests
```

Run a specific integration test:

```bash
cargo test --features integration-tests --test integration_e2e
```

**Step 3: Cleanup (optional)**

The test harness automatically cleans up the Docker containers it starts. If you need to manually stop the database:

```bash
./docker/scripts/stop-postgres.sh
```

### Test Harness Components

#### Docker Manager (`src/devops/docker_manager.rs`)

Provides programmatic control over the development database:

- `start_dev_db()`: Starts the Postgres+AGE container using docker-compose
- `stop_dev_db()`: Stops and removes the container (only if started by the test harness)
- `start_dev_db_with_opts()`: Advanced start with custom options

The harness uses a marker file (`.heimdall_db_started`) to track which containers were started by tests to avoid stopping externally-managed databases.

#### Helper Scripts

Located in `docker/scripts/`:

- `start-postgres.sh`: Shell script to start the database (alternative to programmatic control)
- `stop-postgres.sh`: Shell script to stop the database (only if marker exists)

### Writing Integration Tests

**Basic pattern:**

```rust
use std::env;

#[tokio::test]
async fn my_integration_test() {
    // Gate the test
    if env::var("RUN_DOCKER_INTEGRATION_TESTS").is_err() {
        eprintln!("Skipping Docker integration test; set RUN_DOCKER_INTEGRATION_TESTS=1");
        return;
    }

    // Start database
    vanopticon_heimdall::devops::start_dev_db()
        .await
        .expect("start db");

    // Wait for database to be ready
    let pool = loop {
        match sqlx::PgPool::connect("postgres://heimdall:heimdall@127.0.0.1:5432/heimdall").await {
            Ok(p) => break p,
            Err(_) => tokio::time::sleep(tokio::time::Duration::from_secs(1)).await,
        }
    };

    // Your test logic here
    // ...

    // Cleanup
    vanopticon_heimdall::devops::stop_dev_db()
        .await
        .expect("stop db");
}
```

**Best practices:**

1. Always check `RUN_DOCKER_INTEGRATION_TESTS` before running Docker-dependent code
2. Always call `stop_dev_db()` in cleanup, even if the test fails (use defer patterns or ensure blocks)
3. Use reasonable timeouts when waiting for database readiness
4. Clean up any test data to avoid side effects between tests

### Test Coverage

Current integration test scenarios:

- **Database connectivity**: Verify AGE client can connect and execute Cypher queries
- **NDJSON ingest → persistence**: Upload NDJSON, verify nodes are created in the graph
- **Enrichment mocking**: Mock enrichment providers and verify enrichment nodes (future)
- **Sync partition**: Test multi-Heimdall synchronization scenarios (future)
- **TLS validation**: Verify TLS 1.3 enforcement and certificate validation

### Continuous Integration

Integration tests run in CI via GitHub Actions when:

- The `RUN_DOCKER_INTEGRATION_TESTS` environment variable is set
- Docker is available in the CI environment
- The workflow is configured to start services

See `.github/workflows/` for CI configuration.

### Troubleshooting

**Tests hang during database connection:**

- Ensure Docker is running and accessible
- Check if the container started successfully: `docker ps | grep heimdall-postgres-age`
- Verify port 5432 is not already in use: `lsof -i :5432` (macOS/Linux) or `netstat -ano | findstr 5432` (Windows)

**Tests fail with "docker compose not found":**

- Install Docker Compose: `docker compose version` or `docker-compose --version`
- Ensure Docker daemon is running

**Database state persists between test runs:**

- The test harness uses the marker file to track containers
- If tests are interrupted, manually clean up: `./docker/scripts/stop-postgres.sh`
- To force a clean state: `docker-compose down -v` (removes volumes)

**Permission errors:**

- Ensure your user has Docker permissions (may need to add user to `docker` group on Linux)
- On Windows/macOS, ensure Docker Desktop is running

### Database Schema and Extensions

The test database is automatically configured with:

- **Apache AGE**: Graph database extension for Cypher queries
- **pgvector**: Vector similarity search extension
- **Default graph**: `heimdall_graph` created during initialization

See `docker/postgres-age/initdb/01-extensions.sql` for the initialization script.
