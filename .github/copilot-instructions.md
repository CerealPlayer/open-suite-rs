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
  5. Build Axum routes from `open_suite_rs::router::router()`.
  6. Attach shared state (`Conns`) via `.with_state(...)`.
  7. Serve HTTP on `0.0.0.0:3000` with graceful shutdown on Ctrl+C/SIGTERM.
- `src/lib.rs` exports module roots: `router`, `storage`, and `entities`.
- Router state is defined in `src/router/state.rs` as `Conns { bucket, db }`.
- Object storage logic is in `src/storage.rs` and uses `rust-s3`.

## Key conventions in this codebase

- Router state is strongly typed (`Router<Conns>`) and injected with `.with_state(...)`; handlers should pull dependencies from state instead of globals.
- Startup is fail-fast: required env vars are read with `expect(...)`, and infrastructure connections are established before serving traffic.
- S3 access uses **path-style** buckets (`with_path_style`) to stay compatible with the local RustFS endpoint in `docker-compose.yml`.
- Bucket provisioning happens during boot (`get_bucket` checks existence and creates when missing), so storage setup is part of application startup.
- `Bucket` is aliased as `Box<s3::Bucket>` in `storage.rs`; keep that alias when passing storage through state to preserve existing type ergonomics.

## Implemented HTTP surface

- `GET /health` returns `{ "status": "ok" }`.
- Document routes are nested under `/documents` (see `src/router/documents.rs`):
  - `GET /documents/`: list all document rows from PostgreSQL.
  - `POST /documents/upload`: accept multipart upload, require DOCX MIME type, upload bytes to S3, and persist metadata in DB.
  - `GET /documents/{documentId}`: fetch one document by UUID or return 404.
- API errors are JSON objects shaped as `{ "error": "<message>" }` with appropriate status codes (400/404/500).

## Data model and persistence

- SeaORM entity: `src/entities/document.rs` mapped to `documents` table.
- Current `documents` model fields:
  - `id` (UUID primary key, no auto-increment)
  - `path` (text, S3 object path)
  - `file_name` (text, original normalized file name)
  - `created_at`, `updated_at`, `deleted_at`
- SQL migrations live in `migrations/`:
  - `20260311223753-init.sql` creates the base `documents` table.
  - `20260312194000-add-file-name-to-documents.sql` adds/backfills `file_name`.
