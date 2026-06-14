# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Rustle: a Kanban app (boards/columns/cards/comments/labels) — a single Rust binary serving a JSON API (`/api/*`) and server-rendered HTML pages (Askama templates) with vanilla JS/fetch. Axum + Tokio + sqlx (Postgres, compile-time-checked queries) + tower-cookies session auth (Argon2id passwords, opaque token cookie).

See `README.md` for the full stack rundown and folder structure.

## iOS client

This API also backs a native iOS app. If you change request/response shapes, status codes, the error envelope, or auth/cookie behavior on any `/api/*` endpoint, flag that the iOS client may need matching changes.

App Store constraints that shape the API:
- `DELETE /api/auth/me` must remain available and must cascade-delete all of the user's data + clear the session (Guideline 5.1.1(v) — in-app account deletion).
- `GET /healthz` is used by the iOS app to validate a user-entered server URL — keep its response shape stable.
- Auth relies on `HttpOnly` session cookies (`URLSession` cookie storage on iOS) — don't switch to a scheme (e.g. bearer tokens in response bodies) without updating the iOS client too.

## Commands

All commands go through `./do.sh`, which loads env vars via `env.sh` (from `.env` locally, AWS SSM `/rustle/prod/*` in prod):

```bash
./do.sh build   # cargo build --release
./do.sh run     # cargo run (loads .env), falls back to `pm2 startOrReload ecosystem.config.js` if no .env
./do.sh seed    # cargo run --bin seed — creates DB if missing, runs migrations, wipes + inserts demo data
./do.sh test    # loads .env.test, then `cargo test`
```

- A running Postgres is required (connection info in `.env` / `.env.test`).
- The seed binary creates the database itself if it doesn't exist (`ensure_database` in `src/bin/seed.rs` and `src/main.rs`).
- `SQLX_OFFLINE=true` is set via `.cargo/config.toml`, so `cargo build`/`cargo check` use the cached query metadata in `.sqlx/` instead of hitting a live DB. After changing a `sqlx::query!`/`query_as!` call, regenerate with `cargo sqlx prepare` (requires `DATABASE_URL` pointing at a real, migrated DB).
- Single test: `cargo test --test cards -- move_card_reorders` (env vars from `.env.test` must be loaded — `set -a; . ./.env.test; set +a` first, as `do.sh test` does).
- Integration tests in `tests/` each spin up the real router via `tests/common/mod.rs::spawn` on a random port against a fresh `#[sqlx::test]` database (auto-migrated) — no mocks.

## Architecture

- **`src/router.rs`** — single source of truth for routes. Three router groups are merged: `public_pages`/`auth_api_public`/`public_misc` (no auth), and `private_pages`+`private_api` merged together and wrapped with the `require_user` middleware via `route_layer`. Global layers (in order applied innermost-first): `CookieManagerLayer`, `CompressionLayer`, then the custom `access_log` middleware (outermost — logs every request with IP/method/uri/status/latency via `tracing::info!`).
- **`src/lib.rs`** — `app(pool)` (used by tests, default `AppConfig`) and `app_with_config(pool, config)` (used by `main.rs`) both call `build_router`. Any new top-level module must be declared here.
- **Auth flow**: `require_user` (in `src/auth/middleware.rs`) reads the `session` cookie, looks up `sessions × users WHERE expires_at > now()` (`src/auth/session.rs`), and injects `CurrentUser(User)` into request extensions. Handlers extract it via `CurrentUser(user): CurrentUser` (implements `FromRequestParts`). On rejection: `/api/*` paths get `AppError::Unauthorized` (401 JSON), HTML paths get a redirect to `/login`.
- **Error handling**: every handler returns `AppResult<T>` (`Result<T, AppError>` from `src/error.rs`). `AppError` variants map to status codes + a `{"error": code, "message": ...}` JSON envelope via `IntoResponse`. For page handlers, use `handle_page_error` so an `Unauthorized` becomes a `/login?next=...` redirect when the client wants HTML (checked via `Accept` header).
- **Handlers** (`src/handlers/`) — one file per resource; request DTOs (with `validator` derive) live alongside their handlers. `pages.rs` holds the 6 server-rendered routes (Askama templates from `templates/`).
- **Models** (`src/models/`) — `sqlx::FromRow` + `Serialize` row structs, one per table.
- **Migrations** (`migrations/`) — `0001_init.sql` is the canonical schema; never edit it — add `0003_*.sql` etc. for changes. Migrations run automatically at server startup and by tests/seed.
- **Logging**: `src/logging.rs` adds a `tracing_subscriber::Layer` (`file_log_layer()`) that writes plain-text logs to `logs/<YYYY-MM-DD>/info.log` and `logs/<YYYY-MM-DD>/errors.log` (ERROR-level events get a multi-line block with file/line). This runs alongside the normal stdout subscriber set up in `main.rs`.
- **Static files**: `public/` is served at `/static` via `tower_http::ServeDir` — no build step, edit CSS/JS directly.

## Cross-user data isolation

Boards (and everything under them — columns, cards, comments, labels) are scoped to `owner_id`. Any new query touching these tables must filter by the current user's ID — this is covered by tests in `tests/boards.rs` and `tests/cards.rs`.
