//! Pulumi CLI command definitions
//!
//! Defines the available Pulumi commands with their parameters and categories.

use std::fmt;

/// Category of Pulumi commands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCategory {
    /// Core stack operations (up, preview, destroy, refresh)
    StackOperations,
    /// Stack and config management
    StackManagement,
    /// Project and template operations
    ProjectManagement,
    /// Authentication and organization
    AuthOrg,
    /// Utilities and info
    Utilities,
}

impl CommandCategory {
    pub fn all() -> &'static [CommandCategory] {
        &[
            CommandCategory::StackOperations,
            CommandCategory::StackManagement,
            CommandCategory::ProjectManagement,
            CommandCategory::AuthOrg,
            CommandCategory::Utilities,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            CommandCategory::StackOperations => "Stack Operations",
            CommandCategory::StackManagement => "Stack Management",
            CommandCategory::ProjectManagement => "Project Management",
            CommandCategory::AuthOrg => "Auth & Organization",
            CommandCategory::Utilities => "Utilities",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            CommandCategory::StackOperations => ">>",
            CommandCategory::StackManagement => "[]",
            CommandCategory::ProjectManagement => "{}",
            CommandCategory::AuthOrg => "**",
            CommandCategory::Utilities => "##",
        }
    }
}

impl fmt::Display for CommandCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.title())
    }
}

/// A parameter for a Pulumi command
#[derive(Debug, Clone)]
pub struct CommandParam {
    /// Parameter name (e.g., "stack", "message")
    pub name: &'static str,
    /// Short flag (e.g., "-s")
    pub short: Option<&'static str>,
    /// Long flag (e.g., "--stack")
    pub long: Option<&'static str>,
    /// Description shown in the dialog
    pub description: &'static str,
    /// Whether this parameter is required
    pub required: bool,
    /// Default value if any
    pub default: Option<&'static str>,
    /// Parameter type for input handling
    pub param_type: ParamType,
}

/// Type of parameter for input handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ParamType {
    /// Simple text input
    Text,
    /// Boolean flag (yes/no)
    Flag,
    /// Stack selector (uses stack list)
    Stack,
    /// File path selector
    FilePath,
    /// Secret value (hidden input)
    Secret,
    /// Multi-line text
    MultiLine,
}

/// Execution mode for a command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    /// Command shows streaming output (up, destroy, refresh, preview)
    Streaming,
    /// Command runs quickly and shows result
    Quick,
    /// Command opens interactive mode (not supported in TUI)
    Interactive,
}

/// Definition of a Pulumi CLI command
#[derive(Debug, Clone)]
pub struct PulumiCommand {
    /// Command name (e.g., "up", "preview")
    pub name: &'static str,
    /// CLI command (e.g., ["stack", "ls"] for "pulumi stack ls")
    pub cli_args: &'static [&'static str],
    /// Brief description
    pub description: &'static str,
    /// Category
    pub category: CommandCategory,
    /// Command parameters
    pub params: &'static [CommandParam],
    /// Whether this command needs confirmation before running
    pub needs_confirmation: bool,
    /// Execution mode
    pub execution_mode: ExecutionMode,
    /// Keyboard shortcut hint (e.g., "u" for up)
    pub shortcut: Option<char>,
    /// Whether this command supports working directory selection
    #[allow(dead_code)]
    pub supports_cwd: bool,
}

