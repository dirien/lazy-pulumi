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

            tracing::debug!("ESC environments: requesting URL: {}", url);
            let response = self.client.get(&url).send().await?;

            if !response.status().is_success() {
                let status = response.status().as_u16();
                let message = response.text().await.unwrap_or_default();
                tracing::error!("ESC environments API error: {} - {}", status, message);
                return Err(ApiError::ApiResponse { status, message });
            }

            let text = response.text().await?;
            tracing::debug!("ESC environments API response: {}", &text[..text.len().min(1000)]);

            let data: FlexibleEscResponse = serde_json::from_str(&text).map_err(|e| {
                tracing::error!("Failed to parse ESC environments: {}. Response: {}", e, &text[..text.len().min(2000)]);
                ApiError::Parse(format!("Failed to parse ESC environments: {}", e))
            })?;

            let fetched_count = data.environments.len();
            tracing::info!(
                "ESC environments: fetched {} environments, continuation_token: {:?}",
                fetched_count,
                data.continuation_token
            );
            all_environments.extend(data.environments);

            match data.continuation_token {
                Some(token) if !token.is_empty() => {
                    continuation_token = Some(token);
                }
                _ => break,
            }
        }

        tracing::info!("ESC environments: total {} environments fetched for org '{}'", all_environments.len(), org);
        Ok(all_environments)
    }

    /// Get ESC environment details
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

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

        response.json().await.map_err(ApiError::Http)
    }

    /// Open an ESC environment to get resolved values
    pub async fn open_esc_environment(
        &self,
        org: &str,
        project: &str,
        env: &str,
    ) -> Result<EscOpenResponse, ApiError> {
        let url = format!(
            "{}/api/esc/environments/{}/{}/{}/open",
            self.config.base_url, org, project, env
        );

        let response = self.client.post(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

        response.json().await.map_err(ApiError::Http)
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
            tracing::debug!("Neo tasks API response (first 500 chars): {}", &text[..text.len().min(500)]);

            // Try parsing as { tasks: [...], continuationToken: ... } first
            if let Ok(data) = serde_json::from_str::<TasksResponse>(&text) {
                let fetched_count = data.tasks.len();
                tracing::debug!(
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
                tracing::error!("Failed to parse Neo tasks response. Response: {}", &text[..text.len().min(1000)]);
                return Err(ApiError::Parse("Failed to parse tasks response".to_string()));
            }

            // Safety limit to prevent infinite loops
            if all_tasks.len() > 10000 {
                tracing::warn!("Neo tasks pagination safety limit reached");
                break;
            }
        }

        tracing::info!("Neo tasks: total {} tasks fetched", all_tasks.len());
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
        tracing::debug!("Neo task metadata response: {}", &text[..text.len().min(500)]);

        serde_json::from_str::<NeoTask>(&text)
            .map_err(|e| {
                tracing::error!("Failed to parse Neo task metadata: {}. Response: {}", e, &text[..text.len().min(1000)]);
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
        let url = format!(
            "{}/api/preview/agents/{}/tasks/{}/events?pageSize=100",
            self.config.base_url, org, task_id
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

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

        let data: EventsResponse = response.json().await.unwrap_or(EventsResponse {
            events: vec![],
            continuation_token: None,
        });

        // Convert events to messages, including tool calls
        let messages: Vec<NeoMessage> = data.events.into_iter().filter_map(|event| {
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
                        // Parse the content which might be JSON
                        let display_content = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body.content) {
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
                            message_type: NeoMessageType::ToolResponse,
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
        }).collect();

        Ok(NeoTaskResponse {
            task_id: task_id.to_string(),
            status: None,
            messages,
            has_more: data.continuation_token.is_some(),
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
}
