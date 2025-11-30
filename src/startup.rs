//! Startup checks module
//!
//! Performs validation checks before the application starts:
//! - Pulumi access token is set
//! - Pulumi CLI is accessible

use std::process::Stdio;
use tokio::process::Command;

/// Status of a startup check
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckStatus {
    /// Check is pending (not yet run)
    Pending,
    /// Check is currently running
    Running,
    /// Check passed successfully
    Passed(String),
    /// Check failed with an error message
    Failed(String),
}

impl CheckStatus {
    pub fn is_pending(&self) -> bool {
        matches!(self, CheckStatus::Pending)
    }

    pub fn is_running(&self) -> bool {
        matches!(self, CheckStatus::Running)
    }

    pub fn is_passed(&self) -> bool {
        matches!(self, CheckStatus::Passed(_))
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, CheckStatus::Failed(_))
    }
}

/// Startup check item
#[derive(Debug, Clone)]
pub struct StartupCheck {
    pub name: &'static str,
    pub status: CheckStatus,
}

impl StartupCheck {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            status: CheckStatus::Pending,
        }
    }
}

/// All startup checks
#[derive(Debug, Clone)]
pub struct StartupChecks {
    pub token_check: StartupCheck,
    pub cli_check: StartupCheck,
}

impl Default for StartupChecks {
    fn default() -> Self {
        Self {
            token_check: StartupCheck::new("PULUMI_ACCESS_TOKEN"),
            cli_check: StartupCheck::new("Pulumi CLI"),
        }
    }
}

impl StartupChecks {
    /// Check if all checks have completed (passed or failed)
    pub fn all_complete(&self) -> bool {
        !self.token_check.status.is_pending()
            && !self.token_check.status.is_running()
            && !self.cli_check.status.is_pending()
            && !self.cli_check.status.is_running()
    }

    /// Check if all checks passed
    pub fn all_passed(&self) -> bool {
        self.token_check.status.is_passed() && self.cli_check.status.is_passed()
    }

    /// Check if any check failed
    pub fn any_failed(&self) -> bool {
        self.token_check.status.is_failed() || self.cli_check.status.is_failed()
    }

    /// Check if any check is still running
    pub fn any_running(&self) -> bool {
        self.token_check.status.is_running() || self.cli_check.status.is_running()
    }
}

/// Check if PULUMI_ACCESS_TOKEN environment variable is set
pub fn check_pulumi_token() -> CheckStatus {
    match std::env::var("PULUMI_ACCESS_TOKEN") {
        Ok(token) if !token.is_empty() => {
            // Mask the token for display (show first 4 and last 4 chars)
            let masked = if token.len() > 12 {
                format!("{}...{}", &token[..7], &token[token.len()-4..])
            } else {
                "****".to_string()
            };
            CheckStatus::Passed(format!("Token found: {}", masked))
        }
        Ok(_) => CheckStatus::Failed("PULUMI_ACCESS_TOKEN is empty".to_string()),
        Err(_) => CheckStatus::Failed("PULUMI_ACCESS_TOKEN not set".to_string()),
    }
}

/// Check if Pulumi CLI is available and get version
pub async fn check_pulumi_cli() -> CheckStatus {
    let result = Command::new("pulumi")
        .args(["version"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    match result {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            CheckStatus::Passed(format!("Version: {}", version))
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            CheckStatus::Failed(format!("CLI error: {}", stderr))
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                CheckStatus::Failed("Pulumi CLI not found in PATH".to_string())
            } else {
                CheckStatus::Failed(format!("Failed to run CLI: {}", e))
            }
        }
    }
}
