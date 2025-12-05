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

### Neo Slash Commands
- `GET /api/console/agents/{org}/commands` - List available slash commands

**New task** with slash commands (uses `message` wrapper):
```json
{
  "message": {
    "type": "user_message",
    "content": "{{cmd:name:tag}}",
    "timestamp": "2025-12-05T20:45:08.613Z",
    "commands": {
      "{{cmd:name:tag}}": {
        "name": "command-name",
        "prompt": "Full prompt text...",
        "description": "Short description",
        "builtIn": true,
        "modifiedAt": "0001-01-01T00:00:00.000Z",
        "tag": "hash-string"
      }
    }
  }
}
```

**Existing task** continuation with slash commands (uses `event` wrapper):
```json
{
  "event": {
    "type": "user_message",
    "content": "{{cmd:cmd1:tag1}} {{cmd:cmd2:tag2}}",
    "timestamp": "2025-12-05T21:22:00.773Z",
    "commands": {
      "{{cmd:cmd1:tag1}}": { ... },
      "{{cmd:cmd2:tag2}}": { ... }
    }
  }
}
```

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
