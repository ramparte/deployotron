//! Mock AWS service for shadow world testing
//!
//! Provides mock implementations of all AWS operations without requiring
//! real AWS credentials, Docker, or infrastructure.

use async_trait::async_trait;
use crate::services::{AwsOperations, AwsServiceError, EcsDeploymentConfig, ServiceHealth};
use crate::models::FrameworkType;
use crate::shadow::{ShadowConfig, ShadowState};
use std::sync::Arc;
use tokio::time::Duration;

/// Mock AWS service for testing
pub struct MockAwsService {
    config: ShadowConfig,
    state: Arc<ShadowState>,
    region: String,
}

impl MockAwsService {
    /// Create a new mock AWS service
    ///
    /// # Arguments
    /// * `region` - AWS region (for generating realistic mock URIs)
    /// * `config` - Shadow configuration
    /// * `state` - Shared shadow state for tracking operations
    pub fn new(region: Option<String>, config: ShadowConfig, state: Arc<ShadowState>) -> Self {
        let region = region.unwrap_or_else(|| "us-east-1".to_string());
        Self {
            config,
            state,
            region,
        }
    }
    
    /// Simulate realistic delay for operation
    async fn simulate_delay(&self, millis: u64) {
        if self.config.simulate_delays {
            tokio::time::sleep(Duration::from_millis(millis)).await;
        }
    }
    
    /// Check if operation should fail based on config
    fn check_failure(&self, operation: &str) -> Result<(), AwsServiceError> {
        if self.config.should_fail() {
            Err(AwsServiceError::EcsOperationFailed(
                format!("Simulated failure: {}", operation)
            ))
        } else {
            Ok(())
        }
    }
}

#[async_trait]
impl AwsOperations for MockAwsService {
    async fn ensure_ecr_repository(&self, repository_name: &str) -> Result<String, AwsServiceError> {
        self.simulate_delay(100).await;
        self.check_failure("ensure_ecr_repository")?;
        
        // Check if repository already exists
        if let Some(uri) = self.state.get_ecr_repository(repository_name) {
            return Ok(uri);
        }
        
        // Create new mock repository URI
        let uri = format!(
            "123456789012.dkr.ecr.{}.amazonaws.com/{}",
            self.region,
            repository_name
        );
        
        self.state.add_ecr_repository(repository_name.to_string(), uri.clone());
        
        Ok(uri)
    }
    
    async fn docker_login_ecr(&self) -> Result<(), AwsServiceError> {
        self.simulate_delay(200).await;
        self.check_failure("docker_login_ecr")?;
        
        // Mock login always succeeds - no actual Docker operation
        Ok(())
    }
    
    async fn build_docker_image(
        &self,
        source_dir: &str,
        image_tag: &str,
        framework: &FrameworkType
    ) -> Result<(), AwsServiceError> {
        self.simulate_delay(2000).await; // Building takes longer
        self.check_failure("build_docker_image")?;
        
        // Generate mock Dockerfile if it doesn't exist
        let dockerfile_path = format!("{}/Dockerfile", source_dir);
        if !std::path::Path::new(&dockerfile_path).exists() {
            self.generate_mock_dockerfile(source_dir, framework)?;
        }
        
        // Track built image
        self.state.add_docker_image(image_tag.to_string());
        
        Ok(())
    }
    
    async fn push_docker_image(&self, local_tag: &str, ecr_uri: &str) -> Result<(), AwsServiceError> {
        self.simulate_delay(3000).await; // Pushing takes longer
        self.check_failure("push_docker_image")?;
        
        // Verify image was built
        if !self.state.has_docker_image(local_tag) {
            return Err(AwsServiceError::DockerOperationFailed(
                format!("Image not found: {}", local_tag)
            ));
        }
        
        // Track pushed image with ECR URI
        self.state.add_docker_image(ecr_uri.to_string());
        
        Ok(())
    }
    
