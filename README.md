# gemini-mcp-rs

[![License: GPLv3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust Version](https://img.shields.io/badge/rust-1.77.2%2B-blue.svg)](https://www.rust-lang.org)
[![MCP Compatible](https://img.shields.io/badge/MCP-Compatible-green.svg)](https://modelcontextprotocol.io)

[中文文档](README-zh_CN.md)

A high-performance Rust implementation of MCP (Model Context Protocol) server that wraps the Gemini CLI for AI-driven tasks.

## Quick Start

The easiest way to use gemini-mcp-rs is via npx - no manual installation required:

```bash
npx @missdeer/gemini-mcp-rs
```

This command automatically downloads the appropriate binary for your platform and runs it. To add it to Claude Code:

```bash
claude mcp add gemini-rs -s user --transport stdio -- npx @missdeer/gemini-mcp-rs
```

That's it! The MCP server is now available in Claude Code.

## Features

- **MCP Protocol Support**: Implements the official Model Context Protocol using the Rust SDK
- **Gemini Integration**: Wraps the Gemini CLI to enable AI-driven tasks through MCP
- **Session Management**: Supports multi-turn conversations via session IDs
- **Sandbox Safety**: Configurable sandbox mode for isolated execution
- **Async Runtime**: Built on Tokio for efficient async I/O
- **Cross-platform**: Works on Windows, Linux, and macOS (x64 and arm64)

## Prerequisites

- Rust 1.77.2+ (required for Windows batch file security fix, see [CVE-2024-24576](https://blog.rust-lang.org/2024/04/09/cve-2024-24576.html))
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

### Option 1: NPX (Recommended)

Run directly without installation using npx:

```bash
npx @missdeer/gemini-mcp-rs
```

Or install globally:

```bash
npm install -g @missdeer/gemini-mcp-rs
```

Then add to your Claude MCP configuration:

```bash
claude mcp add gemini-rs -s user --transport stdio -- npx @missdeer/gemini-mcp-rs
```

### Option 2: Quick Install (Linux/macOS)

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

### Option 3: Build from Source

```bash
git clone https://github.com/missdeer/gemini-mcp-rs.git
cd gemini-mcp-rs
cargo build --release
claude mcp add gemini-rs -s user --transport stdio -- $(pwd)/target/release/gemini-mcp-rs
```

### Option 4: Install from Release

Download the appropriate binary for your platform from the [releases page](https://github.com/missdeer/gemini-mcp-rs/releases):

| Platform | Architecture | Asset |
|----------|--------------|-------|
| Linux | x64 | `gemini-mcp-rs_Linux_x86_64.tar.gz` |
| Linux | arm64 | `gemini-mcp-rs_Linux_arm64.tar.gz` |
| macOS | Universal (x64 + arm64) | `gemini-mcp-rs_Darwin_universal.tar.gz` |
| Windows | x64 | `gemini-mcp-rs_Windows_x86_64.zip` |
| Windows | arm64 | `gemini-mcp-rs_Windows_arm64.zip` |

Extract and add to your MCP configuration:

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

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is dual-licensed:

### Non-Commercial / Personal Use - GNU General Public License v3.0

Free for personal projects, educational purposes, open source projects, and non-commercial use. See [LICENSE](LICENSE) for the full GPLv3 license text.

### Commercial / Workplace Use - Commercial License Required

**If you use gemini-mcp-rs in a commercial environment, workplace, or for any commercial purpose, you must obtain a commercial license.**

This includes but is not limited to:
- Using the software at work (any organization)
- Integrating into commercial products or services
- Using for client work or consulting
- Offering as part of a SaaS/cloud service

**Contact**: missdeer@gmail.com for commercial licensing inquiries.

See [LICENSE-COMMERCIAL](LICENSE-COMMERCIAL) for more details.

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=missdeer/gemini-mcp-rs&type=Date)](https://starchart.cc/missdeer/gemini-mcp-rs)

