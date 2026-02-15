// Integration tests for gemini-mcp-rs
// These tests require a real Gemini CLI installation or mock

#[cfg(test)]
mod tests {
    use gemini_mcp_rs::gemini::Options;

    #[tokio::test]
    #[ignore] // Ignore by default - requires Gemini CLI
    async fn test_gemini_execution() {
        let _opts = Options {
            prompt: "Hello, world!".to_string(),
            sandbox: false,
            session_id: None,
            return_all_messages: false,
            model: None,
            timeout_secs: None,
        };

        // This test requires a real Gemini CLI installation
        // Uncomment and set GEMINI_BIN environment variable to run
        // let result = gemini_mcp_rs::gemini::run(opts).await;
        // assert!(result.is_ok());
    }

    /// Regression test: stdin write must be inside the timeout window.
    /// Uses a 2MB prompt to reliably fill pipe buffers (typ. 4KB-64KB)
    /// and a non-consuming child (sleeps 60s) to verify that timeout_secs
    /// bounds total request time including stdin write.
    #[tokio::test]
    async fn test_stdin_timeout_under_backpressure() {
        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();

        // Create a script that sleeps without reading stdin (unique per process)
        #[cfg(windows)]
        let script_path = {
            let path = temp_dir.join(format!("gemini_test_sleep_{}.cmd", pid));
            // ping -n 61 ≈ 60s delay; doesn't read stdin
            std::fs::write(&path, "@echo off\r\nping -n 61 127.0.0.1 >nul\r\n").unwrap();
            path
        };

        #[cfg(not(windows))]
        let script_path = {
            let path = temp_dir.join(format!("gemini_test_sleep_{}.sh", pid));
            std::fs::write(&path, "#!/bin/sh\nsleep 60\n").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
            path
        };

        std::env::set_var("GEMINI_BIN", script_path.to_str().unwrap());

        let opts = Options {
            // 2MB prompt: reliably exceeds pipe buffers (4KB-64KB),
            // forcing write_all to block until child consumes or timeout fires
            prompt: "a".repeat(2 * 1024 * 1024),
            sandbox: false,
            session_id: None,
            return_all_messages: false,
            model: None,
            timeout_secs: Some(2),
        };

        let start = std::time::Instant::now();
        let result = gemini_mcp_rs::gemini::run(opts).await;
        let elapsed = start.elapsed();

        // Clean up
        std::env::remove_var("GEMINI_BIN");
        let _ = std::fs::remove_file(&script_path);

        // Must fail with timeout error
        assert!(result.is_err(), "Expected error, got success");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("timed out"),
            "Expected timeout error, got: {}",
            err_msg
        );

        // Elapsed time must be bounded: timeout (2s) + cleanup (≤5s) + slack
        // Must be well under the 60s sleep, proving timeout_secs works
        assert!(
            elapsed.as_secs() < 15,
            "Request took {:?} with timeout_secs=2; timeout is not bounding total time",
            elapsed
        );
    }
}
