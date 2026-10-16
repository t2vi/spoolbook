# 008 — Accounts & auth

**Status:** Implemented (retroactive spec — see Notes)
**Area:** Accounts & auth (#147)
**Code:** `spoolbook-rs/src/{auth,google_oauth}.rs`; pages `/setup`, `/login`; account + Google
sections of `/settings`

## Summary

Spoolbook has real user accounts (single editor in practice). Nothing is visible without
signing in: every read needs a session, and every change — and the live camera — an editor. A fresh install runs a
setup wizard; an install that used the old shared admin password is migrated silently. A
signed-in user can link a Google account and then sign in with it.

## Key entities

- **User** (`users`) — unique `username`, argon2 `password_hash`, `role` (`Admin` — the only
  role, and the only one allowed to edit), optional unique `google_sub`.
- **Session** (`sessions`) — opaque random 256-bit token (hex) → user, expires 90 days after
  sign-in. Carried in the `spoolbook_session` cookie (`HttpOnly`, `SameSite=Lax`, `Path=/`).
- **Google config** (`app_settings`) — client id, client secret, redirect URI, set per install.

## Functional requirements

### Access rule

- **FR-001** — Every `/api` route requires a signed-in user (a valid, unexpired session);
  every mutating route, and `GET /printers/{id}/camera`, requires an editor (a signed-in
  `Admin`). Otherwise → 401 `{ authenticated: false }`. Routes marked *(signed in)* in these
  specs are reads that need a session. The only routes open without one are those needed to
  get signed in or to offer the source: `GET /api/setup-status`, `POST /api/setup` (FR-010),
  `POST /api/login`, `POST /api/logout`, `GET /api/me`, `GET /api/version`, and Google's
  `status`, `login` and `callback` (FR-030–033). *Verify: integration*

### Setup & sign-in

- **FR-010** — `GET /api/setup-status` (open) → `{ needsSetup }`, true only while no user
  exists. `POST /api/setup` `{ username, password }` creates the first `Admin` user and signs
  them in; once any user exists → 409 `already_set_up`; blank username → 400
  `username_required`; password under 8 chars → 400 `password_too_short`.
  *Verify: integration*
- **FR-011** — At startup, if no user exists and `SPOOLBOOK_ADMIN_PASSWORD` is set (non-empty),
  an `Admin` user `admin` is created with that password, so the setup wizard is skipped. Any
  existing user, or no env var → nothing happens. *Verify: integration*
- **FR-012** — `POST /api/login` `{ username, password }` → on match, a new session and its
  cookie; unknown user or wrong password → 401 with no detail on which. *Verify: integration*
- **FR-013** — `POST /api/logout` deletes the session (if any) and clears the cookie; the old
  cookie stops working. *Verify: integration*
- **FR-014** — `GET /api/me` → `{ authenticated: true, username, googleLinked }` for a valid
  session, else `{ authenticated: false }` (no error, also for a garbage cookie).
  *Verify: integration*
- **FR-015** — Creating a session (setup, sign-in, Google sign-in) first deletes every expired
  session, so they don't pile up. *Verify: integration*
- **FR-016** — Sign-in is throttled per username: after 5 failed attempts for a username within
  15 minutes, `POST /api/login` answers 429 `too_many_attempts` for it — even with the right
  password — until the oldest of those failures is 15 minutes old. Unknown usernames count
  too, so the throttle doesn't reveal which exist. A successful sign-in clears that username's
  failures; failures older than 15 minutes are deleted as new ones are recorded. The sign-in
  page says to try again later. *Verify: integration*

### Account

- **FR-020** — `PUT /api/account` `{ currentPassword, newUsername?, newPassword? }` (editor)
  requires the current password (wrong → 401 `wrong_current_password`) and applies a non-blank
  new username and/or new password. *Verify: integration*
- **FR-021** — A new password under 8 chars → 400 `password_too_short`; a username already
  taken by another user → 400 `username_taken`; on any error nothing is changed (username and
  password are saved together or not at all). *Verify: integration*
- **FR-022** — Changing the password ends every other session of that user (a stolen cookie
  stops working); the session that made the change stays signed in. Changing only the username
  ends none. *Verify: integration*

### Google sign-in

- **FR-030** — `GET /api/auth/google/status` (open) → `{ configured }`: true only when client
  id, secret and redirect URI are all non-empty. *Verify: integration*
- **FR-031** — `GET /api/auth/google/config` (editor) returns client id, redirect URI and
  `secretSet` — never the secret. `PUT` (editor) saves id + redirect URI (trimmed); a blank
  secret keeps the stored one. *Verify: integration*
- **FR-032** — `GET /api/auth/google/login` redirects to Google's consent screen (`openid
  email`, account chooser) with a one-time state valid for 5 minutes; not configured → 404.
  If the requester is signed in, the flow *links*; otherwise it *signs in*.
  *Verify: integration*
- **FR-033** — `GET /api/auth/google/callback`: unknown, reused or expired state →
  `/login?error=google_expired`; missing code/state or a failed token/userinfo exchange →
  `/login?error=google_failed`. Linking stores the Google account id on the user
  (→ `/settings?googleLinked=1`), unless it's already linked to another user
  (→ `/settings?googleError=already_linked_elsewhere`). Signing in starts a session for the user
  linked to that Google account (→ `/`); no linked user → `/login?error=google_not_linked` — a
  Google sign-in never creates a user. *Verify: integration*
- **FR-034** — `DELETE /api/auth/google` (editor) unlinks the current user's Google account.
  *Verify: integration*
- **FR-035** — `GOOGLE_TOKEN_URL` / `GOOGLE_USERINFO_URL` override Google's endpoints (used by
  tests). *Verify: integration*
- **FR-036** — Pending Google sign-in states expire after 5 minutes (FR-033); expired ones are
  dropped whenever a new Google sign-in starts, so abandoned sign-ins don't accumulate in
  memory. *Verify: unit*

### Pages

- **FR-041** — Signed out, every page sends the browser to `/login` (or `/setup` while no account
  exists) and shows no app data or navigation — only the sign-in or setup form, the version and
  the Source link (002/FR-007). *Verify: e2e*
- **FR-040** — `/setup` (shown while setup is needed) creates the first account; `/login` signs
  in with username/password, or with Google when configured, and shows Google errors from the
  query string; the Settings page edits the account (saying why a change was refused) and the
  Google config and links/unlinks.
  *Verify: e2e*

## Notes

- Retroactive: behavior read from the code at `3fdca27`.
- **#178 (fixed):** a short new password was silently ignored (yet `ok`), and a taken
  username was a 500. **#179 (fixed):** session purge, revoke-on-password-change,
  login throttling, OAuth state cleanup (FR-015, FR-016, FR-022, FR-036). **#165:** camera retry route isn't editor-gated.
- **#93:** once `password_hash` becomes nullable (Google-only users), unlinking needs a guard
  so the last credential can't be removed.
- **#91 / #92:** multi-user roles and invite-based Google registration are future work; today
  the `role` check exists but every user is `Admin`.
- **v1.5.1:** reads used to be open to anyone on the LAN; now everything needs a session.
- `POST /api/setup` is open by design until the first user exists — whoever reaches a fresh
  install first owns it.
