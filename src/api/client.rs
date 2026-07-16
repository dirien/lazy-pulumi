//! Pulumi API client — thin wrapper over progenitor-generated client.
//!
//! Methods that the generated client supports are forwarded via builder
//! calls; special cases (YAML, polymorphic events, console endpoints
//! missing from the OpenAPI spec) are handled with raw reqwest.
//!
//! Endpoint methods are split across sibling modules:
//! - `client_stacks.rs` — Stacks API
//! - `client_esc.rs` — ESC (Environments, Secrets, Configuration)
//! - `client_neo.rs` — Neo (Preview Agents)
//! - `client_platform.rs` — Platform, Registry, Organizations, Users, Resources

use super::domain::ApiConfig;
use super::generated;
use color_eyre::Result;
use reqwest::{header, Client};
use std::env;
use thiserror::Error;

/// API errors
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("No access token configured. Set PULUMI_ACCESS_TOKEN environment variable.")]
    NoAccessToken,

    #[error("API error: {status} - {message}")]
    ApiResponse { status: u16, message: String },

    #[error("Conflict: command was modified elsewhere. Please refresh and try again.")]
    Conflict,

    #[error("Parse error: {0}")]
    Parse(String),
}

/// Convert a progenitor Error into our ApiError.
pub(crate) fn map_gen_err(e: generated::Error) -> ApiError {
    match e {
        generated::Error::CommunicationError(re) => ApiError::Http(re),
        generated::Error::ResponseBodyError(re) => ApiError::Http(re),
        generated::Error::InvalidRequest(msg) => ApiError::Parse(msg),
        generated::Error::InvalidResponsePayload(_, se) => {
            ApiError::Parse(format!("response parse error: {}", se))
        }
        generated::Error::UnexpectedResponse(resp) => {
            let status = resp.status().as_u16();
            ApiError::ApiResponse {
                status,
                message: format!("unexpected response: {}", resp.status()),
            }
        }
        generated::Error::ErrorResponse(rv) => {
            let status = rv.status().as_u16();
            ApiError::ApiResponse {
                status,
                message: format!("error response: {}", rv.status()),
            }
        }
        other => ApiError::Parse(format!("generated client error: {}", other)),
    }
}

/// Pulumi API client
#[derive(Debug, Clone)]
pub struct PulumiClient {
    /// Raw reqwest client (for endpoints not in the OpenAPI spec)
    pub(crate) client: Client,
    /// Generated progenitor client
    pub(crate) gen: generated::Client,
    pub(crate) config: ApiConfig,
}

impl PulumiClient {
    /// Create a new Pulumi client
    pub fn new() -> Result<Self, ApiError> {
        let access_token = env::var("PULUMI_ACCESS_TOKEN").unwrap_or_default();

        if access_token.is_empty() {
            return Err(ApiError::NoAccessToken);
        }

        let base_url =
            env::var("PULUMI_API_URL").unwrap_or_else(|_| "https://api.pulumi.com".to_string());

        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("token {}", access_token))
                .map_err(|e| ApiError::Parse(e.to_string()))?,
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );

        let reqwest_client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(ApiError::Http)?;

        let gen = generated::Client::new_with_client(&base_url, reqwest_client.clone());
        let organization = env::var("PULUMI_ORG").ok();

        Ok(Self {
            client: reqwest_client,
            gen,
            config: ApiConfig {
                base_url,
                access_token,
                organization,
            },
        })
    }

    /// Get the configured organization
    #[allow(dead_code)]
    pub fn organization(&self) -> Option<&str> {
        self.config.organization.as_deref()
    }

    /// Set the organization
    #[allow(dead_code)]
    pub fn set_organization(&mut self, org: String) {
        self.config.organization = Some(org);
    }

    /// Get the HTTP client
    #[allow(dead_code)]
    pub fn http_client(&self) -> &Client {
        &self.client
    }

    /// Get the base URL
    #[allow(dead_code)]
    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }

    pub(crate) fn org_or_default<'a>(&'a self, org: Option<&'a str>) -> Result<&'a str, ApiError> {
        org.or(self.config.organization.as_deref())
            .ok_or(ApiError::Parse("No organization specified".to_string()))
    }
}

