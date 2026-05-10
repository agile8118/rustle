# Rustle — Backend Reference for iOS Client

This document is the **complete contract** between the Rustle backend and any client (iOS, web, CLI). It is self-contained: read it end-to-end and you have everything needed to build a client without reading the server source.

---

## 1. What Rustle is

Rustle is a multi-tenant Kanban task tracker. Each user owns:

- One or more **boards** (e.g. "Launch checklist", "Personal todos")
- Each board has ordered **columns** (e.g. "To Do", "In Progress", "Done")
- Each column has ordered **cards** (the work items themselves)
- Cards can have **comments** and **labels**

Boards are private. A user only ever sees their own data — there is no sharing model and no organisation/team layer (yet).

---

## 2. Deployment & connection model (iOS-specific)

The backend is a single self-contained Rust binary (`rustle`) that students will deploy to their own infrastructure (typically AWS). Each student's deployment lives at its own domain.

The iOS app must therefore:

1. **Let the user enter and store a server base URL** (e.g. `https://kanban.alice.dev`). Save it in `UserDefaults` under a key like `RustleServerURL`.
2. **Validate the URL on save** by calling `GET {base}/healthz` and confirming it returns the expected JSON (see §10). Reject the save if the call fails or the response shape is wrong — this is the only safe way to catch typos.
3. **Use HTTPS only.** App Transport Security blocks plain HTTP. Don't add an ATS exception — server operators must use HTTPS (they will, the README tells them so).
4. **Show a clear "Connect to your Rustle server" first-launch screen** if no URL is configured. App Store reviewers reject blank/error first screens.
5. **Provide an "App Review" demo URL + credentials** to Apple in the App Store Connect submission — reviewers won't deploy a server.

All endpoints below are relative to the user-configured base URL.

---

## 3. Authentication: server-side sessions over HttpOnly cookies

### How it works

- On `POST /api/auth/register` or `POST /api/auth/login`, the server generates a 32-byte random token (hex-encoded), stores it in the `sessions` table with a 30-day expiry, and sets it as an `HttpOnly`, `SameSite=Lax`, `Path=/` cookie named **`session`**.
- Every subsequent authenticated request must carry this cookie. The middleware reads it, joins `sessions × users WHERE expires_at > now()`, and on hit injects a `CurrentUser` into the request.
- `POST /api/auth/logout` deletes the row from `sessions` and clears the cookie.
- `DELETE /api/auth/me` deletes the user (cascade-deleting all their data) and clears the cookie.

### iOS implementation notes

- **`URLSession` handles this automatically.** Its default `httpCookieStorage` persists cookies across app launches. No JWT decoding, no `Authorization` headers — just configure a single `URLSession` for the entire app and it Just Works.
- **Send credentials on every request:** ensure `URLRequest.httpShouldHandleCookies = true` (this is the default).
- **On a 401 response**, treat the user as logged out: drop any cached state, return them to the Login screen.
- **Don't read the cookie value yourself.** It's `HttpOnly`-flagged on the server intent, but that flag is web-only; on iOS the cookie is in `HTTPCookieStorage` and you could in principle read it. Don't bother — there's nothing to do with it.
- **App Store Guideline 5.1.1(v):** if you let users create accounts, you must let them delete in-app. Use `DELETE /api/auth/me` (§7.1).

---

## 4. Request/response conventions

- All request and response bodies are **JSON** (`application/json`).
- Response timestamps are **ISO-8601 UTC**, e.g. `"2026-05-10T19:30:26.412499Z"`.
- IDs are **UUIDs** (string), e.g. `"763e45e9-cd38-4066-9240-b612cf40558b"`.
- All write endpoints validate input — see per-endpoint constraints.
- On success, endpoints return the resource as JSON (or `{"ok": true}` for delete-style operations).
- On error, endpoints return the **error envelope** described in §9.

---

## 5. Data models (server JSON shapes)

### `User` (returned as `PublicUser` — never includes the password hash)

```json
{
  "id": "uuid",
  "email": "string (lowercased; unique)",
  "display_name": "string (1-80 chars)",
  "created_at": "iso-8601"
}
```

