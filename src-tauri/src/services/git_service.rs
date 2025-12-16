//! Git operations service
//!
//! Provides functionality for:
//! - Cloning repositories to temporary directories
//! - Detecting framework types from project files
//! - Retrieving commit information

use crate::models::FrameworkType;
use crate::services::GitOperations;
use crate::services::git_trait::CommitInfo;
use git2::{Repository, Oid, Commit};
use std::path::{Path, PathBuf};
use thiserror::Error;
use async_trait::async_trait;
use std::fs;

/// Git service specific errors
#[derive(Error, Debug)]
pub enum GitServiceError {
    #[error("Failed to clone repository: {0}")]
    CloneFailed(String),
    
    #[error("Failed to open repository: {0}")]
    OpenFailed(String),
    
    #[error("Commit not found: {0}")]
    CommitNotFound(String),
    
    #[error("Failed to read file: {0}")]
    FileReadFailed(String),
    
    #[error("Failed to detect framework")]
    FrameworkDetectionFailed,
    
    #[error("Failed to create temporary directory: {0}")]
    TempDirFailed(String),
}

impl From<git2::Error> for GitServiceError {
    fn from(err: git2::Error) -> Self {
        GitServiceError::OpenFailed(err.to_string())
    }
}

impl From<std::io::Error> for GitServiceError {
    fn from(err: std::io::Error) -> Self {
        GitServiceError::FileReadFailed(err.to_string())
    }
}

/// Git service for repository operations
pub struct GitService;

impl GitService {
    /// Create a new GitService instance
    pub fn new() -> Self {
        GitService
    }
    
    /// Clone a repository to a temporary directory
    ///
    /// Returns the path to the cloned repository
    pub async fn clone_repository(&self, repo_url: &str, branch: &str) -> Result<PathBuf, GitServiceError> {
        // Create temporary directory for clone
        let temp_dir = std::env::temp_dir()
            .join("deployotron")
            .join(format!("repo_{}", uuid::Uuid::new_v4()));
        
        fs::create_dir_all(&temp_dir)
            .map_err(|e| GitServiceError::TempDirFailed(e.to_string()))?;
        
        // Clone repository using tokio::task::spawn_blocking for CPU-bound work
        let repo_url = repo_url.to_string();
        let branch = branch.to_string();
        let clone_path = temp_dir.clone();
        
        tokio::task::spawn_blocking(move || {
            // Build clone with branch checkout
            let mut builder = git2::build::RepoBuilder::new();
            builder.branch(&branch);
            
            builder.clone(&repo_url, &clone_path)
                .map_err(|e| GitServiceError::CloneFailed(e.to_string()))?;
            
            Ok::<PathBuf, GitServiceError>(clone_path)
        })
        .await
        .map_err(|e| GitServiceError::CloneFailed(e.to_string()))?
    }
    
    /// Detect the framework type from project files
    pub async fn detect_framework(&self, repo_path: &Path) -> Result<FrameworkType, GitServiceError> {
        let repo_path = repo_path.to_path_buf();
        
        tokio::task::spawn_blocking(move || {
            // Check for package.json (Node.js ecosystem)
            if let Ok(content) = fs::read_to_string(repo_path.join("package.json")) {
                return Self::detect_js_framework(&content);
            }
            
            // Check for requirements.txt or setup.py (Python)
            if repo_path.join("requirements.txt").exists() 
                || repo_path.join("setup.py").exists() 
                || repo_path.join("pyproject.toml").exists() {
                return Ok(FrameworkType::Python);
            }
            
            // Check for Gemfile (Ruby)
            if repo_path.join("Gemfile").exists() {
                return Ok(FrameworkType::Ruby);
            }
            
            // Check for go.mod (Go)
            if repo_path.join("go.mod").exists() {
                return Ok(FrameworkType::Go);
            }
            
            // Check for Cargo.toml (Rust)
            if repo_path.join("Cargo.toml").exists() {
                return Ok(FrameworkType::Rust);
            }
            
            // Default to Other if cannot detect
            Ok(FrameworkType::Other)
        })
        .await
        .map_err(|e| GitServiceError::FrameworkDetectionFailed)?
    }
    
