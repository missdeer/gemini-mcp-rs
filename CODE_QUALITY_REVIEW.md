# Code Quality Review

## Overview
- The project wraps a Gemini CLI invocation behind an MCP tool (`GeminiServer`) with async process management and output parsing in `src/gemini.rs` and `src/server.rs`.
- Validation is performed both at the MCP boundary (tool parameters) and before invoking the CLI to catch common misconfiguration early.

## Strengths
- The MCP tool validates prompt presence, model overrides, and timeout bounds before invoking the CLI, reducing the risk of sending unusable requests downstream.【F:src/server.rs†L74-L116】
- Process management enables concurrent stdout/stderr handling, timeouts, and mandatory child termination on timeout to avoid zombie processes.【F:src/gemini.rs†L233-L255】【F:src/gemini.rs†L262-L383】

## Risks and Recommendations
1. **Unbounded agent response accumulation**
   - `process_json_line` concatenates assistant messages without any size or line-count guard, so large or looping outputs could exhaust memory even when `return_all_messages` is false.【F:src/gemini.rs†L112-L165】
   - _Recommendation_: enforce a maximum aggregated message size or line count similar to the `MAX_MESSAGES_LIMIT` used for `all_messages`.

2. **Non-JSON output handling can still grow large**
   - While the code caps stored non-JSON lines at 1,000, each line is kept in full, so long stderr/stdout lines could still blow up memory and degrade error messages.【F:src/gemini.rs†L275-L379】
   - _Recommendation_: truncate individual non-JSON lines (and `stderr_output`) to a reasonable length before pushing, preserving diagnostics without risking large allocations.

3. **Minimal negative-path and streaming coverage**
   - Existing unit tests focus on struct construction and simple validations; the only end-to-end test is ignored by default, leaving streaming/error scenarios untested (e.g., malformed JSON, timeouts, truncated stderr).【F:tests/integration_tests.rs†L1-L44】【F:tests/server_tests.rs†L1-L24】【F:src/gemini.rs†L414-L520】
   - _Recommendation_: add async tests using a stub process that emits interleaved JSON/non-JSON lines to validate parsing, truncation, and timeout/error composition.

4. **Large `all_messages` payload risk**
   - `return_all_messages` stores up to 10,000 JSON values; heavy tool outputs could still be sizable, especially when returned in the success path for `return_all_messages=true`.【F:src/gemini.rs†L112-L165】【F:src/server.rs†L128-L160】
   - _Recommendation_: enforce a byte-size budget or expose pagination/streaming options to avoid returning overly large responses to MCP clients.
