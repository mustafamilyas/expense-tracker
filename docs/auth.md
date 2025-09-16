# Authentication and Authorization

This document describes the current authentication model, chat relay signing, JWT usage, public endpoints, and recommended refinements for security and UX.

## Overview

- Web clients authenticate with a JSON Web Token (JWT) issued at `/auth/login`.
- Chat-originated requests are authenticated via a signed request from a trusted chat relay/bot using an HMAC signature header, and are group‑scoped.
- Public endpoints: `/auth/login`, `/auth/register`, `/health`, `/version`, `/docs`, `/api-doc/openapi.json`.

## Web Authentication (JWT)

- Clients send `Authorization: Bearer <jwt>`.
- Issued at login with 7-day TTL; contains claims:
  - `sub`: user UUID
  - `typ`: `web`
  - `exp`: expiration timestamp
- Server validates with `HS256` using `JWT_SECRET`.

### Login Response

```
POST /auth/login
{
  "email": "user@example.com",
  "password": "..."
}

200 OK
{
  "token": "<JWT>",
  "user": { "uid": "...", "email": "...", "start_over_date": 1 }
}
```

## Chat Relay Authentication (HMAC)

- Chat requests must include:
  - `X-Relay-Signature: sha256=<hex>` — HMAC-SHA256 of the raw HTTP body, using `CHAT_RELAY_SECRET`.
  - `X-Chat-Binding: <binding_uuid>` — identifies the active chat binding.
- Server verification:
  1) Recompute HMAC over the raw body and compare to the header.
  2) Load binding by UUID; require `status = 'active'` and `revoked_at IS NULL`.
  3) Build an `AuthContext` with `source=Chat`, `user_uid = bound_by`, and `group_uid = binding.group_uid`.

## Authorization Scope

- Web JWT requests are user-scoped.
- Chat relay requests are group‑scoped and user‑attributed:
  - `user_uid` is the user who bound the chat.
  - `group_uid` is the group for which the chat is authorized.
- For write endpoints that include `group_uid` in the request body, the server enforces that it matches the chat context’s `group_uid`.

## Environment Variables

- `JWT_SECRET`: HMAC secret for JWTs.
- `CHAT_RELAY_SECRET`: HMAC secret used to sign chat relay requests.

## OpenAPI / Swagger

- The API defines a `bearerAuth` security scheme (HTTP bearer, JWT). Use the “Authorize” button in Swagger UI to provide your JWT.
- Public endpoints do not require authentication.

## Public Endpoints

- `/auth/login`
- `/auth/register`
- `/health`
- `/version`
- `/docs`, `/api-doc/openapi.json`

## Chat Sign‑In Flow

Recommended flow using `chat_bind_requests` and `chat_bindings`:

1) User types `/sign-in` in chat.
2) Server creates a `ChatBindRequest { platform, p_uid, nonce, expires_at }` and replies with a URL (contains id + nonce) to open in the web dashboard.
3) User logs in to web; server verifies request id+nonce and expiry; user selects expense group to bind.
4) Server creates `ChatBinding { group_uid, platform, p_uid, status='active', bound_by=user_uid }`, marks the request used, and sends a welcome message in chat.

## Refinements & Hardening Roadmap

- Replay Mitigation: Add a `X-Request-Timestamp` header and reject stale signatures (e.g., >5 minutes old).
- Per‑Binding Tokens: Issue an access token per chat binding instead of using a single relay secret.
- Platform Signatures: Verify platform-native signatures (Telegram, WhatsApp) if directly receiving webhooks.
- Role Checks: Enforce group membership/roles from `group_members` for sensitive writes.
- Secret Rotation: Rotate `JWT_SECRET` and `CHAT_RELAY_SECRET`; support multiple valid keys if needed.
- Auditing: Log principal (web user or chat binding) and action with correlation IDs for traceability.

