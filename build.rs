//! Build script for injecting version information at compile time
//! and generating the Pulumi API client via progenitor.
//!
//! When building with GoReleaser, the VERSION and GIT_COMMIT environment
//! variables are set and will be used. Otherwise, falls back to Cargo's
//! package version.

use std::{
    collections::BTreeSet,
    env,
    fs::{self, File},
    path::Path,
};

/// Exact paths from the Pulumi OpenAPI spec that the application uses.
const KEPT_PATHS: &[&str] = &[
    "/api/user",
    "/api/user/stacks",
    "/api/stacks/{orgName}/{projectName}/{stackName}",
    "/api/stacks/{orgName}/{projectName}/{stackName}/updates",
    "/api/esc/environments/{orgName}",
    "/api/esc/environments/{orgName}/{projectName}/{envName}",
    "/api/esc/environments/{orgName}/{projectName}/{envName}/open",
    "/api/esc/environments/{orgName}/{projectName}/{envName}/open/{openSessionID}",
    "/api/preview/agents/{orgName}/tasks",
    "/api/preview/agents/{orgName}/tasks/{taskID}",
    "/api/preview/agents/{orgName}/tasks/{taskID}/events",
    "/api/orgs/{orgName}/search/resourcesv2",
    "/api/orgs/{orgName}/resources/summary",
    "/api/orgs/{orgName}/members",
    "/api/orgs/{orgName}/services",
    "/api/preview/registry/packages",
    "/api/preview/registry/templates",
];

fn main() {
    // ── Version injection (existing logic) ──────────────────────
    println!("cargo:rerun-if-env-changed=VERSION");
    println!("cargo:rerun-if-env-changed=GIT_COMMIT");
    println!("cargo:rerun-if-env-changed=BUILD_DATE");

    if let Ok(version) = env::var("VERSION") {
        println!("cargo:rustc-env=APP_VERSION={}", version);
    }
    if let Ok(commit) = env::var("GIT_COMMIT") {
        println!("cargo:rustc-env=APP_COMMIT={}", commit);
    }
    if let Ok(date) = env::var("BUILD_DATE") {
        println!("cargo:rustc-env=APP_BUILD_DATE={}", date);
    }

    // ── Progenitor code generation ──────────────────────────────
    let spec_path = "openapi/pulumi-spec.json";
    println!("cargo:rerun-if-changed={}", spec_path);

    let file = File::open(spec_path).expect("failed to open openapi/pulumi-spec.json");
    let full_spec: serde_json::Value =
        serde_json::from_reader(file).expect("failed to parse OpenAPI spec");

    let trimmed = trim_spec(&full_spec);

    // Parse trimmed JSON into the OpenAPI struct that progenitor expects
    let spec: openapiv3::OpenAPI =
        serde_json::from_value(trimmed).expect("failed to parse trimmed spec as OpenAPI");

    let settings = progenitor::GenerationSettings::new()
        .with_interface(progenitor::InterfaceStyle::Builder)
        .with_tag(progenitor::TagStyle::Merged)
        .clone();

    let mut generator = progenitor::Generator::new(&settings);

    let tokens = generator
        .generate_tokens(&spec)
        .expect("progenitor code generation failed");

    let ast = syn::parse2(tokens).expect("failed to parse generated tokens");
    let content = prettyplease::unparse(&ast);

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_file = Path::new(&out_dir).join("pulumi_api.rs");
    fs::write(out_file, content).expect("failed to write generated code");
}

