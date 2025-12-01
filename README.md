# gemini-mcp-rs

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)
[![MCP Compatible](https://img.shields.io/badge/MCP-Compatible-green.svg)](https://modelcontextprotocol.io)

A high-performance Rust implementation of MCP (Model Context Protocol) server that wraps the Gemini CLI for AI-driven tasks.

> **Note**: This is a Rust port of the original Python implementation [geminimcp](../geminimcp). It offers the same functionality with improved performance and lower resource usage.

## Features

- **MCP Protocol Support**: Implements the official Model Context Protocol using the Rust SDK
- **Gemini Integration**: Wraps the Gemini CLI to enable AI-driven tasks through MCP
- **Session Management**: Supports multi-turn conversations via session IDs
- **Sandbox Safety**: Configurable sandbox mode for isolated execution
- **Async Runtime**: Built on Tokio for efficient async I/O
- **Cross-platform**: Works on Windows, Linux, and macOS

## Prerequisites

- Rust 1.90+ (uses 2021 edition)
- [Gemini CLI](https://github.com/google-gemini/gemini-cli) installed and configured
- Claude Code or another MCP client

## Building

```bash
# Debug build
cargo build

# Release build
cargo build --release
```

## Running

The server communicates via stdio transport:

```bash
cargo run
```

Or after building:

```bash
./target/release/gemini-mcp-rs
```

## Installation

### Option 1: Quick Install (Linux/macOS)

Install the latest release with a single command:

```bash
curl -sSL https://raw.githubusercontent.com/missdeer/gemini-mcp-rs/master/scripts/install.sh | bash
```

Or install a specific version:

```bash
curl -sSL https://raw.githubusercontent.com/missdeer/gemini-mcp-rs/master/scripts/install.sh | bash -s v0.1.0
```

This script will:
- Detect your platform and architecture
- Download the appropriate binary from GitHub releases
- Install it to `~/.local/bin` (or `/usr/local/bin` if needed)
- Automatically add it to your Claude MCP configuration

### Option 2: Build from Source

```bash
git clone https://github.com/missdeer/gemini-mcp-rs.git
cd gemini-mcp-rs
cargo build --release
claude mcp add gemini-rs -s user --transport stdio -- $(pwd)/target/release/gemini-mcp-rs
```

### Option 3: Install from Release

Download the appropriate binary for your platform from the releases page, extract it, and add to your MCP configuration:

```bash
claude mcp add gemini-rs -s user --transport stdio -- /path/to/gemini-mcp-rs
```

## Tool Usage

The server provides a single `gemini` tool with the following parameters:

### Required Parameters

- `PROMPT` (string): Instruction for the task to send to gemini

### Optional Parameters

- `sandbox` (bool): Run in sandbox mode. Defaults to `False`
- `SESSION_ID` (string): Resume the specified session of the gemini. Defaults to empty string, start a new session
- `return_all_messages` (bool): Return all messages (e.g. reasoning, tool calls, etc.) from the gemini session. Set to `False` by default, only the agent's final reply message is returned
- `model` (string): The model to use for the gemini session. This parameter is strictly prohibited unless explicitly specified by the user

### Return Structure

**Success:**
```json
{
  "success": true,
  "SESSION_ID": "session-uuid",
  "agent_messages": "Gemini's reply content..."
}
```

**With return_all_messages enabled:**
```json
{
  "success": true,
  "SESSION_ID": "session-uuid",
  "agent_messages": "Gemini's reply content...",
  "all_messages": [...]
}
```

**Failure:**
```json
{
  "success": false,
  "error": "Error description"
}
```

## Best Practices

- Always capture and reuse `SESSION_ID` for multi-turn interactions
- Enable `sandbox` mode when file modifications should be isolated
- Use `return_all_messages` only when detailed execution traces are necessary (increases payload size)
- Only pass `model` when the user has explicitly requested a specific model

## Configuration

### Environment Variables

- `GEMINI_BIN`: Override the Gemini CLI binary path. By default, the server uses `gemini` from your PATH. This is useful for:
  - Using a specific Gemini installation location
  - Testing with a custom binary
  - Development environments with multiple Gemini versions

  **Example:**
  ```bash
  export GEMINI_BIN=/usr/local/bin/gemini-custom
  cargo run
  ```

### GEMINI.md Configuration File

The server supports a `GEMINI.md` file in your current working directory. If this file exists, its content will be automatically prepended to every prompt sent to the Gemini CLI. This is useful for:

- **Project-specific context**: Add context about your project that should be included in every request
- **Coding style preferences**: Define consistent style guidelines for generated code
- **Domain-specific knowledge**: Include domain terminology or requirements
- **Response formatting**: Specify how you want responses formatted

**Example GEMINI.md:**
```markdown
# Project Context

You are working on a Rust MCP server project. Please follow these guidelines:
- Use idiomatic Rust code with proper error handling
- Follow the existing code style in the project
- Add comments for complex logic
- Consider performance and memory efficiency
```

**Usage:**
1. Create a file named `GEMINI.md` in your working directory
2. Add your configuration content
3. The content will be automatically prepended to all prompts

**Limitations and Warnings:**
- Maximum file size: 100KB (files larger than this will be ignored with a warning)
- Empty files (or files with only whitespace) are ignored
- File read errors (e.g., permission issues) are logged as warnings
- The original file formatting is preserved (including leading/trailing whitespace and newlines)

**Note:** This repository includes a `GEMINI.md` file that provides project context for AI-driven development. You can create your own `GEMINI.md` in any project directory where you use this MCP server.

## Testing

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Test with a custom Gemini binary
GEMINI_BIN=/path/to/gemini cargo test
```

## Architecture

The project follows a modular architecture:

- `src/main.rs`: Entry point that starts the MCP server
- `src/lib.rs`: Library root that exports modules
- `src/server.rs`: MCP server implementation and tool handlers
- `src/gemini.rs`: Gemini CLI execution and result parsing

## Comparison with Python Implementation

| Feature | gemini-mcp-rs (Rust) | geminimcp (Python) |
|---------|---------------------|-------------------|
| Language | Rust | Python |
| Performance | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| Memory Usage | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| Binary Size | Medium | N/A |
| Startup Time | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| Session Management | ✓ | ✓ |
| Sandbox Support | ✓ | ✓ |

## Related Projects

- [geminimcp](https://github.com/GuDaStudio/geminimcp) - Original Python implementation
- [codex-mcp-rs](https://github.com/missdeer/codex-mcp-rs) - Rust MCP server for Codex CLI

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT License - Copyright (c) 2025 missdeer

See [LICENSE](./LICENSE) for details.

