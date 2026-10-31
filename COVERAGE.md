# What "covers the whole Discord docs" means for rift

Source of truth for scope: the official docs index (https://docs.discord.com/llms.txt).
Legend: ✅ done · ◐ partial · ❌ not yet · ⛔ out of scope for a server-side library.

> The numbers for REST are produced by `rift-coverage` against Discord's own
> OpenAPI spec (below). Everything else is tracked here.

## Gateway, events, interactions

| Docs area | Status | Notes |
|---|---|---|
| Gateway connection (Hello, heartbeat, Identify, Resume, close codes, sharding, zstd-stream, intents, capabilities) | ✅ | Verified live. Resume and zombie recovery are unit-tested. |
| Opcodes and status codes | ✅ | Gateway opcodes and close codes. HTTP/JSON error *codes* surfaced. |
| Rate limits | ✅ | Buckets, global, circuit breaker, bounded memory. |
| Permissions | ✅ | Flags, overwrites, implicit rules, timeout restrictions, obfuscated channel math. |
| Gateway commands (send) | ✅ | Identify, Resume, Heartbeat, Request Guild Members, Presence Update, Voice State Update, Request Soundboard Sounds with typed payload builders and rate limiting. |
| Gateway events (receive) | ✅ | ~70 typed gateway events (`rift-model`) with lazy parsing from raw payloads and unknown event fallback. |
| Webhook events (HTTP) | ✅ | Ed25519 signature verification via `rift-http`. |
| Interactions: receive over gateway, respond, follow-ups | ✅ | Initial responses, deferred responses, follow-ups, autocomplete choices, modal popups. |
| Interactions over HTTP | ✅ | Serverless Ed25519 signature verification (`verify_interaction`, `Ed25519Verifier`). |
| Application commands | ✅ | Typed and validated (`CommandSpec`, `OptionSpec`, permissions, contexts, integration types). |
| Components (message) | ✅ | Typed Components V1 & V2 (ActionRow, Button, SelectMenu, TextInput, TextDisplay, Separator, Container, MediaGallery). |
| Modals | ✅ | Typed modal popup responses and `MODAL_SUBMIT` interaction handling. |
| Autocomplete | ✅ | Application command autocomplete handling with focused option inspection. |
| State Cache | ✅ | Obfuscation-safe in-memory cache (`rift-cache`) with Nov 16 channel obfuscation compliance and bounded FIFO buffers. |
| Framework & Ergonomics | ✅ | `CommandRouter`, `EventContext`, `CachedClient`, `Framework`, and `rift::prelude`. |
| Threads (topic) | ✅ | REST + typed gateway events. |
| OAuth2 (topic) | ✅ | Application info, bot authorization URLs, token endpoints. |
| Voice connections (topic) | ✅ | REST voice state and regions done; voice gateway UDP transport and AEAD cipher done (`rift-voice`). |

## REST resources (230 of 250 operations implemented · 92.0% spec · 100% of all bot-facing resources)

| Resource | Status | Notes |
|---|---|---|
| Message | ✅ | Full message CRUD, reactions, crosspost, pins, bulk delete, multipart file uploads. |
| Channel | ✅ | Full channel CRUD, permission overwrites, invites, typing indicators, pins, threads. |
| Guild | ✅ | Guild settings, members, roles, bans, pruning, widget, voice regions, integrations. |
| User | ✅ | Current user, get user, modify current user, user DMs, connections. |
| Webhook | ✅ | Webhook CRUD, execute, GitHub/Slack webhooks, interaction follow-ups. |
| Application (+ commands) | ✅ | Global/guild commands, permissions, role connection metadata. |
| Audit Log, Auto Moderation | ✅ | Audit log entries, auto-moderation rules & executions. |
| Emoji, Sticker, Soundboard, Poll, Stage Instance | ✅ | Full CRUD and management across all sub-resources. |
| Guild Scheduled Event, Guild Template | ✅ | Scheduled events, users, templates. |
| SKU, Entitlement, Subscription (monetization) | ✅ | Monetization SKUs, entitlements, subscriptions. |
| Application Role Connection Metadata | ✅ | App role connection metadata records. |
| Voice (REST: regions, voice state) | ✅ | Voice regions and voice state updates. |
| Lobby, Partner SDK | ⛔ | Client-side game lobbies (15 ops) and Partner SDK (5 ops); out of scope for server-side bots. |

## Out of scope

Client-side SDKs and non-API content: **Social SDK** (C++/Unity/Unreal), **Embedded App SDK / Activities**
(browser JS), **RPC** (local IPC), **Certified Devices**, design guidelines, policies, growth/marketing guides.
Client-side game lobbies (15 endpoints) and Partner SDK (5 endpoints) are designated as non-bot client APIs.

## How we get to "everything", and stay there

1. **Measure.** `rift-coverage` diffs Discord's OpenAPI 3.1 spec
   (https://github.com/discord/discord-api-spec, `specs/openapi.json`) against `coverage/implemented.txt`:

       cargo run -p rift-coverage -- path/to/openapi.json --write COVERAGE_REST.md

   The spec is a **public preview**: it may be wrong, can change without notice, and omits limits, flag
   values and descriptions. So it drives *coverage and drift detection*, not validation rules.
2. **Write endpoints by hand, from the docs, in priority order**, each with the same quality bar as
   messages and commands: typed ids, local validation of every documented limit, a fake-transport test
   that pins method + route + body, and a line in `coverage/implemented.txt`.
3. **Completed Milestones**:
   - Channel + full Message (with multipart file uploads)
   - Guild/Member/Role
   - User
   - Webhook
   - Typed Components V1 & V2 + Modals + Autocomplete
   - ~70 typed Gateway Events (`rift-model`)
   - Obfuscation-safe opt-in Cache with Nov 16 compliance (`rift-cache`)
   - Auto Moderation, Scheduled Events, Stickers, Emoji, Soundboard, Polls, Invites, Templates, Stage
   - Monetization (SKU, Entitlements, Subscriptions)
   - OAuth2
   - Serverless Ed25519 HTTP interactions & webhook verification (`rift-http`)
   - Ergonomic Ecosystem Layer: `CommandRouter`, `EventContext`, `CachedClient`, `Framework`, and `prelude` (`crates/rift`)
4. **Stay current.** Run `rift-coverage --strict` in CI against the latest spec, and review the docs'
   Change Log (https://docs.discord.com/developers/change-log) on a schedule. The API version is pinned to v10.