### `Board`

```json
{
  "id": "uuid",
  "owner_id": "uuid",
  "title": "string (1-120 chars)",
  "created_at": "iso-8601"
}
```

### `BoardColumn`

```json
{
  "id": "uuid",
  "board_id": "uuid",
  "title": "string (1-80 chars)",
  "position": 0,
  "created_at": "iso-8601"
}
```

### `Card`

```json
{
  "id": "uuid",
  "column_id": "uuid",
  "title": "string (1-200 chars)",
  "description": "string (default '')",
  "position": 0,
  "due_at": "iso-8601 | null",
  "created_at": "iso-8601"
}
```

### `Comment`

```json
{
  "id": "uuid",
  "card_id": "uuid",
  "author_id": "uuid",
  "body": "string (1-5000 chars)",
  "created_at": "iso-8601"
}
```

### `Label`

```json
{
  "id": "uuid",
  "owner_id": "uuid",
  "name": "string (1-40 chars)",
  "color": "string (hex like '#ff6a3d', 4-9 chars)"
}
```

### `BoardDetail` (returned by `GET /api/boards/:id`) — flattened nesting

```json
{
  "id": "uuid",
  "owner_id": "uuid",
  "title": "string",
  "created_at": "iso-8601",
  "columns": [
    {
      "id": "uuid",
      "board_id": "uuid",
      "title": "string",
      "position": 0,
      "created_at": "iso-8601",
      "cards": [ { /* Card */ }, ... ]
    },
    ...
  ]
}
```

Note the flatten: the `Board` fields and `columns` are at the same level. Each column has its `cards` array embedded.

---

## 6. Behavior notes / quirks the iOS client must respect

These are non-obvious behaviours hard-coded into the server. Build the iOS UI assuming them.

1. **Creating a board auto-creates 3 columns** named `"To Do"`, `"In Progress"`, `"Done"` at positions 0, 1, 2. Don't pre-fill them on the client.
2. **Card positions are integer indexes within their column**, starting at 0 and contiguous. Don't try to insert at arbitrary fractional positions.
3. **Card move (`POST /api/cards/:id/move`)** does the position arithmetic transactionally:
   - Closes the gap in the source column (decrements `position` for every card after the moved one).
   - Opens a slot in the target column at the requested position.
   - Sets the card's new `column_id` and `position`.
   - Result: positions stay contiguous. **The client should re-fetch `GET /api/boards/:id` after a move** to get the canonical state — or apply the same arithmetic optimistically and reconcile on the next refresh.
4. **Cross-user access returns 404, never 403.** If user A asks for user B's board/card/column/comment, the server says "not found" — it does not reveal that the resource exists. The iOS UI should therefore present these as plain "not found" errors.
5. **Comment delete is author-scoped.** A user can only delete comments they wrote. A 404 means either the comment doesn't exist or wasn't authored by this user.
6. **Email is case-insensitive and unique.** The DB stores it as `CITEXT`. `Alice@x.com` and `alice@x.com` are the same account. Lowercase on the client when displaying to keep it tidy, but don't transform on submit — the server handles it.
7. **`column_id` in URLs vs `board_columns` in SQL.** The server table is `board_columns` because `columns` is a SQL keyword in many contexts. The JSON field is `column_id` everywhere. You will only see `column_id` from the API.
8. **Default Argon2id parameters.** Password hashing is done server-side; the client just sends the plaintext over HTTPS.
9. **Session lifetime is 30 days, sliding only at re-login.** The server does not refresh the cookie expiry on every request. If a user hasn't logged in for 31 days, they will get 401 and need to log in again.
10. **`due_at` is optional.** When updating a card, setting `due_at: null` clears the due date; omitting the field leaves it unchanged. (This is a `Option<Option<DateTime>>` deserialization quirk on the server side — the iOS encoder needs to actually emit `"due_at": null` to clear, vs omit the key to leave alone.)

---

## 7. Endpoint reference

Every endpoint accepts and returns JSON. **Auth column:**
- ✗ = public (no cookie required)
- ✓ = requires a valid session cookie; returns 401 if missing/expired

