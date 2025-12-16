//! Mock Git service for shadow world testing
//!
//! Provides mock implementations of all Git operations without requiring
//! real Git repositories or the git2 library.

use async_trait::async_trait;
use crate::services::{GitOperations, GitServiceError};
use crate::services::git_trait::CommitInfo;
use crate::models::FrameworkType;
use crate::shadow::{ShadowConfig, ShadowState};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::time::Duration;

/// Mock Git service for testing
pub struct MockGitService {
    config: ShadowConfig,
    state: Arc<ShadowState>,
}

impl MockGitService {
    /// Create a new mock Git service
    ///
    /// # Arguments
    /// * `config` - Shadow configuration
    /// * `state` - Shared shadow state for tracking operations
    pub fn new(config: ShadowConfig, state: Arc<ShadowState>) -> Self {
        Self {
            config,
            state,
        }
    }
    
    /// Simulate realistic delay for operation
    async fn simulate_delay(&self, millis: u64) {
        if self.config.simulate_delays {
            tokio::time::sleep(Duration::from_millis(millis)).await;
        }
    }
    
    /// Check if operation should fail based on config
    fn check_failure(&self, operation: &str) -> Result<(), GitServiceError> {
        if self.config.should_fail() {
            Err(GitServiceError::CloneFailed(
                format!("Simulated failure: {}", operation)
            ))
        } else {
            Ok(())
        }
    }
    
