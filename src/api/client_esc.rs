//! ESC (Environments, Secrets, Configuration) API methods for `PulumiClient`.

use super::client::{map_gen_err, ApiError, PulumiClient};
use super::domain::{EscEnvironmentDetails, EscEnvironmentSummary, EscOpenResponse};

impl PulumiClient {
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
}