    /// Detect specific JavaScript framework from package.json content
    fn detect_js_framework(package_json: &str) -> Result<FrameworkType, GitServiceError> {
        // Parse package.json to detect framework
        let parsed: serde_json::Value = serde_json::from_str(package_json)
            .map_err(|_| GitServiceError::FrameworkDetectionFailed)?;
        
        // Check dependencies and devDependencies
        let deps = parsed.get("dependencies")
            .and_then(|v| v.as_object())
            .map(|o| o.keys().map(|s| s.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();
        
        let dev_deps = parsed.get("devDependencies")
            .and_then(|v| v.as_object())
            .map(|o| o.keys().map(|s| s.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();
        
        let all_deps: Vec<&str> = deps.iter().chain(dev_deps.iter()).copied().collect();
        
        // Check for Next.js
        if all_deps.contains(&"next") {
            return Ok(FrameworkType::NextJs);
        }
        
        // Check for React
        if all_deps.contains(&"react") {
            return Ok(FrameworkType::React);
        }
        
        // Check for Vue
        if all_deps.contains(&"vue") {
            return Ok(FrameworkType::Vue);
        }
        
        // Check for Angular
        if all_deps.contains(&"@angular/core") {
            return Ok(FrameworkType::Angular);
        }
        
        // Default to Node if it's a Node project
        Ok(FrameworkType::Node)
    }
    
    /// Get commit information from repository
    pub async fn get_commit_info(&self, repo_path: &Path, commit_sha: Option<&str>) -> Result<CommitInfo, GitServiceError> {
        let repo_path = repo_path.to_path_buf();
        let commit_sha = commit_sha.map(|s| s.to_string());
        
        tokio::task::spawn_blocking(move || {
            let repo = Repository::open(&repo_path)?;
            
            let commit = if let Some(sha) = commit_sha {
                // Get specific commit by SHA
                let oid = Oid::from_str(&sha)
                    .map_err(|e| GitServiceError::CommitNotFound(e.to_string()))?;
                repo.find_commit(oid)
                    .map_err(|e| GitServiceError::CommitNotFound(e.to_string()))?
            } else {
                // Get HEAD commit
                let head = repo.head()?;
                head.peel_to_commit()?
            };
            
            Ok(Self::commit_to_info(&commit))
        })
        .await
        .map_err(|e| GitServiceError::CommitNotFound(e.to_string()))?
    }
    
    /// Get the latest commit SHA from repository
    pub async fn get_latest_commit_sha(&self, repo_path: &Path) -> Result<String, GitServiceError> {
        let repo_path = repo_path.to_path_buf();
        
        tokio::task::spawn_blocking(move || {
            let repo = Repository::open(&repo_path)?;
            let head = repo.head()?;
            let commit = head.peel_to_commit()?;
            Ok(commit.id().to_string())
        })
        .await
        .map_err(|e| GitServiceError::CommitNotFound(e.to_string()))?
    }
    
    /// Convert git2::Commit to CommitInfo
    fn commit_to_info(commit: &Commit) -> CommitInfo {
        CommitInfo {
            sha: commit.id().to_string(),
            message: commit.message().unwrap_or("").to_string(),
            author: commit.author().name().unwrap_or("Unknown").to_string(),
            timestamp: commit.time().seconds(),
        }
    }
    
    /// Clean up cloned repository directory
    pub async fn cleanup_repository(&self, repo_path: &Path) -> Result<(), GitServiceError> {
        let repo_path = repo_path.to_path_buf();
        
        tokio::task::spawn_blocking(move || {
            if repo_path.exists() {
                fs::remove_dir_all(&repo_path)
                    .map_err(|e| GitServiceError::FileReadFailed(e.to_string()))?;
            }
            Ok(())
        })
        .await
        .map_err(|e| GitServiceError::FileReadFailed(e.to_string()))?
    }
}

impl Default for GitService {
    fn default() -> Self {
        Self::new()
    }
}

// Implement GitOperations trait for GitService
#[async_trait]
impl GitOperations for GitService {
    async fn clone_repository(&self, repo_url: &str, branch: &str) -> Result<PathBuf, GitServiceError> {
        self.clone_repository(repo_url, branch).await
    }
    
    async fn detect_framework(&self, repo_path: &Path) -> Result<FrameworkType, GitServiceError> {
        self.detect_framework(repo_path).await
    }
    
    async fn get_commit_info(
        &self,
        repo_path: &Path,
        commit_sha: Option<&str>
    ) -> Result<CommitInfo, GitServiceError> {
        self.get_commit_info(repo_path, commit_sha).await
    }
    
    async fn get_latest_commit_sha(&self, repo_path: &Path) -> Result<String, GitServiceError> {
        self.get_latest_commit_sha(repo_path).await
    }
    
    async fn cleanup_repository(&self, repo_path: &Path) -> Result<(), GitServiceError> {
        self.cleanup_repository(repo_path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detect_nextjs_framework() {
        let package_json = r#"{
            "dependencies": {
                "next": "13.0.0",
                "react": "18.0.0"
            }
        }"#;
        
        let framework = GitService::detect_js_framework(package_json).unwrap();
        assert_eq!(framework, FrameworkType::NextJs);
    }
    
    #[test]
    fn test_detect_react_framework() {
        let package_json = r#"{
            "dependencies": {
                "react": "18.0.0",
                "react-dom": "18.0.0"
            }
        }"#;
        
        let framework = GitService::detect_js_framework(package_json).unwrap();
        assert_eq!(framework, FrameworkType::React);
    }
    
    #[test]
    fn test_detect_vue_framework() {
        let package_json = r#"{
            "dependencies": {
                "vue": "3.0.0"
            }
        }"#;
        
        let framework = GitService::detect_js_framework(package_json).unwrap();
        assert_eq!(framework, FrameworkType::Vue);
    }
    
    #[test]
    fn test_detect_angular_framework() {
        let package_json = r#"{
            "dependencies": {
                "@angular/core": "15.0.0"
            }
        }"#;
        
        let framework = GitService::detect_js_framework(package_json).unwrap();
        assert_eq!(framework, FrameworkType::Angular);
    }
    
    #[test]
    fn test_detect_node_framework_fallback() {
        let package_json = r#"{
            "dependencies": {
                "express": "4.18.0"
            }
        }"#;
        
        let framework = GitService::detect_js_framework(package_json).unwrap();
        assert_eq!(framework, FrameworkType::Node);
    }
}
