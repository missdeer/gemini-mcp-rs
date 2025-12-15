# @missdeer/gemini-mcp-rs

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![MCP Compatible](https://img.shields.io/badge/MCP-Compatible-green.svg)](https://modelcontextprotocol.io)

NPM package for **gemini-mcp-rs** - A high-performance Rust implementation of MCP (Model Context Protocol) server that wraps the Gemini CLI.

## Installation

```bash
npm install -g @missdeer/gemini-mcp-rs
```

This will automatically download and install the appropriate binary for your platform (Linux, macOS, or Windows).

## Usage with Claude Code

After installation, add to your Claude Code MCP configuration:

```bash
claude mcp add gemini-rs -s user --transport stdio -- gemini-mcp-rs
```

Or manually add to your `~/.claude/settings.json`:

```json
{
  "mcpServers": {
    "gemini-rs": {
      "command": "gemini-mcp-rs",
      "transport": "stdio"
    }
  }
}
```

## Features

- ✨ High-performance Rust implementation
- 🚀 Low memory footprint
- 🔒 Configurable sandbox mode
- 🔄 Session management for multi-turn conversations
- ⚡ Fast async I/O with Tokio

## Supported Platforms

- Linux (x86_64, arm64)
- macOS (x86_64, arm64)
- Windows (x86_64, arm64)

## Prerequisites

You must have the [Gemini CLI](https://github.com/google-gemini/gemini-cli) installed and configured on your system.

## Tool Parameters

The server provides a `gemini` tool with the following parameters:

- **PROMPT** (required): Task instruction
- **sandbox**: Run in sandbox mode (default: false)
- **SESSION_ID**: Resume previous session
- **return_all_messages**: Return full reasoning trace
- **model**: Override Gemini model
- **timeout_secs**: Timeout in seconds (1-3600, default: GEMINI_DEFAULT_TIMEOUT or 600)

## Environment Variables

- **GEMINI_BIN**: Override the Gemini CLI binary path (default: 'gemini')
- **GEMINI_DEFAULT_TIMEOUT**: Default timeout in seconds (1-3600, default: 600)

## Documentation

For detailed documentation, see the [GitHub repository](https://github.com/missdeer/gemini-mcp-rs).

## License

MIT License - Copyright (c) 2025 missdeer

## Related Projects

- [geminimcp](https://github.com/GuDaStudio/geminimcp) - Python implementation
- [codex-mcp-rs](https://github.com/missdeer/codex-mcp-rs) - Codex CLI MCP server

