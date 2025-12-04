//! Command executor for running Pulumi CLI commands
//!
//! Handles running commands as subprocesses with streaming output.
//! Uses a pseudo-TTY (PTY) to make Pulumi output properly stream.

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{BufRead, BufReader};
use std::sync::mpsc as std_mpsc;
use std::thread;
use tokio::sync::mpsc;

use super::types::{CommandExecution, CommandExecutionState, OutputLine};

/// Result from command execution
#[derive(Debug)]
pub enum CommandResult {
    /// New output line received
    OutputLine(OutputLine),
    /// Command completed
    Completed { exit_code: i32 },
    /// Command failed to start
    Failed(String),
}

/// Start executing a command and stream output using PTY
pub fn spawn_command(execution: &CommandExecution, tx: mpsc::Sender<CommandResult>) {
    let args = execution.build_args();
    let display = execution.display_with_params();
    let cwd = execution.get_working_directory();

    // Clone values for the spawned thread
    let args_clone = args.clone();
    let cwd_clone = cwd.clone();

    tokio::spawn(async move {
        log::info!("Executing via PTY: {}", display);
        if let Some(ref dir) = cwd_clone {
            log::info!("Working directory: {}", dir);
        }

        // Use a blocking thread for PTY operations since portable-pty is sync
        let (sync_tx, sync_rx) = std_mpsc::channel::<CommandResult>();

        let pty_thread = thread::spawn(move || {
            // Create PTY system
            let pty_system = native_pty_system();

            // Create a PTY pair with reasonable size
            let pair = match pty_system.openpty(PtySize {
                rows: 50,
                cols: 200,
                pixel_width: 0,
                pixel_height: 0,
            }) {
                Ok(pair) => pair,
                Err(e) => {
                    let _ = sync_tx.send(CommandResult::Failed(format!(
                        "Failed to create PTY: {}",
                        e
                    )));
                    return;
                }
            };

            // Build command
            let mut cmd = CommandBuilder::new("pulumi");
            for arg in &args_clone {
                cmd.arg(arg);
            }

            // Set working directory if specified
            if let Some(ref dir) = cwd_clone {
                cmd.cwd(dir);
            }

            // Set environment variables
            cmd.env("PULUMI_SKIP_UPDATE_CHECK", "true");
            // Don't set PULUMI_NON_INTERACTIVE - we want TTY behavior
            // Use raw output mode to get machine-readable output
            cmd.env("PULUMI_COLOR", "never");
            cmd.env("PYTHONUNBUFFERED", "1");
            cmd.env("TERM", "xterm-256color");

            // Spawn the child process in the PTY
            let mut child = match pair.slave.spawn_command(cmd) {
                Ok(child) => child,
                Err(e) => {
                    let _ = sync_tx.send(CommandResult::Failed(format!(
                        "Failed to spawn command: {}",
                        e
                    )));
                    return;
                }
            };

            // Drop the slave side - we only need the master for reading
            drop(pair.slave);

            // Get a reader for the master side
            let reader = match pair.master.try_clone_reader() {
                Ok(reader) => reader,
                Err(e) => {
                    let _ = sync_tx.send(CommandResult::Failed(format!(
                        "Failed to get PTY reader: {}",
                        e
                    )));
                    return;
                }
            };

            // Read output in a separate thread
            let sync_tx_reader = sync_tx.clone();
            let reader_thread = thread::spawn(move || {
                let buf_reader = BufReader::new(reader);
                let mut last_line: Option<String> = None;

                for line in buf_reader.lines() {
                    match line {
                        Ok(text) => {
                            // Filter out ANSI escape sequences and control characters
                            let clean_text = strip_ansi_codes(&text);

                            // Skip empty lines and duplicate consecutive lines
                            if clean_text.is_empty() {
                                continue;
                            }

                            // Skip if this is the same as the last line (progress updates)
                            if let Some(ref last) = last_line {
                                if is_duplicate_progress_line(last, &clean_text) {
                                    continue;
                                }
                            }

                            // Skip repeated table headers from progress display
                            if is_progress_table_header(&clean_text) {
                                // Only skip if we've seen content before
                                if last_line.is_some() {
                                    continue;
                                }
                            }

                            last_line = Some(clean_text.clone());

                            let output_line = OutputLine {
                                text: clean_text,
                                is_error: false,
                                timestamp: std::time::Instant::now(),
                            };
                            if sync_tx_reader
                                .send(CommandResult::OutputLine(output_line))
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            });

            // Wait for process to complete
            match child.wait() {
                Ok(status) => {
                    // Wait for reader to finish
                    let _ = reader_thread.join();

                    let exit_code = status.exit_code() as i32;
                    log::info!("Command completed with exit code: {}", exit_code);
                    let _ = sync_tx.send(CommandResult::Completed { exit_code });
                }
                Err(e) => {
                    log::error!("Failed to wait for command: {}", e);
                    let _ = sync_tx.send(CommandResult::Failed(e.to_string()));
                }
            }
        });

        // Forward results from sync channel to async channel
        loop {
            match sync_rx.recv() {
                Ok(result) => {
                    let is_terminal = matches!(
                        result,
                        CommandResult::Completed { .. } | CommandResult::Failed(_)
                    );
                    if tx.send(result).await.is_err() {
                        break;
                    }
                    if is_terminal {
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        // Wait for PTY thread to finish
        let _ = pty_thread.join();
    });
}

/// Check if two lines are duplicate progress updates
/// Pulumi updates the same line in place with different counts
fn is_duplicate_progress_line(prev: &str, current: &str) -> bool {
    // If lines are exactly the same, it's a duplicate
    if prev == current {
        return true;
    }

    // Check if both lines are progress table rows (Type/Name/Plan format)
    // These lines look like: "pulumi:pulumi:Stack  project-name  running"
    // Only the status or count changes

    // Extract the first two columns (type and name) and compare
    let prev_parts: Vec<&str> = prev.split_whitespace().collect();
    let curr_parts: Vec<&str> = current.split_whitespace().collect();

    // Both must have at least 2 parts
    if prev_parts.len() >= 2 && curr_parts.len() >= 2 {
        // If type and name are the same, and this looks like a status update
        if prev_parts[0] == curr_parts[0] && prev_parts[1] == curr_parts[1] {
            // Check if the last part is a status indicator
            let statuses = ["running", "creating", "updating", "deleting", "reading"];
            let prev_has_status = prev_parts.last().map(|s| statuses.contains(s)).unwrap_or(false);
            let curr_has_status = curr_parts.last().map(|s| statuses.contains(s)).unwrap_or(false);
            if prev_has_status || curr_has_status {
                return true;
            }
        }
    }

    // Check if both are "Resources:" count lines - keep only the last one
    if prev.starts_with("Resources:") && current.starts_with("Resources:") {
        return true;
    }

    // Check if both are count lines like "102 unchanged"
    if is_resource_count_line(prev) && is_resource_count_line(current) {
        return true;
    }

    false
}

/// Check if a line is a resource count line (e.g., "102 unchanged")
fn is_resource_count_line(line: &str) -> bool {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 {
        // First part should be a number
        if parts[0].parse::<u32>().is_ok() {
            let status_words = ["unchanged", "created", "updated", "deleted", "replaced"];
            return status_words.iter().any(|w| parts[1].contains(w));
        }
    }
    false
}

/// Check if a line is a progress table header
fn is_progress_table_header(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed == "Type"
        || trimmed == "Name"
        || trimmed == "Plan"
        || trimmed == "Status"
        || trimmed == "Type                          Name                    Plan"
        || (trimmed.starts_with("Type") && trimmed.contains("Name") && trimmed.contains("Plan"))
}

/// Strip ANSI escape codes and control characters from text
fn strip_ansi_codes(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // ESC character - skip the escape sequence
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // Skip until we find a letter (end of CSI sequence)
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() || next == 'm' || next == 'K' || next == 'H' {
                        break;
                    }
                }
            } else if chars.peek() == Some(&']') {
                // OSC sequence - skip until ST or BEL
                chars.next();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '\x07' || next == '\\' {
                        break;
                    }
                }
            }
        } else if c == '\r' {
            // Carriage return - skip (handle \r\n as just \n)
            continue;
        } else if c.is_control() && c != '\n' && c != '\t' {
            // Skip other control characters
            continue;
        } else {
            result.push(c);
        }
    }

    result.trim().to_string()
}

/// Check if the command can be run (not interactive)
pub fn can_run_command(execution: &CommandExecution) -> Result<(), String> {
    use super::types::ExecutionMode;

    if execution.command.execution_mode == ExecutionMode::Interactive {
        return Err(format!(
            "Command '{}' requires interactive mode and cannot be run in the TUI. \
             Please run it directly in your terminal.",
            execution.command.name
        ));
    }

    // Check required parameters
    for param in execution.command.params {
        if param.required {
            let value = execution.param_values.get(param.name);
            if value.is_none() || value.map(|v| v.is_empty()).unwrap_or(true) {
                return Err(format!("Required parameter '{}' is missing", param.name));
            }
        }
    }

    Ok(())
}

/// Update execution state based on result
pub fn update_execution_state(execution: &mut CommandExecution, result: CommandResult) {
    match result {
        CommandResult::OutputLine(line) => {
            // Additional deduplication at the state level
            // Skip if this exact line was just added
            if let Some(last) = execution.output_lines.last() {
                if last.text == line.text {
                    return;
                }
            }
            execution.output_lines.push(line);
        }
        CommandResult::Completed { exit_code } => {
            execution.exit_code = Some(exit_code);
            if exit_code == 0 {
                execution.state = CommandExecutionState::Completed;
            } else {
                execution.state = CommandExecutionState::Failed(format!("Exit code: {}", exit_code));
            }
        }
        CommandResult::Failed(error) => {
            execution.state = CommandExecutionState::Failed(error);
        }
    }
}
