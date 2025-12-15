use crate::gemini::{self, Options, MAX_TIMEOUT_SECS, MIN_TIMEOUT_SECS};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use serde::Deserialize;

/// Input parameters for gemini tool
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GeminiArgs {
    /// Instruction for the task to send to gemini
    #[serde(rename = "PROMPT")]
    pub prompt: String,
    /// Run in sandbox mode. Defaults to `False`
    #[serde(default)]
    pub sandbox: bool,
    /// Resume the specified session of the gemini. If not provided or empty, starts a new session
    #[serde(rename = "SESSION_ID", default)]
    pub session_id: Option<String>,
    /// Return all messages (e.g. reasoning, tool calls, etc.) from the gemini session. Set to `False` by default, only the agent's final reply message is returned
    #[serde(default)]
    pub return_all_messages: bool,
    /// The model to use for the gemini session. If not specified, uses GEMINI_FORCE_MODEL
    /// environment variable or the Gemini CLI default
    #[serde(default)]
    pub model: Option<String>,
    /// Timeout in seconds for gemini execution (1-3600). If not specified, uses GEMINI_DEFAULT_TIMEOUT
    /// environment variable or falls back to 600 seconds (10 minutes).
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

#[derive(Clone)]
pub struct GeminiServer {
    tool_router: ToolRouter<GeminiServer>,
}

impl Default for GeminiServer {
    fn default() -> Self {
        Self::new()
    }
}

impl GeminiServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl GeminiServer {
    /// Invokes the Gemini CLI to execute AI-driven tasks, returning structured JSON events and a session identifier for conversation continuity.
    ///
    /// **Return structure:**
    /// - `success`: boolean indicating execution status
    /// - `SESSION_ID`: unique identifier for resuming this conversation in future calls
    /// - `agent_messages`: concatenated assistant response text
    /// - `all_messages`: (optional) complete array of JSON events when `return_all_messages=True`
    /// - `error`: error description when `success=False`
    ///
    /// **Best practices:**
    /// - Always capture and reuse `SESSION_ID` for multi-turn interactions
    /// - Enable `sandbox` mode when file modifications should be isolated
    /// - Use `return_all_messages` only when detailed execution traces are necessary (increases payload size)
    #[tool(
        name = "gemini",
        description = "Invokes the Gemini CLI to execute AI-driven tasks, returning structured JSON events and a session identifier for conversation continuity."
    )]
    async fn gemini(
        &self,
        Parameters(args): Parameters<GeminiArgs>,
    ) -> Result<CallToolResult, McpError> {
        // Validate required parameters
        if args.prompt.trim().is_empty() {
            return Err(McpError::invalid_params(
                "PROMPT is required and must be a non-empty, non-whitespace string",
                None,
            ));
        }

        if let Some(ref model) = args.model {
            if model.trim().is_empty() {
                return Err(McpError::invalid_params(
                    "Model overrides must be explicitly requested as a non-empty, non-whitespace string",
                    None,
                ));
            }
        }

        // Validate timeout_secs if provided
        if let Some(timeout) = args.timeout_secs {
            if !(MIN_TIMEOUT_SECS..=MAX_TIMEOUT_SECS).contains(&timeout) {
                return Err(McpError::invalid_params(
                    format!(
                        "timeout_secs must be between {} and {} seconds",
                        MIN_TIMEOUT_SECS, MAX_TIMEOUT_SECS
                    ),
                    None,
                ));
            }
        }

        // Convert empty string session_id to None
        let session_id = args.session_id.filter(|s| !s.is_empty());

        // Convert empty/whitespace string model to None
        let model = args.model.filter(|m| !m.trim().is_empty());

        // Create options for gemini client
        let opts = Options {
            prompt: args.prompt,
            sandbox: args.sandbox,
            session_id,
            return_all_messages: args.return_all_messages,
            model,
            timeout_secs: args.timeout_secs,
        };

        // Execute gemini
        let result = match gemini::run(opts).await {
            Ok(r) => r,
            Err(e) => {
                return Err(McpError::internal_error(
                    format!("Failed to execute gemini: {}", e),
                    None,
                ));
            }
        };

        // Prepare the response
        if result.success {
            let mut response_text = format!(
                "success: true\nSESSION_ID: {}\nagent_messages: {}",
                result.session_id, result.agent_messages
            );

            if args.return_all_messages && !result.all_messages.is_empty() {
                response_text.push_str(&format!(
                    "\nall_messages: {} events captured",
                    result.all_messages.len()
                ));
                if let Ok(json) = serde_json::to_string_pretty(&result.all_messages) {
                    response_text.push_str(&format!("\n\nFull event log:\n{}", json));
                }
            }

            Ok(CallToolResult::success(vec![Content::text(response_text)]))
        } else {
            let mut error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());

            // Include all_messages in error response if requested for debugging
            if args.return_all_messages && !result.all_messages.is_empty() {
                error_msg.push_str(&format!(
                    "\n\nCaptured {} events before failure:",
                    result.all_messages.len()
                ));
                if let Ok(json) = serde_json::to_string_pretty(&result.all_messages) {
                    error_msg.push_str(&format!("\n{}", json));
                }
            }

            Err(McpError::internal_error(error_msg, None))
        }
    }
}

#[tool_handler]
impl ServerHandler for GeminiServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "This server provides a gemini tool for AI-driven tasks. Use the gemini tool to execute tasks via the Gemini CLI.".to_string(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_args_deserialization() {
        let json = r#"{
            "PROMPT": "test prompt",
            "sandbox": true,
            "SESSION_ID": "session-123",
            "return_all_messages": false,
            "model": "gemini-pro"
        }"#;

        let args: GeminiArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.prompt, "test prompt");
        assert!(args.sandbox);
        assert_eq!(args.session_id, Some("session-123".to_string()));
        assert!(!args.return_all_messages);
        assert_eq!(args.model, Some("gemini-pro".to_string()));
    }

    #[test]
    fn test_gemini_args_empty_session_id_treated_as_none() {
        let json = r#"{
            "PROMPT": "test prompt",
            "SESSION_ID": ""
        }"#;

        let args: GeminiArgs = serde_json::from_str(json).unwrap();
        // Empty session_id is deserialized as Some(""), but will be filtered to None in the handler
        assert_eq!(args.session_id, Some("".to_string()));
    }
}
