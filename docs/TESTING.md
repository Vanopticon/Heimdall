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
