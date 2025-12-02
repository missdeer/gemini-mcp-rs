# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is **gemini-mcp-rs**, a Rust implementation of an MCP (Model Context Protocol) server that wraps the Gemini CLI. It enables Claude Code to invoke Gemini for AI-driven tasks through the MCP protocol.

Related implementations in this workspace:
- `geminimcp/` - Python implementation with session persistence
- `codex-mcp-rs/` - Rust MCP server for Codex CLI

## Build and Development Commands

### Building
```bash
cargo build              # Build in debug mode
cargo build --release    # Build optimized binary
```

### Running
```bash
cargo run                # Run the MCP server (listens on stdio)
cargo run -- --help      # Display help information
cargo run -- --version   # Display version information
```

### Testing
```bash
cargo test               # Run all tests
cargo test --lib         # Run library tests only
```

### Code Quality
```bash
cargo check              # Fast compilation check without producing binary
cargo clippy             # Lint with clippy
cargo fmt                # Format code
```

## Architecture

### Entry Point and Server Setup
The application follows a simple architecture:

1. **main.rs** - Entry point that:
   - Parses command-line arguments (`-h/--help`, `--version`) using clap
   - Initializes the MCP server with stdio transport
   - Provides comprehensive help documentation for users
2. **server.rs** - Defines the `gemini` MCP tool and handles parameter validation
3. **gemini.rs** - Core Gemini CLI wrapper that spawns processes and parses output
4. **lib.rs** - Module declarations

### Data Flow

```
Claude Code (MCP Client)
    ↓
stdio transport
    ↓
MCP Server (main.rs) → server::gemini() tool
    ↓
gemini::run() → spawns `gemini` subprocess
    ↓
Parses JSON-streamed output line-by-line
    ↓
Returns GeminiResult with session_id, agent_messages, all_messages
```

### Key Components

**server.rs:gemini()** - MCP tool function that:
- Validates required parameters (PROMPT)
- Converts empty string session_id/model to None
- Calls `gemini::run()` and formats response as `GeminiOutput`

**gemini.rs:run()** - Core execution function that:
- Reads `GEMINI.md` from the current directory (if it exists) and prepends its content to the user's prompt
- Builds the `gemini` command with proper arguments
- Uses Windows-specific prompt escaping when needed
- Spawns subprocess with stdin=null, stdout/stderr=piped
- Streams stdout line-by-line, parsing JSON events
- Extracts `session_id` (returned as SESSION_ID), `message` items with role="assistant", and error types
- Returns `GeminiResult` with all collected data

### Important Implementation Details

**Session Management**: The `SESSION_ID` (Gemini's `session_id`) enables multi-turn conversations. The server extracts it from JSON output and returns it to the client for subsequent calls.

**Error Handling**: The code checks for:
- Empty SESSION_ID (indicates failed session initialization)
- Empty agent_messages (indicates no response from Gemini)
- Non-zero exit codes from the Gemini subprocess
- JSON parse errors in streamed output

**Platform Differences**: Windows requires special prompt escaping (backslashes, quotes, newlines) to prevent shell interpretation issues.

**Streaming Output**: The Gemini CLI outputs JSONL (JSON Lines). The server reads line-by-line to handle potentially long-running operations and collect all agent messages incrementally.

**GEMINI.md Configuration**: The server supports a `GEMINI.md` configuration file in the current working directory. If present, its content is automatically prepended to every user prompt before sending to the Gemini CLI. This allows for:
- Project-specific context and instructions
- Consistent coding style preferences
- Domain-specific knowledge
- Custom response formatting requirements

Implementation details:
- The file is read asynchronously with proper error handling
- Maximum file size: 100KB (larger files are rejected with a warning)
- Empty files (only whitespace) are ignored with a warning
- File read errors (except not found) are logged as warnings to stderr
- Original formatting is preserved (no trimming of leading/trailing whitespace)
- This repository's `GEMINI.md` provides an example of project context configuration

## Dependencies

The project uses:
- **rmcp** - Official Rust MCP SDK from `modelcontextprotocol/rust-sdk`
- **tokio** - Async runtime (required by rmcp)
- **serde/serde_json** - Serialization for MCP protocol and Gemini output parsing
- **anyhow** - Error handling
- **clap** - Command-line argument parsing for help/version flags

## Gemini CLI Integration

This server wraps the `gemini` command. Key flags used:
- `--prompt <text>` - The task prompt
- `-o stream-json` - Enables JSON output streaming
- `--sandbox` - Enables sandbox mode
- `--model <name>` - Specifies model override
- `--resume <session_id>` - Continues previous session

## Testing Strategy

Unit tests exist for:
- Windows prompt escaping logic (gemini.rs:windows_escape)
- Options struct validation (gemini.rs)
- Server parameter handling (server.rs)
- JSON parsing and error handling (gemini.rs)

Integration tests should mock the gemini CLI subprocess or use a test binary.

