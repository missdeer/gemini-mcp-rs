use anyhow::{Context, Result};
use serde_json::Value;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

const PROMPT_DEPRECATION_WARNING: &str = "The --prompt (-p) flag has been deprecated";
const KEY_SESSION_ID: &str = "session_id";
const KEY_TYPE: &str = "type";
const KEY_ROLE: &str = "role";
const KEY_CONTENT: &str = "content";
const KEY_ERROR: &str = "error";
const KEY_MESSAGE: &str = "message";
const TYPE_MESSAGE: &str = "message";
const ROLE_ASSISTANT: &str = "assistant";
const DEFAULT_TIMEOUT_SECS: u64 = 600; // 10 minutes
pub(crate) const MIN_TIMEOUT_SECS: u64 = 1;
pub(crate) const MAX_TIMEOUT_SECS: u64 = 3600; // 1 hour
const ENV_DEFAULT_TIMEOUT: &str = "GEMINI_DEFAULT_TIMEOUT";
const MAX_MESSAGES_LIMIT: usize = 10000; // Maximum number of messages to store
const MAX_NON_JSON_LINES: usize = 1000; // Maximum non-JSON lines to store
const MAX_STDERR_BYTES: usize = 100_000; // Maximum stderr output to capture (100KB)

/// Get the default timeout from environment variable or use the hardcoded default
fn get_default_timeout() -> u64 {
    std::env::var(ENV_DEFAULT_TIMEOUT)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|&t| (MIN_TIMEOUT_SECS..=MAX_TIMEOUT_SECS).contains(&t))
        .unwrap_or(DEFAULT_TIMEOUT_SECS)
}

#[derive(Debug, Clone)]
pub struct Options {
    pub prompt: String,
    pub sandbox: bool,
    pub session_id: Option<String>,
    pub return_all_messages: bool,
    pub model: Option<String>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug)]
pub struct GeminiResult {
    pub success: bool,
    pub session_id: String,
    pub agent_messages: String,
    pub all_messages: Vec<Value>,
    pub return_all_messages: bool,
    pub error: Option<String>,
}

/// Process a single JSON line from the gemini CLI output
fn process_json_line(line_data: &Value, result: &mut GeminiResult, return_all_messages: bool) {
    // Collect all messages if requested - store the raw Value to handle objects, arrays, and primitives
    // Limit the number of messages to prevent memory exhaustion
    if return_all_messages && result.all_messages.len() < MAX_MESSAGES_LIMIT {
        result.all_messages.push(line_data.clone());
    }

    // Extract session_id
    if let Some(session_id) = line_data.get(KEY_SESSION_ID).and_then(|v| v.as_str()) {
        if !session_id.is_empty() {
            result.session_id = session_id.to_string();
        }
    }

    // Extract agent messages
    let item_type = line_data
        .get(KEY_TYPE)
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let item_role = line_data
        .get(KEY_ROLE)
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if item_type == TYPE_MESSAGE && item_role == ROLE_ASSISTANT {
        if let Some(content) = line_data.get(KEY_CONTENT).and_then(|v| v.as_str()) {
            // Skip if it's just the CLI's own deprecation warning
            if content == PROMPT_DEPRECATION_WARNING {
                return;
            }
            if !result.agent_messages.is_empty() {
                result.agent_messages.push('\n');
            }
            result.agent_messages.push_str(content);
        }
    }

    // Check for errors (case-insensitive) - look for explicit error indicators
    let item_type_lower = item_type.to_lowercase();
    let has_explicit_error = item_type_lower.contains("fail") || item_type_lower.contains("error");
    let has_error_obj = line_data.get(KEY_ERROR).is_some();

    if has_explicit_error || has_error_obj {
        result.success = false;
        if let Some(error_obj) = line_data.get(KEY_ERROR).and_then(|v| v.as_object()) {
            if let Some(msg) = error_obj.get(KEY_MESSAGE).and_then(|v| v.as_str()) {
                result.error = Some(format!("gemini error: {}", msg));
            }
        } else if let Some(msg) = line_data.get(KEY_MESSAGE).and_then(|v| v.as_str()) {
            result.error = Some(format!("gemini error: {}", msg));
        }
    }
}