### 7.1 Auth

| Method | Path | Auth | Notes |
|---|---|---|---|
| POST | `/api/auth/register` | ✗ | Creates user, sets session cookie, returns `PublicUser`. Returns **409** if email already in use. |
| POST | `/api/auth/login` | ✗ | Validates credentials, sets session cookie, returns `PublicUser`. Returns **401** on bad email or bad password (no enumeration). |
| POST | `/api/auth/logout` | ✓ | Revokes the session, clears the cookie, returns `{"ok": true}`. Always 200 if cookie is present. |
| GET | `/api/auth/me` | ✓ | Returns the current `PublicUser`. |
| PATCH | `/api/auth/password` | ✓ | Changes password. Body `{ "current": "...", "new": "..." }`. Returns 401 if `current` is wrong. |
| **DELETE** | **`/api/auth/me`** | ✓ | **Deletes the account and all data.** Cascades through `sessions`, `boards`, `columns`, `cards`, `comments`, `labels`, `card_labels`. Clears the cookie. Returns `{"ok": true}`. **Required for App Store compliance (Guideline 5.1.1(v)).** |

#### Request bodies

```jsonc
// POST /api/auth/register
{
  "email": "alice@example.com",       // valid email
  "password": "atleast8chars",        // 8-256 chars
  "display_name": "Alice Liddell"     // 1-80 chars
}

// POST /api/auth/login
{ "email": "alice@example.com", "password": "atleast8chars" }

// POST /api/auth/logout
{}                                    // no body fields used

// PATCH /api/auth/password
{ "current": "old", "new": "newpass8" }
```

### 7.2 Boards

| Method | Path | Auth | Returns |
|---|---|---|---|
| GET | `/api/boards` | ✓ | `Board[]` ordered `created_at DESC` |
| POST | `/api/boards` | ✓ | `Board` (also creates 3 default columns — see §6.1) |
| GET | `/api/boards/:id` | ✓ | `BoardDetail` (board + columns + cards). 404 if not yours. |
| PATCH | `/api/boards/:id` | ✓ | Updated `Board`. 404 if not yours. |
| DELETE | `/api/boards/:id` | ✓ | `{"ok": true}`. Cascade-deletes columns, cards, comments. 404 if not yours. |

```jsonc
// POST /api/boards  /  PATCH /api/boards/:id
{ "title": "Launch checklist" }       // 1-120 chars
```

### 7.3 Columns

| Method | Path | Auth | Notes |
|---|---|---|---|
| POST | `/api/boards/:id/columns` | ✓ | Appends a column at the end. Returns `BoardColumn`. |
| PATCH | `/api/columns/:id` | ✓ | Update title and/or position. Returns updated `BoardColumn`. |
| DELETE | `/api/columns/:id` | ✓ | Deletes column + cascades to cards/comments. |

```jsonc
// POST /api/boards/:id/columns
{ "title": "Backlog" }                // 1-80 chars

// PATCH /api/columns/:id   — both fields optional
{ "title": "WIP", "position": 1 }
```

### 7.4 Cards

| Method | Path | Auth | Notes |
|---|---|---|---|
| POST | `/api/columns/:id/cards` | ✓ | Appends a card at the end of the column. |
| GET | `/api/cards/:id` | ✓ | Returns `Card`. 404 if not in a board you own. |
| PATCH | `/api/cards/:id` | ✓ | Patch any of `title`, `description`, `due_at`. |
| POST | `/api/cards/:id/move` | ✓ | Cross-column move with position rebalancing (§6.3). |
| DELETE | `/api/cards/:id` | ✓ | Deletes card + cascades to comments. Reflows positions. |

```jsonc
// POST /api/columns/:id/cards
{ "title": "Audit a11y", "description": "Tab order, focus rings." }
//        title: 1-200 chars,  description optional

// PATCH /api/cards/:id  — every field optional; semantics in §6.10
{ "title": "...", "description": "...", "due_at": "2026-06-01T12:00:00Z" }

// POST /api/cards/:id/move
{ "column_id": "uuid", "position": 0 }
```

