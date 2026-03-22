<!-- Managed by agent: keep sections and order; edit content, not structure. Last updated: 2026-03-22 -->

# AGENTS.md — src/api/

## Overview
Async HTTP client for Pulumi Cloud REST API using reqwest + progenitor-generated code.

**Related docs**: `../AGENTS.md` (architecture), `../app/AGENTS.md` (data loading), root `AGENTS.md` (env vars), **[`ADDING_ENDPOINTS.md`](ADDING_ENDPOINTS.md)** (how to add new API endpoints)

## Key Files

| File | Purpose |
|------|---------|
| `client.rs` | `PulumiClient` — thin wrappers over generated client or raw reqwest |
| `domain.rs` | App-level types (used by UI, handlers, state) |
| `convert.rs` | `From<generated::Type>` impls mapping generated -> domain types |
| `generated.rs` | `include!()` wrapper for progenitor output (**DO NOT EDIT**) |
| `mod.rs` | Public re-exports |
| `ADDING_ENDPOINTS.md` | Step-by-step guide for adding new API endpoints |

## API Endpoints

### Stacks
- `GET /api/user/stacks?organization={org}` — List stacks
- Pagination: `continuationToken` query param

### ESC Environments
- `GET /api/esc/environments/{org}` — List environments
- `GET /api/esc/environments/{org}/{project}/{env}` — Get YAML definition (plain text)
- `PATCH /api/esc/environments/{org}/{project}/{env}` — Update YAML (Content-Type: `application/x-yaml`)
- `POST /api/esc/environments/{org}/{project}/{env}/open` — Open session (returns `{id, diagnostics}`)
- `GET /api/esc/environments/{org}/{project}/{env}/open/{sessionId}` — Get resolved values

Field names: uses `created`/`modified` (NOT `createdAt`/`modifiedAt`)

### Neo (Preview Agents)
- `GET /api/preview/agents/{org}/tasks` — List tasks (pageSize, continuationToken)
- `GET /api/preview/agents/{org}/tasks/{taskId}` — Get task metadata
- `POST /api/preview/agents/{org}/tasks` — Create task
- `PATCH /api/preview/agents/{org}/tasks/{taskId}` — Update task settings (e.g., sharing)
- `GET /api/preview/agents/{org}/tasks/{taskId}/events` — Get events
- `POST /api/preview/agents/{org}/tasks/{taskId}` — Send user event (message, confirmation, cancel)

**Task Status**: `"running"` or `"idle"`

**Task Sharing**: `PATCH` with `{ "isShared": true }`

**User Event Types** (POST to task): `user_message`, `user_confirmation` (`{ "approved": true/false }`), `user_cancel`

**Console Event Types** (GET events): `user_message`, `assistant_message`, `set_task_name`, `exec_tool_call`, `tool_response`, `user_approval_request`

**Entity Types** in task: `stack`, `repository`, `pull_request`, `policy_issue`

### Neo Slash Commands
- `GET /api/console/agents/{org}/commands` — List slash commands
- `POST /api/console/agents/{org}/commands` — Create custom slash command
- `PATCH /api/console/agents/{org}/commands/{name}` — Update (requires `If-Match` header)
- `DELETE /api/console/agents/{org}/commands/{name}` — Delete (requires `If-Match` header)

**Optimistic Concurrency**: PATCH and DELETE require `If-Match` header with the command's `tag` value. Returns 409 Conflict if stale.

**New task** with slash commands uses `message` wrapper with `{{cmd:name:tag}}` content format.
**Existing task** continuation uses `event` wrapper.

### Resource Search
- `GET /api/orgs/{org}/search/resourcesv2` — Search resources (v2)
- Pagination: `page` (1-based), `size` params

### Dashboard Data
- `GET /api/console/orgs/{org}/stacks/updates/recent?limit=N` — Recent updates
- `GET /api/orgs/{org}/resources/summary?granularity=daily&lookbackDays=N` — Resource chart

## Serde Notes
- API may return `null` for array fields — use `null_to_empty_vec` deserializer
- Extra fields from API — use `#[serde(default)]` to ignore

## Testing API Manually
```bash
TOKEN=$(cat .env | head -1)
curl -s -H "Authorization: token $TOKEN" \
  "https://api.pulumi.com/api/preview/agents/{ORG}/tasks"
```

## Setup
- Requires `PULUMI_ACCESS_TOKEN` env var
- Generated code from `build.rs` — run `cargo check` after modifying `KEPT_PATHS`

## Build & tests
- `cargo check` — triggers `build.rs` code generation
- `cargo test` — includes conversion tests in `convert.rs`

## Code style
- Domain types: `#[derive(Debug, Clone, Serialize, Deserialize)]` + `#[serde(rename_all = "camelCase")]`
- Nullable arrays: use `null_to_empty_vec` or `#[serde(default)]`
- Client methods: use `org_or_default()`, return `Result<T, ApiError>`

## Security
- Never log full API tokens
- Validate response status before deserializing

## Checklist
- [ ] Domain type in `domain.rs`
- [ ] Conversion in `convert.rs` with test (Route A)
- [ ] Client method in `client.rs`
- [ ] Export in `mod.rs`
- [ ] `cargo test` + `cargo clippy -- -D warnings` pass

## Examples
> See `ADDING_ENDPOINTS.md` for complete step-by-step examples.

## When stuck
- Check `ADDING_ENDPOINTS.md` for the full guide
- Inspect generated code: `grep 'pub fn' target/debug/build/lazy-pulumi-*/out/pulumi_api.rs`
- Check root `AGENTS.md` for project conventions

## House Rules
- Always follow `ADDING_ENDPOINTS.md` when adding new endpoints
- Route A (generated via `KEPT_PATHS`) preferred when endpoint is in OpenAPI spec with JSON
- Route B (raw reqwest) for endpoints not in spec, YAML content types, or polymorphic responses
