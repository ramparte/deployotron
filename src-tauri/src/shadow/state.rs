//! Shadow state management
//!
//! Tracks mock state for AWS resources, Docker images, and Git repositories.
//! All state is stored in-memory and can be reset for testing.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Thread-safe shadow state for mock operations
#[derive(Debug, Clone)]
pub struct ShadowState {
    inner: Arc<Mutex<StateInner>>,
}

/// Internal state storage
#[derive(Debug, Default)]
struct StateInner {
    /// ECR repositories: name -> URI
    ecr_repositories: HashMap<String, String>,
    
    /// Docker images: tag -> built status
    docker_images: HashMap<String, bool>,
    
    /// ECS task definitions: family -> ARN
    task_definitions: HashMap<String, String>,
    
    /// ECS services: "cluster:service" -> status
    services: HashMap<String, ServiceStatus>,
    
    /// Git repositories: URL -> cloned path
    cloned_repos: HashMap<String, String>,
    
    /// CloudWatch logs: "log_group:stream" -> messages
    logs: HashMap<String, Vec<String>>,
}

/// ECS service health status
#[derive(Debug, Clone)]
pub struct ServiceStatus {
    pub running_count: i32,
    pub desired_count: i32,
    pub pending_count: i32,
}

impl ShadowState {
    /// Create a new shadow state instance
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(StateInner::default())),
        }
    }
    
    // ===== ECR Operations =====
    
    /// Add ECR repository to state
    pub fn add_ecr_repository(&self, name: String, uri: String) {
        let mut inner = self.inner.lock().unwrap();
        inner.ecr_repositories.insert(name, uri);
    }
    
    /// Get ECR repository URI by name
    pub fn get_ecr_repository(&self, name: &str) -> Option<String> {
        let inner = self.inner.lock().unwrap();
        inner.ecr_repositories.get(name).cloned()
    }
    
    // ===== Docker Operations =====
    
    /// Mark Docker image as built
    pub fn add_docker_image(&self, tag: String) {
        let mut inner = self.inner.lock().unwrap();
        inner.docker_images.insert(tag, true);
    }
    
    /// Check if Docker image exists
    pub fn has_docker_image(&self, tag: &str) -> bool {
        let inner = self.inner.lock().unwrap();
        inner.docker_images.get(tag).copied().unwrap_or(false)
    }
    
    // ===== ECS Operations =====
    
    /// Add ECS task definition
    pub fn add_task_definition(&self, family: String, arn: String) {
        let mut inner = self.inner.lock().unwrap();
        inner.task_definitions.insert(family, arn);
    }
    
    /// Get ECS task definition ARN
    pub fn get_task_definition(&self, family: &str) -> Option<String> {
        let inner = self.inner.lock().unwrap();
        inner.task_definitions.get(family).cloned()
    }
    
    /// Set ECS service status
    pub fn set_service_status(&self, cluster: &str, service: &str, status: ServiceStatus) {
        let mut inner = self.inner.lock().unwrap();
        let key = format!("{}:{}", cluster, service);
        inner.services.insert(key, status);
    }
    
    /// Get ECS service status
    pub fn get_service_status(&self, cluster: &str, service: &str) -> Option<ServiceStatus> {
        let inner = self.inner.lock().unwrap();
        let key = format!("{}:{}", cluster, service);
        inner.services.get(&key).cloned()
    }
    
    // ===== Git Operations =====
    
    /// Record cloned repository
    pub fn add_cloned_repo(&self, url: String, path: String) {
        let mut inner = self.inner.lock().unwrap();
        inner.cloned_repos.insert(url, path);
    }
    
    /// Get cloned repository path
    pub fn get_cloned_repo(&self, url: &str) -> Option<String> {
        let inner = self.inner.lock().unwrap();
        inner.cloned_repos.get(url).cloned()
    }
    
    // ===== CloudWatch Operations =====
    
    /// Add log message
    pub fn add_log(&self, log_group: &str, stream: &str, message: String) {
        let mut inner = self.inner.lock().unwrap();
        let key = format!("{}:{}", log_group, stream);
        inner.logs.entry(key).or_insert_with(Vec::new).push(message);
    }
    
    /// Get log messages
    pub fn get_logs(&self, log_group: &str, stream: &str, limit: usize) -> Vec<String> {
        let inner = self.inner.lock().unwrap();
        let key = format!("{}:{}", log_group, stream);
        
        inner.logs
            .get(&key)
            .map(|logs| {
                logs.iter()
                    .rev()
                    .take(limit)
                    .rev()
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
    
    // ===== Testing Utilities =====
    
    /// Reset all state (useful for tests)
    pub fn reset(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.ecr_repositories.clear();
        inner.docker_images.clear();
        inner.task_definitions.clear();
        inner.services.clear();
        inner.cloned_repos.clear();
        inner.logs.clear();
    }
}

impl Default for ShadowState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ecr_repository() {
        let state = ShadowState::new();
        
        assert!(state.get_ecr_repository("test-repo").is_none());
        
        state.add_ecr_repository(
            "test-repo".to_string(),
            "123456.dkr.ecr.us-east-1.amazonaws.com/test-repo".to_string()
        );
        
        assert!(state.get_ecr_repository("test-repo").is_some());
        assert!(state.get_ecr_repository("test-repo").unwrap().contains("test-repo"));
    }
    
    #[test]
    fn test_docker_image() {
        let state = ShadowState::new();
        
        assert!(!state.has_docker_image("myapp:latest"));
        
        state.add_docker_image("myapp:latest".to_string());
        
        assert!(state.has_docker_image("myapp:latest"));
    }
    
    #[test]
    fn test_task_definition() {
        let state = ShadowState::new();
        
        assert!(state.get_task_definition("my-task").is_none());
        
        state.add_task_definition(
            "my-task".to_string(),
            "arn:aws:ecs:us-east-1:123456789012:task-definition/my-task:1".to_string()
        );
        
        assert!(state.get_task_definition("my-task").is_some());
    }
    
    #[test]
    fn test_service_status() {
        let state = ShadowState::new();
        
        assert!(state.get_service_status("my-cluster", "my-service").is_none());
        
        state.set_service_status(
            "my-cluster",
            "my-service",
            ServiceStatus {
                running_count: 1,
                desired_count: 1,
                pending_count: 0,
            }
        );
        
        let status = state.get_service_status("my-cluster", "my-service").unwrap();
        assert_eq!(status.running_count, 1);
        assert_eq!(status.desired_count, 1);
    }
    
    #[test]
    fn test_cloned_repo() {
        let state = ShadowState::new();
        
        assert!(state.get_cloned_repo("https://github.com/test/repo").is_none());
        
        state.add_cloned_repo(
            "https://github.com/test/repo".to_string(),
            "/tmp/repo_123".to_string()
        );
        
        assert!(state.get_cloned_repo("https://github.com/test/repo").is_some());
    }
    
    #[test]
    fn test_logs() {
        let state = ShadowState::new();
        
        assert_eq!(state.get_logs("/ecs/my-task", "stream-1", 10).len(), 0);
        
        state.add_log("/ecs/my-task", "stream-1", "Log line 1".to_string());
        state.add_log("/ecs/my-task", "stream-1", "Log line 2".to_string());
        state.add_log("/ecs/my-task", "stream-1", "Log line 3".to_string());
        
        let logs = state.get_logs("/ecs/my-task", "stream-1", 10);
        assert_eq!(logs.len(), 3);
        assert_eq!(logs[0], "Log line 1");
    }
    
    #[test]
    fn test_reset() {
        let state = ShadowState::new();
        
        state.add_ecr_repository("repo".to_string(), "uri".to_string());
        state.add_docker_image("image:tag".to_string());
        
        assert!(state.get_ecr_repository("repo").is_some());
        assert!(state.has_docker_image("image:tag"));
        
        state.reset();
        
        assert!(state.get_ecr_repository("repo").is_none());
        assert!(!state.has_docker_image("image:tag"));
    }
}
