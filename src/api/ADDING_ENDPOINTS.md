# Adding New API Endpoints

Step-by-step guide for adding a new Pulumi Cloud API endpoint to the codebase.

**Related docs**: `CLAUDE.md` (endpoint reference), `../../CLAUDE.md` (build commands)

## Step 0: Update the OpenAPI Spec

Before adding a new endpoint, make sure you have the latest Pulumi Cloud OpenAPI spec:

```bash
cargo xtask update-spec
```

This downloads the latest spec from `https://api.pulumi.com/api/openapi/pulumi-spec.json`
and writes it to `openapi/pulumi-spec.json`. If the endpoint you need was recently added
to the Pulumi API, it may only appear in the latest spec.

## Architecture Overview

The API client uses a two-tier approach:

```
openapi/pulumi-spec.json          Full Pulumi OpenAPI spec (~500+ endpoints)
         │
         ▼
build.rs  ──►  KEPT_PATHS filter  Only endpoints we actually use
         │
         ▼
progenitor ──►  OUT_DIR/pulumi_api.rs   Generated Rust client + types
         │
         ▼
src/api/generated.rs               Wraps generated code (suppresses warnings)
src/api/convert.rs                 From<generated::Type> → domain::Type
src/api/domain.rs                  App-level types used everywhere
src/api/client.rs                  PulumiClient methods (thin wrappers)
```

There are **two routes** for adding an endpoint:

| Route | When to use | Files touched |
|-------|-------------|---------------|
| **A. Generated** | Endpoint exists in OpenAPI spec with JSON request/response | `build.rs`, `domain.rs`, `convert.rs`, `client.rs` |
| **B. Raw reqwest** | Endpoint missing from spec, uses YAML/plain text, or has polymorphic responses | `domain.rs`, `client.rs` |

## Route A: Generated Endpoint (OpenAPI spec)

Use this when the endpoint is in `openapi/pulumi-spec.json` and uses standard JSON.

### Step 1: Add the path to `KEPT_PATHS` in `build.rs`

```rust
// build.rs
const KEPT_PATHS: &[&str] = &[
    // ... existing paths ...
    "/api/your/new/endpoint/{paramName}",  // ← add here
];
```

The build script will automatically:
- Extract this path (and all HTTP methods on it) from the full spec
- Recursively resolve all `$ref` schema dependencies
- Generate typed Rust structs and client methods

### Step 2: Rebuild to see generated types

```bash
cargo check 2>&1 | head -5   # triggers build.rs, compiles generated code
```

If the build fails, the endpoint may need patching (see Troubleshooting below).

### Step 3: Find the generated method name

The generated client creates method names from the `operationId` in the spec.
To find yours:

```bash
# Search the full spec for your path's operationId
cat openapi/pulumi-spec.json | \
  python3 -c "import sys,json; d=json.load(sys.stdin); \
  [print(f'{m}: {op.get(\"operationId\",\"?\")}') \
   for p,item in d['paths'].items() if '/your/new' in p \
   for m,op in item.items() if m in ('get','post','put','patch','delete')]"
```

Or inspect the generated output directly:

```bash
# Find method names in generated code
grep 'pub fn ' target/debug/build/lazy-pulumi-*/out/pulumi_api.rs
```

### Step 4: Add domain types in `domain.rs`

Define the app-level struct that the rest of the application will use:

```rust
// src/api/domain.rs

/// Your new domain type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YourNewType {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub optional_field: Option<String>,
}
```

### Step 5: Add conversion in `convert.rs`

Map the generated type to your domain type:

```rust
// src/api/convert.rs
use super::generated::types as gen;

impl From<gen::GeneratedTypeName> for domain::YourNewType {
    fn from(g: gen::GeneratedTypeName) -> Self {
        Self {
            id: g.id,
            name: g.name,
            optional_field: g.optional_field,
        }
    }
}
```

To discover the generated type name and its fields:

```bash
grep 'pub struct' target/debug/build/lazy-pulumi-*/out/pulumi_api.rs | grep -i 'YourType'
```

### Step 6: Add client method in `client.rs`

```rust
// src/api/client.rs

/// Description of what this endpoint does
pub async fn your_new_method(
    &self,
    org: Option<&str>,
) -> Result<Vec<YourNewType>, ApiError> {
    let org = self.org_or_default(org)?;

    let resp = self
        .gen
        .generated_method_name()   // from Step 3
        .org_name(org)             // builder-style params
        .send()
        .await
        .map_err(map_gen_err)?;

    let data = resp.into_inner();
    Ok(data.items.into_iter().map(Into::into).collect())
}
```

### Step 7: Export from `mod.rs`

```rust
// src/api/mod.rs
pub use domain::YourNewType;
```

