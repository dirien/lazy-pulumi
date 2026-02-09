use anyhow::{bail, Context, Result};
use std::path::PathBuf;

const SPEC_URL: &str = "https://api.pulumi.com/api/openapi/pulumi-spec.json";

fn workspace_root() -> PathBuf {
    let xtask_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    xtask_dir
        .parent()
        .expect("xtask crate must be inside workspace")
        .to_path_buf()
}

fn update_spec() -> Result<()> {
    let dest = workspace_root().join("openapi/pulumi-spec.json");

    println!("Downloading OpenAPI spec from {SPEC_URL} ...");

    let body = reqwest::blocking::get(SPEC_URL)
        .context("failed to request OpenAPI spec")?
        .error_for_status()
        .context("server returned an error")?
        .text()
        .context("failed to read response body")?;

    let spec: serde_json::Value =
        serde_json::from_str(&body).context("response is not valid JSON")?;

    if spec.get("openapi").is_none() {
        bail!("downloaded JSON does not contain an \"openapi\" key — is the URL correct?");
    }

    let pretty = serde_json::to_string_pretty(&spec).context("failed to pretty-print spec JSON")?;

    std::fs::create_dir_all(dest.parent().expect("dest has parent"))
        .context("failed to create openapi/ directory")?;
    std::fs::write(&dest, &pretty)
        .with_context(|| format!("failed to write {}", dest.display()))?;

    let version = spec
        .pointer("/info/version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let path_count = spec
        .get("paths")
        .and_then(|p| p.as_object())
        .map_or(0, |m| m.len());
    let size_kb = pretty.len() / 1024;

    println!("Wrote {} ({size_kb} KB)", dest.display());
    println!("  spec version : {version}");
    println!("  paths        : {path_count}");

    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.first().map(|s| s.as_str()) {
        Some("update-spec") => update_spec(),
        Some(cmd) => bail!("unknown xtask command: {cmd}\n\nAvailable commands:\n  update-spec   Download the latest Pulumi OpenAPI spec"),
        None => bail!("usage: cargo xtask <command>\n\nAvailable commands:\n  update-spec   Download the latest Pulumi OpenAPI spec"),
    }
}
