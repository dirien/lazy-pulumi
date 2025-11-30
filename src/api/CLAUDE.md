# src/api/ - Pulumi Cloud API Client

Async HTTP client for Pulumi Cloud REST API using reqwest.

**Related docs**: `../CLAUDE.md` (architecture), `../app/CLAUDE.md` (data loading), `../../CLAUDE.md` (env vars)

## Files

- `client.rs` - HTTP client implementation with bearer token auth
- `types.rs` - API response/request types (serde)
- `mod.rs` - Re-exports

## API Endpoints

### Stacks
- `GET /api/user/stacks?organization={org}` - List stacks
- Pagination: `continuationToken` query param

### ESC Environments
- `GET /api/esc/environments/{org}` - List environments
- `GET /api/esc/environments/{org}/{project}/{env}` - Get YAML definition (plain text)
- `PATCH /api/esc/environments/{org}/{project}/{env}` - Update YAML (Content-Type: `application/x-yaml`)
- `POST /api/esc/environments/{org}/{project}/{env}/open` - Open session (returns `{id, diagnostics}`)
- `GET /api/esc/environments/{org}/{project}/{env}/open/{sessionId}` - Get resolved values

Field names: uses `created`/`modified` (NOT `createdAt`/`modifiedAt`)

### Neo (Preview Agents)
- `GET /api/preview/agents/{org}/tasks` - List tasks (pageSize, continuationToken)
- `GET /api/preview/agents/{org}/tasks/{taskId}` - Get task metadata
- `POST /api/preview/agents/{org}/tasks` - Create task
- `GET /api/preview/agents/{org}/tasks/{taskId}/events` - Get events
- `POST /api/preview/agents/{org}/tasks/{taskId}` - Send message

Event body types: `user_message`, `assistant_message`, `set_task_name`, `exec_tool_call`, `tool_response`, `user_approval_request`

### Resource Search
- `GET /api/orgs/{org}/search/resourcesv2` - Search resources (v2 endpoint)
- Pagination: `page` (1-based), `size` params

### Dashboard Data
- `GET /api/console/orgs/{org}/stacks/updates/recent?limit=N` - Recent updates
- `GET /api/orgs/{org}/resources/summary?granularity=daily&lookbackDays=N` - Resource chart

## Testing API Manually

```bash
TOKEN=$(cat .env | head -1)
curl -s -H "Authorization: token $TOKEN" \
  "https://api.pulumi.com/api/preview/agents/{ORG}/tasks"
```

## Serde Notes

- API may return `null` for array fields - use `null_to_empty_vec` deserializer
- Extra fields from API - use `#[serde(default)]` to ignore
