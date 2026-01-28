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
        format!(
            "{}/{}/{}",
            self.org_name, self.project_name, self.stack_name
        )
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

/// Organization-level stack update (includes stack info)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OrgStackUpdate {
    pub org_name: String,
    pub project_name: String,
    pub stack_name: String,
    pub kind: String,
    pub result: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub version: i32,
    pub resource_changes: Option<ResourceChanges>,
    /// Username who performed the update
    pub requested_by: Option<String>,
}

#[allow(dead_code)]
impl OrgStackUpdate {
    pub fn stack_display(&self) -> String {
        format!("{}/{}", self.project_name, self.stack_name)
    }

    pub fn start_time_formatted(&self) -> String {
        if let Some(dt) = DateTime::from_timestamp(self.start_time, 0) {
            dt.format("%Y-%m-%d %H:%M").to_string()
        } else {
            "Unknown".to_string()
        }
    }

    pub fn result_symbol(&self) -> &str {
        match self.result.as_str() {
            "succeeded" => "✓",
            "failed" => "✗",
            "in-progress" => "⟳",
            _ => "?",
        }
    }

    pub fn changes_summary(&self) -> String {
        if let Some(ref changes) = self.resource_changes {
            let mut parts = Vec::new();
            if let Some(c) = changes.create {
                if c > 0 {
                    parts.push(format!("+{}", c));
                }
            }
            if let Some(u) = changes.update {
                if u > 0 {
                    parts.push(format!("~{}", u));
                }
            }
            if let Some(d) = changes.delete {
                if d > 0 {
                    parts.push(format!("-{}", d));
                }
            }
            if parts.is_empty() {
                "no changes".to_string()
            } else {
                parts.join(" ")
            }
        } else {
            String::new()
        }
    }
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
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EscEnvironmentsResponse {
    #[serde(default)]
    pub environments: Vec<EscEnvironmentSummary>,
    /// Continuation token for pagination (API may use either field name)
    #[serde(default, alias = "nextToken")]
    pub continuation_token: Option<String>,
}

/// ESC Environment summary from list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscEnvironmentSummary {
    #[serde(default)]
    pub organization: String,
    #[serde(default)]
    pub project: String,
    #[serde(default)]
    pub name: String,
    /// Created timestamp (API returns "created", not "createdAt")
    #[serde(default)]
    pub created: Option<String>,
    /// Modified timestamp (API returns "modified", not "modifiedAt")
    #[serde(default)]
    pub modified: Option<String>,
}

/// ESC Environment details
/// Note: The API response varies - may include yaml, definition, or other fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EscEnvironmentDetails {
    #[serde(default)]
    pub yaml: Option<String>,
    #[serde(default)]
    pub definition: Option<serde_json::Value>,
    // Additional fields that might be present
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub modified: Option<String>,
    #[serde(default)]
    pub revision: Option<i64>,
    // Catch any other fields
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/// ESC Open session response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscOpenResponse {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub properties: Option<serde_json::Value>,
    #[serde(default)]
    pub values: Option<serde_json::Value>,
}

/// Helper to deserialize null as empty Vec
fn null_to_empty_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    let opt: Option<Vec<T>> = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

/// Neo Task
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeoTask {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    /// Task status: "running" or "idle"
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    /// User who created the task (API returns as "createdBy")
    #[serde(default, alias = "createdBy")]
    pub started_by: Option<NeoTaskUser>,
    /// Whether this task is shared with other org members
    #[serde(default)]
    pub is_shared: Option<bool>,
    /// When the task was first shared (null if never shared)
    #[serde(default)]
    pub shared_at: Option<String>,
    /// Linked pull requests
    #[serde(default, deserialize_with = "null_to_empty_vec")]
    pub linked_prs: Vec<NeoLinkedPR>,
    /// Involved entities (stacks, environments, etc.)
    #[serde(default, deserialize_with = "null_to_empty_vec")]
    pub entities: Vec<NeoEntity>,
    /// Active policies
    #[serde(default, deserialize_with = "null_to_empty_vec")]
    pub policies: Vec<NeoPolicy>,
}

/// User who started a Neo task
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeoTaskUser {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub login: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
}

/// Linked Pull Request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeoLinkedPR {
    #[serde(default)]
    pub number: Option<i32>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
}

