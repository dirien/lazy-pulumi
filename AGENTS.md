<!-- FOR AI AGENTS - Human readability is a side effect, not a goal -->
<!-- Managed by agent: keep sections and order; edit content, not structure -->
<!-- Last updated: 2026-03-22 -->

# AGENTS.md

**Precedence:** the **closest `AGENTS.md`** to the files you're changing wins. Root holds global defaults only.

## Commands
> Source: Cargo.toml, CI workflow — verified against codebase

| Task | Command | ~Time |
|------|---------|-------|
| Check | `cargo check` | ~15s |
| Lint | `cargo clippy -- -D warnings` | ~20s |
| Format check | `cargo fmt --check` | ~1s |
| Test (all) | `cargo test` | ~30s |
| Build (release) | `cargo build --release` | ~90s |
| Run | `cargo run --release` | ~90s+ |
| Update OpenAPI spec | `cargo xtask update-spec` | ~5s |

## Environment

| Variable | Required | Purpose |
|----------|----------|---------|
| `PULUMI_ACCESS_TOKEN` | Yes | Pulumi Cloud auth (`pul-xxxxxxxxxxxx`) |
| `PULUMI_ORG` | No | Default org name |
| `PULUMI_API_URL` | No | API base (default: `https://api.pulumi.com`) |

Token can also be stored in `.env` (just the token value, no variable name).

## Workflow
1. **Before coding**: Read nearest `AGENTS.md` + check Golden Samples
2. **After each change**: `cargo check` -> `cargo clippy -- -D warnings` -> `cargo test`
3. **Before committing**: `cargo test && cargo clippy -- -D warnings && cargo fmt --check`
4. **Before claiming done**: Show test/lint output as evidence

## File Map
```
Cargo.toml       <- Workspace: [".", "xtask"]
build.rs         <- Progenitor code gen from OpenAPI spec
openapi/         <- pulumi-spec.json (cargo xtask update-spec)
xtask/           <- Developer tooling
src/             <- Application source (see scoped AGENTS.md)
  app/           <- TEA: state, handlers, data loading
  api/           <- Pulumi Cloud API client
  commands/      <- Pulumi CLI PTY execution
  components/    <- Reusable widgets
  ui/            <- View rendering
```

## Golden Samples
| For | Reference | Key patterns |
|-----|-----------|--------------|
| Domain types | `src/api/domain.rs` | `#[derive(Debug, Clone, Serialize, Deserialize)]`, `#[serde(rename_all = "camelCase")]` |
| Type conversion | `src/api/convert.rs` | `From<gen::Type>` impls, unit tests at bottom |
| API client | `src/api/client.rs` | Generated wrapper or raw reqwest, `org_or_default()` |
| Key handler | `src/app/handlers/` | `handle_*_key()`, `FocusMode` checks (split by tab) |
| UI view | `src/ui/stacks.rs` | Layout splitting, `StatefulList` rendering |
| Adding endpoint | `src/api/ADDING_ENDPOINTS.md` | Route A (generated) vs Route B (raw reqwest) |

## Utilities
| Need | Use | Location |
|------|-----|----------|
| Scrollable list | `StatefulList<T>` | `src/components/list.rs` |
| Text input | `TextInput` | `src/components/input.rs` |
| Multi-line editor | `TextEditor` | `src/components/editor.rs` |
| Loading spinner | `Spinner` | `src/components/spinner.rs` |
| API client | `PulumiClient` | `src/api/client.rs` |
| Syntax highlighting | syntect | `src/ui/syntax.rs` |
| Markdown rendering | parser | `src/ui/markdown.rs` |

## Heuristics
| When | Do |
|------|-----|
| Adding API endpoint | Follow `src/api/ADDING_ENDPOINTS.md` |
| Adding new tab/view | `Tab` variant -> handler -> render fn -> wire in `mod.rs` |
| Adding key binding | Appropriate `handle_*_key()`, check `FocusMode` |
| Null array from API | `null_to_empty_vec` or `#[serde(default)]` |
| Adding dependency | Ask first |

## Architecture

**TEA pattern**: Model (`app/types.rs`) -> Update (`app/handlers/`) -> View (`ui/`)

**Key concepts**: `FocusMode::Normal` vs `Input`, tokio channels for async, `build.rs` generates API client from `KEPT_PATHS` subset of OpenAPI spec.

## Boundaries

### Always Do
- Run pre-commit checks; add tests; use `Result<T, E>` or `.expect("reason")`; use `&str` over `String`; doc public APIs

### Ask First
- New deps, CI changes, public API changes, new `KEPT_PATHS`, spec patching changes

### Never Do
- `.unwrap()` in library code; edit `generated.rs` or `pulumi-spec.json`; commit secrets; push to main

## Terminology
| Term | Means |
|------|-------|
| TEA | The Elm Architecture: Model -> Update -> View |
| Neo | Pulumi's AI agent (preview) |
| ESC | Pulumi Environments, Secrets, and Configuration |
| progenitor | OpenAPI-to-Rust code gen in `build.rs` |
| KEPT_PATHS | API path allowlist in `build.rs` |

<!-- AGENTS-GENERATED:START scope-index -->
## Index of scoped AGENTS.md
| Directory | Focus |
|-----------|-------|
| `src/` | Architecture, TEA pattern, app flow |
| `src/api/` | API endpoints, pagination, adding endpoints |
| `src/app/` | State machine, handlers, Neo chat, commands |
| `src/components/` | Widgets: StatefulList, TextInput, TextEditor, Spinner |
| `src/ui/` | View rendering, dashboard, Neo chat, markdown |
<!-- AGENTS-GENERATED:END scope-index -->

> **Agents**: When editing files in a listed directory, load its AGENTS.md first.

## When instructions conflict
The nearest `AGENTS.md` wins. Explicit user prompts override files.