### 7.5 Comments

| Method | Path | Auth | Notes |
|---|---|---|---|
| GET | `/api/cards/:id/comments` | ✓ | `Comment[]` ordered `created_at ASC`. |
| POST | `/api/cards/:id/comments` | ✓ | Creates a comment with the current user as author. |
| DELETE | `/api/comments/:id` | ✓ | **Author-scoped.** 404 if you didn't write it. |

```jsonc
// POST /api/cards/:id/comments
{ "body": "Looks good to me." }       // 1-5000 chars
```

### 7.6 Labels

| Method | Path | Auth | Notes |
|---|---|---|---|
| GET | `/api/labels` | ✓ | All labels owned by the user, ordered by name. |
| POST | `/api/labels` | ✓ | Creates a label. Returns `Label`. |
| POST | `/api/cards/:id/labels` | ✓ | Attaches an existing owned label to a card. Idempotent. |
| DELETE | `/api/cards/:id/labels/:label_id` | ✓ | Detaches a label from a card. |

```jsonc
// POST /api/labels
{ "name": "urgent", "color": "#dc2626" }   // color optional, default '#888888'

// POST /api/cards/:id/labels
{ "label_id": "uuid" }
```

### 7.7 Health

| Method | Path | Auth | Returns |
|---|---|---|---|
| GET | `/healthz` | ✗ | `{ "status": "ok", "db": "ok" \| "error" }` |

Use this on the iOS Settings screen to validate the server URL the user typed.

---

## 8. Server-rendered web pages (irrelevant to iOS, listed for completeness)

The same backend serves a web frontend on `/`, `/login`, `/register`, `/dashboard`, `/board/:id`, `/settings`, plus `/static/*` for assets. The iOS app should ignore these and only hit `/api/*` and `/healthz`.

---

## 9. Error envelope

Every error response from `/api/*` looks like:

```json
{ "error": "code", "message": "human-readable detail" }
```

| HTTP status | `error` code | When |
|---|---|---|
| 400 | `validation` | Request body failed validation (length, format). `message` describes which field. |
| 401 | `unauthorized` | No session cookie, expired session, or wrong password on login. |
| 403 | `forbidden` | Reserved (currently unused — server returns 404 for cross-user attempts). |
| 404 | `not_found` | Resource doesn't exist or isn't owned by the current user. |
| 409 | `conflict` | Duplicate email on registration. |
| 500 | `internal` | Unexpected server / DB error. Show generic "Something went wrong." on iOS. |

### iOS handling cheatsheet

- **401** → wipe local user state, push the Login screen on top. Don't auto-retry.
- **404** → show "couldn't find that" inline. Don't differentiate "doesn't exist" from "not yours" — that's intentional.
- **409** on register → show "Email already in use".
- **400** → surface `message` near the offending input.
- **500** or network failure → generic toast/banner, offer Retry.

---

## 10. The first call iOS should make: `/healthz`

```
GET https://{server}/healthz
→ 200
{ "status": "ok", "db": "ok" }
```

When the user saves a server URL in Settings:
1. Call `GET /healthz`.
2. If non-2xx, show "Could not reach this server".
3. If body is not `{"status":"ok","db":"ok"}` (or `db == "error"`), show "Server is up but its database isn't ready".
4. Only persist the URL if both checks pass.

---

## 11. Constraints summary (for client-side validation)

| Field | Min | Max | Notes |
|---|---|---|---|
| Email | — | — | Must be a valid email (format check on server uses `validator` crate). |
| Password | 8 | 256 | At registration and password change. |
| Display name | 1 | 80 | |
| Board title | 1 | 120 | |
| Column title | 1 | 80 | |
| Card title | 1 | 200 | |
| Card description | 0 | unbounded | But keep reasonable for iOS UX. |
| Comment body | 1 | 5000 | |
| Label name | 1 | 40 | |
| Label color | 4 | 9 | Hex string like `#fff` or `#ff6a3d`. |

Mirror these in the iOS client; don't rely solely on server validation — give immediate feedback.

---

## 12. Suggested iOS architecture sketch