/// Entity involved in a Neo task
/// Supports types: "stack", "repository", "pull_request", "policy_issue"
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeoEntity {
    /// Entity type: "stack", "repository", "pull_request", "policy_issue"
    #[serde(rename = "type")]
    #[serde(default)]
    pub entity_type: Option<String>,
    /// Entity name (used by stack, repository)
    #[serde(default)]
    pub name: Option<String>,
    /// Project name (used by stack)
    #[serde(default)]
    pub project: Option<String>,
    /// Stack name (used by stack)
    #[serde(default)]
    pub stack: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    /// Organization name (used by repository)
    #[serde(default)]
    pub org: Option<String>,
    /// Git forge type (used by repository, e.g., "github", "gitlab")
    #[serde(default)]
    pub forge: Option<String>,
    /// Policy issue ID (used by policy_issue)
    #[serde(default)]
    pub id: Option<String>,
}

/// Policy associated with a Neo task
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeoPolicy {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub pack_name: Option<String>,
    #[serde(default)]
    pub enforcement_level: Option<String>,
}

/// Neo Message type enum
#[derive(Debug, Clone, PartialEq)]
pub enum NeoMessageType {
    UserMessage,
    AssistantMessage,
    ToolCall,
    ToolResponse,
    /// Tool response that resulted in an error (is_error: true from API)
    ToolError,
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

/// Neo Update Task request (for PATCH endpoint)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct NeoUpdateTaskRequest {
    /// Whether to share the task with other org members
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_shared: Option<bool>,
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

// ─────────────────────────────────────────────────────────────
// Neo Slash Commands Types
// ─────────────────────────────────────────────────────────────

/// Neo Slash Command (available from /commands API)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeoSlashCommand {
    /// Command name (e.g., "get-started")
    pub name: String,
    /// Full prompt text to be sent
    pub prompt: String,
    /// Short description shown to user
    pub description: String,
    /// Whether this is a built-in command
    #[serde(default)]
    pub built_in: bool,
    /// Last modified timestamp
    #[serde(default)]
    pub modified_at: Option<String>,
    /// Unique tag/hash for this command version
    #[serde(default)]
    pub tag: Option<String>,
}

impl NeoSlashCommand {
    /// Generate the command reference string for the API payload
    /// Format: {{cmd:name:tag}}
    pub fn command_reference(&self) -> String {
        let tag = self.tag.as_deref().unwrap_or("");
        format!("{{{{cmd:{}:{}}}}}", self.name, tag)
    }
}

/// Message structure for creating a task (supports both plain text and commands)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeoCreateTaskMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub content: String,
    pub timestamp: String,
    /// Commands map - only present when using slash commands
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commands: Option<std::collections::HashMap<String, NeoSlashCommandPayload>>,
}

/// Slash command data included in the payload
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeoSlashCommandPayload {
    pub name: String,
    pub prompt: String,
    pub description: String,
    pub built_in: bool,
    pub modified_at: String,
    pub tag: String,
}

/// Resource search result
#[allow(dead_code)]
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
        self.display_name
            .clone()
            .unwrap_or_else(|| self.name.clone())
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

// ─────────────────────────────────────────────────────────────
// Resource Summary Types (for resource count over time chart)
// ─────────────────────────────────────────────────────────────

/// Daily resource count data point
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSummaryPoint {
    pub year: i32,
    pub month: i32,
    pub day: i32,
    pub resources: i64,
    #[serde(default)]
    pub resource_hours: Option<i64>,
}

impl ResourceSummaryPoint {
    /// Get the date as a formatted string (e.g., "Nov 30")
    #[allow(dead_code)]
    pub fn date_label(&self) -> String {
        let month_name = match self.month {
            1 => "Jan",
            2 => "Feb",
            3 => "Mar",
            4 => "Apr",
            5 => "May",
            6 => "Jun",
            7 => "Jul",
            8 => "Aug",
            9 => "Sep",
            10 => "Oct",
            11 => "Nov",
            12 => "Dec",
            _ => "???",
        };
        format!("{} {}", month_name, self.day)
    }
}

/// Resource summary API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSummaryResponse {
    pub summary: Vec<ResourceSummaryPoint>,
}