/// Build a trimmed OpenAPI spec that only contains the paths we need
/// and the schemas they reference (collected recursively).
fn trim_spec(full: &serde_json::Value) -> serde_json::Value {
    let empty_obj = serde_json::Value::Object(serde_json::Map::new());

    // 1. Filter paths
    let all_paths = full.get("paths").and_then(|v| v.as_object());
    let mut kept_paths = serde_json::Map::new();

    if let Some(paths) = all_paths {
        for (path, value) in paths {
            if should_keep_path(path) {
                kept_paths.insert(path.clone(), value.clone());
            }
        }
    }

    // 2. Strip operations that use unsupported content types (e.g. application/x-yaml)
    strip_unsupported_content_types(&mut kept_paths);

    // 3. Remove paths that ended up with no operations after stripping
    kept_paths.retain(|_, v| {
        if let Some(obj) = v.as_object() {
            // Keep if there's at least one HTTP method remaining
            obj.keys().any(|k| {
                matches!(
                    k.as_str(),
                    "get" | "post" | "put" | "patch" | "delete" | "head" | "options"
                )
            })
        } else {
            false
        }
    });

    // 4. Fix operations that have mixed typed/untyped success responses
    //    (progenitor requires at most one response type)
    fix_mixed_success_responses(&mut kept_paths);

    // 5. Collect all $ref references from kept paths (recursively)
    let mut needed_schemas = BTreeSet::new();
    collect_refs(
        &serde_json::Value::Object(kept_paths.clone()),
        &mut needed_schemas,
    );

    // 5. Recursively resolve schema references
    let all_schemas = full
        .get("components")
        .and_then(|c| c.get("schemas"))
        .and_then(|s| s.as_object());

    if let Some(schemas) = all_schemas {
        let mut prev_count = 0;
        while needed_schemas.len() != prev_count {
            prev_count = needed_schemas.len();
            let current: Vec<String> = needed_schemas.iter().cloned().collect();
            for name in &current {
                if let Some(schema) = schemas.get(name) {
                    collect_refs(schema, &mut needed_schemas);
                }
            }
        }
    }

    // 6. Build trimmed schemas
    let mut kept_schemas = serde_json::Map::new();
    if let Some(schemas) = all_schemas {
        for name in &needed_schemas {
            if let Some(schema) = schemas.get(name) {
                kept_schemas.insert(name.clone(), schema.clone());
            }
        }
    }

    // 6b. Patch schemas where the real API diverges from the spec.
    // The User schema marks `tokenInfo` as required, but the API returns null
    // for non-machine-token users. Make it optional.
    patch_nullable_fields(&mut kept_schemas);

    // 7. Build the trimmed spec
    let mut trimmed = serde_json::Map::new();
    trimmed.insert(
        "openapi".to_string(),
        full.get("openapi")
            .cloned()
            .unwrap_or_else(|| serde_json::Value::String("3.0.3".to_string())),
    );
    trimmed.insert(
        "info".to_string(),
        full.get("info")
            .cloned()
            .unwrap_or_else(|| empty_obj.clone()),
    );
    trimmed.insert("paths".to_string(), serde_json::Value::Object(kept_paths));

    let mut components = serde_json::Map::new();
    components.insert(
        "schemas".to_string(),
        serde_json::Value::Object(kept_schemas),
    );

    // Preserve securitySchemes if present
    if let Some(sec) = full
        .get("components")
        .and_then(|c| c.get("securitySchemes"))
    {
        components.insert("securitySchemes".to_string(), sec.clone());
    }

    trimmed.insert(
        "components".to_string(),
        serde_json::Value::Object(components),
    );

    // Preserve top-level security if present
    if let Some(security) = full.get("security") {
        trimmed.insert("security".to_string(), security.clone());
    }

    serde_json::Value::Object(trimmed)
}

/// Check if a path should be kept based on exact matching.
fn should_keep_path(path: &str) -> bool {
    KEPT_PATHS.contains(&path)
}