### Step 8: Add tests

Add a conversion test in `convert.rs` and optionally an integration test in `client.rs`:

```rust
// src/api/convert.rs — at the bottom in #[cfg(test)] mod tests
#[test]
fn your_new_type_conversion() {
    let gen_val = gen::GeneratedTypeName { /* ... */ };
    let domain_val: domain::YourNewType = gen_val.into();
    assert_eq!(domain_val.name, "expected");
}
```

### Step 9: Verify

```bash
cargo test
cargo clippy -- -D warnings
```

## Route B: Raw Reqwest Endpoint

Use this when:
- The endpoint is **not in the OpenAPI spec** (e.g. `/api/console/...` endpoints)
- The endpoint uses **non-JSON content types** (e.g. `application/x-yaml`)
- The response is **polymorphic** and needs custom deserialization

### Step 1: Add domain types in `domain.rs`

Same as Route A, Step 4.

### Step 2: Add client method in `client.rs` using raw reqwest

```rust
// src/api/client.rs

/// Description — not in OpenAPI spec, raw reqwest.
pub async fn your_new_method(
    &self,
    org: &str,
    param: &str,
) -> Result<YourNewType, ApiError> {
    let url = format!(
        "{}/api/your/endpoint/{}",
        self.config.base_url, param
    );

    let response = self.client.get(&url).send().await?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let message = response.text().await.unwrap_or_default();
        return Err(ApiError::ApiResponse { status, message });
    }

    // For complex responses, define a local deserialization struct:
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ApiResponse {
        items: Vec<YourNewType>,
    }

    let data: ApiResponse = response.json().await.map_err(ApiError::Http)?;
    Ok(data.items)
}
```

### Step 3: Export, test, verify

Same as Route A Steps 7-9.

## Troubleshooting

### Build fails after adding a path to `KEPT_PATHS`

**Unsupported content type** (e.g. `application/x-yaml`):
The `strip_unsupported_content_types` function in `build.rs` removes operations
that use YAML or plain text. If your endpoint has *only* unsupported content types,
the path will be empty after stripping and removed entirely. Use Route B instead.

**Mixed success responses** (e.g. 200 JSON + 204 No Content):
Progenitor requires at most one response type. The `fix_mixed_success_responses`
function removes untyped success responses when a typed one exists. If your endpoint
has a different pattern, add a similar fix in `build.rs`.

**Nullable fields the spec marks as required**:
The live API sometimes returns `null` for fields the spec says are required.
Add a patch in `patch_nullable_fields` in `build.rs`:

```rust
fn patch_nullable_fields(schemas: &mut serde_json::Map<String, serde_json::Value>) {
    // ... existing patches ...

    // YourSchema.yourField — null when condition X applies
    remove_required(schemas, "YourSchema", "yourField");
    // For array fields that can be null, also set nullable:
    set_nullable(schemas, "YourSchema", "yourField");
}
```

### Generated method name doesn't match expectations

Progenitor derives method names from `operationId` in the spec. To override:

```rust
// build.rs — in GenerationSettings
let settings = progenitor::GenerationSettings::new()
    .with_interface(progenitor::InterfaceStyle::Builder)
    .with_tag(progenitor::TagStyle::Merged)
    .with_rename("operationId", "desired_method_name")  // if needed
    .clone();
```

### Generated type has wrong field types

Some OpenAPI specs use `integer` with `format: int64` but the app expects `i32`.
Handle this in the `From` impl in `convert.rs`:

```rust
resource_count: g.resource_count.map(|r| r as i32),
```

## File Reference

| File | Purpose |
|------|---------|
| `openapi/pulumi-spec.json` | Full Pulumi Cloud OpenAPI spec (source of truth) |
| `build.rs` | Trims spec to `KEPT_PATHS`, runs progenitor, writes `OUT_DIR/pulumi_api.rs` |
| `src/api/generated.rs` | `include!()` wrapper that suppresses warnings on generated code |
| `src/api/domain.rs` | App-level types (used by UI, handlers, state) |
| `src/api/convert.rs` | `From<generated::Type>` impls mapping generated → domain types |
| `src/api/client.rs` | `PulumiClient` methods — thin wrappers over generated client or raw reqwest |
| `src/api/mod.rs` | Public re-exports |

## Checklist

When adding a new endpoint, verify each item:

- [ ] Path added to `KEPT_PATHS` (Route A) or using raw reqwest (Route B)
- [ ] Domain type added in `domain.rs`
- [ ] `From` conversion added in `convert.rs` (Route A only)
- [ ] Client method added in `client.rs`
- [ ] Type exported from `mod.rs` (if used outside `api/`)
- [ ] Unit test for conversion (Route A)
- [ ] `cargo test` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
