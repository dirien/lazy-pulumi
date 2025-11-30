//! Common types for the Pulumi API

use chrono::DateTime;
use serde::{Deserialize, Serialize};

/// API configuration
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ApiConfig {
    pub base_url: String,
    pub access_token: String,
    pub organization: Option<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.pulumi.com".to_string(),
            access_token: String::new(),
            organization: None,
        }
    }
}

/// Pulumi Stack summary
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stack {
    pub org_name: String,
    pub project_name: String,
    pub stack_name: String,
    #[serde(default)]
    pub last_update: Option<i64>,
    #[serde(default)]
    pub resource_count: Option<i32>,
    #[serde(default)]
    pub url: Option<String>,
}

impl Stack {
    #[allow(dead_code)]
    pub fn full_name(&self) -> String {
        format!("{}/{}/{}", self.org_name, self.project_name, self.stack_name)
    }

    pub fn last_update_formatted(&self) -> String {
        match self.last_update {
            Some(ts) => {
                if let Some(dt) = DateTime::from_timestamp(ts, 0) {
                    dt.format("%Y-%m-%d %H:%M:%S").to_string()
                } else {
                    "Unknown".to_string()
                }
            }
            None => "Never".to_string(),
        }
    }
}

/// Stack list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StacksResponse {
    pub stacks: Vec<Stack>,
    #[serde(default)]
    pub continuation_token: Option<String>,
}

/// Stack update info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackUpdate {
    pub version: i32,
    #[serde(default)]
    pub start_time: Option<i64>,
    #[serde(default)]
    pub end_time: Option<i64>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub resource_changes: Option<ResourceChanges>,
}

/// Resource changes in an update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceChanges {
    #[serde(default)]
    pub create: Option<i32>,
    #[serde(default)]
    pub update: Option<i32>,
    #[serde(default)]
    pub delete: Option<i32>,
    #[serde(default)]
    pub same: Option<i32>,
}

/// ESC Environment summary
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct EscEnvironment {
    pub organization: String,
    pub project: String,
    pub name: String,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub modified: Option<String>,
}

#[allow(dead_code)]
impl EscEnvironment {
    pub fn full_name(&self) -> String {
        format!("{}/{}/{}", self.organization, self.project, self.name)
    }
}

/// ESC Environment list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscEnvironmentsResponse {
    pub environments: Vec<EscEnvironmentSummary>,
    #[serde(default)]
    pub next_token: Option<String>,
}

/// ESC Environment summary from list
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EscEnvironmentSummary {
    pub organization: String,
    pub project: String,
    pub name: String,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub modified_at: Option<String>,
}

/// ESC Environment details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscEnvironmentDetails {
    #[serde(default)]
    pub yaml: Option<String>,
    #[serde(default)]
    pub definition: Option<serde_json::Value>,
}

/// ESC Open session response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscOpenResponse {
    pub id: String,
    #[serde(default)]
    pub properties: Option<serde_json::Value>,
    #[serde(default)]
    pub values: Option<serde_json::Value>,
}

/// Neo Task
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeoTask {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

/// Neo Message type enum
#[derive(Debug, Clone, PartialEq)]
pub enum NeoMessageType {
    UserMessage,
    AssistantMessage,
    ToolCall,
    ToolResponse,
    ApprovalRequest,
    TaskNameChange,
}

/// Neo Tool Call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeoToolCall {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub args: Option<serde_json::Value>,
}

/// Neo Message
#[derive(Debug, Clone)]
pub struct NeoMessage {
    #[allow(dead_code)]
    pub role: String,
    pub content: String,
    pub message_type: NeoMessageType,
    #[allow(dead_code)]
    pub timestamp: Option<String>,
    /// Tool calls (for assistant messages with tools)
    pub tool_calls: Vec<NeoToolCall>,
    /// Tool name (for tool responses)
    pub tool_name: Option<String>,
}

/// Neo Create Task API response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeoCreateTaskResponse {
    pub task_id: String,
}

/// Neo Task response (internal struct, not from JSON)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct NeoTaskResponse {
    pub task_id: String,
    pub status: Option<String>,
    pub messages: Vec<NeoMessage>,
    pub has_more: bool,
    pub requires_approval: bool,
}

/// Neo Tasks list response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct NeoTasksResponse {
    pub tasks: Vec<NeoTask>,
}

