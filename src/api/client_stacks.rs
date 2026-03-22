//! Stacks API methods for `PulumiClient`.

use super::client::{map_gen_err, ApiError, PulumiClient};
use super::domain::{OrgStackUpdate, ResourceChanges, Stack, StackUpdate};

impl PulumiClient {
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
                        Some(ResourceChanges {
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
    ) -> Result<Vec<OrgStackUpdate>, ApiError> {
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
            resource_changes: Option<ResourceChanges>,
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

        let updates: Vec<OrgStackUpdate> = items
            .into_iter()
            .filter_map(|item| {
                let last_update = item.last_update?;
                let info = last_update.info?;

                Some(OrgStackUpdate {
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
}
