# Rustle

A small, modern Kanban app — boards, columns, cards, comments, labels — built as a single Rust binary that serves both a JSON API and server-rendered HTML pages.

---

## The stack

| Layer | What it is |
|---|---|
| **Web framework** | [Axum](https://github.com/tokio-rs/axum) 0.7 — async HTTP, type-safe extractors, tower middleware |
| **Async runtime** | [Tokio](https://tokio.rs) 1 — powers Axum |
| **Database** | PostgreSQL (16+) accessed through [sqlx](https://github.com/launchbadge/sqlx) 0.8 with **compile-time-checked** queries and built-in migrations |
| **Auth** | Server-side sessions stored in Postgres, opaque random token in an `HttpOnly` cookie. Passwords hashed with **Argon2id** |
| **Cookies** | [`tower-cookies`](https://docs.rs/tower-cookies) middleware |
| **Static files** | [`tower-http`](https://docs.rs/tower-http) `ServeDir` mounts `/static` from the `public/` folder |
| **HTML** | [Askama](https://github.com/djc/askama) compile-time templates (Jinja-like syntax) |
| **Frontend** | Plain HTML + CSS + a small amount of vanilla JavaScript using `fetch()` — no framework, no build step |
| **Validation** | `validator` crate — email + length checks on all request DTOs |
| **Logging** | `tracing` + `tracing-subscriber` |
| **Tests** | `#[sqlx::test]` (per-test fresh database, automatic migrations) + `reqwest` client driving the real router |

You only need `cargo` and a running Postgres on your machine. There is no bundler, no Node, no Docker.

---

## How to run it

### Prerequisites

- Rust (any recent stable, e.g. `1.80+`)
- PostgreSQL 14+ running locally with a user named `joseph` (no password)

The connection details live in `.env`:

```env
DATABASE_URL=postgres://joseph@localhost:5432/rustle
APP_HOST=127.0.0.1
APP_PORT=7070
COOKIE_SECURE=false
```

### Dev

```bash
# 1. Create + seed the database (only needs to be done once, or whenever you want fresh demo data)
cargo run --bin seed

# 2. Start the server with auto-restart on file change
cargo run

# Or, if you have `cargo-watch` installed, get hot-reload:
cargo install cargo-watch    # one-time
cargo watch -x run
```

The server prints its address on startup. Open <http://127.0.0.1:7070> — you'll see the landing page. Sign in with the demo credentials the seeder prints (`ada@rustle.dev` / `password123`).

### Production

Build a release binary and run it directly. No external runtime needed.

```bash
# Optimised build (slow first time, fast after)
cargo build --release

# Run with production-friendly env. Replace creds for your real DB.
DATABASE_URL=postgres://user:pass@db.host:5432/rustle \
APP_HOST=0.0.0.0 \
APP_PORT=7070 \
COOKIE_SECURE=true \
RUST_LOG=rustle=info,tower_http=warn \
./target/release/rustle
```

Notes:
- `COOKIE_SECURE=true` forces session cookies to require HTTPS — only set it when serving behind TLS.
- Migrations run automatically at startup.
- Put it behind nginx / Caddy / Cloudflare for TLS termination.

---

## How to run the tests

```bash
# Uses the test database listed in .env.test (postgres://joseph@localhost:5432/rustle_test)
cargo test
```

What runs:

- **Unit tests** in `src/auth/password.rs` (Argon2 hash + verify round-trip).
- **Integration tests** in `tests/`:
  - `auth.rs` — register / login / logout / change password / duplicate email rejection / wrong password rejection
  - `boards.rs` — full CRUD, plus cross-user isolation (one user cannot see or delete another user's board)
  - `cards.rs` — card moves between columns reorder positions correctly, deletes cascade to comments, labels can be attached
  - `pages.rs` — landing renders, dashboard redirects when logged out, healthz, static CSS served

Each integration test spins up the real Axum router on a random port against a **fresh, migrated database** — no mocks. You will see ~16 tests pass.

---

## Seeding the database

```bash
cargo run --bin seed
```

What it does, in order:

1. Reads `DATABASE_URL` from `.env` (or the env).
2. **If the database doesn't exist, it creates it** — no need to `createdb` by hand.
3. Runs all SQL migrations from `migrations/`.
4. Wipes any existing rows (`TRUNCATE users CASCADE`).
5. Inserts:
   - Two users — `ada@rustle.dev` and `turing@rustle.dev`, both with password `password123`
   - One board owned by Ada, with three columns and five demo cards
   - One comment and one `urgent` label attached to the first card

Re-run the seed any time you want a clean demo state.

---

## Folder structure

```
rust-app/
├── Cargo.toml              # crate manifest + dependencies
├── .env                    # dev DB URL, server host/port, log level
├── .env.test               # DB URL used by `cargo test`
├── README.md               # this file
│
├── migrations/             # SQL migrations (applied at startup + by seed/tests)
│   ├── 0001_init.sql       # users, sessions, boards, board_columns, cards, comments, labels
│   └── 0002_indexes.sql    # supporting indexes
│
├── public/                 # served at /static via tower_http::ServeDir — no build step
│   ├── css/app.css         # all styles (uses CSS custom properties for theming)
│   ├── js/auth.js          # login / register / logout glue + shared `RustleApi.json` helper
│   ├── js/dashboard.js     # "New board" modal
│   ├── js/board.js         # Kanban board: drag-drop cards, inline rename, add/delete columns
│   ├── js/settings.js      # change-password form
│   └── img/logo.svg
│
├── templates/              # Askama (Jinja-like) HTML, compiled at build time
│   ├── base.html           # shared shell: top nav, footer, script slots
│   ├── landing.html        # marketing page (/)
│   ├── login.html          # /login
│   ├── register.html       # /register
│   ├── dashboard.html      # /dashboard — list of boards + new-board modal
│   ├── board.html          # /board/:id — Kanban view
│   └── settings.html       # /settings — account + change password
│
├── src/
│   ├── main.rs             # entry point: env, tracing, pool, migrations, listener
│   ├── lib.rs              # re-exports modules; `pub fn app(pool)` used by tests
│   ├── config.rs           # AppConfig parsed from environment
│   ├── state.rs            # AppState shared with every handler (pool + config)
│   ├── error.rs            # AppError enum + IntoResponse — uniform JSON error envelopes
│   ├── router.rs           # ⭐ single source of truth: every route is registered here
│   │
│   ├── auth/
│   │   ├── mod.rs
│   │   ├── password.rs     # Argon2id hash + verify (with unit tests)
│   │   ├── session.rs      # session token generation, DB lookup, cookie helpers
│   │   └── middleware.rs   # require_user middleware + CurrentUser extractor
│   │
│   ├── models/             # row structs (sqlx::FromRow) + serde::Serialize
│   │   ├── user.rs board.rs column.rs card.rs comment.rs label.rs
│   │
│   ├── handlers/           # one file per resource — request DTOs live next to handlers
│   │   ├── pages.rs        # the 6 server-rendered HTML routes
│   │   ├── auth.rs         # /api/auth/{register,login,logout,me,password}
│   │   ├── boards.rs       # /api/boards…
│   │   ├── columns.rs      # /api/boards/:id/columns + /api/columns/:id
│   │   ├── cards.rs        # /api/columns/:id/cards + /api/cards/:id (incl. move)
│   │   ├── comments.rs     # /api/cards/:id/comments + /api/comments/:id
│   │   ├── labels.rs       # /api/labels + attach/detach on a card
│   │   └── health.rs       # /healthz
│   │
│   └── bin/
│       └── seed.rs         # `cargo run --bin seed` — creates DB + demo data
│
└── tests/
    ├── common/mod.rs       # spawns the real app on a random port
    ├── auth.rs boards.rs cards.rs pages.rs
```

### Files worth knowing

- **`src/router.rs`** — when you want to know "what routes exist?", read this. Every URL the app responds to is here, plus which middleware applies.
- **`src/error.rs`** — every handler returns `AppResult<T>`. Errors map to consistent JSON for `/api/*` and to a `/login` redirect for unauthenticated page requests.
- **`src/auth/middleware.rs`** — the `require_user` middleware that gates protected routes; also exposes `CurrentUser` so handlers can write `CurrentUser(user): CurrentUser` in their signature.
- **`migrations/0001_init.sql`** — the canonical schema. If you add a column, write `migrations/0003_*.sql` rather than editing this file.
- **`templates/base.html`** — every page extends this, so global layout changes happen in one place.
- **`public/css/app.css`** — top of the file declares CSS custom properties (`--accent`, `--bg`, etc.); change those to retheme.

---

## License

MIT.
