//! Main Pulumi API client

use super::types::{
    ApiConfig, EscEnvironmentDetails, EscEnvironmentSummary, EscOpenResponse,
    NeoCreateTaskResponse, NeoMessage, NeoMessageType, NeoTask, NeoTaskResponse, NeoToolCall,
    RegistryPackage, RegistryPackagesResponse, RegistryTemplate, RegistryTemplatesResponse,
    Resource, Service, ServicesResponse, Stack, StacksResponse, StackUpdate, User,
};
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

    #[error("Parse error: {0}")]
    Parse(String),
}

/// Pulumi API client
#[derive(Debug, Clone)]
pub struct PulumiClient {
    client: Client,
    config: ApiConfig,
}

impl PulumiClient {
    /// Create a new Pulumi client
    pub fn new() -> Result<Self, ApiError> {
        let access_token = env::var("PULUMI_ACCESS_TOKEN").unwrap_or_default();

        if access_token.is_empty() {
            return Err(ApiError::NoAccessToken);
        }

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

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(ApiError::Http)?;

        let organization = env::var("PULUMI_ORG").ok();

        Ok(Self {
            client,
            config: ApiConfig {
                base_url: env::var("PULUMI_API_URL")
                    .unwrap_or_else(|_| "https://api.pulumi.com".to_string()),
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

    // ─────────────────────────────────────────────────────────────
    // Stacks API
    // ─────────────────────────────────────────────────────────────

    /// List all stacks
    pub async fn list_stacks(&self, org: Option<&str>) -> Result<Vec<Stack>, ApiError> {
        let org = org
            .or(self.config.organization.as_deref())
            .ok_or(ApiError::Parse("No organization specified".to_string()))?;

        let url = format!("{}/api/user/stacks?organization={}", self.config.base_url, org);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

        let data: StacksResponse = response.json().await?;
        Ok(data.stacks)
    }

    /// Get stack details
    #[allow(dead_code)]
    pub async fn get_stack(
        &self,
        org: &str,
        project: &str,
        stack: &str,
    ) -> Result<Stack, ApiError> {
        let url = format!(
            "{}/api/stacks/{}/{}/{}",
            self.config.base_url, org, project, stack
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

        response.json().await.map_err(ApiError::Http)
    }

    /// Get stack updates history
    pub async fn get_stack_updates(
        &self,
        org: &str,
        project: &str,
        stack: &str,
    ) -> Result<Vec<StackUpdate>, ApiError> {
        let url = format!(
            "{}/api/stacks/{}/{}/{}/updates?pageSize=20",
            self.config.base_url, org, project, stack
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

        #[derive(serde::Deserialize)]
        struct UpdatesResponse {
            updates: Vec<StackUpdate>,
        }

        let data: UpdatesResponse = response.json().await?;
        Ok(data.updates)
    }

    /// Get recent updates across all stacks in the organization
    /// Uses the console API which returns all data in a single call
    pub async fn get_org_recent_updates(
        &self,
        org: Option<&str>,
        limit: usize,
    ) -> Result<Vec<super::types::OrgStackUpdate>, ApiError> {
        let org = org
            .or(self.config.organization.as_deref())
            .ok_or(ApiError::Parse("No organization specified".to_string()))?;

        let url = format!(
            "{}/api/console/orgs/{}/stacks/updates/recent?limit={}",
            self.config.base_url, org, limit
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RecentUpdateItem {
            #[serde(default)]
            org_name: String,
            /// Stack name
            #[serde(default)]
            name: String,
            #[serde(default)]
            project: String,
            #[serde(default)]
            last_update: Option<LastUpdate>,
        }

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct LastUpdate {
            #[serde(default)]
            info: Option<UpdateInfo>,
            #[serde(default)]
            version: i32,
            #[serde(default)]
            requested_by: Option<RequestedBy>,
        }

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct UpdateInfo {
            #[serde(default)]
            kind: String,
            #[serde(default)]
            result: String,
            #[serde(default)]
            start_time: Option<i64>,
            #[serde(default)]
            end_time: Option<i64>,
            #[serde(default)]
            resource_changes: Option<super::types::ResourceChanges>,
        }

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct RequestedBy {
            #[serde(default)]
            name: Option<String>,
            #[serde(default)]
            github_login: Option<String>,
        }

        let items: Vec<RecentUpdateItem> = response.json().await?;

        let updates: Vec<super::types::OrgStackUpdate> = items
            .into_iter()
            .filter_map(|item| {
                let last_update = item.last_update?;
                let info = last_update.info?;

                Some(super::types::OrgStackUpdate {
                    org_name: item.org_name,
                    project_name: item.project,
                    stack_name: item.name,
                    kind: info.kind,
                    result: info.result,
                    start_time: info.start_time?,
                    end_time: info.end_time,
                    version: last_update.version,
                    resource_changes: info.resource_changes,
                    requested_by: last_update.requested_by.and_then(|r| {
                        // Prefer github_login, fall back to name
                        r.github_login.or(r.name)
                    }),
                })
            })
            .collect();

        Ok(updates)
    }

    // ─────────────────────────────────────────────────────────────
    // ESC API
    // ─────────────────────────────────────────────────────────────

    /// List ESC environments (with pagination to get all results)
    pub async fn list_esc_environments(
        &self,
        org: Option<&str>,
    ) -> Result<Vec<EscEnvironmentSummary>, ApiError> {
        let org = org
            .or(self.config.organization.as_deref())
            .ok_or(ApiError::Parse("No organization specified".to_string()))?;

        let mut all_environments = Vec::new();
        let mut continuation_token: Option<String> = None;

        // Use a flexible response struct that captures any continuation token field
        #[derive(serde::Deserialize, Debug)]
        struct FlexibleEscResponse {
            #[serde(default)]
            environments: Vec<EscEnvironmentSummary>,
            #[serde(default, alias = "nextToken", alias = "next_token")]
            continuation_token: Option<String>,
        }

        loop {
            let url = match &continuation_token {
                Some(token) => format!(
                    "{}/api/esc/environments/{}?continuationToken={}",
                    self.config.base_url,
                    org,
                    urlencoding::encode(token)
                ),
                None => format!("{}/api/esc/environments/{}", self.config.base_url, org),
            };

            log::debug!("ESC environments: requesting URL: {}", url);
            let response = self.client.get(&url).send().await?;

            if !response.status().is_success() {
                let status = response.status().as_u16();
                let message = response.text().await.unwrap_or_default();
                log::error!("ESC environments API error: {} - {}", status, message);
                return Err(ApiError::ApiResponse { status, message });
            }

            let text = response.text().await?;
            log::debug!("ESC environments API response: {}", &text[..text.len().min(1000)]);

            let data: FlexibleEscResponse = serde_json::from_str(&text).map_err(|e| {
                log::error!("Failed to parse ESC environments: {}. Response: {}", e, &text[..text.len().min(2000)]);
                ApiError::Parse(format!("Failed to parse ESC environments: {}", e))
            })?;

            let fetched_count = data.environments.len();
            log::info!(
                "ESC environments: fetched {} environments, continuation_token: {:?}",
                fetched_count,
                data.continuation_token
            );
            // Populate organization field since API doesn't include it (implied from URL)
            let envs_with_org: Vec<EscEnvironmentSummary> = data.environments.into_iter().map(|mut env| {
                if env.organization.is_empty() {
                    env.organization = org.to_string();
                }
                env
            }).collect();
            all_environments.extend(envs_with_org);

            match data.continuation_token {
                Some(token) if !token.is_empty() => {
                    continuation_token = Some(token);
                }
                _ => break,
            }
        }

        log::info!("ESC environments: total {} environments fetched for org '{}'", all_environments.len(), org);
        Ok(all_environments)
    }

    /// Get ESC environment details (YAML definition)
    /// The API returns the YAML content directly as a string
    pub async fn get_esc_environment(
        &self,
        org: &str,
        project: &str,
        env: &str,
    ) -> Result<EscEnvironmentDetails, ApiError> {
        let url = format!(
            "{}/api/esc/environments/{}/{}/{}",
            self.config.base_url, org, project, env
        );

        log::debug!("GET ESC environment: {}", url);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

        let text = response.text().await?;
        log::debug!("ESC environment details response: {}", &text[..text.len().min(500)]);

        // The API returns YAML content directly as text, not JSON
        // So we just return it as the yaml field
        Ok(EscEnvironmentDetails {
            yaml: Some(text),
            definition: None,
            created: None,
            modified: None,
            revision: None,
            extra: std::collections::HashMap::new(),
        })
    }

    /// Open an ESC environment to get resolved values
    /// This is a two-step process: first open the session, then read the values
    pub async fn open_esc_environment(
        &self,
        org: &str,
        project: &str,
        env: &str,
    ) -> Result<EscOpenResponse, ApiError> {
        // Step 1: Open the environment session
        let open_url = format!(
            "{}/api/esc/environments/{}/{}/{}/open",
            self.config.base_url, org, project, env
        );

        log::debug!("POST ESC environment open: {}", open_url);
        let response = self.client.post(&open_url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

        // Parse the open response to get the session ID
        // Note: diagnostics can be an array of objects like:
        // {"diagnostics":[{"range":...,"summary":"no matching item","path":"values.stackRefs"}]}
        #[derive(serde::Deserialize, Debug)]
        struct DiagnosticItem {
            #[serde(default)]
            summary: Option<String>,
            #[serde(default)]
            path: Option<String>,
        }

        #[derive(serde::Deserialize, Debug)]
        struct OpenSessionResponse {
            #[serde(default)]
            id: Option<serde_json::Value>, // Can be number, string, or missing if error
            #[serde(default)]
            diagnostics: Option<Vec<DiagnosticItem>>,
        }

        let text = response.text().await?;
        log::debug!("ESC environment open response: {}", &text[..text.len().min(500)]);

        let open_response: OpenSessionResponse = serde_json::from_str(&text).map_err(|e| {
            log::error!("Failed to parse ESC open response: {}. Response: {}", e, &text[..text.len().min(1000)]);
            ApiError::Parse(format!("Failed to parse open response: {}", e))
        })?;

        // Check for diagnostics errors (environment has configuration issues)
        if let Some(diagnostics) = &open_response.diagnostics {
            if !diagnostics.is_empty() {
                let error_messages: Vec<String> = diagnostics
                    .iter()
                    .filter_map(|d| {
                        let summary = d.summary.as_deref().unwrap_or("Unknown error");
                        let path = d.path.as_deref().map(|p| format!(" at {}", p)).unwrap_or_default();
                        Some(format!("{}{}", summary, path))
                    })
                    .collect();
                let combined = error_messages.join("; ");
                log::warn!("ESC environment has diagnostics: {}", combined);
                return Err(ApiError::Parse(format!("Environment error: {}", combined)));
            }
        }

        // Convert session ID to string (it can be returned as number or string)
        let session_id = match open_response.id {
            Some(serde_json::Value::Number(n)) => n.to_string(),
            Some(serde_json::Value::String(s)) => s,
            _ => return Err(ApiError::Parse("No session ID returned - environment may have errors".to_string())),
        };

        log::debug!("ESC environment session opened: id={}", session_id);

        // Step 2: Read the resolved values from the open session
        let read_url = format!(
            "{}/api/esc/environments/{}/{}/{}/open/{}",
            self.config.base_url, org, project, env, session_id
        );

        log::debug!("GET ESC environment open values: {}", read_url);
        let values_response = self.client.get(&read_url).send().await?;

        if !values_response.status().is_success() {
            let status = values_response.status().as_u16();
            let message = values_response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

        let values_text = values_response.text().await?;
        log::debug!("ESC environment values response: {}", &values_text[..values_text.len().min(500)]);

        // Parse the values as JSON
        let values: serde_json::Value = serde_json::from_str(&values_text).map_err(|e| {
            log::error!("Failed to parse ESC values: {}. Response: {}", e, &values_text[..values_text.len().min(1000)]);
            ApiError::Parse(format!("Failed to parse values: {}", e))
        })?;

        Ok(EscOpenResponse {
            id: Some(session_id),
            properties: None,
            values: Some(values),
        })
    }

    /// Update an ESC environment definition (YAML content)
    pub async fn update_esc_environment(
        &self,
        org: &str,
        project: &str,
        env: &str,
        yaml_content: &str,
    ) -> Result<(), ApiError> {
        let url = format!(
            "{}/api/esc/environments/{}/{}/{}",
            self.config.base_url, org, project, env
        );

        log::debug!("PATCH ESC environment: {}", url);

        let response = self
            .client
            .patch(&url)
            .header("Content-Type", "application/x-yaml")
            .body(yaml_content.to_string())
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            log::error!("ESC environment update error: {} - {}", status, message);
            return Err(ApiError::ApiResponse { status, message });
        }

        log::info!("ESC environment updated successfully: {}/{}/{}", org, project, env);
        Ok(())
    }

    // ─────────────────────────────────────────────────────────────
    // Neo API (Preview Agents API)
    // ─────────────────────────────────────────────────────────────

    /// List Neo tasks (with pagination to get all results)
    pub async fn list_neo_tasks(&self, org: Option<&str>) -> Result<Vec<NeoTask>, ApiError> {
        let org = org
            .or(self.config.organization.as_deref())
            .ok_or(ApiError::Parse("No organization specified".to_string()))?;

        let mut all_tasks = Vec::new();
        let mut continuation_token: Option<String> = None;
        let page_size = 100;

        #[derive(serde::Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct TasksResponse {
            #[serde(default)]
            tasks: Vec<NeoTask>,
            #[serde(default)]
            continuation_token: Option<String>,
        }

        loop {
            let url = match &continuation_token {
                Some(token) => format!(
                    "{}/api/preview/agents/{}/tasks?pageSize={}&continuationToken={}",
                    self.config.base_url,
                    org,
                    page_size,
                    urlencoding::encode(token)
                ),
                None => format!(
                    "{}/api/preview/agents/{}/tasks?pageSize={}",
                    self.config.base_url, org, page_size
                ),
            };

            let response = self.client.get(&url).send().await?;

            if !response.status().is_success() {
                let status = response.status().as_u16();
                let message = response.text().await.unwrap_or_default();
                return Err(ApiError::ApiResponse { status, message });
            }

            let text = response.text().await?;
            log::debug!("Neo tasks API response (first 500 chars): {}", &text[..text.len().min(500)]);

            // Try parsing as { tasks: [...], continuationToken: ... } first
            if let Ok(data) = serde_json::from_str::<TasksResponse>(&text) {
                let fetched_count = data.tasks.len();
                log::debug!(
                    "Neo tasks: fetched {} tasks, continuation_token: {:?}",
                    fetched_count,
                    data.continuation_token
                );
                all_tasks.extend(data.tasks);

                // Check if there are more pages
                match data.continuation_token {
                    Some(token) if !token.is_empty() => {
                        continuation_token = Some(token);
                    }
                    _ => {
                        // No more pages - also break if we got fewer than page_size
                        if fetched_count < page_size {
                            break;
                        }
                        // If we got exactly page_size but no token, still break
                        break;
                    }
                }
            } else if let Ok(tasks) = serde_json::from_str::<Vec<NeoTask>>(&text) {
                // Try parsing as direct array (no pagination in this format)
                all_tasks.extend(tasks);
                break;
            } else {
                // Log and return error
                log::error!("Failed to parse Neo tasks response. Response: {}", &text[..text.len().min(1000)]);
                return Err(ApiError::Parse("Failed to parse tasks response".to_string()));
            }

            // Safety limit to prevent infinite loops
            if all_tasks.len() > 10000 {
                log::warn!("Neo tasks pagination safety limit reached");
                break;
            }
        }

        log::info!("Neo tasks: total {} tasks fetched", all_tasks.len());
        Ok(all_tasks)
    }

    /// Get a single Neo task's metadata by ID
    pub async fn get_neo_task(&self, org: &str, task_id: &str) -> Result<NeoTask, ApiError> {
        let url = format!(
            "{}/api/preview/agents/{}/tasks/{}",
            self.config.base_url, org, task_id
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

        let text = response.text().await?;
        log::debug!("Neo task metadata response: {}", &text[..text.len().min(500)]);

        serde_json::from_str::<NeoTask>(&text)
            .map_err(|e| {
                log::error!("Failed to parse Neo task metadata: {}. Response: {}", e, &text[..text.len().min(1000)]);
                ApiError::Parse(format!("Failed to parse task metadata: {}", e))
            })
    }

    /// Create a new Neo task
    pub async fn create_neo_task(
        &self,
        org: &str,
        query: &str,
    ) -> Result<NeoTaskResponse, ApiError> {
        let url = format!("{}/api/preview/agents/{}/tasks", self.config.base_url, org);

        let timestamp = chrono::Utc::now().to_rfc3339();
        let body = serde_json::json!({
            "message": {
                "type": "user_message",
                "content": query,
                "timestamp": timestamp
            }
        });

        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

        let create_response: NeoCreateTaskResponse = response.json().await.map_err(ApiError::Http)?;

        Ok(NeoTaskResponse {
            task_id: create_response.task_id,
            status: None,
            messages: vec![],
            has_more: false,
            requires_approval: false,
        })
    }

    /// Continue/respond to a Neo task
    pub async fn continue_neo_task(
        &self,
        org: &str,
        task_id: &str,
        query: Option<&str>,
    ) -> Result<NeoTaskResponse, ApiError> {
        // If no query, just get the events instead
        if query.is_none() {
            return self.get_neo_task_events(org, task_id).await;
        }

        let url = format!(
            "{}/api/preview/agents/{}/tasks/{}",
            self.config.base_url, org, task_id
        );

        let timestamp = chrono::Utc::now().to_rfc3339();
        let body = serde_json::json!({
            "event": {
                "type": "user_message",
                "content": query.unwrap(),
                "timestamp": timestamp
            }
        });

        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

        // Response is 202 Accepted with no body, so return with the task_id
        Ok(NeoTaskResponse {
            task_id: task_id.to_string(),
            status: None,
            messages: vec![],
            has_more: false,
            requires_approval: false,
        })
    }

    /// Get Neo task events (messages)
    pub async fn get_neo_task_events(
        &self,
        org: &str,
        task_id: &str,
    ) -> Result<NeoTaskResponse, ApiError> {
        #[derive(serde::Deserialize, Debug)]
        struct ToolCallRaw {
            #[serde(default)]
            id: String,
            #[serde(default)]
            name: String,
            #[serde(default)]
            args: Option<serde_json::Value>,
        }

        #[derive(serde::Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        #[allow(dead_code)]
        struct EventBody {
            /// The type of event body
            #[serde(rename = "type")]
            #[serde(default)]
            body_type: String,
            /// Content can be a string (user/assistant messages) or JSON object (tool responses)
            #[serde(default)]
            #[serde(deserialize_with = "deserialize_content")]
            content: String,
            #[serde(default)]
            timestamp: Option<String>,
            /// Tool calls for assistant messages
            #[serde(default)]
            tool_calls: Vec<ToolCallRaw>,
            /// Tool name for tool responses, also used for task name in set_task_name events
            #[serde(default)]
            name: Option<String>,
            /// Tool call ID for tool responses
            #[serde(default)]
            tool_call_id: Option<String>,
            /// Message for approval requests
            #[serde(default)]
            message: Option<String>,
            /// Whether this tool response is an error
            #[serde(default)]
            is_error: bool,
        }

        /// Custom deserializer that handles content being either a string or JSON object
        fn deserialize_content<'de, D>(deserializer: D) -> Result<String, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            use serde::Deserialize;
            let value = serde_json::Value::deserialize(deserializer)?;
            match value {
                serde_json::Value::String(s) => Ok(s),
                serde_json::Value::Null => Ok(String::new()),
                other => Ok(other.to_string()),
            }
        }

        #[derive(serde::Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        #[allow(dead_code)]
        struct TaskEvent {
            #[serde(rename = "type")]
            event_type: String,
            #[serde(default)]
            event_body: Option<EventBody>,
        }

        #[derive(serde::Deserialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct EventsResponse {
            #[serde(default)]
            events: Vec<TaskEvent>,
            #[serde(default)]
            continuation_token: Option<String>,
        }

        // Helper to convert an event to a message
        fn event_to_message(event: TaskEvent) -> Option<NeoMessage> {
            event.event_body.and_then(|body| {
                match body.body_type.as_str() {
                    "user_message" => Some(NeoMessage {
                        role: "user".to_string(),
                        content: body.content,
                        message_type: NeoMessageType::UserMessage,
                        timestamp: body.timestamp,
                        tool_calls: vec![],
                        tool_name: None,
                    }),
                    "assistant_message" => {
                        let tool_calls: Vec<NeoToolCall> = body.tool_calls.into_iter().map(|tc| {
                            NeoToolCall {
                                id: tc.id,
                                name: tc.name,
                                args: tc.args,
                            }
                        }).collect();
                        Some(NeoMessage {
                            role: "assistant".to_string(),
                            content: body.content,
                            message_type: NeoMessageType::AssistantMessage,
                            timestamp: body.timestamp,
                            tool_calls,
                            tool_name: None,
                        })
                    },
                    "exec_tool_call" => Some(NeoMessage {
                        role: "tool".to_string(),
                        content: format!("Executing: {}", body.name.as_deref().unwrap_or("unknown")),
                        message_type: NeoMessageType::ToolCall,
                        timestamp: body.timestamp,
                        tool_calls: vec![],
                        tool_name: body.name,
                    }),
                    "tool_response" => {
                        // Check if this is an error response
                        let is_error = body.is_error;

                        // Parse the content which might be JSON
                        let display_content = if is_error {
                            // For errors, show the full error message (don't truncate)
                            body.content.clone()
                        } else if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body.content) {
                            if let Some(result) = json.get("result") {
                                // Truncate long results
                                let result_str = result.to_string();
                                if result_str.len() > 200 {
                                    format!("{}...", &result_str[..200])
                                } else {
                                    result_str
                                }
                            } else {
                                body.content.clone()
                            }
                        } else {
                            body.content.clone()
                        };
                        Some(NeoMessage {
                            role: "tool_result".to_string(),
                            content: display_content,
                            message_type: if is_error { NeoMessageType::ToolError } else { NeoMessageType::ToolResponse },
                            timestamp: body.timestamp,
                            tool_calls: vec![],
                            tool_name: body.name,
                        })
                    },
                    "user_approval_request" => Some(NeoMessage {
                        role: "system".to_string(),
                        content: body.message.unwrap_or_else(|| "Approval requested".to_string()),
                        message_type: NeoMessageType::ApprovalRequest,
                        timestamp: body.timestamp,
                        tool_calls: vec![],
                        tool_name: None,
                    }),
                    "set_task_name" => Some(NeoMessage {
                        role: "system".to_string(),
                        content: format!("Task: {}", body.name.clone().unwrap_or_default()),
                        message_type: NeoMessageType::TaskNameChange,
                        timestamp: body.timestamp,
                        tool_calls: vec![],
                        tool_name: None,
                    }),
                    _ => None,
                }
            })
        }

        // Paginate through all events
        let mut all_messages: Vec<NeoMessage> = Vec::new();
        let mut continuation_token: Option<String> = None;
        let max_pages = 10; // Safety limit to prevent infinite loops

        for _ in 0..max_pages {
            let url = if let Some(ref token) = continuation_token {
                format!(
                    "{}/api/preview/agents/{}/tasks/{}/events?pageSize=100&continuationToken={}",
                    self.config.base_url, org, task_id, token
                )
            } else {
                format!(
                    "{}/api/preview/agents/{}/tasks/{}/events?pageSize=100",
                    self.config.base_url, org, task_id
                )
            };

            let response = self.client.get(&url).send().await?;

            if !response.status().is_success() {
                let status = response.status().as_u16();
                let message = response.text().await.unwrap_or_default();
                return Err(ApiError::ApiResponse { status, message });
            }

            let data: EventsResponse = response.json().await.unwrap_or(EventsResponse {
                events: vec![],
                continuation_token: None,
            });

            // Convert events to messages
            let page_messages: Vec<NeoMessage> = data.events
                .into_iter()
                .filter_map(event_to_message)
                .collect();

            all_messages.extend(page_messages);

            // Check if there are more pages
            if data.continuation_token.is_none() {
                break;
            }
            continuation_token = data.continuation_token;
        }

        Ok(NeoTaskResponse {
            task_id: task_id.to_string(),
            status: None,
            messages: all_messages,
            has_more: false, // We've fetched all pages
            requires_approval: false,
        })
    }

    // ─────────────────────────────────────────────────────────────
    // Resource Search API
    // ─────────────────────────────────────────────────────────────

    /// Search resources (with pagination to get all results)
    pub async fn search_resources(
        &self,
        org: Option<&str>,
        query: &str,
    ) -> Result<Vec<Resource>, ApiError> {
        let org = org
            .or(self.config.organization.as_deref())
            .ok_or(ApiError::Parse("No organization specified".to_string()))?;

        let mut all_resources = Vec::new();
        let mut page = 1;
        let page_size = 100;

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Pagination {
            #[serde(default)]
            next: Option<String>,
        }

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SearchResponse {
            #[serde(default)]
            resources: Vec<Resource>,
            #[serde(default)]
            pagination: Option<Pagination>,
        }

        loop {
            let url = format!(
                "{}/api/orgs/{}/search/resourcesv2?query={}&page={}&size={}",
                self.config.base_url,
                org,
                urlencoding::encode(query),
                page,
                page_size
            );

            let response = self.client.get(&url).send().await?;

            if !response.status().is_success() {
                let status = response.status().as_u16();
                let message = response.text().await.unwrap_or_default();
                return Err(ApiError::ApiResponse { status, message });
            }

            let data: SearchResponse = response.json().await?;
            let fetched_count = data.resources.len();
            all_resources.extend(data.resources);

            // Check if there's a next page
            let has_next = data.pagination
                .as_ref()
                .and_then(|p| p.next.as_ref())
                .is_some();

            // Stop if no next page or we got fewer results than page size
            if !has_next || fetched_count < page_size {
                break;
            }

            page += 1;

            // Safety limit to prevent infinite loops (10,000 resources max via page-based pagination)
            if page > 100 {
                break;
            }
        }

        Ok(all_resources)
    }

    // ─────────────────────────────────────────────────────────────
    // Users API
    // ─────────────────────────────────────────────────────────────

    /// List organization members
    #[allow(dead_code)]
    pub async fn list_users(&self, org: Option<&str>) -> Result<Vec<User>, ApiError> {
        let org = org
            .or(self.config.organization.as_deref())
            .ok_or(ApiError::Parse("No organization specified".to_string()))?;

        let url = format!("{}/api/orgs/{}/members", self.config.base_url, org);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

        #[derive(serde::Deserialize)]
        struct MembersResponse {
            members: Vec<User>,
        }

        let data: MembersResponse = response.json().await?;
        Ok(data.members)
    }

    /// Get current user info
    #[allow(dead_code)]
    pub async fn get_current_user(&self) -> Result<User, ApiError> {
        let url = format!("{}/api/user", self.config.base_url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

        response.json().await.map_err(ApiError::Http)
    }

    // ─────────────────────────────────────────────────────────────
    // Platform API (Services, Components, Templates)
    // ─────────────────────────────────────────────────────────────

    /// List services in an organization
    pub async fn list_services(&self, org: Option<&str>) -> Result<Vec<Service>, ApiError> {
        let org = org
            .or(self.config.organization.as_deref())
            .ok_or(ApiError::Parse("No organization specified".to_string()))?;

        let url = format!("{}/api/orgs/{}/services", self.config.base_url, org);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

        let data: ServicesResponse = response.json().await?;
        Ok(data.services)
    }

    /// List registry packages (components)
    pub async fn list_registry_packages(
        &self,
        org: Option<&str>,
    ) -> Result<Vec<RegistryPackage>, ApiError> {
        let org = org
            .or(self.config.organization.as_deref())
            .ok_or(ApiError::Parse("No organization specified".to_string()))?;

        let url = format!(
            "{}/api/preview/registry/packages?orgLogin={}&limit=50",
            self.config.base_url, org
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

        let data: RegistryPackagesResponse = response.json().await?;
        Ok(data.packages)
    }

    /// List registry templates
    pub async fn list_registry_templates(
        &self,
        org: Option<&str>,
    ) -> Result<Vec<RegistryTemplate>, ApiError> {
        let org = org
            .or(self.config.organization.as_deref())
            .ok_or(ApiError::Parse("No organization specified".to_string()))?;

        let url = format!(
            "{}/api/preview/registry/templates?orgLogin={}",
            self.config.base_url, org
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

        let data: RegistryTemplatesResponse = response.json().await?;
        Ok(data.templates)
    }

    // ─────────────────────────────────────────────────────────────
    // Organizations API
    // ─────────────────────────────────────────────────────────────

    /// List organizations for current user
    pub async fn list_organizations(&self) -> Result<Vec<String>, ApiError> {
        // The organizations are returned as part of the /api/user response
        let url = format!("{}/api/user", self.config.base_url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

        #[derive(serde::Deserialize)]
        struct UserResponse {
            #[serde(default)]
            organizations: Vec<OrgInfo>,
            // The user's own username is also an "org" for personal stacks
            #[serde(rename = "githubLogin")]
            #[serde(default)]
            github_login: Option<String>,
        }

        #[derive(serde::Deserialize)]
        struct OrgInfo {
            #[serde(rename = "githubLogin")]
            github_login: String,
        }

        let data: UserResponse = response.json().await?;

        // Collect organizations
        let mut orgs: Vec<String> = data
            .organizations
            .into_iter()
            .map(|o| o.github_login)
            .collect();

        // Also add the user's personal org (their username) if available
        if let Some(user_login) = data.github_login {
            if !orgs.contains(&user_login) {
                orgs.insert(0, user_login);
            }
        }

        Ok(orgs)
    }

    /// Fetch README content from a URL
    pub async fn fetch_readme(&self, readme_url: &str) -> Result<String, ApiError> {
        let response = self.client.get(readme_url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

        response.text().await.map_err(ApiError::Http)
    }

    // ─────────────────────────────────────────────────────────────
    // Resource Summary API
    // ─────────────────────────────────────────────────────────────

    /// Get resource count summary over time (for dashboard chart)
    pub async fn get_resource_summary(
        &self,
        org: Option<&str>,
        granularity: &str,
        lookback_days: i32,
    ) -> Result<Vec<super::types::ResourceSummaryPoint>, ApiError> {
        let org = org
            .or(self.config.organization.as_deref())
            .ok_or(ApiError::Parse("No organization specified".to_string()))?;

        let url = format!(
            "{}/api/orgs/{}/resources/summary?granularity={}&lookbackDays={}",
            self.config.base_url, org, granularity, lookback_days
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

        let data: super::types::ResourceSummaryResponse = response.json().await?;
        Ok(data.summary)
    }
}
