//! Neo (Preview Agents) API methods for `PulumiClient`.

use super::client::{map_gen_err, ApiError, PulumiClient};
use super::domain::{
    NeoCreateTaskMessage, NeoMessage, NeoMessageType, NeoSlashCommand, NeoSlashCommandPayload,
    NeoTaskResponse, NeoToolCall, NeoUpdateTaskRequest,
};
use super::generated;

impl PulumiClient {
    // ─────────────────────────────────────────────────────────────
    // Neo API (Preview Agents API)
    // ─────────────────────────────────────────────────────────────

    /// List Neo tasks (with pagination to get all results)
    pub async fn list_neo_tasks(
        &self,
        org: Option<&str>,
    ) -> Result<Vec<super::domain::NeoTask>, ApiError> {
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

            let tasks: Vec<super::domain::NeoTask> =
                data.tasks.into_iter().map(Into::into).collect();
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
    pub async fn get_neo_task(
        &self,
        org: &str,
        task_id: &str,
    ) -> Result<super::domain::NeoTask, ApiError> {
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
    ) -> Result<super::domain::NeoTask, ApiError> {
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
        approval_mode: Option<&str>,
        permission_mode: Option<&str>,
        plan_mode: Option<bool>,
    ) -> Result<NeoTaskResponse, ApiError> {
        let url = format!("{}/api/preview/agents/{}/tasks", self.config.base_url, org);

        let timestamp = chrono::Utc::now().to_rfc3339();
        let mut body = serde_json::json!({
            "message": {
                "type": "user_message",
                "content": query,
                "timestamp": timestamp
            }
        });

        if let Some(mode) = approval_mode {
            body["approvalMode"] = serde_json::Value::String(mode.to_string());
        }
        if let Some(mode) = permission_mode {
            body["permissionMode"] = serde_json::Value::String(mode.to_string());
        }
        if let Some(mode) = plan_mode {
            body["planMode"] = serde_json::Value::Bool(mode);
        }

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
        approval_mode: Option<&str>,
        permission_mode: Option<&str>,
        plan_mode: Option<bool>,
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

        let mut body = serde_json::json!({ "message": message });

        if let Some(mode) = approval_mode {
            body["approvalMode"] = serde_json::Value::String(mode.to_string());
        }
        if let Some(mode) = permission_mode {
            body["permissionMode"] = serde_json::Value::String(mode.to_string());
        }
        if let Some(mode) = plan_mode {
            body["planMode"] = serde_json::Value::Bool(mode);
        }

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
}
