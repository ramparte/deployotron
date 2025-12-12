use serde::{Deserialize, Serialize};

/// Represents a deployment target environment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Development,
    Staging,
    Production,
}

/// Current status of a deployment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentStatus {
    Pending,
    InProgress,
    Success,
    Failed,
    RolledBack,
}

/// Supported application framework types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FrameworkType {
    NextJs,
    React,
    Vue,
    Angular,
    Node,
    Python,
    Ruby,
    Go,
    Rust,
    Other,
}

/// AWS credentials for deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub region: String,
}

/// Git repository credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCredentials {
    pub username: String,
    pub token: String,
    pub provider: String, // e.g., "github", "gitlab", "bitbucket"
}

/// A deployment project configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Unique project identifier (UUID v4)
    pub id: String,
    
    /// Human-readable project name
    pub name: String,
    
    /// Git repository URL
    pub repository_url: String,
    
    /// Branch to deploy from
    pub branch: String,
    
    /// Framework type of the project
    pub framework: FrameworkType,
    
    /// Target deployment environment
    pub environment: Environment,
    
    /// AWS ECS cluster name
    pub aws_cluster: String,
    
    /// AWS ECS service name
    pub aws_service: String,
    
    /// ECR repository URI
    pub ecr_repository: String,
    
    /// Unix timestamp of creation (seconds since epoch)
    pub created_at: i64,
    
    /// Unix timestamp of last update (seconds since epoch)
    pub updated_at: i64,
}

/// A deployment record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    /// Unique deployment identifier (UUID v4)
    pub id: String,
    
    /// Associated project ID
    pub project_id: String,
    
    /// Current deployment status
    pub status: DeploymentStatus,
    
    /// Git commit SHA being deployed
    pub commit_sha: String,
    
    /// Optional commit message
    pub commit_message: Option<String>,
    
    /// Docker image tag used for deployment
    pub image_tag: String,
    
    /// Unix timestamp when deployment started (seconds since epoch)
    pub started_at: i64,
    
    /// Unix timestamp when deployment completed (seconds since epoch), None if in progress
    pub completed_at: Option<i64>,
    
    /// Error message if deployment failed
    pub error_message: Option<String>,
    
    /// JSON string containing deployment logs
    pub logs: Option<String>,
}

impl Project {
    /// Create a new project with generated ID and timestamps
    pub fn new(
        name: String,
        repository_url: String,
        branch: String,
        framework: FrameworkType,
        environment: Environment,
        aws_cluster: String,
        aws_service: String,
        ecr_repository: String,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            repository_url,
            branch,
            framework,
            environment,
            aws_cluster,
            aws_service,
            ecr_repository,
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Update the updated_at timestamp
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().timestamp();
    }
}

impl Deployment {
    /// Create a new deployment with generated ID and timestamp
    pub fn new(
        project_id: String,
        commit_sha: String,
        commit_message: Option<String>,
        image_tag: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            project_id,
            status: DeploymentStatus::Pending,
            commit_sha,
            commit_message,
            image_tag,
            started_at: chrono::Utc::now().timestamp(),
            completed_at: None,
            error_message: None,
            logs: None,
        }
    }
    
    /// Mark deployment as completed with given status
    pub fn complete(&mut self, status: DeploymentStatus, error_message: Option<String>) {
        self.status = status;
        self.completed_at = Some(chrono::Utc::now().timestamp());
        self.error_message = error_message;
    }
    
    /// Append logs to existing logs
    pub fn append_logs(&mut self, new_logs: &str) {
        match &mut self.logs {
            Some(logs) => logs.push_str(new_logs),
            None => self.logs = Some(new_logs.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_creation() {
        let project = Project::new(
            "Test Project".to_string(),
            "https://github.com/user/repo".to_string(),
            "main".to_string(),
            FrameworkType::NextJs,
            Environment::Development,
            "my-cluster".to_string(),
            "my-service".to_string(),
            "123456.dkr.ecr.us-east-1.amazonaws.com/my-repo".to_string(),
        );
        
        assert!(!project.id.is_empty());
        assert_eq!(project.name, "Test Project");
        assert_eq!(project.framework, FrameworkType::NextJs);
        assert!(project.created_at > 0);
        assert_eq!(project.created_at, project.updated_at);
    }

    #[test]
    fn test_deployment_creation() {
        let deployment = Deployment::new(
            "project-123".to_string(),
            "abc123def456".to_string(),
            Some("Initial commit".to_string()),
            "v1.0.0".to_string(),
        );
        
        assert!(!deployment.id.is_empty());
        assert_eq!(deployment.status, DeploymentStatus::Pending);
        assert!(deployment.completed_at.is_none());
    }

    #[test]
    fn test_deployment_completion() {
        let mut deployment = Deployment::new(
            "project-123".to_string(),
            "abc123".to_string(),
            None,
            "v1.0.0".to_string(),
        );
        
        deployment.complete(DeploymentStatus::Success, None);
        assert_eq!(deployment.status, DeploymentStatus::Success);
        assert!(deployment.completed_at.is_some());
    }

    #[test]
    fn test_deployment_logs() {
        let mut deployment = Deployment::new(
            "project-123".to_string(),
            "abc123".to_string(),
            None,
            "v1.0.0".to_string(),
        );
        
        deployment.append_logs("Line 1\n");
        deployment.append_logs("Line 2\n");
        
        assert_eq!(deployment.logs, Some("Line 1\nLine 2\n".to_string()));
    }
}
