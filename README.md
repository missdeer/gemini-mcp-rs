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

### Command-Line Options

```bash
# Display help information
./target/release/gemini-mcp-rs --help

# Display version information
./target/release/gemini-mcp-rs --version
```

The `--help` flag provides comprehensive documentation including:
- Environment variables
- MCP client configuration examples
- All supported tool parameters
- GEMINI.md configuration file support
- Return structure format
- Best practices and security information

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
- `model` (string): The model to use for the gemini session. If not specified, uses `GEMINI_FORCE_MODEL` environment variable or the Gemini CLI default
- `timeout_secs` (int): Timeout in seconds for gemini execution (1-3600). Defaults to `GEMINI_DEFAULT_TIMEOUT` environment variable or 600 seconds (10 minutes)

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

- `GEMINI_DEFAULT_TIMEOUT`: Default timeout in seconds for gemini execution (1-3600). If not set, defaults to 600 seconds (10 minutes). This can be overridden per-request using the `timeout_secs` parameter.

  **Example:**
  ```bash
  export GEMINI_DEFAULT_TIMEOUT=300  # 5 minutes
  cargo run
  ```

- `GEMINI_FORCE_MODEL`: Default model to use when no `model` parameter is provided in the request. This is overridden by explicit `model` parameters.

  **Example:**
  ```bash
  export GEMINI_FORCE_MODEL=gemini-2.0-flash
  cargo run
  ```

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

- `src/main.rs`: Entry point that parses CLI arguments and starts the MCP server
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

