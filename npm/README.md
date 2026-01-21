# @missdeer/gemini-mcp-rs

[![License: GPLv3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![MCP Compatible](https://img.shields.io/badge/MCP-Compatible-green.svg)](https://modelcontextprotocol.io)

NPM package for **gemini-mcp-rs** - A high-performance Rust implementation of MCP (Model Context Protocol) server that wraps the Gemini CLI.

## Quick Start

Run directly without installation:

```bash
npx @missdeer/gemini-mcp-rs
```

Add to Claude Code in one command:

```bash
claude mcp add gemini-rs -s user --transport stdio -- npx @missdeer/gemini-mcp-rs
```

## Installation (Optional)

For frequent use, install globally:

```bash
npm install -g @missdeer/gemini-mcp-rs
```

This will automatically download and install the appropriate binary for your platform (Linux, macOS, or Windows).

Then add to Claude Code:

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
- **model**: Override Gemini model (default: GEMINI_FORCE_MODEL or Gemini CLI default)
- **timeout_secs**: Timeout in seconds (1-3600, default: GEMINI_DEFAULT_TIMEOUT or 600)

## Environment Variables

- **GEMINI_BIN**: Override the Gemini CLI binary path (default: 'gemini')
- **GEMINI_DEFAULT_TIMEOUT**: Default timeout in seconds (1-3600, default: 600)
- **GEMINI_FORCE_MODEL**: Default model when request omits 'model' parameter

## Documentation

For detailed documentation, see the [GitHub repository](https://github.com/missdeer/gemini-mcp-rs).

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

## Related Projects

- [codex-mcp-rs](https://github.com/missdeer/codex-mcp-rs) - Codex CLI MCP server