/// Fix operations that have both a typed (e.g. 200 with JSON) and an
/// untyped (e.g. 204 no content) success response. Progenitor requires
/// at most one response type, so we remove the untyped success responses.
fn fix_mixed_success_responses(paths: &mut serde_json::Map<String, serde_json::Value>) {
    let http_methods = ["get", "post", "put", "patch", "delete", "head", "options"];

    for (_path, path_item) in paths.iter_mut() {
        let Some(obj) = path_item.as_object_mut() else {
            continue;
        };
        for method in &http_methods {
            let Some(op) = obj.get_mut(*method) else {
                continue;
            };
            let Some(responses) = op.get_mut("responses").and_then(|r| r.as_object_mut()) else {
                continue;
            };

            // Check if we have mixed typed/untyped success responses
            let has_typed = responses.iter().any(|(status, resp)| {
                (status.starts_with('2') || status == "default")
                    && resp
                        .get("content")
                        .and_then(|c| c.get("application/json"))
                        .is_some()
            });

            if has_typed {
                // Remove untyped success responses (like 204 no content)
                let to_remove: Vec<String> = responses
                    .iter()
                    .filter_map(|(status, resp)| {
                        if status.starts_with('2')
                            && resp
                                .get("content")
                                .and_then(|c| c.get("application/json"))
                                .is_none()
                        {
                            Some(status.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                for key in to_remove {
                    responses.remove(&key);
                }
            }
        }
    }
}

/// Remove HTTP methods from paths whose request body or responses use
/// unsupported content types (e.g. `application/x-yaml`).
fn strip_unsupported_content_types(paths: &mut serde_json::Map<String, serde_json::Value>) {
    let http_methods = ["get", "post", "put", "patch", "delete", "head", "options"];
    let unsupported = ["application/x-yaml", "text/plain"];

    for (_path, path_item) in paths.iter_mut() {
        let Some(obj) = path_item.as_object_mut() else {
            continue;
        };
        let methods_to_remove: Vec<String> = http_methods
            .iter()
            .filter_map(|&method| {
                let op = obj.get(method)?;
                if operation_uses_content_type(op, &unsupported) {
                    Some(method.to_string())
                } else {
                    None
                }
            })
            .collect();
        for method in methods_to_remove {
            obj.remove(&method);
        }
    }
}

/// Check if an operation uses any of the given content types in its
/// requestBody or responses.
fn operation_uses_content_type(op: &serde_json::Value, types: &[&str]) -> bool {
    // Check requestBody.content
    if let Some(content) = op
        .get("requestBody")
        .and_then(|rb| rb.get("content"))
        .and_then(|c| c.as_object())
    {
        for ct in types {
            if content.contains_key(*ct) {
                return true;
            }
        }
    }
    // Check responses.*.content
    if let Some(responses) = op.get("responses").and_then(|r| r.as_object()) {
        for resp in responses.values() {
            if let Some(content) = resp.get("content").and_then(|c| c.as_object()) {
                for ct in types {
                    if content.contains_key(*ct) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Patch schemas where the live API diverges from the OpenAPI spec.
///
/// Several fields are marked `required` in the spec but the API returns
/// `null` for them in practice. We fix the spec so progenitor handles
/// the nullability correctly.
fn patch_nullable_fields(schemas: &mut serde_json::Map<String, serde_json::Value>) {
    // User.tokenInfo — only populated for machine tokens, null for humans.
    remove_required(schemas, "User", "tokenInfo");

    // AgentTask.entities — null when there are no linked entities.
    // For array fields, removing from required alone isn't enough; progenitor
    // still generates Vec with #[serde(default)] which can't deserialize null.
    // We must also mark the property as nullable.
    remove_required(schemas, "AgentTask", "entities");
    set_nullable(schemas, "AgentTask", "entities");
}

/// Helper: remove a field from a schema's `required` array.
fn remove_required(
    schemas: &mut serde_json::Map<String, serde_json::Value>,
    schema_name: &str,
    field_name: &str,
) {
    if let Some(schema) = schemas.get_mut(schema_name) {
        if let Some(required) = schema.get_mut("required").and_then(|r| r.as_array_mut()) {
            required.retain(|v| v.as_str() != Some(field_name));
        }
    }
}

/// Helper: set `nullable: true` on a schema property.
fn set_nullable(
    schemas: &mut serde_json::Map<String, serde_json::Value>,
    schema_name: &str,
    field_name: &str,
) {
    if let Some(prop) = schemas
        .get_mut(schema_name)
        .and_then(|s| s.get_mut("properties"))
        .and_then(|p| p.get_mut(field_name))
    {
        if let Some(obj) = prop.as_object_mut() {
            obj.insert("nullable".to_string(), serde_json::Value::Bool(true));
        }
    }
}

/// Recursively collect schema names from `$ref` values like
/// `#/components/schemas/FooBar`.
fn collect_refs(value: &serde_json::Value, out: &mut BTreeSet<String>) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::String(r)) = map.get("$ref") {
                if let Some(name) = r.strip_prefix("#/components/schemas/") {
                    out.insert(name.to_string());
                }
            }
            for v in map.values() {
                collect_refs(v, out);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                collect_refs(v, out);
            }
        }
        _ => {}
    }
}
