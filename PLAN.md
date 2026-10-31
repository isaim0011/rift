# PLAN: Discord REST Operations for Webhooks, Applications, and OAuth2

## Goal
Implement the missing Discord REST operations for:
- `webhooks` (10 missing of 15, reaching 15/15, 100%)
- `applications` (25 missing of 33, reaching 33/33, 100%)
- `oauth2` (4 missing of 4, reaching 4/4, 100%)

Put code in three new files:
- `crates/rift-rest/src/webhook.rs`
- `crates/rift-rest/src/application.rs`
- `crates/rift-rest/src/oauth.rs`

Export public items and declare `mod` in `crates/rift-rest/src/lib.rs`.
Append newly implemented routes to `coverage/implemented.txt`.
Update `COVERAGE_REST.md` via `cargo run -p rift-coverage -- openapi.json --write COVERAGE_REST.md`.

## File Layout & Architecture
1. `crates/rift-rest/src/webhook.rs`:
   - `Webhook` struct with typed ids (`Snowflake`), deserialization, and custom `Debug` (redacting token).
   - `WebhookType` enum (Incoming=1, ChannelFollower=2, Application=3).
   - `WebhookSourceGuild`, `WebhookSourceChannel`.
   - `ExecuteWebhook` builder & validator (reusing `validate_text_parts` from `message.rs`).
   - `ModifyWebhook`, `ModifyWebhookWithToken`, `WebhookMessageEdit`.
   - `RestClient` methods:
     - `get_webhook` (`GET /webhooks/{webhook_id}`)
     - `modify_webhook` (`PATCH /webhooks/{webhook_id}`)
     - `delete_webhook` (`DELETE /webhooks/{webhook_id}`)
     - `get_webhook_with_token` (`GET /webhooks/{webhook_id}/{webhook_token}`)
     - `modify_webhook_with_token` (`PATCH /webhooks/{webhook_id}/{webhook_token}`)
     - `delete_webhook_with_token` (`DELETE /webhooks/{webhook_id}/{webhook_token}`)
     - `execute_webhook` (`POST /webhooks/{webhook_id}/{webhook_token}`)
     - `execute_github_compatible_webhook` (`POST /webhooks/{webhook_id}/{webhook_token}/github`)
     - `execute_slack_compatible_webhook` (`POST /webhooks/{webhook_id}/{webhook_token}/slack`)
     - `get_original_webhook_message` (`GET /webhooks/{webhook_id}/{webhook_token}/messages/@original`)
     - `get_webhook_message` (`GET /webhooks/{webhook_id}/{webhook_token}/messages/{message_id}`)
     - `edit_webhook_message` (`PATCH /webhooks/{webhook_id}/{webhook_token}/messages/{message_id}`)
     - `delete_webhook_message` (`DELETE /webhooks/{webhook_id}/{webhook_token}/messages/{message_id}`)
   - Comprehensive route-pinning tests with fake transport and `tokio::test(start_paused = true)`.

2. `crates/rift-rest/src/application.rs`:
   - `Application` struct (deserializing `PrivateApplicationResponse` / `ApplicationResponse`), `Team`, `TeamMember`, `InstallParams`.
   - `ModifyApplication` request struct & validation.
   - `ActivityInstance`, `ActivityLocation`.
   - `ActivitiesAttachment`, `upload_application_attachment` multipart implementation.
   - `ApplicationEmoji`, `CreateApplicationEmoji`, `ModifyApplicationEmoji`, `ApplicationEmojis`.
   - `Entitlement`, `CreateEntitlement`, `GetEntitlements` query builder.
   - `CommandPermission`, `GuildCommandPermissions`, `CommandPermissionType`.
   - `RoleConnectionMetadata`, `RoleConnectionMetadataType`.
   - `EditCommand` request struct & validation.
   - `RestClient` methods for all 25 operations.
   - Route-pinning tests for all operations.

3. `crates/rift-rest/src/oauth.rs`:
   - `OAuth2Authorization`, `OAuth2Key`, `OAuth2Keys`, `OpenIdUserInfo`.
   - `RestClient` methods:
     - `get_current_authorization_info` (`GET /oauth2/@me`)
     - `get_current_oauth2_application` (`GET /oauth2/applications/@me`)
     - `get_oauth2_keys` (`GET /oauth2/keys`)
     - `get_openid_userinfo` (`GET /oauth2/userinfo`)
   - Route-pinning tests for all 4 operations.

4. `crates/rift-rest/src/lib.rs`:
   - Add `mod webhook; mod application; mod oauth;`
   - Re-export key types.

5. `coverage/implemented.txt`:
   - Append the 39 new routes.

6. Verification:
   - Run `cargo test -p rift-rest`
   - Run `cargo run -p rift-coverage -- openapi.json --write COVERAGE_REST.md`

## Test Strategy
- Pin each route's method and normalized/expanded URL against fake transport.
- Test validation failures (empty messages, lengths exceeding documented limits, max embed counts, tag limits, etc.) to ensure invalid requests never hit the network.
- Test token redaction in Debug format.
- Run complete rift-rest suite to prevent regressions.