/// Build the gemini command with the given options
fn build_command(opts: &Options) -> Command {
    let gemini_bin = std::env::var("GEMINI_BIN").unwrap_or_else(|_| "gemini".to_string());

    let mut cmd = Command::new(gemini_bin);
    cmd.arg("--prompt");
    // Command::arg() on all platforms already does correct shell quoting,
    // so we pass the prompt as-is without manual escaping
    cmd.arg(&opts.prompt);
    cmd.arg("-o");
    cmd.arg("stream-json");

    // Add optional flags
    if opts.sandbox {
        cmd.arg("--sandbox");
    }
    if let Some(ref model) = opts.model {
        cmd.args(["--model", model]);
    }
    if let Some(ref session_id) = opts.session_id {
        cmd.args(["--resume", session_id]);
    }

    // Configure process
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    cmd
}

/// Execute Gemini CLI with the given options and return the result
pub async fn run(opts: Options) -> Result<GeminiResult> {
    // Validate options
    if opts.prompt.trim().is_empty() {
        return Err(anyhow::anyhow!(
            "Prompt must be a non-empty, non-whitespace string"
        ));
    }

    if let Some(timeout) = opts.timeout_secs {
        if !(MIN_TIMEOUT_SECS..=MAX_TIMEOUT_SECS).contains(&timeout) {
            return Err(anyhow::anyhow!(
                "timeout_secs must be between {} and {} seconds",
                MIN_TIMEOUT_SECS,
                MAX_TIMEOUT_SECS
            ));
        }
    }

    let timeout_duration =
        Duration::from_secs(opts.timeout_secs.unwrap_or_else(get_default_timeout));

    // Build and spawn the command with kill_on_drop enabled
    let mut cmd = build_command(&opts);
    cmd.kill_on_drop(true);
    let mut child = cmd.spawn().context("Failed to spawn gemini command")?;

    match timeout(
        timeout_duration,
        run_with_child(&mut child, opts.return_all_messages),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => {
            // Explicitly kill the child process on timeout to avoid zombies
            let _ = child.kill().await;
            let _ = child.wait().await;
            Err(anyhow::anyhow!(
                "Gemini command timed out after {} seconds",
                timeout_duration.as_secs()
            ))
        }
    }
}

/// Inner function that reads from a spawned child process
async fn run_with_child(
    child: &mut tokio::process::Child,
    return_all_messages: bool,
) -> Result<GeminiResult> {
    // Read stdout and stderr
    let stdout = child.stdout.take().context("Failed to get stdout")?;
    let stderr = child.stderr.take().context("Failed to get stderr")?;

    let mut result = GeminiResult {
        success: true,
        session_id: String::new(),
        agent_messages: String::new(),
        all_messages: Vec::new(),
        return_all_messages,
        error: None,
    };

    // Read stdout and stderr concurrently
    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();
    let mut stderr_output = String::new();
    let mut stderr_truncated = false;
    let mut non_json_lines = Vec::with_capacity(100); // Start with reasonable capacity
    let mut valid_json_seen = false;
    let mut stdout_closed = false;
    let mut stderr_closed = false;
    while !stdout_closed || !stderr_closed {
        tokio::select! {
            line = stdout_reader.next_line(), if !stdout_closed => {
                let line = line.context("Failed to read from stdout")?;

                match line {
                    Some(line) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }

                        // Parse JSON line
                        let line_data: Value = match serde_json::from_str(trimmed) {
                            Ok(data) => {
                                valid_json_seen = true;
                                data
                            }
                            Err(_) => {
                                // Collect non-JSON lines for potential logging (with limit)
                                if non_json_lines.len() < MAX_NON_JSON_LINES {
                                    non_json_lines.push(trimmed.to_string());
                                }
                                continue;
                            }
                        };

                        // Process the parsed JSON line
                        process_json_line(&line_data, &mut result, return_all_messages);
                    }
                    None => stdout_closed = true,
                }
            }
            line = stderr_reader.next_line(), if !stderr_closed => {
                match line {
                    Ok(Some(line)) => {
                        // Only capture stderr up to the limit
                        if stderr_output.len() < MAX_STDERR_BYTES && !stderr_truncated {
                            if !stderr_output.is_empty() {
                                stderr_output.push('\n');
                            }
                            let remaining = MAX_STDERR_BYTES - stderr_output.len();
                            if line.len() <= remaining {
                                stderr_output.push_str(&line);
                            } else {
                                stderr_output.push_str(&line[..remaining]);
                                stderr_output.push_str("\n... (stderr truncated)");
                                stderr_truncated = true;
                            }
                        }
                    }
                    Ok(None) => stderr_closed = true,
                    Err(e) => {
                        eprintln!("Warning: Failed to read from stderr: {}", e);
                        stderr_closed = true;
                    }
                }
            }
        }
    }

    // Wait for process to finish
    let status = child
        .wait()
        .await
        .context("Failed to wait for gemini command")?;

    if !status.success() {
        result.success = false;
        let error_msg = if let Some(ref err) = result.error {
            err.clone()
        } else {
            format!("gemini command failed with exit code: {:?}", status.code())
        };

        let mut full_error = error_msg;
        if !stderr_output.is_empty() {
            full_error = format!("{}\nStderr: {}", full_error, stderr_output);
        }
        // Always include non-JSON output on failure to help with diagnosis
        if !non_json_lines.is_empty() {
            full_error = format!(
                "{}\nNon-JSON output: {}",
                full_error,
                non_json_lines.join("\n")
            );
        }
        result.error = Some(full_error);
    } else if !non_json_lines.is_empty() && !valid_json_seen {
        // Process succeeded but no valid JSON was seen
        result.success = false;
        result.error = Some(format!(
            "No valid JSON output received from gemini CLI.\nOutput: {}",
            non_json_lines.join("\n")
        ));
    }

    Ok(enforce_required_fields(result))
}

