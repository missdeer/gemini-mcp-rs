use anyhow::Result;
use clap::Parser;
use gemini_mcp_rs::server::GeminiServer;
use rmcp::{transport::stdio, ServiceExt};

/// MCP server wrapping the Gemini CLI for AI-driven tasks
#[derive(Parser)]
#[command(
    name = "gemini-mcp-rs",
    version,
    about = "MCP server that provides AI-driven tasks through the Gemini CLI",
    long_about = None,
    after_help = "ENVIRONMENT VARIABLES:
  GEMINI_BIN                   Override the gemini binary path (default: 'gemini')
  GEMINI_DEFAULT_TIMEOUT       Default timeout in seconds (1-3600, default: 600)
  GEMINI_FORCE_MODEL           Default model when request omits 'model' parameter

USAGE:
  This server communicates via stdio using the Model Context Protocol (MCP).
  It should be configured in your MCP client (e.g., Claude Desktop) settings.

  Example MCP client configuration:
    {
      \"mcpServers\": {
        \"gemini\": {
          \"command\": \"/path/to/gemini-mcp-rs\"
        }
      }
    }

SUPPORTED PARAMETERS:
  The 'gemini' tool accepts the following parameters:

  PROMPT (required)            Task instruction to send to Gemini
  sandbox                      Run in sandbox mode (default: false)
  SESSION_ID                   Resume an existing session (from previous response)
  return_all_messages          Return all messages including reasoning (default: false)
  model                        Model to use (default: GEMINI_FORCE_MODEL or Gemini CLI default)
  timeout_secs                 Timeout in seconds (1-3600, default: GEMINI_DEFAULT_TIMEOUT or 600)

GEMINI.md SUPPORT:
  If a GEMINI.md file exists in the working directory, its content will be
  automatically prepended to the prompt as a system prompt. This allows you to
  define project-specific instructions or context for all Gemini invocations.
  Maximum file size: 100KB

RETURN STRUCTURE:
  The tool returns:
  - success: boolean indicating execution status
  - SESSION_ID: unique identifier for resuming conversations
  - agent_messages: concatenated assistant response text
  - all_messages: (optional) complete JSON events when return_all_messages=true
  - error: error description when success=false

BEST PRACTICES:
  - Always capture and reuse SESSION_ID for multi-turn interactions
  - Enable sandbox mode when file modifications should be isolated
  - Use return_all_messages only when detailed traces are needed
  - Keep GEMINI.md under 100KB for optimal performance

SECURITY:
  - Timeouts are enforced to prevent unbounded execution
  - Sandbox mode restricts operations to isolated environments
  - Maximum timeout is 3600 seconds (1 hour)

For more information, visit: https://github.com/missdeer/gemini-mcp-rs"
)]
struct Cli {}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments (this will handle -h/--help and --version)
    let _cli = Cli::parse();

    // Create an instance of our gemini server
    let service = GeminiServer::new().serve(stdio()).await.inspect_err(|e| {
        eprintln!("serving error: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}