    async fn register_task_definition(&self, config: &EcsDeploymentConfig) -> Result<String, AwsServiceError> {
        self.simulate_delay(500).await;
        self.check_failure("register_task_definition")?;
        
        // Generate mock task definition ARN
        let task_arn = format!(
            "arn:aws:ecs:{}:123456789012:task-definition/{}:1",
            self.region,
            config.task_family
        );
        
        self.state.add_task_definition(config.task_family.clone(), task_arn.clone());
        
        Ok(task_arn)
    }
    
    async fn deploy_service(
        &self,
        config: &EcsDeploymentConfig,
        task_definition_arn: &str
    ) -> Result<(), AwsServiceError> {
        self.simulate_delay(800).await;
        self.check_failure("deploy_service")?;
        
        // Set service to deploying state initially
        self.state.set_service_status(
            &config.cluster_name,
            &config.service_name,
            crate::shadow::ServiceStatus {
                running_count: 0,
                desired_count: config.desired_count,
                pending_count: config.desired_count,
            }
        );
        
        // Simulate gradual transition to running
        // In real scenario, get_service_health will be polled
        
        Ok(())
    }
    
    async fn get_service_health(
        &self,
        cluster_name: &str,
        service_name: &str
    ) -> Result<ServiceHealth, AwsServiceError> {
        self.simulate_delay(300).await;
        self.check_failure("get_service_health")?;
        
        // Get current status or create initial status
        let status = self.state.get_service_status(cluster_name, service_name)
            .unwrap_or(crate::shadow::ServiceStatus {
                running_count: 0,
                desired_count: 1,
                pending_count: 1,
            });
        
        // Simulate progression towards healthy state
        // If not yet running, gradually move pending to running
        let mut new_status = status.clone();
        
        if new_status.pending_count > 0 && new_status.running_count < new_status.desired_count {
            new_status.pending_count -= 1;
            new_status.running_count += 1;
            
            self.state.set_service_status(
                cluster_name,
                service_name,
                new_status.clone()
            );
        }
        
        Ok(ServiceHealth {
            running_count: new_status.running_count,
            desired_count: new_status.desired_count,
            pending_count: new_status.pending_count,
            is_healthy: new_status.running_count == new_status.desired_count 
                        && new_status.pending_count == 0,
        })
    }
    
