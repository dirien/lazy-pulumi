//! Platform, Registry, Organizations, Users, and Resource API methods for `PulumiClient`.

use super::client::{map_gen_err, strip_frontmatter, ApiError, PulumiClient};
use super::domain::{
    RegistryPackage, RegistryTemplate, Resource, ResourceSummaryPoint, Service, TemplateRuntime,
    User,
};

impl PulumiClient {
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

    /// List registry packages (components), optionally filtered by visibility.
    ///
    /// Uses raw reqwest (Route B) because the API returns fields that
    /// diverge from the OpenAPI spec (e.g. `PackageParameterization.parameter`
    /// is a plain base64 string, not an array of byte strings).
    pub async fn list_registry_packages(
        &self,
        org: Option<&str>,
        visibility: Option<&str>,
    ) -> Result<Vec<RegistryPackage>, ApiError> {
        let org = self.org_or_default(org)?;

        let mut url = format!(
            "{}/api/preview/registry/packages?orgLogin={}&limit=50",
            self.config.base_url, org
        );
        if let Some(vis) = visibility {
            url.push_str(&format!("&visibility={}", vis));
        }

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(ApiError::ApiResponse { status, message });
        }

        #[derive(serde::Deserialize)]
        struct PackagesResponse {
            #[serde(default)]
            packages: Vec<PackageItem>,
        }

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct PackageItem {
            #[serde(default)]
            name: String,
            #[serde(default)]
            publisher: Option<String>,
            #[serde(default)]
            source: Option<String>,
            #[serde(default)]
            version: Option<String>,
            #[serde(default)]
            title: Option<String>,
            #[serde(default)]
            description: Option<String>,
            #[serde(default)]
            logo_url: Option<String>,
            #[serde(default)]
            repo_url: Option<String>,
            #[serde(default, rename = "readmeURL")]
            readme_url: Option<String>,
        }

        let data: PackagesResponse = response.json().await?;

        Ok(data
            .packages
            .into_iter()
            .map(|p| RegistryPackage {
                name: p.name,
                publisher: p.publisher,
                source: p.source,
                version: p.version,
                title: p.title,
                description: p.description,
                logo_url: p.logo_url,
                repository_url: p.repo_url,
                readme_url: p.readme_url,
                readme_content: None,
            })
            .collect())
    }

    /// List registry templates.
    ///
    /// Uses raw reqwest (Route B) because the API returns the `runtime`
    /// field as either a plain string (e.g. `"npm"`) or a full object
    /// (`{"name": "nodejs", "options": {...}}`), which the generated
    /// `TemplateRuntimeInfo` struct cannot handle.
    pub async fn list_registry_templates(
        &self,
        org: Option<&str>,
    ) -> Result<Vec<RegistryTemplate>, ApiError> {
        let org = self.org_or_default(org)?;

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

        #[derive(serde::Deserialize)]
        struct TemplatesResponse {
            #[serde(default)]
            templates: Vec<TemplateItem>,
        }

        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct TemplateItem {
            #[serde(default)]
            name: String,
            #[serde(default)]
            publisher: Option<String>,
            #[serde(default)]
            source: Option<String>,
            #[serde(default)]
            display_name: Option<String>,
            #[serde(default)]
            description: Option<String>,
            #[serde(default)]
            language: Option<String>,
            /// Can be a string like `"npm"` or an object like
            /// `{"name": "nodejs", "options": {...}}`.
            #[serde(default)]
            runtime: Option<serde_json::Value>,
        }

        let data: TemplatesResponse = response.json().await?;

        Ok(data
            .templates
            .into_iter()
            .map(|t| {
                let runtime = t.runtime.and_then(|v| match &v {
                    serde_json::Value::String(s) => Some(TemplateRuntime {
                        name: s.clone(),
                        options: None,
                    }),
                    serde_json::Value::Object(map) => {
                        let name = map
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or_default()
                            .to_string();
                        Some(TemplateRuntime {
                            name,
                            options: None,
                        })
                    }
                    _ => None,
                });

                let display_name = match t.display_name {
                    Some(ref dn) if dn.is_empty() => None,
                    other => other,
                };

                RegistryTemplate {
                    name: t.name,
                    publisher: t.publisher,
                    source: t.source,
                    version: None,
                    display_name,
                    description: t.description,
                    language: t.language,
                    runtime,
                    project_name: None,
                }
            })
            .collect())
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

        let text = response.text().await.map_err(ApiError::Http)?;

        // Strip YAML frontmatter (---...---) that many provider READMEs include
        Ok(strip_frontmatter(&text))
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
