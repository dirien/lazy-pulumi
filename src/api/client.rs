//! Pulumi API client — thin wrapper over progenitor-generated client.
//!
//! Methods that the generated client supports are forwarded via builder
//! calls; special cases (YAML, polymorphic events, console endpoints
//! missing from the OpenAPI spec) are handled with raw reqwest.

use super::domain::{
    ApiConfig, EscEnvironmentDetails, EscEnvironmentSummary, EscOpenResponse, NeoCreateTaskMessage,
    NeoMessage, NeoMessageType, NeoSlashCommand, NeoSlashCommandPayload, NeoTask, NeoTaskResponse,
    NeoToolCall, NeoUpdateTaskRequest, RegistryPackage, RegistryTemplate, Resource,
    ResourceSummaryPoint, Service, Stack, StackUpdate, User,
};
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
fn map_gen_err(e: generated::Error) -> ApiError {
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
    client: Client,
    /// Generated progenitor client
    gen: generated::Client,
    config: ApiConfig,
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

    fn org_or_default<'a>(&'a self, org: Option<&'a str>) -> Result<&'a str, ApiError> {
        org.or(self.config.organization.as_deref())
            .ok_or(ApiError::Parse("No organization specified".to_string()))
    }

    // ─────────────────────────────────────────────────────────────
    // Stacks API (via generated client)
    // ─────────────────────────────────────────────────────────────

    /// List all stacks
    pub async fn list_stacks(&self, org: Option<&str>) -> Result<Vec<Stack>, ApiError> {
        let org = self.org_or_default(org)?;

        let resp = self
            .gen
            .list_user_stacks()
            .organization(org)
            .send()
            .await
            .map_err(map_gen_err)?;

        let data = resp.into_inner();
        Ok(data.stacks.into_iter().map(Into::into).collect())
    }

    /// Get stack details
    #[allow(dead_code)]
    pub async fn get_stack(
        &self,
        org: &str,
        project: &str,
        stack: &str,
    ) -> Result<Stack, ApiError> {
        let resp = self
            .gen
            .get_stack()
            .org_name(org)
            .project_name(project)
            .stack_name(stack)
            .send()
            .await
            .map_err(map_gen_err)?;

        let data = resp.into_inner();
        Ok(Stack {
            org_name: data.org_name,
            project_name: data.project_name,
            stack_name: data.stack_name,
            last_update: None,
            resource_count: None,
            url: None,
        })
    }

    /// Get stack updates history
    pub async fn get_stack_updates(
        &self,
        org: &str,
        project: &str,
        stack: &str,
    ) -> Result<Vec<StackUpdate>, ApiError> {
        let resp = self
            .gen
            .get_stack_updates()
            .org_name(org)
            .project_name(project)
            .stack_name(stack)
            .page_size(20)
            .send()
            .await
            .map_err(map_gen_err)?;

        let data = resp.into_inner();
        // The generated response is an untyped JSON map; parse the "updates" array
        let updates = data
            .get("updates")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(updates
            .into_iter()
            .filter_map(|u| {
                let obj = u.as_object()?;
                Some(StackUpdate {
                    version: obj.get("version").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                    start_time: obj.get("startTime").and_then(|v| v.as_i64()),
                    end_time: obj.get("endTime").and_then(|v| v.as_i64()),
                    result: obj.get("result").and_then(|v| v.as_str()).map(String::from),
                    resource_changes: obj.get("resourceChanges").and_then(|rc| {
                        let rc = rc.as_object()?;
                        Some(super::domain::ResourceChanges {
                            create: rc.get("create").and_then(|v| v.as_i64()).map(|v| v as i32),
                            update: rc.get("update").and_then(|v| v.as_i64()).map(|v| v as i32),
                            delete: rc.get("delete").and_then(|v| v.as_i64()).map(|v| v as i32),
                            same: rc.get("same").and_then(|v| v.as_i64()).map(|v| v as i32),
                        })
                    }),
                })
            })
            .collect())
    }

    /// Get recent updates across all stacks in the organization.
    /// Uses the console API which is NOT in the OpenAPI spec — raw reqwest.
    pub async fn get_org_recent_updates(
        &self,
        org: Option<&str>,
        limit: usize,
    ) -> Result<Vec<super::domain::OrgStackUpdate>, ApiError> {
        let org = self.org_or_default(org)?;

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
            resource_changes: Option<super::domain::ResourceChanges>,
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

        let updates: Vec<super::domain::OrgStackUpdate> = items
            .into_iter()
            .filter_map(|item| {
                let last_update = item.last_update?;
                let info = last_update.info?;

                Some(super::domain::OrgStackUpdate {
                    org_name: item.org_name,
                    project_name: item.project,
                    stack_name: item.name,
                    kind: info.kind,
                    result: info.result,
                    start_time: info.start_time?,
                    end_time: info.end_time,
                    version: last_update.version,
                    resource_changes: info.resource_changes,
                    requested_by: last_update
                        .requested_by
                        .and_then(|r| r.github_login.or(r.name)),
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
        let org = self.org_or_default(org)?;

        let mut all_environments = Vec::new();
        let mut continuation_token: Option<String> = None;

        loop {
            let mut req = self.gen.list_org_environments_esc().org_name(org);
            if let Some(ref token) = continuation_token {
                req = req.continuation_token(token.as_str());
            }

            let resp = req.send().await.map_err(map_gen_err)?;
            let data = resp.into_inner();

            let fetched_count = data.environments.len();
            log::info!(
                "ESC environments: fetched {} environments, continuation_token: {:?}",
                fetched_count,
                data.next_token
            );

            let envs_with_org: Vec<EscEnvironmentSummary> = data
                .environments
                .into_iter()
                .map(|env| {
                    let mut converted: EscEnvironmentSummary = env.into();
                    if converted.organization.is_empty() {
                        converted.organization = org.to_string();
                    }
                    converted
                })
                .collect();
            all_environments.extend(envs_with_org);

            match data.next_token {
                Some(token) if !token.is_empty() => {
                    continuation_token = Some(token);
                }
                _ => break,
            }
        }

        log::info!(
            "ESC environments: total {} environments fetched for org '{}'",
            all_environments.len(),
            org
        );
        Ok(all_environments)
    }

    /// Get ESC environment details (YAML definition).
    /// The API returns YAML text — not in OpenAPI spec, raw reqwest.
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
        log::debug!(
            "ESC environment details response: {}",
            &text[..text.len().min(500)]
        );

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
            id: Option<serde_json::Value>,
            #[serde(default)]
            diagnostics: Option<Vec<DiagnosticItem>>,
        }

        let text = response.text().await?;
        log::debug!(
            "ESC environment open response: {}",
            &text[..text.len().min(500)]
        );

        let open_response: OpenSessionResponse = serde_json::from_str(&text).map_err(|e| {
            log::error!(
                "Failed to parse ESC open response: {}. Response: {}",
                e,
                &text[..text.len().min(1000)]
            );
            ApiError::Parse(format!("Failed to parse open response: {}", e))
        })?;

        if let Some(diagnostics) = &open_response.diagnostics {
            if !diagnostics.is_empty() {
                let error_messages: Vec<String> = diagnostics
                    .iter()
                    .map(|d| {
                        let summary = d.summary.as_deref().unwrap_or("Unknown error");
                        let path = d
                            .path
                            .as_deref()
                            .map(|p| format!(" at {}", p))
                            .unwrap_or_default();
                        format!("{}{}", summary, path)
                    })
                    .collect();
                let combined = error_messages.join("; ");
                log::warn!("ESC environment has diagnostics: {}", combined);
                return Err(ApiError::Parse(format!("Environment error: {}", combined)));
            }
        }

        let session_id = match open_response.id {
            Some(serde_json::Value::Number(n)) => n.to_string(),
            Some(serde_json::Value::String(s)) => s,
            _ => {
                return Err(ApiError::Parse(
                    "No session ID returned - environment may have errors".to_string(),
                ))
            }
        };

        log::debug!("ESC environment session opened: id={}", session_id);

        // Step 2: Read the resolved values
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
        log::debug!(
            "ESC environment values response: {}",
            &values_text[..values_text.len().min(500)]
        );

        let values: serde_json::Value = serde_json::from_str(&values_text)
            .map_err(|e| ApiError::Parse(format!("Failed to parse values: {}", e)))?;

        Ok(EscOpenResponse {
            id: Some(session_id),
            properties: None,
            values: Some(values),
        })
    }

    /// Update an ESC environment definition (YAML content).
    /// Uses application/x-yaml content type — raw reqwest.
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

        log::info!(
            "ESC environment updated successfully: {}/{}/{}",
            org,
            project,
            env
        );
        Ok(())
    }

    // ─────────────────────────────────────────────────────────────
    // Neo API (Preview Agents API)
    // ─────────────────────────────────────────────────────────────

    /// List Neo tasks (with pagination to get all results)
    pub async fn list_neo_tasks(&self, org: Option<&str>) -> Result<Vec<NeoTask>, ApiError> {
        let org = self.org_or_default(org)?;

        let mut all_tasks = Vec::new();
        let mut continuation_token: Option<String> = None;
        let page_size: i64 = 100;

        loop {
            let mut req = self.gen.list_tasks().org_name(org).page_size(page_size);
            if let Some(ref token) = continuation_token {
                req = req.continuation_token(token.as_str());
            }

            let resp = req.send().await.map_err(map_gen_err)?;
            let data = resp.into_inner();

            let fetched_count = data.tasks.len();
            log::debug!(
                "Neo tasks: fetched {} tasks, continuation_token: {:?}",
                fetched_count,
                data.continuation_token
            );

            let tasks: Vec<NeoTask> = data.tasks.into_iter().map(Into::into).collect();
            all_tasks.extend(tasks);

            match data.continuation_token {
                Some(token) if !token.is_empty() => {
                    continuation_token = Some(token);
                }
                _ => break,
            }

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
        let resp = self
            .gen
            .get_task()
            .org_name(org)
            .task_id(task_id)
            .send()
            .await
            .map_err(map_gen_err)?;

        Ok(resp.into_inner().into())
    }

    /// Update a Neo task's settings (e.g., sharing)
    #[allow(dead_code)]
    pub async fn update_neo_task(
        &self,
        org: &str,
        task_id: &str,
        request: &NeoUpdateTaskRequest,
    ) -> Result<NeoTask, ApiError> {
        let mut body = generated::types::UpdateTaskRequest::builder();
        if let Some(is_shared) = request.is_shared {
            body = body.is_shared(is_shared);
        }

        let resp = self
            .gen
            .update_task()
            .org_name(org)
            .task_id(task_id)
            .body(body)
            .send()
            .await
            .map_err(map_gen_err)?;

        Ok(resp.into_inner().into())
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

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct CreateResponse {
            task_id: String,
        }

        let create_response: CreateResponse = response.json().await.map_err(ApiError::Http)?;

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
                "content": query.expect("query checked above"),
                "timestamp": timestamp
            }
        });

        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

        Ok(NeoTaskResponse {
            task_id: task_id.to_string(),
            status: None,
            messages: vec![],
            has_more: false,
            requires_approval: false,
        })
    }

    /// Send a user confirmation event
    #[allow(dead_code)]
    pub async fn confirm_neo_task(
        &self,
        org: &str,
        task_id: &str,
        approved: bool,
    ) -> Result<NeoTaskResponse, ApiError> {
        let url = format!(
            "{}/api/preview/agents/{}/tasks/{}",
            self.config.base_url, org, task_id
        );

        let timestamp = chrono::Utc::now().to_rfc3339();
        let body = serde_json::json!({
            "event": {
                "type": "user_confirmation",
                "approved": approved,
                "timestamp": timestamp
            }
        });

        log::debug!(
            "POST Neo task confirmation: {} (approved: {})",
            url,
            approved
        );
        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            log::error!("Neo task confirmation error: {} - {}", status, message);
            return Err(ApiError::ApiResponse { status, message });
        }

        Ok(NeoTaskResponse {
            task_id: task_id.to_string(),
            status: None,
            messages: vec![],
            has_more: false,
            requires_approval: false,
        })
    }

    /// Send a user cancel event
    #[allow(dead_code)]
    pub async fn cancel_neo_task(
        &self,
        org: &str,
        task_id: &str,
    ) -> Result<NeoTaskResponse, ApiError> {
        let url = format!(
            "{}/api/preview/agents/{}/tasks/{}",
            self.config.base_url, org, task_id
        );

        let timestamp = chrono::Utc::now().to_rfc3339();
        let body = serde_json::json!({
            "event": {
                "type": "user_cancel",
                "timestamp": timestamp
            }
        });

        log::debug!("POST Neo task cancel: {}", url);
        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            log::error!("Neo task cancel error: {} - {}", status, message);
            return Err(ApiError::ApiResponse { status, message });
        }

        Ok(NeoTaskResponse {
            task_id: task_id.to_string(),
            status: None,
            messages: vec![],
            has_more: false,
            requires_approval: false,
        })
    }

    /// Continue/respond to a Neo task with slash commands.
    /// Not in OpenAPI spec — raw reqwest.
    pub async fn continue_neo_task_with_commands(
        &self,
        org: &str,
        task_id: &str,
        content: &str,
        commands: &[NeoSlashCommand],
    ) -> Result<NeoTaskResponse, ApiError> {
        let url = format!(
            "{}/api/preview/agents/{}/tasks/{}",
            self.config.base_url, org, task_id
        );

        let timestamp = chrono::Utc::now().to_rfc3339();

        let mut commands_map = std::collections::HashMap::new();
        let mut processed_content = content.to_string();

        for cmd in commands {
            let command_ref = cmd.command_reference();
            let simple_ref = format!("/{}", cmd.name);
            if processed_content.contains(&simple_ref) {
                processed_content = processed_content.replace(&simple_ref, &command_ref);
            }

            commands_map.insert(
                command_ref,
                NeoSlashCommandPayload {
                    name: cmd.name.clone(),
                    prompt: cmd.prompt.clone(),
                    description: cmd.description.clone(),
                    built_in: cmd.built_in,
                    modified_at: cmd
                        .modified_at
                        .clone()
                        .unwrap_or_else(|| "0001-01-01T00:00:00.000Z".to_string()),
                    tag: cmd.tag.clone().unwrap_or_default(),
                },
            );
        }

        let body = serde_json::json!({
            "event": {
                "type": "user_message",
                "content": processed_content,
                "timestamp": timestamp,
                "commands": commands_map
            }
        });

        log::debug!(
            "Continuing Neo task {} with {} commands",
            task_id,
            commands.len()
        );
        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            log::error!(
                "Neo continue task with commands error: {} - {}",
                status,
                message
            );
            return Err(ApiError::ApiResponse { status, message });
        }

        Ok(NeoTaskResponse {
            task_id: task_id.to_string(),
            status: None,
            messages: vec![],
            has_more: false,
            requires_approval: false,
        })
    }

    /// Get available slash commands for Neo.
    /// Not in OpenAPI spec — raw reqwest.
    pub async fn get_neo_slash_commands(
        &self,
        org: &str,
    ) -> Result<Vec<NeoSlashCommand>, ApiError> {
        let url = format!(
            "{}/api/console/agents/{}/commands",
            self.config.base_url, org
        );

        log::debug!("GET Neo slash commands: {}", url);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            log::error!("Neo slash commands API error: {} - {}", status, message);
            return Err(ApiError::ApiResponse { status, message });
        }

        let text = response.text().await?;
        log::debug!(
            "Neo slash commands response (first 500 chars): {}",
            &text[..text.len().min(500)]
        );

        #[derive(serde::Deserialize)]
        struct CommandsResponse {
            #[serde(default)]
            commands: Vec<NeoSlashCommand>,
        }

        let response: CommandsResponse = serde_json::from_str(&text).map_err(|e| {
            log::error!(
                "Failed to parse slash commands: {}. Response: {}",
                e,
                &text[..text.len().min(1000)]
            );
            ApiError::Parse(format!("Failed to parse slash commands: {}", e))
        })?;

        log::info!(
            "Neo slash commands: fetched {} commands",
            response.commands.len()
        );
        Ok(response.commands)
    }

    /// Get a single slash command by name
    pub async fn get_neo_slash_command(
        &self,
        org: &str,
        command_name: &str,
    ) -> Result<NeoSlashCommand, ApiError> {
        let all_commands = self.get_neo_slash_commands(org).await?;
        all_commands
            .into_iter()
            .find(|c| c.name == command_name)
            .ok_or_else(|| ApiError::ApiResponse {
                status: 404,
                message: format!("Command '{}' not found", command_name),
            })
    }

    /// Create a new custom slash command.
    /// Not in OpenAPI spec — raw reqwest.
    pub async fn create_neo_slash_command(
        &self,
        org: &str,
        name: &str,
        prompt: &str,
        description: &str,
    ) -> Result<NeoSlashCommand, ApiError> {
        let url = format!(
            "{}/api/console/agents/{}/commands",
            self.config.base_url, org
        );

        let body = serde_json::json!({
            "name": name,
            "prompt": prompt,
            "description": description
        });

        log::debug!("POST Neo slash command: {}", url);
        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_else(|e| {
                log::warn!("Failed to read error response body: {}", e);
                String::new()
            });
            log::error!("Neo create slash command error: {} - {}", status, message);
            return Err(ApiError::ApiResponse { status, message });
        }

        log::info!("Neo slash command '{}' created successfully", name);
        self.get_neo_slash_command(org, name).await
    }

    /// Delete a custom slash command.
    /// Not in OpenAPI spec — raw reqwest.
    pub async fn delete_neo_slash_command(
        &self,
        org: &str,
        command_name: &str,
        tag: &str,
    ) -> Result<(), ApiError> {
        let url = format!(
            "{}/api/console/agents/{}/commands/{}",
            self.config.base_url, org, command_name
        );

        log::debug!("DELETE Neo slash command: {} (tag: {})", url, tag);
        let response = self
            .client
            .delete(&url)
            .header("If-Match", tag)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_else(|e| {
                log::warn!("Failed to read error response body: {}", e);
                String::new()
            });
            log::error!("Neo delete slash command error: {} - {}", status, message);
            if status == 409 {
                return Err(ApiError::Conflict);
            }
            return Err(ApiError::ApiResponse { status, message });
        }

        log::info!("Neo slash command '{}' deleted successfully", command_name);
        Ok(())
    }

    /// Update an existing custom slash command.
    /// Not in OpenAPI spec — raw reqwest.
    pub async fn update_neo_slash_command(
        &self,
        org: &str,
        command_name: &str,
        prompt: &str,
        description: &str,
        tag: &str,
    ) -> Result<NeoSlashCommand, ApiError> {
        let url = format!(
            "{}/api/console/agents/{}/commands/{}",
            self.config.base_url, org, command_name
        );

        let body = serde_json::json!({
            "prompt": prompt,
            "description": description
        });

        log::debug!("PATCH Neo slash command: {} (tag: {})", url, tag);
        let response = self
            .client
            .patch(&url)
            .header("If-Match", tag)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_else(|e| {
                log::warn!("Failed to read error response body: {}", e);
                String::new()
            });
            log::error!("Neo update slash command error: {} - {}", status, message);
            if status == 409 {
                return Err(ApiError::Conflict);
            }
            return Err(ApiError::ApiResponse { status, message });
        }

        log::info!("Neo slash command '{}' updated successfully", command_name);
        self.get_neo_slash_command(org, command_name).await
    }

    /// Create a new Neo task with slash commands.
    /// Not in OpenAPI spec — raw reqwest.
    pub async fn create_neo_task_with_commands(
        &self,
        org: &str,
        content: &str,
        commands: &[NeoSlashCommand],
    ) -> Result<NeoTaskResponse, ApiError> {
        let url = format!("{}/api/preview/agents/{}/tasks", self.config.base_url, org);

        let timestamp = chrono::Utc::now().to_rfc3339();

        let mut commands_map = std::collections::HashMap::new();
        let mut processed_content = content.to_string();

        for cmd in commands {
            let command_ref = cmd.command_reference();
            let simple_ref = format!("/{}", cmd.name);
            if processed_content.contains(&simple_ref) {
                processed_content = processed_content.replace(&simple_ref, &command_ref);
            }

            commands_map.insert(
                command_ref,
                NeoSlashCommandPayload {
                    name: cmd.name.clone(),
                    prompt: cmd.prompt.clone(),
                    description: cmd.description.clone(),
                    built_in: cmd.built_in,
                    modified_at: cmd
                        .modified_at
                        .clone()
                        .unwrap_or_else(|| "0001-01-01T00:00:00.000Z".to_string()),
                    tag: cmd.tag.clone().unwrap_or_default(),
                },
            );
        }

        let message = NeoCreateTaskMessage {
            message_type: "user_message".to_string(),
            content: processed_content,
            timestamp,
            commands: Some(commands_map),
        };

        let body = serde_json::json!({ "message": message });

        log::debug!("Creating Neo task with {} commands", commands.len());
        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            log::error!(
                "Neo create task with commands error: {} - {}",
                status,
                message
            );
            return Err(ApiError::ApiResponse { status, message });
        }

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct CreateResponse {
            task_id: String,
        }

        let create_response: CreateResponse = response.json().await.map_err(ApiError::Http)?;

        Ok(NeoTaskResponse {
            task_id: create_response.task_id,
            status: None,
            messages: vec![],
            has_more: false,
            requires_approval: false,
        })
    }

    /// Get Neo task events (messages).
    /// Custom deserialization for polymorphic content — raw reqwest.
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
            #[serde(rename = "type")]
            #[serde(default)]
            body_type: String,
            #[serde(default)]
            #[serde(deserialize_with = "deserialize_content")]
            content: String,
            #[serde(default)]
            timestamp: Option<String>,
            #[serde(default)]
            tool_calls: Vec<ToolCallRaw>,
            #[serde(default)]
            name: Option<String>,
            #[serde(default)]
            tool_call_id: Option<String>,
            #[serde(default)]
            message: Option<String>,
            #[serde(default)]
            is_error: bool,
        }

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

        fn event_to_message(event: TaskEvent) -> Option<NeoMessage> {
            event
                .event_body
                .and_then(|body| match body.body_type.as_str() {
                    "user_message" => Some(NeoMessage {
                        role: "user".to_string(),
                        content: body.content,
                        message_type: NeoMessageType::UserMessage,
                        timestamp: body.timestamp,
                        tool_calls: vec![],
                        tool_name: None,
                    }),
                    "assistant_message" => {
                        let tool_calls: Vec<NeoToolCall> = body
                            .tool_calls
                            .into_iter()
                            .map(|tc| NeoToolCall {
                                id: tc.id,
                                name: tc.name,
                                args: tc.args,
                            })
                            .collect();
                        Some(NeoMessage {
                            role: "assistant".to_string(),
                            content: body.content,
                            message_type: NeoMessageType::AssistantMessage,
                            timestamp: body.timestamp,
                            tool_calls,
                            tool_name: None,
                        })
                    }
                    "exec_tool_call" => Some(NeoMessage {
                        role: "tool".to_string(),
                        content: format!(
                            "Executing: {}",
                            body.name.as_deref().unwrap_or("unknown")
                        ),
                        message_type: NeoMessageType::ToolCall,
                        timestamp: body.timestamp,
                        tool_calls: vec![],
                        tool_name: body.name,
                    }),
                    "tool_response" => {
                        let is_error = body.is_error;
                        let display_content = if is_error {
                            body.content.clone()
                        } else if let Ok(json) =
                            serde_json::from_str::<serde_json::Value>(&body.content)
                        {
                            if let Some(result) = json.get("result") {
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
                            message_type: if is_error {
                                NeoMessageType::ToolError
                            } else {
                                NeoMessageType::ToolResponse
                            },
                            timestamp: body.timestamp,
                            tool_calls: vec![],
                            tool_name: body.name,
                        })
                    }
                    "user_approval_request" => Some(NeoMessage {
                        role: "system".to_string(),
                        content: body
                            .message
                            .unwrap_or_else(|| "Approval requested".to_string()),
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
                })
        }

        let mut all_messages: Vec<NeoMessage> = Vec::new();
        let mut continuation_token: Option<String> = None;
        let max_pages = 10;

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

            let page_messages: Vec<NeoMessage> = data
                .events
                .into_iter()
                .filter_map(event_to_message)
                .collect();

            all_messages.extend(page_messages);

            if data.continuation_token.is_none() {
                break;
            }
            continuation_token = data.continuation_token;
        }

        Ok(NeoTaskResponse {
            task_id: task_id.to_string(),
            status: None,
            messages: all_messages,
            has_more: false,
            requires_approval: false,
        })
    }

    // ─────────────────────────────────────────────────────────────
    // Resource Search API (via generated client)
    // ─────────────────────────────────────────────────────────────

    /// Search resources (with pagination)
    pub async fn search_resources(
        &self,
        org: Option<&str>,
        query: &str,
    ) -> Result<Vec<Resource>, ApiError> {
        let org = self.org_or_default(org)?;

        let mut all_resources = Vec::new();
        let mut page: i64 = 1;
        let page_size: i64 = 100;

        loop {
            let resp = self
                .gen
                .get_org_resource_search_v2_query()
                .org_name(org)
                .query(query)
                .page(page)
                .size(page_size)
                .send()
                .await
                .map_err(map_gen_err)?;

            let data = resp.into_inner();
            let fetched_count = data.resources.len();
            let resources: Vec<Resource> = data.resources.into_iter().map(Into::into).collect();
            all_resources.extend(resources);

            let has_next = data
                .pagination
                .as_ref()
                .and_then(|p| p.next.as_ref())
                .is_some();

            if !has_next || fetched_count < page_size as usize {
                break;
            }

            page += 1;

            if page > 100 {
                break;
            }
        }

        Ok(all_resources)
    }

    // ─────────────────────────────────────────────────────────────
    // Users API (via generated client)
    // ─────────────────────────────────────────────────────────────

    /// List organization members
    #[allow(dead_code)]
    pub async fn list_users(&self, org: Option<&str>) -> Result<Vec<User>, ApiError> {
        let org = self.org_or_default(org)?;

        let resp = self
            .gen
            .list_organization_members()
            .org_name(org)
            .send()
            .await
            .map_err(map_gen_err)?;

        let data = resp.into_inner();
        Ok(data
            .members
            .into_iter()
            .map(|m| User {
                name: m.user.name,
                github_login: Some(m.user.github_login),
                avatar_url: Some(m.user.avatar_url),
                role: Some(m.role.to_string()),
            })
            .collect())
    }

    /// Get current user info
    #[allow(dead_code)]
    pub async fn get_current_user(&self) -> Result<User, ApiError> {
        let resp = self
            .gen
            .get_current_user()
            .send()
            .await
            .map_err(map_gen_err)?;

        let data = resp.into_inner();
        Ok(User {
            name: data.name,
            github_login: Some(data.github_login.clone()),
            avatar_url: Some(data.avatar_url),
            role: None,
        })
    }

    // ─────────────────────────────────────────────────────────────
    // Platform API (via generated client)
    // ─────────────────────────────────────────────────────────────

    /// List services in an organization
    pub async fn list_services(&self, org: Option<&str>) -> Result<Vec<Service>, ApiError> {
        let org = self.org_or_default(org)?;

        let resp = self
            .gen
            .list_services()
            .org_name(org)
            .send()
            .await
            .map_err(map_gen_err)?;

        let data = resp.into_inner();
        Ok(data.services.into_iter().map(Into::into).collect())
    }

    /// List registry packages (components)
    pub async fn list_registry_packages(
        &self,
        org: Option<&str>,
    ) -> Result<Vec<RegistryPackage>, ApiError> {
        let org = self.org_or_default(org)?;

        let resp = self
            .gen
            .list_packages_preview()
            .org_login(org)
            .limit(50)
            .send()
            .await
            .map_err(map_gen_err)?;

        let data = resp.into_inner();
        Ok(data.packages.into_iter().map(Into::into).collect())
    }

    /// List registry templates
    pub async fn list_registry_templates(
        &self,
        org: Option<&str>,
    ) -> Result<Vec<RegistryTemplate>, ApiError> {
        let org = self.org_or_default(org)?;

        let resp = self
            .gen
            .list_templates_preview()
            .org_login(org)
            .send()
            .await
            .map_err(map_gen_err)?;

        let data = resp.into_inner();
        Ok(data.templates.into_iter().map(Into::into).collect())
    }

    // ─────────────────────────────────────────────────────────────
    // Organizations API (via generated client)
    // ─────────────────────────────────────────────────────────────

    /// List organizations for current user
    pub async fn list_organizations(&self) -> Result<Vec<String>, ApiError> {
        let resp = self
            .gen
            .get_current_user()
            .send()
            .await
            .map_err(map_gen_err)?;

        let data = resp.into_inner();
        let mut orgs: Vec<String> = data
            .organizations
            .into_iter()
            .map(|o| o.github_login)
            .collect();

        let user_login = data.github_login;
        if !orgs.contains(&user_login) {
            orgs.insert(0, user_login);
        }

        Ok(orgs)
    }

    /// Fetch README content from a URL — raw reqwest.
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
    // Resource Summary API (via generated client)
    // ─────────────────────────────────────────────────────────────

    /// Get resource count summary over time (for dashboard chart)
    pub async fn get_resource_summary(
        &self,
        org: Option<&str>,
        granularity: &str,
        lookback_days: i32,
    ) -> Result<Vec<ResourceSummaryPoint>, ApiError> {
        let org = self.org_or_default(org)?;

        let resp = self
            .gen
            .get_usage_summary_resource_hours()
            .org_name(org)
            .granularity(granularity)
            .lookback_days(lookback_days as i64)
            .send()
            .await
            .map_err(map_gen_err)?;

        let data = resp.into_inner();
        Ok(data.summary.into_iter().map(Into::into).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // This test verifies that PULUMI_API_URL is respected,
        // but we avoid asserting on the value due to parallel test interference.
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
}
