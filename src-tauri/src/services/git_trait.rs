//! Git operations trait
//!
//! Defines the trait for Git operations that can be implemented by both
//! real git2 library service and mock service for testing.

use async_trait::async_trait;
use crate::services::GitServiceError;
use crate::models::FrameworkType;
use std::path::{Path, PathBuf};

/// Git commit information
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub sha: String,
    pub message: String,
    pub author: String,
    pub timestamp: i64,
}

/// Trait for Git operations
///
/// This trait allows swapping between real git2 operations and mock
/// implementations for testing without requiring Git repositories.
#[async_trait]
pub trait GitOperations: Send + Sync {
    /// Clone a repository to a temporary directory
    ///
    /// # Arguments
    /// * `repo_url` - Git repository URL
    /// * `branch` - Branch name to checkout
    ///
    /// # Returns
    /// Path to cloned repository
    async fn clone_repository(&self, repo_url: &str, branch: &str) -> Result<PathBuf, GitServiceError>;
    
    /// Detect the framework type from project files
    ///
    /// # Arguments
    /// * `repo_path` - Path to repository directory
    ///
    /// # Returns
    /// Detected framework type
    async fn detect_framework(&self, repo_path: &Path) -> Result<FrameworkType, GitServiceError>;
    
    /// Get commit information from repository
    ///
    /// # Arguments
    /// * `repo_path` - Path to repository directory
    /// * `commit_sha` - Optional specific commit SHA (None = HEAD)
    ///
    /// # Returns
    /// Commit information (SHA, message, author, timestamp)
    async fn get_commit_info(
        &self,
        repo_path: &Path,
        commit_sha: Option<&str>
    ) -> Result<CommitInfo, GitServiceError>;
    
    /// Get the latest commit SHA from repository
    ///
    /// # Arguments
    /// * `repo_path` - Path to repository directory
    ///
    /// # Returns
    /// Latest commit SHA string
    async fn get_latest_commit_sha(&self, repo_path: &Path) -> Result<String, GitServiceError>;
    
    /// Clean up cloned repository directory
    ///
    /// # Arguments
    /// * `repo_path` - Path to repository directory to remove
    async fn cleanup_repository(&self, repo_path: &Path) -> Result<(), GitServiceError>;
}
