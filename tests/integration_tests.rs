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

    #[test]
    fn test_options_validation() {
        let opts = Options {
            prompt: "test".to_string(),
            sandbox: true,
            session_id: Some("session-123".to_string()),
            return_all_messages: true,
            model: Some("gemini-pro".to_string()),
            timeout_secs: Some(300),
        };

        assert_eq!(opts.prompt, "test");
        assert!(opts.sandbox);
        assert_eq!(opts.session_id, Some("session-123".to_string()));
        assert!(opts.return_all_messages);
        assert_eq!(opts.model, Some("gemini-pro".to_string()));
        assert_eq!(opts.timeout_secs, Some(300));
    }

    #[tokio::test]
    async fn test_gemini_md_config_prepending() {
        // This test verifies that GEMINI.md content is properly read and would be prepended
        // We use the internal read_gemini_config_from_path function for integration testing
        use tokio::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("GEMINI.md");
        let config_content = "You are a helpful assistant working on a Rust project.";
        fs::write(&config_path, config_content).await.unwrap();

        // Call the internal function used by the real implementation
        let loaded_config = gemini_mcp_rs::gemini::read_gemini_config_from_path(&config_path).await;

        // Verify the config was loaded correctly
        assert_eq!(loaded_config, Some(config_content.to_string()));

        // The full prepending logic (config + user prompt) is tested in unit tests
        // This integration test verifies the complete file reading path works end-to-end
    }
}
