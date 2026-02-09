//! Conversion layer between progenitor-generated types and domain types.
//!
//! Each `From` impl maps a generated API response type into the
//! domain type that the rest of the application works with.

use super::domain;
use super::generated::types as gen;

// ─────────────────────────────────────────────────────────────
// Stack conversions
// ─────────────────────────────────────────────────────────────

impl From<gen::AppStackSummary> for domain::Stack {
    fn from(s: gen::AppStackSummary) -> Self {
        Self {
            org_name: s.org_name,
            project_name: s.project_name,
            stack_name: s.stack_name,
            last_update: s.last_update,
            resource_count: s.resource_count.map(|r| r as i32),
            url: None,
        }
    }
}

// ─────────────────────────────────────────────────────────────
// ESC Environment conversions
// ─────────────────────────────────────────────────────────────

impl From<gen::OrgEnvironment> for domain::EscEnvironmentSummary {
    fn from(e: gen::OrgEnvironment) -> Self {
        Self {
            organization: e.organization.unwrap_or_default(),
            project: e.project.unwrap_or_default(),
            name: e.name.unwrap_or_default(),
            created: Some(e.created),
            modified: Some(e.modified),
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Neo Task conversions
// ─────────────────────────────────────────────────────────────

impl From<gen::AgentTask> for domain::NeoTask {
    fn from(t: gen::AgentTask) -> Self {
        let status_str = match &t.status {
            gen::AgentTaskStatus::Running => "running",
            gen::AgentTaskStatus::Idle => "idle",
        };
        Self {
            id: t.id,
            name: Some(t.name),
            status: Some(status_str.to_string()),
            created_at: Some(t.created_at.to_rfc3339()),
            updated_at: None,
            url: None,
            started_by: Some(domain::NeoTaskUser {
                name: Some(t.created_by.name.clone()),
                login: Some(t.created_by.github_login.clone()),
                avatar_url: Some(t.created_by.avatar_url.clone()),
            }),
            is_shared: Some(t.is_shared),
            shared_at: t.shared_at.map(|d| d.to_rfc3339()),
            linked_prs: vec![],
            entities: t
                .entities
                .unwrap_or_default()
                .into_iter()
                .map(Into::into)
                .collect(),
            policies: vec![],
        }
    }
}

impl From<gen::AgentEntity> for domain::NeoEntity {
    fn from(e: gen::AgentEntity) -> Self {
        Self {
            entity_type: Some(e.type_.clone()),
            name: None,
            project: None,
            stack: None,
            url: None,
            org: None,
            forge: None,
            id: None,
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Slash Command conversions
// ─────────────────────────────────────────────────────────────

impl From<gen::AgentSlashCommand> for domain::NeoSlashCommand {
    fn from(c: gen::AgentSlashCommand) -> Self {
        Self {
            name: c.name,
            prompt: c.prompt,
            description: c.description,
            built_in: c.built_in,
            modified_at: Some(c.modified_at.to_rfc3339()),
            tag: Some(c.tag),
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Resource conversions
// ─────────────────────────────────────────────────────────────

impl From<gen::ResourceResult> for domain::Resource {
    fn from(r: gen::ResourceResult) -> Self {
        Self {
            resource_type: r.type_.unwrap_or_default(),
            name: r.name.unwrap_or_default(),
            id: r.id,
            stack: r.stack,
            project: r.project,
            package: Some(r.package),
            modified: r.modified,
        }
    }
}

impl From<gen::ResourceCountSummary> for domain::ResourceSummaryPoint {
    fn from(r: gen::ResourceCountSummary) -> Self {
        Self {
            year: r.year as i32,
            month: r.month.unwrap_or(0) as i32,
            day: r.day.unwrap_or(0) as i32,
            resources: r.resources,
            resource_hours: Some(r.resource_hours),
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Service conversions
// ─────────────────────────────────────────────────────────────

impl From<gen::Service> for domain::Service {
    fn from(s: gen::Service) -> Self {
        // Convert the HashMap<String, i64> item_count_summary to our typed struct
        let item_count_summary = {
            let stacks = s
                .item_count_summary
                .get("stacks")
                .copied()
                .map(|v| v as i32);
            let envs = s
                .item_count_summary
                .get("environments")
                .copied()
                .map(|v| v as i32);
            if stacks.is_some() || envs.is_some() {
                Some(domain::ServiceItemCountSummary {
                    stacks,
                    environments: envs,
                })
            } else {
                None
            }
        };

        Self {
            organization_name: s.organization_name,
            name: s.name,
            description: Some(s.description),
            owner: if s.owner.name.is_empty() {
                None
            } else {
                Some(domain::ServiceOwner {
                    owner_type: "member".to_string(),
                    name: s.owner.name,
                })
            },
            item_count_summary,
            created_at: s.created.map(|d| d.to_rfc3339()),
            modified_at: None,
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Registry Package conversions
// ─────────────────────────────────────────────────────────────

impl From<gen::PackageMetadata> for domain::RegistryPackage {
    fn from(p: gen::PackageMetadata) -> Self {
        Self {
            name: p.name,
            publisher: Some(p.publisher),
            source: Some(p.source),
            version: Some(p.version),
            title: p.title,
            description: p.description,
            logo_url: p.logo_url,
            repository_url: p.repo_url,
            readme_url: Some(p.readme_url),
            readme_content: None,
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Registry Template conversions
// ─────────────────────────────────────────────────────────────

impl From<gen::Template> for domain::RegistryTemplate {
    fn from(t: gen::Template) -> Self {
        Self {
            name: t.name,
            publisher: Some(t.publisher),
            source: Some(t.source),
            version: None,
            display_name: Some(t.display_name),
            description: t.description,
            language: Some(t.language.to_string()),
            runtime: t.runtime.map(|r| domain::TemplateRuntime {
                name: r.name.unwrap_or_default(),
                options: None,
            }),
            project_name: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;

    // ─────────────────────────────────────────────────────────────
    // Helper: build generated types using the progenitor builders
    // ─────────────────────────────────────────────────────────────

    fn make_stack_summary(
        org: &str,
        project: &str,
        stack: &str,
        last_update: Option<i64>,
        resource_count: Option<i64>,
    ) -> gen::AppStackSummary {
        gen::AppStackSummary::builder()
            .id("stack-id-1")
            .org_name(org)
            .project_name(project)
            .stack_name(stack)
            .last_update(last_update)
            .resource_count(resource_count)
            .try_into()
            .expect("valid AppStackSummary")
    }

    fn make_org_environment(
        org: Option<&str>,
        project: Option<&str>,
        name: Option<&str>,
        created: &str,
        modified: &str,
    ) -> gen::OrgEnvironment {
        let referrer_metadata: gen::EnvironmentReferrerMetadata =
            gen::EnvironmentReferrerMetadata::builder()
                .environment_referrer_count(0_i64)
                .insights_account_referrer_count(0_i64)
                .stack_referrer_count(0_i64)
                .try_into()
                .expect("valid EnvironmentReferrerMetadata");
        let settings: gen::EnvironmentSettings = gen::EnvironmentSettings::builder()
            .deletion_protected(false)
            .try_into()
            .expect("valid EnvironmentSettings");
        gen::OrgEnvironment::builder()
            .id("env-id-1")
            .organization(org.map(String::from))
            .project(project.map(String::from))
            .name(name.map(String::from))
            .created(created)
            .modified(modified)
            .referrer_metadata(referrer_metadata)
            .settings(settings)
            .tags(HashMap::new())
            .try_into()
            .expect("valid OrgEnvironment")
    }

    fn make_user_info(name: &str) -> gen::UserInfo {
        let user: gen::UserInfo = gen::UserInfo::builder()
            .name(name)
            .github_login("gh-login")
            .avatar_url("https://example.com/avatar.png")
            .try_into()
            .expect("valid UserInfo");
        user
    }

    fn make_agent_task(
        id: &str,
        name: &str,
        status: gen::AgentTaskStatus,
        is_shared: bool,
        entities: Vec<gen::AgentEntity>,
    ) -> gen::AgentTask {
        gen::AgentTask::builder()
            .id(id)
            .name(name)
            .status(status)
            .is_shared(is_shared)
            .created_at(Utc::now())
            .created_by(make_user_info("test-user"))
            .entities(entities)
            .try_into()
            .expect("valid AgentTask")
    }

    fn make_agent_entity(type_: &str) -> gen::AgentEntity {
        gen::AgentEntity::builder()
            .type_(type_)
            .try_into()
            .expect("valid AgentEntity")
    }

    fn make_slash_command(
        name: &str,
        prompt: &str,
        description: &str,
        built_in: bool,
        tag: &str,
    ) -> gen::AgentSlashCommand {
        gen::AgentSlashCommand::builder()
            .name(name)
            .prompt(prompt)
            .description(description)
            .built_in(built_in)
            .modified_at(Utc::now())
            .tag(tag)
            .try_into()
            .expect("valid AgentSlashCommand")
    }

    fn make_resource_count_summary(
        year: i64,
        month: Option<i64>,
        day: Option<i64>,
        resources: i64,
        resource_hours: i64,
    ) -> gen::ResourceCountSummary {
        gen::ResourceCountSummary::builder()
            .year(year)
            .month(month)
            .day(day)
            .resources(resources)
            .resource_hours(resource_hours)
            .try_into()
            .expect("valid ResourceCountSummary")
    }

    fn make_service_member(name: &str) -> gen::ServiceMember {
        let member: gen::ServiceMember = gen::ServiceMember::builder()
            .name(name)
            .type_("member")
            .avatar_url("")
            .try_into()
            .expect("valid ServiceMember");
        member
    }

    fn make_service(
        org_name: &str,
        name: &str,
        description: &str,
        owner_name: &str,
        item_counts: HashMap<String, i64>,
    ) -> gen::Service {
        gen::Service::builder()
            .organization_name(org_name)
            .name(name)
            .description(description)
            .owner(make_service_member(owner_name))
            .item_count_summary(item_counts)
            .members(vec![])
            .properties(vec![])
            .try_into()
            .expect("valid Service")
    }

    fn make_package_metadata(
        name: &str,
        publisher: &str,
        source: &str,
        version: &str,
    ) -> gen::PackageMetadata {
        gen::PackageMetadata::builder()
            .name(name)
            .publisher(publisher)
            .source(source)
            .version(version)
            .created_at(Utc::now())
            .is_featured(false)
            .package_status(gen::PackageMetadataPackageStatus::Ga)
            .readme_url("https://example.com/readme")
            .schema_url("https://example.com/schema")
            .visibility(gen::PackageMetadataVisibility::Public)
            .try_into()
            .expect("valid PackageMetadata")
    }

    fn make_template(
        name: &str,
        publisher: &str,
        source: &str,
        display_name: &str,
        language: gen::TemplateLanguage,
    ) -> gen::Template {
        gen::Template::builder()
            .name(name)
            .publisher(publisher)
            .source(source)
            .display_name(display_name)
            .language(language)
            .download_url("https://example.com/download")
            .url("https://example.com/template")
            .visibility(gen::TemplateVisibility::Public)
            .updated_at(Utc::now())
            .try_into()
            .expect("valid Template")
    }

    // ═════════════════════════════════════════════════════════════
    // Stack conversion tests
    // ═════════════════════════════════════════════════════════════

    #[test]
    fn stack_conversion_maps_all_fields() {
        let gen_stack =
            make_stack_summary("my-org", "my-project", "dev", Some(1700000000), Some(42));
        let stack: domain::Stack = gen_stack.into();

        assert_eq!(stack.org_name, "my-org");
        assert_eq!(stack.project_name, "my-project");
        assert_eq!(stack.stack_name, "dev");
        assert_eq!(stack.last_update, Some(1700000000));
        assert_eq!(stack.resource_count, Some(42)); // i64 → i32 cast
        assert!(stack.url.is_none(), "url should always be None");
    }

    #[test]
    fn stack_conversion_with_none_optional_fields() {
        let gen_stack = make_stack_summary("org", "proj", "stack", None, None);
        let stack: domain::Stack = gen_stack.into();

        assert_eq!(stack.last_update, None);
        assert_eq!(stack.resource_count, None);
    }

    #[test]
    fn stack_conversion_resource_count_i64_to_i32() {
        // Verify large i64 values are cast to i32
        let gen_stack = make_stack_summary("o", "p", "s", None, Some(100_000));
        let stack: domain::Stack = gen_stack.into();
        assert_eq!(stack.resource_count, Some(100_000_i32));
    }

    #[test]
    fn stack_conversion_with_empty_strings() {
        let gen_stack = make_stack_summary("", "", "", None, None);
        let stack: domain::Stack = gen_stack.into();

        assert_eq!(stack.org_name, "");
        assert_eq!(stack.project_name, "");
        assert_eq!(stack.stack_name, "");
    }

    // ═════════════════════════════════════════════════════════════
    // ESC Environment conversion tests
    // ═════════════════════════════════════════════════════════════

    #[test]
    fn esc_environment_conversion_maps_all_fields() {
        let gen_env = make_org_environment(
            Some("my-org"),
            Some("my-project"),
            Some("production"),
            "2024-01-15T10:00:00Z",
            "2024-06-20T14:30:00Z",
        );
        let env: domain::EscEnvironmentSummary = gen_env.into();

        assert_eq!(env.organization, "my-org");
        assert_eq!(env.project, "my-project");
        assert_eq!(env.name, "production");
        assert_eq!(env.created, Some("2024-01-15T10:00:00Z".to_string()));
        assert_eq!(env.modified, Some("2024-06-20T14:30:00Z".to_string()));
    }

    #[test]
    fn esc_environment_conversion_none_fields_default_to_empty() {
        let gen_env = make_org_environment(
            None,
            None,
            None,
            "2024-01-01T00:00:00Z",
            "2024-01-01T00:00:00Z",
        );
        let env: domain::EscEnvironmentSummary = gen_env.into();

        assert_eq!(
            env.organization, "",
            "None organization should become empty string"
        );
        assert_eq!(env.project, "", "None project should become empty string");
        assert_eq!(env.name, "", "None name should become empty string");
    }

    // ═════════════════════════════════════════════════════════════
    // Neo Task conversion tests
    // ═════════════════════════════════════════════════════════════

    #[test]
    fn neo_task_conversion_running_status() {
        let gen_task = make_agent_task(
            "task-123",
            "Deploy infrastructure",
            gen::AgentTaskStatus::Running,
            false,
            vec![],
        );
        let task: domain::NeoTask = gen_task.into();

        assert_eq!(task.id, "task-123");
        assert_eq!(task.name, Some("Deploy infrastructure".to_string()));
        assert_eq!(task.status, Some("running".to_string()));
        assert_eq!(task.is_shared, Some(false));
        assert!(task.created_at.is_some());
        assert!(task.updated_at.is_none());
        assert!(task.url.is_none());
        assert!(task.linked_prs.is_empty());
        assert!(task.policies.is_empty());
    }

    #[test]
    fn neo_task_conversion_idle_status() {
        let gen_task = make_agent_task(
            "task-456",
            "idle task",
            gen::AgentTaskStatus::Idle,
            true,
            vec![],
        );
        let task: domain::NeoTask = gen_task.into();

        assert_eq!(task.status, Some("idle".to_string()));
        assert_eq!(task.is_shared, Some(true));
    }

    #[test]
    fn neo_task_conversion_started_by_user() {
        let gen_task = make_agent_task("t1", "task", gen::AgentTaskStatus::Running, false, vec![]);
        let task: domain::NeoTask = gen_task.into();

        let user = task.started_by.expect("should have started_by");
        assert_eq!(user.name, Some("test-user".to_string()));
        assert_eq!(user.login, Some("gh-login".to_string()));
        assert_eq!(
            user.avatar_url,
            Some("https://example.com/avatar.png".to_string())
        );
    }

    #[test]
    fn neo_task_conversion_with_entities() {
        let entities = vec![make_agent_entity("stack"), make_agent_entity("repository")];
        let gen_task = make_agent_task(
            "t2",
            "task with entities",
            gen::AgentTaskStatus::Running,
            false,
            entities,
        );
        let task: domain::NeoTask = gen_task.into();

        assert_eq!(task.entities.len(), 2);
    }

    #[test]
    fn neo_task_conversion_created_at_is_rfc3339() {
        let gen_task = make_agent_task(
            "t-rfc",
            "rfc test",
            gen::AgentTaskStatus::Idle,
            false,
            vec![],
        );
        let task: domain::NeoTask = gen_task.into();

        let created = task.created_at.expect("should have created_at");
        // RFC 3339 timestamps contain 'T' separator and timezone
        assert!(
            created.contains('T'),
            "should be RFC 3339 format: {created}"
        );
    }

    #[test]
    fn neo_task_conversion_shared_at_some() {
        let now = Utc::now();
        let gen_task: gen::AgentTask = gen::AgentTask::builder()
            .id("t-shared")
            .name("shared task")
            .status(gen::AgentTaskStatus::Running)
            .is_shared(true)
            .created_at(now)
            .created_by(make_user_info("user"))
            .entities(vec![])
            .shared_at(Some(now))
            .try_into()
            .expect("valid AgentTask");
        let task: domain::NeoTask = gen_task.into();

        assert!(task.shared_at.is_some(), "shared_at should be set");
        let shared = task.shared_at.unwrap();
        assert!(
            shared.contains('T'),
            "shared_at should be RFC 3339: {shared}"
        );
    }

    #[test]
    fn neo_task_conversion_shared_at_none() {
        let gen_task = make_agent_task(
            "t-noshare",
            "not shared",
            gen::AgentTaskStatus::Idle,
            false,
            vec![],
        );
        let task: domain::NeoTask = gen_task.into();
        assert!(task.shared_at.is_none());
    }

    // ═════════════════════════════════════════════════════════════
    // Agent Entity conversion tests
    // ═════════════════════════════════════════════════════════════

    #[test]
    fn agent_entity_conversion_maps_type() {
        let entity = make_agent_entity("stack");
        let neo_entity: domain::NeoEntity = entity.into();

        assert_eq!(neo_entity.entity_type, Some("stack".to_string()));
        // All other fields should be None since AgentEntity only has type_
        assert!(neo_entity.project.is_none());
        assert!(neo_entity.stack.is_none());
        assert!(neo_entity.url.is_none());
        assert!(neo_entity.org.is_none());
        assert!(neo_entity.forge.is_none());
        assert!(neo_entity.id.is_none());
    }

    #[test]
    fn agent_entity_conversion_various_types() {
        for type_name in &["stack", "repository", "pull_request", "policy_issue"] {
            let entity = make_agent_entity(type_name);
            let neo_entity: domain::NeoEntity = entity.into();
            assert_eq!(neo_entity.entity_type, Some(type_name.to_string()));
        }
    }

    // ═════════════════════════════════════════════════════════════
    // Slash Command conversion tests
    // ═════════════════════════════════════════════════════════════

    #[test]
    fn slash_command_conversion_maps_all_fields() {
        let cmd = make_slash_command(
            "deploy",
            "Deploy the infrastructure to production",
            "Deploys infra",
            false,
            "abc123",
        );
        let neo_cmd: domain::NeoSlashCommand = cmd.into();

        assert_eq!(neo_cmd.name, "deploy");
        assert_eq!(neo_cmd.prompt, "Deploy the infrastructure to production");
        assert_eq!(neo_cmd.description, "Deploys infra");
        assert!(!neo_cmd.built_in);
        assert_eq!(neo_cmd.tag, Some("abc123".to_string()));
        assert!(neo_cmd.modified_at.is_some());
    }

    #[test]
    fn slash_command_conversion_built_in_true() {
        let cmd = make_slash_command("get-started", "prompt", "desc", true, "tag1");
        let neo_cmd: domain::NeoSlashCommand = cmd.into();

        assert!(neo_cmd.built_in);
    }

    #[test]
    fn slash_command_conversion_modified_at_is_rfc3339() {
        let cmd = make_slash_command("test", "p", "d", false, "t");
        let neo_cmd: domain::NeoSlashCommand = cmd.into();

        let modified = neo_cmd.modified_at.expect("should have modified_at");
        assert!(modified.contains('T'), "should be RFC 3339: {modified}");
    }

    // ═════════════════════════════════════════════════════════════
    // Resource conversion tests
    // ═════════════════════════════════════════════════════════════

    #[test]
    fn resource_conversion_maps_all_fields() {
        let gen_resource: gen::ResourceResult = gen::ResourceResult::builder()
            .type_(Some("aws:s3:Bucket".to_string()))
            .name(Some("my-bucket".to_string()))
            .id(Some("bucket-123".to_string()))
            .stack(Some("dev".to_string()))
            .project(Some("my-project".to_string()))
            .package("aws")
            .module("s3")
            .modified(Some("2024-01-15".to_string()))
            .try_into()
            .expect("valid ResourceResult");

        let resource: domain::Resource = gen_resource.into();

        assert_eq!(resource.resource_type, "aws:s3:Bucket");
        assert_eq!(resource.name, "my-bucket");
        assert_eq!(resource.id, Some("bucket-123".to_string()));
        assert_eq!(resource.stack, Some("dev".to_string()));
        assert_eq!(resource.project, Some("my-project".to_string()));
        assert_eq!(resource.package, Some("aws".to_string()));
        assert_eq!(resource.modified, Some("2024-01-15".to_string()));
    }

    #[test]
    fn resource_conversion_none_type_defaults_to_empty() {
        let gen_resource: gen::ResourceResult = gen::ResourceResult::builder()
            .package("pkg")
            .module("mod")
            .try_into()
            .expect("valid ResourceResult");

        let resource: domain::Resource = gen_resource.into();

        assert_eq!(
            resource.resource_type, "",
            "None type_ should default to empty string"
        );
        assert_eq!(
            resource.name, "",
            "None name should default to empty string"
        );
    }

    #[test]
    fn resource_conversion_package_always_wrapped_in_some() {
        let gen_resource: gen::ResourceResult = gen::ResourceResult::builder()
            .package("pulumi")
            .module("mod")
            .try_into()
            .expect("valid ResourceResult");

        let resource: domain::Resource = gen_resource.into();

        assert_eq!(resource.package, Some("pulumi".to_string()));
    }

    // ═════════════════════════════════════════════════════════════
    // Resource Count Summary conversion tests
    // ═════════════════════════════════════════════════════════════

    #[test]
    fn resource_count_summary_maps_all_fields() {
        let gen_summary = make_resource_count_summary(2024, Some(6), Some(15), 500, 12000);
        let point: domain::ResourceSummaryPoint = gen_summary.into();

        assert_eq!(point.year, 2024);
        assert_eq!(point.month, 6);
        assert_eq!(point.day, 15);
        assert_eq!(point.resources, 500);
        assert_eq!(point.resource_hours, Some(12000));
    }

    #[test]
    fn resource_count_summary_none_month_and_day_default_to_zero() {
        let gen_summary = make_resource_count_summary(2024, None, None, 100, 5000);
        let point: domain::ResourceSummaryPoint = gen_summary.into();

        assert_eq!(point.month, 0, "None month should default to 0");
        assert_eq!(point.day, 0, "None day should default to 0");
    }

    #[test]
    fn resource_count_summary_i64_to_i32_cast() {
        let gen_summary = make_resource_count_summary(2024, Some(12), Some(31), 999, 99999);
        let point: domain::ResourceSummaryPoint = gen_summary.into();

        assert_eq!(point.year, 2024_i32);
        assert_eq!(point.month, 12_i32);
        assert_eq!(point.day, 31_i32);
    }

    // ═════════════════════════════════════════════════════════════
    // Service conversion tests
    // ═════════════════════════════════════════════════════════════

    #[test]
    fn service_conversion_maps_all_fields() {
        let mut counts = HashMap::new();
        counts.insert("stacks".to_string(), 5_i64);
        counts.insert("environments".to_string(), 3_i64);

        let gen_service = make_service(
            "my-org",
            "my-service",
            "A description",
            "admin-user",
            counts,
        );
        let service: domain::Service = gen_service.into();

        assert_eq!(service.organization_name, "my-org");
        assert_eq!(service.name, "my-service");
        assert_eq!(service.description, Some("A description".to_string()));

        let owner = service.owner.expect("should have owner");
        assert_eq!(owner.name, "admin-user");
        assert_eq!(owner.owner_type, "member");

        let summary = service
            .item_count_summary
            .expect("should have item_count_summary");
        assert_eq!(summary.stacks, Some(5));
        assert_eq!(summary.environments, Some(3));
    }

    #[test]
    fn service_conversion_empty_owner_name_becomes_none() {
        let gen_service = make_service("org", "svc", "desc", "", HashMap::new());
        let service: domain::Service = gen_service.into();

        assert!(
            service.owner.is_none(),
            "empty owner name should result in None owner"
        );
    }

    #[test]
    fn service_conversion_empty_item_counts_becomes_none() {
        let gen_service = make_service("org", "svc", "desc", "user", HashMap::new());
        let service: domain::Service = gen_service.into();

        assert!(
            service.item_count_summary.is_none(),
            "empty item_count_summary HashMap should result in None"
        );
    }

    #[test]
    fn service_conversion_partial_item_counts() {
        let mut counts = HashMap::new();
        counts.insert("stacks".to_string(), 10_i64);
        // no "environments" key

        let gen_service = make_service("org", "svc", "desc", "user", counts);
        let service: domain::Service = gen_service.into();

        let summary = service
            .item_count_summary
            .expect("should have summary with stacks only");
        assert_eq!(summary.stacks, Some(10));
        assert!(summary.environments.is_none());
    }

    #[test]
    fn service_conversion_i64_to_i32_cast_for_counts() {
        let mut counts = HashMap::new();
        counts.insert("stacks".to_string(), 1000_i64);
        counts.insert("environments".to_string(), 500_i64);

        let gen_service = make_service("org", "svc", "desc", "user", counts);
        let service: domain::Service = gen_service.into();

        let summary = service.item_count_summary.unwrap();
        assert_eq!(summary.stacks, Some(1000_i32));
        assert_eq!(summary.environments, Some(500_i32));
    }

    #[test]
    fn service_conversion_created_at_is_rfc3339_when_present() {
        let now = Utc::now();
        let gen_service: gen::Service = gen::Service::builder()
            .organization_name("org")
            .name("svc")
            .description("desc")
            .owner(make_service_member("user"))
            .item_count_summary(HashMap::new())
            .members(vec![])
            .properties(vec![])
            .created(Some(now))
            .try_into()
            .expect("valid Service");

        let service: domain::Service = gen_service.into();
        let created = service.created_at.expect("should have created_at");
        assert!(created.contains('T'), "should be RFC 3339: {created}");
    }

    #[test]
    fn service_conversion_modified_at_always_none() {
        let gen_service = make_service("org", "svc", "desc", "user", HashMap::new());
        let service: domain::Service = gen_service.into();
        assert!(service.modified_at.is_none());
    }

    // ═════════════════════════════════════════════════════════════
    // Registry Package conversion tests
    // ═════════════════════════════════════════════════════════════

    #[test]
    fn package_conversion_maps_all_fields() {
        let gen_pkg = make_package_metadata("aws", "pulumi", "registry", "6.0.0");
        let pkg: domain::RegistryPackage = gen_pkg.into();

        assert_eq!(pkg.name, "aws");
        assert_eq!(pkg.publisher, Some("pulumi".to_string()));
        assert_eq!(pkg.source, Some("registry".to_string()));
        assert_eq!(pkg.version, Some("6.0.0".to_string()));
        assert_eq!(
            pkg.readme_url,
            Some("https://example.com/readme".to_string())
        );
        assert!(
            pkg.readme_content.is_none(),
            "readme_content should always be None"
        );
    }

    #[test]
    fn package_conversion_optional_fields_none() {
        let gen_pkg = make_package_metadata("test-pkg", "pub", "src", "1.0.0");
        let pkg: domain::RegistryPackage = gen_pkg.into();

        // These optional fields are not set on minimal PackageMetadata
        assert!(pkg.title.is_none());
        assert!(pkg.description.is_none());
        assert!(pkg.logo_url.is_none());
        assert!(pkg.repository_url.is_none());
    }

    #[test]
    fn package_conversion_with_all_optional_fields() {
        let gen_pkg: gen::PackageMetadata = gen::PackageMetadata::builder()
            .name("gcp")
            .publisher("pulumi")
            .source("registry")
            .version("7.0.0")
            .created_at(Utc::now())
            .is_featured(true)
            .package_status(gen::PackageMetadataPackageStatus::Ga)
            .readme_url("https://example.com/readme")
            .schema_url("https://example.com/schema")
            .visibility(gen::PackageMetadataVisibility::Public)
            .title(Some("Google Cloud Platform".to_string()))
            .description(Some("GCP resources".to_string()))
            .logo_url(Some("https://example.com/gcp-logo.png".to_string()))
            .repo_url(Some("https://github.com/pulumi/pulumi-gcp".to_string()))
            .try_into()
            .expect("valid PackageMetadata");

        let pkg: domain::RegistryPackage = gen_pkg.into();

        assert_eq!(pkg.title, Some("Google Cloud Platform".to_string()));
        assert_eq!(pkg.description, Some("GCP resources".to_string()));
        assert_eq!(
            pkg.logo_url,
            Some("https://example.com/gcp-logo.png".to_string())
        );
        assert_eq!(
            pkg.repository_url,
            Some("https://github.com/pulumi/pulumi-gcp".to_string())
        );
    }

    // ═════════════════════════════════════════════════════════════
    // Registry Template conversion tests
    // ═════════════════════════════════════════════════════════════

    #[test]
    fn template_conversion_maps_all_fields() {
        let gen_tmpl = make_template(
            "aws-typescript",
            "pulumi",
            "registry",
            "AWS TypeScript Starter",
            gen::TemplateLanguage::Typescript,
        );
        let tmpl: domain::RegistryTemplate = gen_tmpl.into();

        assert_eq!(tmpl.name, "aws-typescript");
        assert_eq!(tmpl.publisher, Some("pulumi".to_string()));
        assert_eq!(tmpl.source, Some("registry".to_string()));
        assert!(tmpl.version.is_none(), "version should always be None");
        assert_eq!(
            tmpl.display_name,
            Some("AWS TypeScript Starter".to_string())
        );
        assert_eq!(tmpl.language, Some("typescript".to_string()));
        assert!(
            tmpl.project_name.is_none(),
            "project_name should always be None"
        );
    }

    #[test]
    fn template_conversion_all_languages() {
        let languages = vec![
            (gen::TemplateLanguage::Python, "python"),
            (gen::TemplateLanguage::Go, "go"),
            (gen::TemplateLanguage::Dotnet, "dotnet"),
            (gen::TemplateLanguage::Java, "java"),
            (gen::TemplateLanguage::Javascript, "javascript"),
            (gen::TemplateLanguage::Typescript, "typescript"),
            (gen::TemplateLanguage::Yaml, "yaml"),
            (gen::TemplateLanguage::Unknown, "unknown"),
        ];

        for (lang_enum, expected_str) in languages {
            let gen_tmpl = make_template("t", "p", "s", "d", lang_enum);
            let tmpl: domain::RegistryTemplate = gen_tmpl.into();
            assert_eq!(
                tmpl.language,
                Some(expected_str.to_string()),
                "language {:?} should map to '{}'",
                tmpl.language,
                expected_str
            );
        }
    }

    #[test]
    fn template_conversion_with_runtime() {
        let gen_tmpl: gen::Template = gen::Template::builder()
            .name("tmpl")
            .publisher("pub")
            .source("src")
            .display_name("Template")
            .language(gen::TemplateLanguage::Python)
            .download_url("https://example.com/dl")
            .url("https://example.com/tmpl")
            .visibility(gen::TemplateVisibility::Public)
            .updated_at(Utc::now())
            .runtime(Some(gen::TemplateRuntimeInfo {
                name: Some("python".to_string()),
                options: HashMap::new(),
            }))
            .try_into()
            .expect("valid Template");

        let tmpl: domain::RegistryTemplate = gen_tmpl.into();

        let runtime = tmpl.runtime.expect("should have runtime");
        // TemplateRuntimeInfo.name is Option<String> in generated type,
        // domain::TemplateRuntime.name is String — conversion uses unwrap_or_default
        assert_eq!(runtime.name, "python");
        assert!(
            runtime.options.is_none(),
            "options should be None (not mapped)"
        );
    }

    #[test]
    fn template_conversion_without_runtime() {
        let gen_tmpl = make_template("t", "p", "s", "d", gen::TemplateLanguage::Go);
        let tmpl: domain::RegistryTemplate = gen_tmpl.into();

        assert!(tmpl.runtime.is_none());
    }

    #[test]
    fn template_conversion_optional_description() {
        let gen_tmpl: gen::Template = gen::Template::builder()
            .name("tmpl")
            .publisher("pub")
            .source("src")
            .display_name("Display")
            .language(gen::TemplateLanguage::Typescript)
            .download_url("https://example.com/dl")
            .url("https://example.com/tmpl")
            .visibility(gen::TemplateVisibility::Public)
            .updated_at(Utc::now())
            .description(Some("A template description".to_string()))
            .try_into()
            .expect("valid Template");

        let tmpl: domain::RegistryTemplate = gen_tmpl.into();
        assert_eq!(tmpl.description, Some("A template description".to_string()));
    }
}