/// Resource search result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSearchResult {
    #[serde(default)]
    pub total: Option<i64>,
    pub resources: Vec<Resource>,
}

/// Pulumi Resource
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub name: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub stack: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub package: Option<String>,
    #[serde(default)]
    pub modified: Option<String>,
}

/// Policy violation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct PolicyViolation {
    #[serde(default)]
    pub id: Option<String>,
    pub message: String,
    #[serde(default)]
    pub enforcement_level: Option<String>,
    #[serde(default)]
    pub policy_name: Option<String>,
    #[serde(default)]
    pub policy_pack: Option<String>,
}

/// User/member info
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct User {
    pub name: String,
    #[serde(default)]
    pub github_login: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
}

// ─────────────────────────────────────────────────────────────
// Platform Types (Services, Components, Templates)
// ─────────────────────────────────────────────────────────────

/// Service owner information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceOwner {
    #[serde(rename = "type")]
    pub owner_type: String,
    pub name: String,
}

/// Service item count summary
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServiceItemCountSummary {
    #[serde(default)]
    pub stacks: Option<i32>,
    #[serde(default)]
    pub environments: Option<i32>,
}

/// Pulumi Service
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    pub organization_name: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub owner: Option<ServiceOwner>,
    #[serde(default)]
    pub item_count_summary: Option<ServiceItemCountSummary>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub modified_at: Option<String>,
}

impl Service {
    #[allow(dead_code)]
    pub fn display_name(&self) -> String {
        self.name.clone()
    }

    pub fn item_count(&self) -> String {
        if let Some(ref summary) = self.item_count_summary {
            let stacks = summary.stacks.unwrap_or(0);
            let envs = summary.environments.unwrap_or(0);
            format!("{} stacks, {} envs", stacks, envs)
        } else {
            "0 items".to_string()
        }
    }
}

/// Services list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesResponse {
    #[serde(default)]
    pub services: Vec<Service>,
    #[serde(default)]
    pub continuation_token: Option<String>,
}

/// Registry Package/Component
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryPackage {
    pub name: String,
    #[serde(default)]
    pub publisher: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub logo_url: Option<String>,
    #[serde(default)]
    pub repository_url: Option<String>,
    /// URL to the README markdown content
    #[serde(default, rename = "readmeURL")]
    pub readme_url: Option<String>,
    /// Loaded README content (fetched separately)
    #[serde(skip)]
    pub readme_content: Option<String>,
}

impl RegistryPackage {
    pub fn display_name(&self) -> String {
        self.title.clone().unwrap_or_else(|| self.name.clone())
    }

    /// Get a unique key for this package (used for README caching)
    pub fn key(&self) -> String {
        format!(
            "{}/{}/{}",
            self.source.as_deref().unwrap_or("pulumi"),
            self.publisher.as_deref().unwrap_or("unknown"),
            self.name
        )
    }

    pub fn full_name(&self) -> String {
        let source = self.source.as_deref().unwrap_or("pulumi");
        let publisher = self.publisher.as_deref().unwrap_or("unknown");
        format!("{}/{}/{}", source, publisher, self.name)
    }
}

/// Registry packages list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryPackagesResponse {
    #[serde(default)]
    pub packages: Vec<RegistryPackage>,
    #[serde(default)]
    pub continuation_token: Option<String>,
}

/// Template runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateRuntime {
    pub name: String,
    #[serde(default)]
    pub options: Option<serde_json::Value>,
}

/// Registry Template
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryTemplate {
    pub name: String,
    #[serde(default)]
    pub publisher: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub runtime: Option<TemplateRuntime>,
    #[serde(default)]
    pub project_name: Option<String>,
}

impl RegistryTemplate {
    pub fn display(&self) -> String {
        self.display_name.clone().unwrap_or_else(|| self.name.clone())
    }

    pub fn full_name(&self) -> String {
        let source = self.source.as_deref().unwrap_or("private");
        let publisher = self.publisher.as_deref().unwrap_or("unknown");
        format!("{}/{}/{}", source, publisher, self.name)
    }
}

/// Registry templates list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryTemplatesResponse {
    #[serde(default)]
    pub templates: Vec<RegistryTemplate>,
    #[serde(default)]
    pub continuation_token: Option<String>,
}
