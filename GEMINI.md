# Gemini Project Context: gemini-mcp-rs

This document provides a comprehensive overview of the `gemini-mcp-rs` project, its structure, and development conventions to be used as a context for AI-driven development.

## 1. Project Overview

`gemini-mcp-rs` is a high-performance, cross-platform server written in Rust that implements the Model Context Protocol (MCP). It acts as a wrapper around the Gemini CLI, allowing MCP-compatible clients (like Claude Code) to interact with the Gemini large language model.

### Key Technologies

- **Language:** Rust (2021 Edition)
- **Core Framework:** Tokio for asynchronous I/O.
- **Protocol:** Implements MCP using the `rmcp` Rust SDK.
- **Dependencies:** `serde` for serialization/deserialization, `anyhow` for error handling.

### Architecture

The project is structured into several key components:

- **`src/main.rs`:** The main entry point that initializes and runs the MCP server.
- **`src/server.rs`:** Contains the core server logic, defining the `gemini` tool and handling MCP requests.
- **`src/gemini.rs`:** Implements the logic for executing the Gemini CLI as a child process and parsing its output.
- **`npm/` directory:** Contains a JavaScript wrapper to package the Rust binary for NPM distribution. The `install.js` script downloads the appropriate pre-compiled binary from GitHub Releases upon installation.
- **`Makefile`:** Provides a convenient set of commands for common development tasks like building, testing, and linting.

## 2. Building and Running

The `Makefile` provides the most convenient way to build, test, and run the project.

### Key Commands

- **Build for release:**
  ```bash
  make build-release
  # Equivalent to: cargo build --release
  ```

- **Run all checks (format, lint, test):**
  ```bash
  make check
  ```

- **Run all tests:**
  ```bash
  make test
  # Equivalent to: cargo test --all-features
  ```

- **Format code:**
  ```bash
  make fmt
  # Equivalent to: cargo fmt
  ```

- **Run linter:**
  ```bash
  make clippy
  # Equivalent to: cargo clippy --all-targets --all-features -- -D warnings
  ```

- **Run the server (debug build):**
  The server communicates over stdio.
  ```bash
  cargo run
  ```

### Environment Variables

- **`GEMINI_BIN`:** To run tests or the server with a specific Gemini CLI binary, set this environment variable.
  ```bash
  export GEMINI_BIN=/path/to/custom/gemini
  cargo test
  ```

## 3. Development Conventions

Adherence to the established conventions is crucial for maintaining code quality and consistency.

### Code Style

- **Formatting:** All code must be formatted with `cargo fmt`. The `make fmt` command can be used.
- **Linting:** Code should be free of warnings from `cargo clippy`. Use `make clippy` to run the linter with strict settings (`-D warnings`).

### Testing

- All new features or bug fixes should be accompanied by relevant tests.
- All tests must pass before submitting code. Run `make test` to execute the full test suite.
- The project includes unit tests (`src/lib.rs`), integration tests (`tests/integration_tests.rs`), and server-specific tests (`tests/server_tests.rs`).

### Commits and Pull Requests

- **Commit Messages:** Follow the [Conventional Commits](https://www.conventionalcommits.org/) specification. This is important for the automated release process.
  - Example: `feat: add support for streaming responses`
  - Example: `fix: correctly parse multi-line JSON output`

### Versioning and Releases

- The project version is managed in three separate files and must be kept in sync:
  1.  `Cargo.toml`
  2.  `npm/package.json`
  3.  `server.json`
- The `make check-version` script can be used to verify consistency.
- Releases are automated via GitHub Actions. Pushing a new tag (e.g., `v0.2.0`) will trigger a workflow that builds binaries for all target platforms, creates a GitHub Release, and publishes the package to NPM.
