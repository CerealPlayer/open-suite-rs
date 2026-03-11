# Copilot Instructions for `open-suite-rs`

## Build, test, and lint commands

- Build: `cargo build`
- Fast compile check: `cargo check`
- Run all tests: `cargo test`
- Run a single unit test by name: `cargo test <test_name>`
- Run a single integration-test target (if added under `tests/`): `cargo test --test <test_file_stem> <test_name>`
- Lint: `cargo clippy --all-targets --all-features`
- Format: `cargo fmt`
- Run app locally: `cargo run` (requires `DATABASE_URL`, `S3_REGION`, `S3_ENDPOINT`)
- Start local dependencies: `docker compose up -d` (Postgres + RustFS S3-compatible service)

## High-level architecture

- The binary entrypoint is `src/main.rs`. Startup flow:
  1. Load `.env` via `dotenvy`.
  2. Build S3 region/endpoint from `S3_REGION` + `S3_ENDPOINT`.
  3. Open PostgreSQL connection from `DATABASE_URL` using SeaORM.
  4. Ensure an S3 bucket exists via `storage::get_bucket("test", region)`.
  5. Assemble shared state (`Conns`) and attach it to Axum router.
  6. Serve HTTP on `0.0.0.0:3000`.
- Routing lives in `src/lib.rs` via `get_router()`. Current surface is `/health`.
- Shared app state is defined in `src/config.rs` as `Conns { bucket, db }`.
- Object storage logic is isolated to `src/storage.rs` and uses `rust-s3`.

## Key conventions in this codebase

- Router state is strongly typed (`Router<Conns>`) and injected with `.with_state(...)`; handlers should pull dependencies from state instead of globals.
- Startup is fail-fast: required env vars are read with `expect(...)`, and infrastructure connections are established before serving traffic.
- S3 access uses **path-style** buckets (`with_path_style`) to stay compatible with the local RustFS endpoint in `docker-compose.yml`.
- Bucket provisioning happens during boot (`get_bucket` checks existence and creates when missing), so storage setup is part of application startup.
- `Bucket` is aliased as `Box<s3::Bucket>` in `storage.rs`; keep that alias when passing storage through state to preserve existing type ergonomics.