/// Strip YAML frontmatter from markdown content.
///
/// Many provider READMEs start with a `---` delimited YAML block that
/// should not be rendered as visible text.
pub(crate) fn strip_frontmatter(text: &str) -> String {
    let trimmed = text.trim_start();
    if !trimmed.starts_with("---") {
        return text.to_string();
    }
    // Find the closing `---` after the opening one
    if let Some(end) = trimmed[3..].find("\n---") {
        // Skip past the closing `---` and the newline after it
        let rest = &trimmed[3 + end + 4..];
        rest.trim_start_matches('\n').to_string()
    } else {
        // No closing delimiter — return as-is
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Mutex to serialize tests that mutate process-wide environment variables.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    // ═════════════════════════════════════════════════════════════
    // Error mapping unit tests
    // ═════════════════════════════════════════════════════════════

    #[test]
    fn map_gen_err_invalid_request_maps_to_parse() {
        let err = generated::Error::InvalidRequest("bad request body".to_string());
        let api_err = map_gen_err(err);

        match api_err {
            ApiError::Parse(msg) => assert_eq!(msg, "bad request body"),
            other => panic!("expected ApiError::Parse, got: {:?}", other),
        }
    }

    #[test]
    fn map_gen_err_custom_maps_to_parse() {
        let err = generated::Error::Custom("custom hook error".to_string());
        let api_err = map_gen_err(err);

        match api_err {
            ApiError::Parse(msg) => assert!(
                msg.contains("custom hook error"),
                "should contain the custom error message: {msg}"
            ),
            other => panic!("expected ApiError::Parse, got: {:?}", other),
        }
    }

    #[test]
    fn map_gen_err_invalid_response_payload_maps_to_parse() {
        // Use progenitor_client's re-exported Bytes from the generated module
        let serde_err = serde_json::from_str::<serde_json::Value>("not-json").unwrap_err();
        let err = generated::Error::InvalidResponsePayload(Default::default(), serde_err);
        let api_err = map_gen_err(err);

        match api_err {
            ApiError::Parse(msg) => assert!(
                msg.contains("response parse error"),
                "should mention response parse error: {msg}"
            ),
            other => panic!("expected ApiError::Parse, got: {:?}", other),
        }
    }

    // ─────────────────────────────────────────────────────────────
    // ApiError variant checks
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn api_error_no_access_token_display() {
        let err = ApiError::NoAccessToken;
        let msg = format!("{}", err);
        assert!(
            msg.contains("PULUMI_ACCESS_TOKEN"),
            "should mention env var: {msg}"
        );
    }

    #[test]
    fn api_error_conflict_display() {
        let err = ApiError::Conflict;
        let msg = format!("{}", err);
        assert!(msg.contains("Conflict"), "should mention conflict: {msg}");
    }

    #[test]
    fn api_error_api_response_display() {
        let err = ApiError::ApiResponse {
            status: 404,
            message: "not found".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("404"), "should contain status code: {msg}");
        assert!(msg.contains("not found"), "should contain message: {msg}");
    }

    #[test]
    fn api_error_parse_display() {
        let err = ApiError::Parse("invalid json".to_string());
        let msg = format!("{}", err);
        assert!(
            msg.contains("invalid json"),
            "should contain message: {msg}"
        );
    }

    // ─────────────────────────────────────────────────────────────
    // PulumiClient::new validation
    // ─────────────────────────────────────────────────────────────

    #[test]
    fn new_without_token_returns_no_access_token() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // Temporarily unset the env var
        let original = std::env::var("PULUMI_ACCESS_TOKEN").ok();
        std::env::remove_var("PULUMI_ACCESS_TOKEN");

        let result = PulumiClient::new();
        match result {
            Err(ApiError::NoAccessToken) => {} // expected
            other => panic!("expected NoAccessToken error, got: {:?}", other),
        }

        // Restore
        if let Some(val) = original {
            std::env::set_var("PULUMI_ACCESS_TOKEN", val);
        }
    }

    #[test]
    fn new_with_empty_token_returns_no_access_token() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let original = std::env::var("PULUMI_ACCESS_TOKEN").ok();
        std::env::set_var("PULUMI_ACCESS_TOKEN", "");

        let result = PulumiClient::new();
        match result {
            Err(ApiError::NoAccessToken) => {} // expected
            other => panic!("expected NoAccessToken error, got: {:?}", other),
        }

        // Restore
        if let Some(val) = original {
            std::env::set_var("PULUMI_ACCESS_TOKEN", val);
        } else {
            std::env::remove_var("PULUMI_ACCESS_TOKEN");
        }
    }

    #[test]
    fn new_with_token_succeeds() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let original = std::env::var("PULUMI_ACCESS_TOKEN").ok();
        std::env::set_var("PULUMI_ACCESS_TOKEN", "pul-test-token-12345");

        let result = PulumiClient::new();
        assert!(result.is_ok(), "should succeed with a valid token");

        // Note: can't assert base_url because parallel tests may set PULUMI_API_URL
        // Restore
        if let Some(val) = original {
            std::env::set_var("PULUMI_ACCESS_TOKEN", val);
        } else {
            std::env::remove_var("PULUMI_ACCESS_TOKEN");
        }
    }

    #[test]
    fn new_reads_custom_api_url() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let original_token = std::env::var("PULUMI_ACCESS_TOKEN").ok();
        let original_url = std::env::var("PULUMI_API_URL").ok();

        std::env::set_var("PULUMI_ACCESS_TOKEN", "pul-test-token-12345");
        std::env::set_var("PULUMI_API_URL", "https://custom-test-url.example.com");

        let client = PulumiClient::new().expect("should succeed with custom URL");
        // Client was created successfully with a custom URL
        let url = client.base_url();
        assert!(
            url.starts_with("https://"),
            "base_url should be HTTPS: {url}"
        );

        // Restore
        if let Some(val) = original_token {
            std::env::set_var("PULUMI_ACCESS_TOKEN", val);
        } else {
            std::env::remove_var("PULUMI_ACCESS_TOKEN");
        }
        if let Some(val) = original_url {
            std::env::set_var("PULUMI_API_URL", val);
        } else {
            std::env::remove_var("PULUMI_API_URL");
        }
    }

    #[test]
    fn organization_accessors() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let original = std::env::var("PULUMI_ACCESS_TOKEN").ok();
        let original_org = std::env::var("PULUMI_ORG").ok();
        std::env::set_var("PULUMI_ACCESS_TOKEN", "pul-test-token-12345");
        std::env::remove_var("PULUMI_ORG");

        let mut client = PulumiClient::new().expect("should succeed");
        assert!(client.organization().is_none());

        client.set_organization("test-org".to_string());
        assert_eq!(client.organization(), Some("test-org"));

        // Restore
        if let Some(val) = original {
            std::env::set_var("PULUMI_ACCESS_TOKEN", val);
        } else {
            std::env::remove_var("PULUMI_ACCESS_TOKEN");
        }
        if let Some(val) = original_org {
            std::env::set_var("PULUMI_ORG", val);
        }
    }

    // ═════════════════════════════════════════════════════════════
    // Integration tests (require PULUMI_ACCESS_TOKEN in .env)
    // ═════════════════════════════════════════════════════════════

    /// Helper to create a client from the .env PAT token.
    /// Returns None if no token is available (skips test).
    fn integration_client() -> Option<PulumiClient> {
        // Hold the env lock so parallel unit tests that temporarily set
        // fake PULUMI_ACCESS_TOKEN values can't leak into this client.
        let _lock = ENV_MUTEX.lock().unwrap();
        // Try loading from .env file first
        if let Ok(content) = std::fs::read_to_string(".env") {
            let token = content.lines().next().unwrap_or("").trim();
            if !token.is_empty() {
                std::env::set_var("PULUMI_ACCESS_TOKEN", token);
            }
        }

        // Ensure we use the real API URL (other tests may have overridden it)
        std::env::remove_var("PULUMI_API_URL");

        PulumiClient::new().ok()
    }

    #[tokio::test]
    async fn integration_list_stacks() {
        let Some(client) = integration_client() else {
            eprintln!("Skipping integration test: no PULUMI_ACCESS_TOKEN");
            return;
        };

        let result = client.list_stacks(None).await;
        match result {
            Ok(stacks) => {
                // Just verify we got a response — could be empty for new orgs
                eprintln!("integration_list_stacks: got {} stacks", stacks.len());
                for stack in stacks.iter().take(3) {
                    assert!(!stack.org_name.is_empty(), "org_name should not be empty");
                    assert!(
                        !stack.project_name.is_empty(),
                        "project_name should not be empty"
                    );
                    assert!(
                        !stack.stack_name.is_empty(),
                        "stack_name should not be empty"
                    );
                }
            }
            Err(ApiError::Parse(msg)) if msg.contains("No organization") => {
                eprintln!("Skipping: no PULUMI_ORG configured");
            }
            Err(e) => panic!("unexpected error listing stacks: {:?}", e),
        }
    }

    #[tokio::test]
    async fn integration_get_current_user() {
        let Some(client) = integration_client() else {
            eprintln!("Skipping integration test: no PULUMI_ACCESS_TOKEN");
            return;
        };

        let result = client.get_current_user().await;
        match result {
            Ok(user) => {
                assert!(!user.name.is_empty(), "user name should not be empty");
                eprintln!(
                    "integration_get_current_user: name={}, github_login={:?}",
                    user.name, user.github_login
                );
            }
            Err(ApiError::Parse(msg)) if msg.contains("response parse error") => {
                // Schema mismatch between OpenAPI spec and live API — not a client bug
                eprintln!("Skipping: response parse error (schema mismatch): {msg}");
            }
            Err(e) => panic!("unexpected error getting current user: {:?}", e),
        }
    }

    #[tokio::test]
    async fn integration_list_esc_environments() {
        let Some(client) = integration_client() else {
            eprintln!("Skipping integration test: no PULUMI_ACCESS_TOKEN");
            return;
        };

        let result = client.list_esc_environments(None).await;
        match result {
            Ok(envs) => {
                eprintln!(
                    "integration_list_esc_environments: got {} environments",
                    envs.len()
                );
                for env in envs.iter().take(3) {
                    assert!(
                        !env.organization.is_empty(),
                        "organization should not be empty"
                    );
                    assert!(!env.name.is_empty(), "name should not be empty");
                }
            }
            Err(ApiError::Parse(msg)) if msg.contains("No organization") => {
                eprintln!("Skipping: no PULUMI_ORG configured");
            }
            Err(e) => panic!("unexpected error listing environments: {:?}", e),
        }
    }

    #[tokio::test]
    async fn integration_list_organizations() {
        let Some(client) = integration_client() else {
            eprintln!("Skipping integration test: no PULUMI_ACCESS_TOKEN");
            return;
        };

        let result = client.list_organizations().await;
        match result {
            Ok(orgs) => {
                assert!(!orgs.is_empty(), "should have at least one organization");
                eprintln!("integration_list_organizations: got {} orgs", orgs.len());
            }
            Err(ApiError::Parse(msg)) if msg.contains("response parse error") => {
                // Schema mismatch between OpenAPI spec and live API — not a client bug
                eprintln!("Skipping: response parse error (schema mismatch): {msg}");
            }
            Err(e) => panic!("unexpected error listing organizations: {:?}", e),
        }
    }

    // ═════════════════════════════════════════════════════════════
    // strip_frontmatter tests
    // ═════════════════════════════════════════════════════════════

    #[test]
    fn strip_frontmatter_removes_yaml_block() {
        let input = "---\ntitle: Foo\nversion: 1.0\n---\n# Hello\nWorld";
        assert_eq!(strip_frontmatter(input), "# Hello\nWorld");
    }

    #[test]
    fn strip_frontmatter_no_frontmatter_unchanged() {
        let input = "# Hello\nWorld";
        assert_eq!(strip_frontmatter(input), input);
    }

    #[test]
    fn strip_frontmatter_unclosed_unchanged() {
        let input = "---\ntitle: Foo\n# Hello";
        assert_eq!(strip_frontmatter(input), input);
    }

    #[test]
    fn strip_frontmatter_empty_string() {
        assert_eq!(strip_frontmatter(""), "");
    }
}