    /// Create mock project files based on detected framework
    fn create_mock_project(&self, path: &Path, repo_url: &str) -> Result<FrameworkType, GitServiceError> {
        // Determine framework from URL or randomly
        let framework = self.determine_framework_from_url(repo_url);
        
        // Create appropriate mock project files
        match framework {
            FrameworkType::NextJs => {
                let package_json = r#"{
  "name": "mock-nextjs-app",
  "version": "1.0.0",
  "dependencies": {
    "next": "13.0.0",
    "react": "18.0.0",
    "react-dom": "18.0.0"
  },
  "scripts": {
    "dev": "next dev",
    "build": "next build",
    "start": "next start"
  }
}"#;
                std::fs::write(path.join("package.json"), package_json)
                    .map_err(|e| GitServiceError::FileReadFailed(e.to_string()))?;
            }
            FrameworkType::React => {
                let package_json = r#"{
  "name": "mock-react-app",
  "version": "1.0.0",
  "dependencies": {
    "react": "18.0.0",
    "react-dom": "18.0.0",
    "react-scripts": "5.0.0"
  },
  "scripts": {
    "start": "react-scripts start",
    "build": "react-scripts build"
  }
}"#;
                std::fs::write(path.join("package.json"), package_json)
                    .map_err(|e| GitServiceError::FileReadFailed(e.to_string()))?;
            }
            FrameworkType::Vue => {
                let package_json = r#"{
  "name": "mock-vue-app",
  "version": "1.0.0",
  "dependencies": {
    "vue": "3.0.0"
  }
}"#;
                std::fs::write(path.join("package.json"), package_json)
                    .map_err(|e| GitServiceError::FileReadFailed(e.to_string()))?;
            }
            FrameworkType::Angular => {
                let package_json = r#"{
  "name": "mock-angular-app",
  "version": "1.0.0",
  "dependencies": {
    "@angular/core": "15.0.0",
    "@angular/common": "15.0.0"
  }
}"#;
                std::fs::write(path.join("package.json"), package_json)
                    .map_err(|e| GitServiceError::FileReadFailed(e.to_string()))?;
            }
            FrameworkType::Node => {
                let package_json = r#"{
  "name": "mock-node-app",
  "version": "1.0.0",
  "dependencies": {
    "express": "4.18.0"
  },
  "scripts": {
    "start": "node index.js"
  }
}"#;
                std::fs::write(path.join("package.json"), package_json)
                    .map_err(|e| GitServiceError::FileReadFailed(e.to_string()))?;
                
                // Create index.js
                std::fs::write(path.join("index.js"), "console.log('Mock Node.js app');")
                    .map_err(|e| GitServiceError::FileReadFailed(e.to_string()))?;
            }
            FrameworkType::Python => {
                let requirements = "flask==2.0.0\nrequests==2.28.0";
                std::fs::write(path.join("requirements.txt"), requirements)
                    .map_err(|e| GitServiceError::FileReadFailed(e.to_string()))?;
                
                // Create main.py
                std::fs::write(path.join("main.py"), "print('Mock Python app')")
                    .map_err(|e| GitServiceError::FileReadFailed(e.to_string()))?;
            }
            FrameworkType::Go => {
                let go_mod = "module example.com/mock-app\n\ngo 1.20\n";
                std::fs::write(path.join("go.mod"), go_mod)
                    .map_err(|e| GitServiceError::FileReadFailed(e.to_string()))?;
            }
            FrameworkType::Rust => {
                let cargo_toml = r#"[package]
name = "mock-rust-app"
version = "0.1.0"
edition = "2021"

[dependencies]
"#;
                std::fs::write(path.join("Cargo.toml"), cargo_toml)
                    .map_err(|e| GitServiceError::FileReadFailed(e.to_string()))?;
            }
            _ => {
                // Create a generic README
                std::fs::write(path.join("README.md"), "# Mock Application")
                    .map_err(|e| GitServiceError::FileReadFailed(e.to_string()))?;
            }
        }
        
        Ok(framework)
    }
    
    /// Determine framework from repository URL
    fn determine_framework_from_url(&self, repo_url: &str) -> FrameworkType {
        let url_lower = repo_url.to_lowercase();
        
        if url_lower.contains("next") {
            FrameworkType::NextJs
        } else if url_lower.contains("react") {
            FrameworkType::React
        } else if url_lower.contains("vue") {
            FrameworkType::Vue
        } else if url_lower.contains("angular") {
            FrameworkType::Angular
        } else if url_lower.contains("python") || url_lower.contains("flask") || url_lower.contains("django") {
            FrameworkType::Python
        } else if url_lower.contains("node") || url_lower.contains("express") {
            FrameworkType::Node
        } else if url_lower.contains("go") || url_lower.contains("golang") {
            FrameworkType::Go
        } else if url_lower.contains("rust") {
            FrameworkType::Rust
        } else {
            // Default to Node.js for generic repos
            FrameworkType::Node
        }
    }
    
    /// Generate mock commit SHA from repo URL
    fn generate_commit_sha(&self, repo_url: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        repo_url.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

#[async_trait]
impl GitOperations for MockGitService {
    async fn clone_repository(&self, repo_url: &str, branch: &str) -> Result<PathBuf, GitServiceError> {
        self.simulate_delay(1000).await;
        self.check_failure("clone_repository")?;
        
        // Create temp directory
        let temp_dir = std::env::temp_dir()
            .join("deployotron_shadow")
            .join(format!("repo_{}", uuid::Uuid::new_v4()));
        
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| GitServiceError::TempDirFailed(e.to_string()))?;
        
        // Create mock project files
        self.create_mock_project(&temp_dir, repo_url)?;
        
        // Track in state
        self.state.add_cloned_repo(
            repo_url.to_string(),
            temp_dir.to_string_lossy().to_string()
        );
        
        Ok(temp_dir)
    }
    
    async fn detect_framework(&self, repo_path: &Path) -> Result<FrameworkType, GitServiceError> {
        self.simulate_delay(100).await;
        self.check_failure("detect_framework")?;
        
        // Check for package.json (Node.js ecosystem)
        if let Ok(content) = std::fs::read_to_string(repo_path.join("package.json")) {
            return self.detect_js_framework(&content);
        }
        
        // Check for Python
        if repo_path.join("requirements.txt").exists() 
            || repo_path.join("setup.py").exists()
            || repo_path.join("pyproject.toml").exists() {
            return Ok(FrameworkType::Python);
        }
        
        // Check for Ruby
        if repo_path.join("Gemfile").exists() {
            return Ok(FrameworkType::Ruby);
        }
        
        // Check for Go
        if repo_path.join("go.mod").exists() {
            return Ok(FrameworkType::Go);
        }
        
        // Check for Rust
        if repo_path.join("Cargo.toml").exists() {
            return Ok(FrameworkType::Rust);
        }
        
        Ok(FrameworkType::Other)
    }
    
    async fn get_commit_info(
        &self,
        repo_path: &Path,
        commit_sha: Option<&str>
    ) -> Result<CommitInfo, GitServiceError> {
        self.simulate_delay(200).await;
        self.check_failure("get_commit_info")?;
        
        // Generate consistent mock commit info
        let sha = commit_sha
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                // Generate from repo path for consistency
                self.generate_commit_sha(&repo_path.to_string_lossy())
            });
        
        Ok(CommitInfo {
            sha: sha[..16].to_string(), // Use first 16 chars like real git
            message: "Mock commit: Initial implementation".to_string(),
            author: "Mock Developer".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        })
    }
    
    async fn get_latest_commit_sha(&self, repo_path: &Path) -> Result<String, GitServiceError> {
        self.simulate_delay(100).await;
        self.check_failure("get_latest_commit_sha")?;
        
        let sha = self.generate_commit_sha(&repo_path.to_string_lossy());
        Ok(sha[..16].to_string())
    }
    
    async fn cleanup_repository(&self, repo_path: &Path) -> Result<(), GitServiceError> {
        self.simulate_delay(100).await;
        
        // Actually remove the temp directory
        if repo_path.exists() {
            std::fs::remove_dir_all(repo_path)
                .map_err(|e| GitServiceError::FileReadFailed(e.to_string()))?;
        }
        
        Ok(())
    }
}