    async fn fetch_logs(
        &self,
        log_group: &str,
        log_stream: &str,
        limit: i32
    ) -> Result<Vec<String>, AwsServiceError> {
        self.simulate_delay(400).await;
        self.check_failure("fetch_logs")?;
        
        // Return mock logs or empty if none exist
        let logs = self.state.get_logs(log_group, log_stream, limit as usize);
        
        // If no logs exist, add some mock logs
        if logs.is_empty() {
            let mock_logs = vec![
                format!("[{}] Container started", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")),
                format!("[{}] Application initializing...", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")),
                format!("[{}] Server listening on port 3000", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")),
            ];
            
            for log in &mock_logs {
                self.state.add_log(log_group, log_stream, log.clone());
            }
            
            return Ok(mock_logs);
        }
        
        Ok(logs)
    }
}

impl MockAwsService {
    /// Generate mock Dockerfile for testing
    fn generate_mock_dockerfile(&self, source_dir: &str, framework: &FrameworkType) -> Result<(), AwsServiceError> {
        let dockerfile_content = match framework {
            FrameworkType::NextJs => {
                r#"# Mock Dockerfile for Next.js
FROM node:18-alpine
WORKDIR /app
COPY package*.json ./
RUN npm ci --only=production
COPY . .
RUN npm run build
EXPOSE 3000
CMD ["npm", "start"]
"#
            }
            FrameworkType::React => {
                r#"# Mock Dockerfile for React
FROM node:18-alpine
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build
RUN npm install -g serve
EXPOSE 3000
CMD ["serve", "-s", "build", "-l", "3000"]
"#
            }
            FrameworkType::Node => {
                r#"# Mock Dockerfile for Node.js
FROM node:18-alpine
WORKDIR /app
COPY package*.json ./
RUN npm ci --only=production
COPY . .
EXPOSE 3000
CMD ["node", "index.js"]
"#
            }
            FrameworkType::Python => {
                r#"# Mock Dockerfile for Python
FROM python:3.11-slim
WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt
COPY . .
EXPOSE 8000
CMD ["python", "main.py"]
"#
            }
            _ => {
                r#"# Mock Dockerfile - Generic
FROM alpine:latest
WORKDIR /app
COPY . .
EXPOSE 8080
CMD ["sh", "-c", "echo 'Running mock application'"]
"#
            }
        };
        
        let dockerfile_path = format!("{}/Dockerfile", source_dir);
        std::fs::write(&dockerfile_path, dockerfile_content)
            .map_err(|e| AwsServiceError::DockerOperationFailed(
                format!("Failed to write mock Dockerfile: {}", e)
            ))?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shadow::ShadowState;
    
    fn create_test_service() -> MockAwsService {
        let config = ShadowConfig {
            enabled: true,
            failure_rate: 0.0,
            simulate_delays: false,
        };
        let state = Arc::new(ShadowState::new());
        
        MockAwsService::new(Some("us-east-1".into()), config, state)
    }
    
    #[tokio::test]
    async fn test_ensure_ecr_repository() {
        let service = create_test_service();
        
        let uri = service.ensure_ecr_repository("test-repo").await.unwrap();
        
        assert!(uri.contains("test-repo"));
        assert!(uri.contains("us-east-1"));
        
        // Second call should return same URI
        let uri2 = service.ensure_ecr_repository("test-repo").await.unwrap();
        assert_eq!(uri, uri2);
    }
    
    #[tokio::test]
    async fn test_docker_login_ecr() {
        let service = create_test_service();
        
        let result = service.docker_login_ecr().await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_build_docker_image() {
        let service = create_test_service();
        
        // Create temp directory
        let temp_dir = std::env::temp_dir().join("test_build");
        std::fs::create_dir_all(&temp_dir).unwrap();
        
        let result = service.build_docker_image(
            temp_dir.to_str().unwrap(),
            "test-app:latest",
            &FrameworkType::NextJs
        ).await;
        
        assert!(result.is_ok());
        assert!(service.state.has_docker_image("test-app:latest"));
        
        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }
    
    #[tokio::test]
    async fn test_register_task_definition() {
        let service = create_test_service();
        
        let config = EcsDeploymentConfig {
            cluster_name: "test-cluster".to_string(),
            service_name: "test-service".to_string(),
            task_family: "test-task".to_string(),
            container_name: "test-container".to_string(),
            image_uri: "test-image".to_string(),
            cpu: "256".to_string(),
            memory: "512".to_string(),
            port: 3000,
            desired_count: 1,
        };
        
        let arn = service.register_task_definition(&config).await.unwrap();
        
        assert!(arn.contains("test-task"));
        assert!(arn.starts_with("arn:aws:ecs:"));
    }
    
    #[tokio::test]
    async fn test_service_health_progression() {
        let service = create_test_service();
        
        let config = EcsDeploymentConfig {
            cluster_name: "test-cluster".to_string(),
            service_name: "test-service".to_string(),
            task_family: "test-task".to_string(),
            container_name: "test-container".to_string(),
            image_uri: "test-image".to_string(),
            cpu: "256".to_string(),
            memory: "512".to_string(),
            port: 3000,
            desired_count: 1,
        };
        
        // Deploy service
        service.deploy_service(&config, "arn:test").await.unwrap();
        
        // Check initial health - should have pending tasks
        let health1 = service.get_service_health("test-cluster", "test-service").await.unwrap();
        assert_eq!(health1.desired_count, 1);
        assert!(!health1.is_healthy);
        
        // Check again - should progress
        let health2 = service.get_service_health("test-cluster", "test-service").await.unwrap();
        assert!(health2.running_count >= health1.running_count);
    }
}