impl PulumiCommand {
    /// Get the full command string for display
    pub fn display_command(&self) -> String {
        if self.cli_args.is_empty() {
            format!("pulumi {}", self.name)
        } else {
            format!("pulumi {}", self.cli_args.join(" "))
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Command Definitions
// ─────────────────────────────────────────────────────────────

/// Stack selection parameter
const PARAM_STACK: CommandParam = CommandParam {
    name: "stack",
    short: Some("-s"),
    long: Some("--stack"),
    description: "Target stack name",
    required: false,
    default: None,
    param_type: ParamType::Stack,
};

/// Yes/skip confirmation parameter
const PARAM_YES: CommandParam = CommandParam {
    name: "yes",
    short: Some("-y"),
    long: Some("--yes"),
    description: "Skip confirmation prompts",
    required: false,
    default: Some("true"),
    param_type: ParamType::Flag,
};

/// Message parameter for updates
const PARAM_MESSAGE: CommandParam = CommandParam {
    name: "message",
    short: Some("-m"),
    long: Some("--message"),
    description: "Update message",
    required: false,
    default: None,
    param_type: ParamType::Text,
};

/// Config key parameter
const PARAM_CONFIG_KEY: CommandParam = CommandParam {
    name: "key",
    short: None,
    long: None,
    description: "Configuration key",
    required: true,
    default: None,
    param_type: ParamType::Text,
};

/// Config value parameter
const PARAM_CONFIG_VALUE: CommandParam = CommandParam {
    name: "value",
    short: None,
    long: None,
    description: "Configuration value",
    required: true,
    default: None,
    param_type: ParamType::Text,
};

/// Secret flag for config
const PARAM_SECRET: CommandParam = CommandParam {
    name: "secret",
    short: None,
    long: Some("--secret"),
    description: "Treat value as secret",
    required: false,
    default: None,
    param_type: ParamType::Flag,
};

/// Stack name for creation (positional argument for stack init/select/rm)
const PARAM_STACK_NAME: CommandParam = CommandParam {
    name: "name",
    short: None,
    long: None,
    description: "Stack name to create",
    required: true,
    default: None,
    param_type: ParamType::Text,
};

/// Initial stack name for new project (uses -s, --stack)
const PARAM_NEW_STACK: CommandParam = CommandParam {
    name: "stack",
    short: Some("-s"),
    long: Some("--stack"),
    description: "Initial stack name",
    required: false,
    default: None,
    param_type: ParamType::Text,
};

/// Template parameter for new project
const PARAM_TEMPLATE: CommandParam = CommandParam {
    name: "template",
    short: None,
    long: None,
    description: "Template name (e.g., aws-typescript)",
    required: false,
    default: None,
    param_type: ParamType::Text,
};

/// Project name parameter
const PARAM_PROJECT_NAME: CommandParam = CommandParam {
    name: "name",
    short: Some("-n"),
    long: Some("--name"),
    description: "Project name",
    required: false,
    default: None,
    param_type: ParamType::Text,
};

/// Diff flag for preview
const PARAM_DIFF: CommandParam = CommandParam {
    name: "diff",
    short: None,
    long: Some("--diff"),
    description: "Show detailed diff",
    required: false,
    default: None,
    param_type: ParamType::Flag,
};

/// Target parameter
const PARAM_TARGET: CommandParam = CommandParam {
    name: "target",
    short: Some("-t"),
    long: Some("--target"),
    description: "Target specific resources (URN)",
    required: false,
    default: None,
    param_type: ParamType::Text,
};

/// JSON output flag
const PARAM_JSON: CommandParam = CommandParam {
    name: "json",
    short: Some("-j"),
    long: Some("--json"),
    description: "Output as JSON",
    required: false,
    default: None,
    param_type: ParamType::Flag,
};

/// Working directory parameter (special - handled separately)
const PARAM_CWD: CommandParam = CommandParam {
    name: "cwd",
    short: Some("-C"),
    long: Some("--cwd"),
    description: "Working directory (leave empty for current)",
    required: false,
    default: None,
    param_type: ParamType::FilePath,
};

/// Description for new project
const PARAM_DESCRIPTION: CommandParam = CommandParam {
    name: "description",
    short: Some("-d"),
    long: Some("--description"),
    description: "Project description",
    required: false,
    default: None,
    param_type: ParamType::Text,
};

/// Generate only flag for new project
const PARAM_GENERATE_ONLY: CommandParam = CommandParam {
    name: "generate-only",
    short: Some("-g"),
    long: Some("--generate-only"),
    description: "Generate project files only (skip install)",
    required: false,
    default: None,
    param_type: ParamType::Flag,
};

// ─────────────────────────────────────────────────────────────
// All Commands
// ─────────────────────────────────────────────────────────────

pub static PULUMI_COMMANDS: &[PulumiCommand] = &[
    // Stack Operations
    PulumiCommand {
        name: "up",
        cli_args: &["up"],
        description: "Deploy infrastructure changes",
        category: CommandCategory::StackOperations,
        params: &[
            PARAM_CWD,
            PARAM_STACK,
            PARAM_YES,
            PARAM_MESSAGE,
            PARAM_TARGET,
            PARAM_DIFF,
        ],
        needs_confirmation: true,
        execution_mode: ExecutionMode::Streaming,
        shortcut: Some('u'),
        supports_cwd: true,
    },
    PulumiCommand {
        name: "preview",
        cli_args: &["preview"],
        description: "Preview changes without deploying",
        category: CommandCategory::StackOperations,
        params: &[PARAM_CWD, PARAM_STACK, PARAM_DIFF, PARAM_JSON],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Streaming,
        shortcut: Some('p'),
        supports_cwd: true,
    },
    PulumiCommand {
        name: "destroy",
        cli_args: &["destroy"],
        description: "Destroy all infrastructure",
        category: CommandCategory::StackOperations,
        params: &[PARAM_CWD, PARAM_STACK, PARAM_YES, PARAM_TARGET],
        needs_confirmation: true,
        execution_mode: ExecutionMode::Streaming,
        shortcut: Some('d'),
        supports_cwd: true,
    },
    PulumiCommand {
        name: "refresh",
        cli_args: &["refresh"],
        description: "Refresh state from cloud provider",
        category: CommandCategory::StackOperations,
        params: &[PARAM_CWD, PARAM_STACK, PARAM_YES],
        needs_confirmation: true,
        execution_mode: ExecutionMode::Streaming,
        shortcut: Some('r'),
        supports_cwd: true,
    },
    PulumiCommand {
        name: "cancel",
        cli_args: &["cancel"],
        description: "Cancel running update",
        category: CommandCategory::StackOperations,
        params: &[PARAM_CWD, PARAM_STACK, PARAM_YES],
        needs_confirmation: true,
        execution_mode: ExecutionMode::Quick,
        shortcut: None,
        supports_cwd: true,
    },
    PulumiCommand {
        name: "watch",
        cli_args: &["watch"],
        description: "Watch for file changes and update",
        category: CommandCategory::StackOperations,
        params: &[PARAM_CWD, PARAM_STACK],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Streaming,
        shortcut: Some('w'),
        supports_cwd: true,
    },
    // Stack Management
    PulumiCommand {
        name: "stack ls",
        cli_args: &["stack", "ls"],
        description: "List all stacks",
        category: CommandCategory::StackManagement,
        params: &[PARAM_CWD, PARAM_JSON],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Quick,
        shortcut: None,
        supports_cwd: true,
    },
    PulumiCommand {
        name: "stack select",
        cli_args: &["stack", "select"],
        description: "Select active stack",
        category: CommandCategory::StackManagement,
        params: &[PARAM_CWD, PARAM_STACK_NAME],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Quick,
        shortcut: None,
        supports_cwd: true,
    },
    PulumiCommand {
        name: "stack init",
        cli_args: &["stack", "init"],
        description: "Create a new stack",
        category: CommandCategory::StackManagement,
        params: &[PARAM_CWD, PARAM_STACK_NAME],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Quick,
        shortcut: None,
        supports_cwd: true,
    },
    PulumiCommand {
        name: "stack rm",
        cli_args: &["stack", "rm"],
        description: "Remove a stack",
        category: CommandCategory::StackManagement,
        params: &[PARAM_CWD, PARAM_STACK_NAME, PARAM_YES],
        needs_confirmation: true,
        execution_mode: ExecutionMode::Quick,
        shortcut: None,
        supports_cwd: true,
    },
    PulumiCommand {
        name: "stack output",
        cli_args: &["stack", "output"],
        description: "Show stack outputs",
        category: CommandCategory::StackManagement,
        params: &[PARAM_CWD, PARAM_STACK, PARAM_JSON],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Quick,
        shortcut: Some('o'),
        supports_cwd: true,
    },
    PulumiCommand {
        name: "stack history",
        cli_args: &["stack", "history"],
        description: "Show stack update history",
        category: CommandCategory::StackManagement,
        params: &[PARAM_CWD, PARAM_STACK, PARAM_JSON],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Quick,
        shortcut: None,
        supports_cwd: true,
    },
    PulumiCommand {
        name: "stack export",
        cli_args: &["stack", "export"],
        description: "Export stack state",
        category: CommandCategory::StackManagement,
        params: &[PARAM_CWD, PARAM_STACK],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Quick,
        shortcut: None,
        supports_cwd: true,
    },
    PulumiCommand {
        name: "config set",
        cli_args: &["config", "set"],
        description: "Set a config value",
        category: CommandCategory::StackManagement,
        params: &[
            PARAM_CWD,
            PARAM_CONFIG_KEY,
            PARAM_CONFIG_VALUE,
            PARAM_SECRET,
            PARAM_STACK,
        ],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Quick,
        shortcut: None,
        supports_cwd: true,
    },
    PulumiCommand {
        name: "config get",
        cli_args: &["config", "get"],
        description: "Get a config value",
        category: CommandCategory::StackManagement,
        params: &[PARAM_CWD, PARAM_CONFIG_KEY, PARAM_STACK],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Quick,
        shortcut: None,
        supports_cwd: true,
    },
    PulumiCommand {
        name: "config",
        cli_args: &["config"],
        description: "Show all config values",
        category: CommandCategory::StackManagement,
        params: &[PARAM_CWD, PARAM_STACK, PARAM_JSON],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Quick,
        shortcut: Some('c'),
        supports_cwd: true,
    },
    PulumiCommand {
        name: "config rm",
        cli_args: &["config", "rm"],
        description: "Remove a config value",
        category: CommandCategory::StackManagement,
        params: &[PARAM_CWD, PARAM_CONFIG_KEY, PARAM_STACK],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Quick,
        shortcut: None,
        supports_cwd: true,
    },
    // Project Management
    PulumiCommand {
        name: "new",
        cli_args: &["new", "--force"],
        description: "Create a new Pulumi project",
        category: CommandCategory::ProjectManagement,
        params: &[
            PARAM_CWD,
            PARAM_TEMPLATE,
            PARAM_PROJECT_NAME,
            PARAM_NEW_STACK,
            PARAM_DESCRIPTION,
            PARAM_GENERATE_ONLY,
            PARAM_YES,
        ],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Streaming,
        shortcut: Some('n'),
        supports_cwd: true,
    },
    PulumiCommand {
        name: "import",
        cli_args: &["import"],
        description: "Import existing resources",
        category: CommandCategory::ProjectManagement,
        params: &[PARAM_CWD, PARAM_STACK, PARAM_YES],
        needs_confirmation: true,
        execution_mode: ExecutionMode::Streaming,
        shortcut: None,
        supports_cwd: true,
    },
    PulumiCommand {
        name: "install",
        cli_args: &["install"],
        description: "Install plugins and dependencies",
        category: CommandCategory::ProjectManagement,
        params: &[PARAM_CWD],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Streaming,
        shortcut: None,
        supports_cwd: true,
    },
    PulumiCommand {
        name: "logs",
        cli_args: &["logs"],
        description: "Show aggregated resource logs",
        category: CommandCategory::ProjectManagement,
        params: &[PARAM_CWD, PARAM_STACK, PARAM_JSON],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Streaming,
        shortcut: Some('l'),
        supports_cwd: true,
    },
    // Auth & Organization
    PulumiCommand {
        name: "login",
        cli_args: &["login"],
        description: "Log in to Pulumi Cloud",
        category: CommandCategory::AuthOrg,
        params: &[],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Quick,
        shortcut: None,
        supports_cwd: false,
    },
    PulumiCommand {
        name: "logout",
        cli_args: &["logout"],
        description: "Log out of Pulumi Cloud",
        category: CommandCategory::AuthOrg,
        params: &[],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Quick,
        shortcut: None,
        supports_cwd: false,
    },
    PulumiCommand {
        name: "whoami",
        cli_args: &["whoami"],
        description: "Show current logged-in user",
        category: CommandCategory::AuthOrg,
        params: &[],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Quick,
        shortcut: None,
        supports_cwd: false,
    },
    PulumiCommand {
        name: "org get-default",
        cli_args: &["org", "get-default"],
        description: "Show default organization",
        category: CommandCategory::AuthOrg,
        params: &[],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Quick,
        shortcut: None,
        supports_cwd: false,
    },
    PulumiCommand {
        name: "console",
        cli_args: &["console"],
        description: "Open in Pulumi Console",
        category: CommandCategory::AuthOrg,
        params: &[PARAM_CWD, PARAM_STACK],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Quick,
        shortcut: None,
        supports_cwd: true,
    },
    // Utilities
    PulumiCommand {
        name: "version",
        cli_args: &["version"],
        description: "Show Pulumi version",
        category: CommandCategory::Utilities,
        params: &[],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Quick,
        shortcut: None,
        supports_cwd: false,
    },
    PulumiCommand {
        name: "about",
        cli_args: &["about"],
        description: "Show environment info",
        category: CommandCategory::Utilities,
        params: &[PARAM_CWD],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Quick,
        shortcut: None,
        supports_cwd: true,
    },
    PulumiCommand {
        name: "plugin ls",
        cli_args: &["plugin", "ls"],
        description: "List installed plugins",
        category: CommandCategory::Utilities,
        params: &[PARAM_JSON],
        needs_confirmation: false,
        execution_mode: ExecutionMode::Quick,
        shortcut: None,
        supports_cwd: false,
    },
    PulumiCommand {
        name: "state delete",
        cli_args: &["state", "delete"],
        description: "Delete resource from state",
        category: CommandCategory::Utilities,
        params: &[PARAM_CWD, PARAM_STACK, PARAM_YES],
        needs_confirmation: true,
        execution_mode: ExecutionMode::Quick,
        shortcut: None,
        supports_cwd: true,
    },
];

/// Get commands by category
pub fn commands_by_category(category: CommandCategory) -> Vec<&'static PulumiCommand> {
    PULUMI_COMMANDS
        .iter()
        .filter(|cmd| cmd.category == category)
        .collect()
}

/// Get all categories with their commands count
#[allow(dead_code)]
pub fn categories_with_counts() -> Vec<(CommandCategory, usize)> {
    CommandCategory::all()
        .iter()
        .map(|&cat| (cat, commands_by_category(cat).len()))
        .collect()
}

/// Execution state of a command
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum CommandExecutionState {
    /// Not running
    Idle,
    /// Waiting for user to fill parameters
    AwaitingInput,
    /// Waiting for confirmation
    AwaitingConfirmation,
    /// Currently executing
    Running,
    /// Completed successfully
    Completed,
    /// Failed with error
    Failed(String),
}

/// A command execution instance with parameters
#[derive(Debug, Clone)]
pub struct CommandExecution {
    /// The command being executed
    pub command: &'static PulumiCommand,
    /// Parameter values (key -> value)
    pub param_values: std::collections::HashMap<String, String>,
    /// Execution state
    pub state: CommandExecutionState,
    /// Output lines collected
    pub output_lines: Vec<OutputLine>,
    /// Exit code if completed
    pub exit_code: Option<i32>,
}

/// A line of command output
#[derive(Debug, Clone)]
pub struct OutputLine {
    /// The text content
    pub text: String,
    /// Whether this is stderr (vs stdout)
    pub is_error: bool,
    /// Timestamp
    #[allow(dead_code)]
    pub timestamp: std::time::Instant,
}

impl CommandExecution {
    pub fn new(command: &'static PulumiCommand) -> Self {
        Self {
            command,
            param_values: std::collections::HashMap::new(),
            state: CommandExecutionState::AwaitingInput,
            output_lines: Vec::new(),
            exit_code: None,
        }
    }

    /// Get the working directory (defaults to current directory if empty or unspecified)
    pub fn get_working_directory(&self) -> Option<String> {
        self.param_values
            .get("cwd")
            .filter(|v| !v.is_empty())
            .cloned()
            .or_else(|| {
                std::env::current_dir()
                    .ok()
                    .and_then(|p| p.to_str().map(|s| s.to_string()))
            })
    }

    /// Build the full command line arguments
    /// Note: The "cwd" parameter is not included here as it's handled via current_dir()
    pub fn build_args(&self) -> Vec<String> {
        let mut args: Vec<String> = self
            .command
            .cli_args
            .iter()
            .map(|s| s.to_string())
            .collect();

        for param in self.command.params {
            // Skip cwd parameter - it's handled separately via current_dir()
            if param.name == "cwd" {
                continue;
            }

            if let Some(value) = self.param_values.get(param.name) {
                if value.is_empty() {
                    continue;
                }

                match param.param_type {
                    ParamType::Flag => {
                        if value == "true" || value == "yes" {
                            if let Some(long) = param.long {
                                args.push(long.to_string());
                            } else if let Some(short) = param.short {
                                args.push(short.to_string());
                            }
                        }
                    }
                    _ => {
                        // For positional arguments (no flags), just add the value
                        if param.long.is_none() && param.short.is_none() {
                            args.push(value.clone());
                        } else if let Some(long) = param.long {
                            args.push(long.to_string());
                            args.push(value.clone());
                        } else if let Some(short) = param.short {
                            args.push(short.to_string());
                            args.push(value.clone());
                        }
                    }
                }
            }
        }

        args
    }

    /// Get the display command string with parameters
    pub fn display_with_params(&self) -> String {
        let args = self.build_args();
        let cwd_prefix = self
            .get_working_directory()
            .map(|d| format!("(in {}) ", d))
            .unwrap_or_default();
        format!("{}pulumi {}", cwd_prefix, args.join(" "))
    }
}