(Not prescriptive. Use what fits.)

- **`APIClient`** — wraps a single `URLSession` configured with `httpShouldSetCookies = true`. Exposes typed methods (`func login(email:password:) async throws -> User`, etc.). Handles base-URL composition from `UserDefaults`.
- **`Codable` model structs** mirroring §5. Use `JSONDecoder` with `.iso8601` date strategy.
- **`AuthState` (ObservableObject / Observable)** — tracks `case loggedOut, loggedIn(User), needsServerURL`. Drives the root view hierarchy.
- **`BoardStore`** — fetches `BoardDetail`, holds the optimistic local state for drags, calls `move` on commit.
- **Settings screen** — server URL input, validate via `/healthz`, "Sign out", **"Delete account"** (calls `DELETE /api/auth/me` after a destructive-action confirmation).
- **Drag-drop** — SwiftUI `.draggable` / `.dropDestination` for cards on iPadOS, or `UIKitDragController` for iPhone if you want the polish.
- **Empty / error states** — first-launch "Connect to your server", logged-out, no-boards, no-columns, no-cards, network-error. App Review will hit each one.

---

## 13. Testing the iOS client against a local server

1. On the dev Mac, run `cargo run` (server listens on `127.0.0.1:7070` by default).
2. iOS Simulator: set the server URL to `http://127.0.0.1:7070`.
   - **The Simulator can use plain HTTP for `127.0.0.1`** — but you'll need an ATS exception in `Info.plist` *only for development builds*:
     ```xml
     <key>NSAppTransportSecurity</key>
     <dict>
       <key>NSAllowsLocalNetworking</key><true/>
     </dict>
     ```
   - Strip this from production builds. Apple is fine with `NSAllowsLocalNetworking` (it's the right flag), and the production app will only ever talk to the user's HTTPS server.
3. To reset state: re-run `cargo run --bin seed` — gives you `ada@rustle.dev` / `password123`.

---

## 14. What's intentionally **not** in the backend

So the iOS client doesn't try to call them:

- No real-time updates (no WebSockets, SSE, or long-poll). Refresh on user action or on a manual pull-to-refresh.
- No push notifications endpoint. Add later if needed (would require APNs credentials server-side).
- No file/image attachments on cards.
- No sharing / collaborators / multi-user boards.
- No full-text search.
- No password reset flow (no email sender configured). Account recovery is manual / DB-side for now.
- No rate limiting. Don't hammer the server in tight loops.

---

## 15. Quick reference card

```
BASE = https://{user-configured-server}

Auth:
  POST   {BASE}/api/auth/register        {email,password,display_name}
  POST   {BASE}/api/auth/login           {email,password}
  POST   {BASE}/api/auth/logout          {}
  GET    {BASE}/api/auth/me
  PATCH  {BASE}/api/auth/password        {current,new}
  DELETE {BASE}/api/auth/me

Boards:
  GET    {BASE}/api/boards
  POST   {BASE}/api/boards               {title}
  GET    {BASE}/api/boards/:id
  PATCH  {BASE}/api/boards/:id           {title}
  DELETE {BASE}/api/boards/:id

Columns:
  POST   {BASE}/api/boards/:id/columns   {title}
  PATCH  {BASE}/api/columns/:id          {title?,position?}
  DELETE {BASE}/api/columns/:id

Cards:
  POST   {BASE}/api/columns/:id/cards    {title,description?}
  GET    {BASE}/api/cards/:id
  PATCH  {BASE}/api/cards/:id            {title?,description?,due_at?}
  POST   {BASE}/api/cards/:id/move       {column_id,position}
  DELETE {BASE}/api/cards/:id

Comments:
  GET    {BASE}/api/cards/:id/comments
  POST   {BASE}/api/cards/:id/comments   {body}
  DELETE {BASE}/api/comments/:id

Labels:
  GET    {BASE}/api/labels
  POST   {BASE}/api/labels               {name,color?}
  POST   {BASE}/api/cards/:id/labels     {label_id}
  DELETE {BASE}/api/cards/:id/labels/:label_id

Health:
  GET    {BASE}/healthz
```