impl MockGitService {
    /// Detect JavaScript framework from package.json content
    fn detect_js_framework(&self, package_json: &str) -> Result<FrameworkType, GitServiceError> {
        let parsed: serde_json::Value = serde_json::from_str(package_json)
            .map_err(|_| GitServiceError::FrameworkDetectionFailed)?;
        
        let deps = parsed.get("dependencies")
            .and_then(|v| v.as_object())
            .map(|o| o.keys().map(|s| s.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();
        
        let dev_deps = parsed.get("devDependencies")
            .and_then(|v| v.as_object())
            .map(|o| o.keys().map(|s| s.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();
        
        let all_deps: Vec<&str> = deps.iter().chain(dev_deps.iter()).copied().collect();
        
        if all_deps.contains(&"next") {
            Ok(FrameworkType::NextJs)
        } else if all_deps.contains(&"react") {
            Ok(FrameworkType::React)
        } else if all_deps.contains(&"vue") {
            Ok(FrameworkType::Vue)
        } else if all_deps.contains(&"@angular/core") {
            Ok(FrameworkType::Angular)
        } else {
            Ok(FrameworkType::Node)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shadow::ShadowState;
    
    fn create_test_service() -> MockGitService {
        let config = ShadowConfig {
            enabled: true,
            failure_rate: 0.0,
            simulate_delays: false,
        };
        let state = Arc::new(ShadowState::new());
        
        MockGitService::new(config, state)
    }
    
    #[tokio::test]
    async fn test_clone_repository() {
        let service = create_test_service();
        
        let repo_path = service.clone_repository(
            "https://github.com/test/nextjs-app",
            "main"
        ).await.unwrap();
        
        assert!(repo_path.exists());
        assert!(repo_path.join("package.json").exists());
        
        // Cleanup
        service.cleanup_repository(&repo_path).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_detect_framework_nextjs() {
        let service = create_test_service();
        
        let repo_path = service.clone_repository(
            "https://github.com/test/nextjs-app",
            "main"
        ).await.unwrap();
        
        let framework = service.detect_framework(&repo_path).await.unwrap();
        assert_eq!(framework, FrameworkType::NextJs);
        
        // Cleanup
        service.cleanup_repository(&repo_path).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_detect_framework_python() {
        let service = create_test_service();
        
        let repo_path = service.clone_repository(
            "https://github.com/test/python-app",
            "main"
        ).await.unwrap();
        
        let framework = service.detect_framework(&repo_path).await.unwrap();
        assert_eq!(framework, FrameworkType::Python);
        
        // Cleanup
        service.cleanup_repository(&repo_path).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_get_commit_info() {
        let service = create_test_service();
        
        let repo_path = service.clone_repository(
            "https://github.com/test/app",
            "main"
        ).await.unwrap();
        
        let commit_info = service.get_commit_info(&repo_path, None).await.unwrap();
        
        assert!(!commit_info.sha.is_empty());
        assert!(!commit_info.message.is_empty());
        assert_eq!(commit_info.author, "Mock Developer");
        
        // Cleanup
        service.cleanup_repository(&repo_path).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_get_latest_commit_sha() {
        let service = create_test_service();
        
        let repo_path = service.clone_repository(
            "https://github.com/test/app",
            "main"
        ).await.unwrap();
        
        let sha = service.get_latest_commit_sha(&repo_path).await.unwrap();
        assert!(!sha.is_empty());
        
        // Cleanup
        service.cleanup_repository(&repo_path).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_repository_tracking() {
        let service = create_test_service();
        
        let repo_url = "https://github.com/test/app";
        let repo_path = service.clone_repository(repo_url, "main").await.unwrap();
        
        // Verify it's tracked in state
        assert!(service.state.get_cloned_repo(repo_url).is_some());
        
        // Cleanup
        service.cleanup_repository(&repo_path).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_detect_framework_react() {
        let service = create_test_service();
        
        let repo_path = service.clone_repository(
            "https://github.com/test/react-app",
            "main"
        ).await.unwrap();
        
        let framework = service.detect_framework(&repo_path).await.unwrap();
        assert_eq!(framework, FrameworkType::React);
        
        service.cleanup_repository(&repo_path).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_detect_framework_vue() {
        let service = create_test_service();
        
        let repo_path = service.clone_repository(
            "https://github.com/test/vue-app",
            "main"
        ).await.unwrap();
        
        let framework = service.detect_framework(&repo_path).await.unwrap();
        assert_eq!(framework, FrameworkType::Vue);
        
        service.cleanup_repository(&repo_path).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_detect_framework_angular() {
        let service = create_test_service();
        
        let repo_path = service.clone_repository(
            "https://github.com/test/angular-app",
            "main"
        ).await.unwrap();
        
        let framework = service.detect_framework(&repo_path).await.unwrap();
        assert_eq!(framework, FrameworkType::Angular);
        
        service.cleanup_repository(&repo_path).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_detect_framework_go() {
        let service = create_test_service();
        
        let repo_path = service.clone_repository(
            "https://github.com/test/go-app",
            "main"
        ).await.unwrap();
        
        let framework = service.detect_framework(&repo_path).await.unwrap();
        assert_eq!(framework, FrameworkType::Go);
        
        service.cleanup_repository(&repo_path).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_detect_framework_rust() {
        let service = create_test_service();
        
        let repo_path = service.clone_repository(
            "https://github.com/test/rust-app",
            "main"
        ).await.unwrap();
        
        let framework = service.detect_framework(&repo_path).await.unwrap();
        assert_eq!(framework, FrameworkType::Rust);
        
        service.cleanup_repository(&repo_path).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_detect_framework_node() {
        let service = create_test_service();
        
        let repo_path = service.clone_repository(
            "https://github.com/test/node-app",
            "main"
        ).await.unwrap();
        
        let framework = service.detect_framework(&repo_path).await.unwrap();
        assert_eq!(framework, FrameworkType::Node);
        
        // Verify index.js was created
        assert!(repo_path.join("index.js").exists());
        
        service.cleanup_repository(&repo_path).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_commit_sha_consistency() {
        let service = create_test_service();
        
        let repo_path = service.clone_repository(
            "https://github.com/test/app",
            "main"
        ).await.unwrap();
        
        // Get commit SHA multiple times
        let sha1 = service.get_latest_commit_sha(&repo_path).await.unwrap();
        let sha2 = service.get_latest_commit_sha(&repo_path).await.unwrap();
        
        // Should be consistent
        assert_eq!(sha1, sha2);
        
        service.cleanup_repository(&repo_path).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_commit_info_fields() {
        let service = create_test_service();
        
        let repo_path = service.clone_repository(
            "https://github.com/test/app",
            "main"
        ).await.unwrap();
        
        let commit_info = service.get_commit_info(&repo_path, None).await.unwrap();
        
        // Verify all fields are populated
        assert!(!commit_info.sha.is_empty());
        assert_eq!(commit_info.sha.len(), 16); // Should be 16 chars
        assert!(!commit_info.message.is_empty());
        assert_eq!(commit_info.author, "Mock Developer");
        assert!(commit_info.timestamp > 0);
        
        service.cleanup_repository(&repo_path).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_commit_info_with_custom_sha() {
        let service = create_test_service();
        
        let repo_path = service.clone_repository(
            "https://github.com/test/app",
            "main"
        ).await.unwrap();
        
        let custom_sha = "abc123def456789012345678";
        let commit_info = service.get_commit_info(&repo_path, Some(custom_sha)).await.unwrap();
        
        // Should use provided SHA
        assert!(commit_info.sha.starts_with("abc123def456"));
        
        service.cleanup_repository(&repo_path).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_cleanup_removes_directory() {
        let service = create_test_service();
        
        let repo_path = service.clone_repository(
            "https://github.com/test/app",
            "main"
        ).await.unwrap();
        
        assert!(repo_path.exists());
        
        service.cleanup_repository(&repo_path).await.unwrap();
        
        assert!(!repo_path.exists());
    }
    
    #[tokio::test]
    async fn test_cleanup_nonexistent_directory() {
        let service = create_test_service();
        
        let fake_path = std::env::temp_dir().join("nonexistent_repo_12345");
        
        // Should not error on nonexistent directory
        let result = service.cleanup_repository(&fake_path).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_failure_injection_clone() {
        let config = ShadowConfig {
            enabled: true,
            failure_rate: 1.0, // Always fail
            simulate_delays: false,
        };
        let state = Arc::new(ShadowState::new());
        let service = MockGitService::new(config, state);
        
        let result = service.clone_repository("https://github.com/test/app", "main").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Simulated failure"));
    }
    
    #[tokio::test]
    async fn test_failure_injection_detect_framework() {
        let config = ShadowConfig {
            enabled: true,
            failure_rate: 1.0,
            simulate_delays: false,
        };
        let state = Arc::new(ShadowState::new());
        let service = MockGitService::new(config, state);
        
        let fake_path = std::env::temp_dir().join("test_repo");
        let result = service.detect_framework(&fake_path).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_url_framework_detection_nextjs() {
        let service = create_test_service();
        let framework = service.determine_framework_from_url("https://github.com/test/my-nextjs-app");
        assert_eq!(framework, FrameworkType::NextJs);
    }
    
    #[tokio::test]
    async fn test_url_framework_detection_flask() {
        let service = create_test_service();
        let framework = service.determine_framework_from_url("https://github.com/test/flask-api");
        assert_eq!(framework, FrameworkType::Python);
    }
    
    #[tokio::test]
    async fn test_url_framework_detection_golang() {
        let service = create_test_service();
        let framework = service.determine_framework_from_url("https://github.com/test/golang-service");
        assert_eq!(framework, FrameworkType::Go);
    }
    
    #[tokio::test]
    async fn test_url_framework_detection_default() {
        let service = create_test_service();
        let framework = service.determine_framework_from_url("https://github.com/test/generic-app");
        assert_eq!(framework, FrameworkType::Node); // Default
    }
    
    #[tokio::test]
    async fn test_multiple_repos_different_frameworks() {
        let service = create_test_service();
        
        let repos = vec![
            ("https://github.com/test/nextjs-app", FrameworkType::NextJs),
            ("https://github.com/test/react-app", FrameworkType::React),
            ("https://github.com/test/python-app", FrameworkType::Python),
        ];
        
        for (url, expected_framework) in repos {
            let repo_path = service.clone_repository(url, "main").await.unwrap();
            let framework = service.detect_framework(&repo_path).await.unwrap();
            assert_eq!(framework, expected_framework);
            service.cleanup_repository(&repo_path).await.unwrap();
        }
    }
    
    #[tokio::test]
    async fn test_mock_project_files_nextjs() {
        let service = create_test_service();
        
        let repo_path = service.clone_repository(
            "https://github.com/test/nextjs-app",
            "main"
        ).await.unwrap();
        
        // Verify package.json exists and has correct content
        let package_json = std::fs::read_to_string(repo_path.join("package.json")).unwrap();
        assert!(package_json.contains("next"));
        assert!(package_json.contains("react"));
        
        service.cleanup_repository(&repo_path).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_mock_project_files_python() {
        let service = create_test_service();
        
        let repo_path = service.clone_repository(
            "https://github.com/test/python-app",
            "main"
        ).await.unwrap();
        
        // Verify requirements.txt and main.py exist
        assert!(repo_path.join("requirements.txt").exists());
        assert!(repo_path.join("main.py").exists());
        
        let requirements = std::fs::read_to_string(repo_path.join("requirements.txt")).unwrap();
        assert!(requirements.contains("flask"));
        
        service.cleanup_repository(&repo_path).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_sha_generation_deterministic() {
        let service = create_test_service();
        
        // Same URL should produce same SHA
        let sha1 = service.generate_commit_sha("https://github.com/test/app");
        let sha2 = service.generate_commit_sha("https://github.com/test/app");
        
        assert_eq!(sha1, sha2);
    }
    
    #[tokio::test]
    async fn test_sha_generation_different_urls() {
        let service = create_test_service();
        
        let sha1 = service.generate_commit_sha("https://github.com/test/app1");
        let sha2 = service.generate_commit_sha("https://github.com/test/app2");
        
        // Different URLs should produce different SHAs
        assert_ne!(sha1, sha2);
    }
}