fn enforce_required_fields(mut result: GeminiResult) -> GeminiResult {
    let mut errors = Vec::new();

    if result.session_id.is_empty() {
        errors.push("Failed to get `SESSION_ID` from the gemini session.".to_string());
    }

    // Only require agent_messages if return_all_messages is false and all_messages is empty
    if result.agent_messages.is_empty() && !result.return_all_messages {
        errors.push("Failed to get `agent_messages` from the gemini session.\nYou can try to set `return_all_messages` to `True` to get the full information.".to_string());
    } else if result.agent_messages.is_empty()
        && result.return_all_messages
        && result.all_messages.is_empty()
    {
        errors.push("Failed to get any messages from the gemini session.".to_string());
    }

    if !errors.is_empty() {
        result.success = false;
        let new_error = errors.join("\n");
        let existing_error = result.error.take().filter(|s| !s.is_empty());
        result.error = match existing_error {
            Some(prev) => Some(format!("{}\n{}", prev, new_error)),
            None => Some(new_error),
        };
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_options_creation() {
        let opts = Options {
            prompt: "test prompt".to_string(),
            sandbox: false,
            session_id: None,
            return_all_messages: false,
            model: None,
            timeout_secs: None,
        };

        assert_eq!(opts.prompt, "test prompt");
        assert!(!opts.sandbox);
    }

    #[test]
    fn test_options_with_session() {
        let opts = Options {
            prompt: "resume task".to_string(),
            sandbox: true,
            session_id: Some("test-session-123".to_string()),
            return_all_messages: true,
            model: Some("gemini-pro".to_string()),
            timeout_secs: Some(300),
        };

        assert_eq!(opts.session_id, Some("test-session-123".to_string()));
        assert_eq!(opts.model, Some("gemini-pro".to_string()));
        assert!(opts.return_all_messages);
        assert!(opts.sandbox);
    }

    #[test]
    fn test_enforce_required_fields_requires_session_id() {
        let result = GeminiResult {
            success: true,
            session_id: String::new(),
            agent_messages: "msg".to_string(),
            all_messages: Vec::new(),
            return_all_messages: false,
            error: None,
        };

        let updated = enforce_required_fields(result);

        assert!(!updated.success);
        assert!(updated
            .error
            .as_ref()
            .unwrap()
            .contains("Failed to get `SESSION_ID`"));
    }

    #[test]
    fn test_enforce_required_fields_requires_agent_messages_when_not_returning_all() {
        let result = GeminiResult {
            success: true,
            session_id: "session".to_string(),
            agent_messages: String::new(),
            all_messages: Vec::new(),
            return_all_messages: false,
            error: None,
        };

        let updated = enforce_required_fields(result);

        assert!(!updated.success);
        assert!(updated
            .error
            .as_ref()
            .unwrap()
            .contains("Failed to get `agent_messages`"));
    }

    #[test]
    fn test_enforce_required_fields_allows_empty_agent_messages_with_all_messages() {
        let result = GeminiResult {
            success: true,
            session_id: "session".to_string(),
            agent_messages: String::new(),
            all_messages: vec![serde_json::json!({"type": "tool_use"})],
            return_all_messages: true,
            error: None,
        };

        let updated = enforce_required_fields(result);

        assert!(updated.success);
        assert!(updated.error.is_none());
    }

    #[test]
    fn test_build_command_basic() {
        let opts = Options {
            prompt: "test prompt".to_string(),
            sandbox: false,
            session_id: None,
            return_all_messages: false,
            model: None,
            timeout_secs: None,
        };

        let cmd = build_command(&opts);
        let program = cmd.as_std().get_program();

        // Should use "gemini" as the binary name (or GEMINI_BIN env var)
        assert!(program == "gemini" || program.to_string_lossy().contains("gemini"));
    }

    #[test]
    fn test_build_command_with_all_options() {
        let opts = Options {
            prompt: "complex prompt".to_string(),
            sandbox: true,
            session_id: Some("session-123".to_string()),
            return_all_messages: true,
            model: Some("gemini-pro".to_string()),
            timeout_secs: Some(120),
        };

        let cmd = build_command(&opts);
        let program = cmd.as_std().get_program();

        // Should use "gemini" as the binary name
        assert!(program == "gemini" || program.to_string_lossy().contains("gemini"));
    }

    #[test]
    fn test_build_command_with_session_only() {
        let opts = Options {
            prompt: "resume".to_string(),
            sandbox: false,
            session_id: Some("abc-123".to_string()),
            return_all_messages: false,
            model: None,
            timeout_secs: None,
        };

        let cmd = build_command(&opts);
        let program = cmd.as_std().get_program();

        assert!(program == "gemini" || program.to_string_lossy().contains("gemini"));
    }

    /// RAII guard to restore environment variable on drop
    struct EnvVarGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvVarGuard {
        fn new(key: &'static str) -> Self {
            let original = std::env::var(key).ok();
            Self { key, original }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(val) => std::env::set_var(self.key, val),
                None => std::env::remove_var(self.key),
            }
        }
    }

    // Note: This test covers all env var scenarios in a single test to avoid
    // race conditions when tests run in parallel (env vars are process-global state)
    #[test]
    fn test_get_default_timeout_env_var() {
        // Save and restore the original env var value using RAII guard
        let _guard = EnvVarGuard::new(ENV_DEFAULT_TIMEOUT);

        // Test without env var
        std::env::remove_var(ENV_DEFAULT_TIMEOUT);
        assert_eq!(get_default_timeout(), DEFAULT_TIMEOUT_SECS);

        // Test with valid values
        std::env::set_var(ENV_DEFAULT_TIMEOUT, "300");
        assert_eq!(get_default_timeout(), 300);

        // Test boundary: minimum valid value
        std::env::set_var(ENV_DEFAULT_TIMEOUT, "1");
        assert_eq!(get_default_timeout(), 1);

        // Test boundary: maximum valid value
        std::env::set_var(ENV_DEFAULT_TIMEOUT, "3600");
        assert_eq!(get_default_timeout(), 3600);

        // Test with whitespace (should be trimmed)
        std::env::set_var(ENV_DEFAULT_TIMEOUT, "  300  ");
        assert_eq!(get_default_timeout(), 300);

        // Test with non-numeric value (should fallback)
        std::env::set_var(ENV_DEFAULT_TIMEOUT, "invalid");
        assert_eq!(get_default_timeout(), DEFAULT_TIMEOUT_SECS);

        // Test with zero (invalid, should fallback)
        std::env::set_var(ENV_DEFAULT_TIMEOUT, "0");
        assert_eq!(get_default_timeout(), DEFAULT_TIMEOUT_SECS);

        // Test with value > MAX_TIMEOUT_SECS (invalid, should fallback)
        std::env::set_var(ENV_DEFAULT_TIMEOUT, "3601");
        assert_eq!(get_default_timeout(), DEFAULT_TIMEOUT_SECS);

        // Test with negative value (should fallback since u64 parse fails)
        std::env::set_var(ENV_DEFAULT_TIMEOUT, "-100");
        assert_eq!(get_default_timeout(), DEFAULT_TIMEOUT_SECS);

        // Test with empty string (should fallback)
        std::env::set_var(ENV_DEFAULT_TIMEOUT, "");
        assert_eq!(get_default_timeout(), DEFAULT_TIMEOUT_SECS);

        // Test with floating point (should fallback since u64 parse fails)
        std::env::set_var(ENV_DEFAULT_TIMEOUT, "300.5");
        assert_eq!(get_default_timeout(), DEFAULT_TIMEOUT_SECS);

        // Test with leading zeros (should work)
        std::env::set_var(ENV_DEFAULT_TIMEOUT, "0300");
        assert_eq!(get_default_timeout(), 300);

        // Test with very large number (should fallback)
        std::env::set_var(ENV_DEFAULT_TIMEOUT, "999999999999999999999");
        assert_eq!(get_default_timeout(), DEFAULT_TIMEOUT_SECS);

        // Test with special characters (should fallback)
        std::env::set_var(ENV_DEFAULT_TIMEOUT, "300s");
        assert_eq!(get_default_timeout(), DEFAULT_TIMEOUT_SECS);

        // Test with tabs (should be trimmed)
        std::env::set_var(ENV_DEFAULT_TIMEOUT, "\t300\t");
        assert_eq!(get_default_timeout(), 300);

        // Test with newlines (should be trimmed)
        std::env::set_var(ENV_DEFAULT_TIMEOUT, "\n300\n");
        assert_eq!(get_default_timeout(), 300);
    }

    #[test]
    fn test_timeout_validation_in_run() {
        // Test that invalid timeout_secs values are rejected
        let opts = Options {
            prompt: "test".to_string(),
            sandbox: false,
            session_id: None,
            return_all_messages: false,
            model: None,
            timeout_secs: Some(0), // Invalid: below minimum
        };

        // We can't actually run the command, but we can verify the validation logic
        // by checking that the error message mentions timeout
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(run(opts));
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("timeout_secs"));
        assert!(err_msg.contains("1"));
        assert!(err_msg.contains("3600"));
    }

    #[test]
    fn test_timeout_validation_above_max() {
        let opts = Options {
            prompt: "test".to_string(),
            sandbox: false,
            session_id: None,
            return_all_messages: false,
            model: None,
            timeout_secs: Some(3601), // Invalid: above maximum
        };

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(run(opts));
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("timeout_secs"));
    }

    #[test]
    fn test_timeout_validation_valid_boundaries() {
        // Test minimum valid value (1 second)
        let opts_min = Options {
            prompt: "test".to_string(),
            sandbox: false,
            session_id: None,
            return_all_messages: false,
            model: None,
            timeout_secs: Some(1), // Valid: minimum
        };

        // This will fail because gemini CLI doesn't exist, but it should pass validation
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(run(opts_min));
        // Error should be about spawning gemini, not about timeout validation
        if let Err(e) = result {
            assert!(
                !e.to_string().contains("timeout_secs"),
                "Should not fail on timeout validation"
            );
        }

        // Test maximum valid value (3600 seconds)
        let opts_max = Options {
            prompt: "test".to_string(),
            sandbox: false,
            session_id: None,
            return_all_messages: false,
            model: None,
            timeout_secs: Some(3600), // Valid: maximum
        };

        let result = runtime.block_on(run(opts_max));
        if let Err(e) = result {
            assert!(
                !e.to_string().contains("timeout_secs"),
                "Should not fail on timeout validation"
            );
        }
    }
}
