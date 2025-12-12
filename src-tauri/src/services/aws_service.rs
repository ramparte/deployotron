//! AWS deployment service
//!
//! Provides functionality for:
//! - Initializing AWS SDK clients (ECS, ECR, CloudWatch)
//! - Managing ECR repositories
//! - Building and pushing Docker images
//! - Deploying to ECS (task definitions, services)
//! - Fetching CloudWatch logs
//! - Monitoring service health
use tokio::process::Command;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_ecr::{Client as EcrClient, types::ImageIdentifier};
use aws_sdk_ecs::{Client as EcsClient, types::{TaskDefinition, ContainerDefinition, PortMapping, LogConfiguration}};
use aws_sdk_cloudwatchlogs::{Client as CloudWatchClient};
use tokio::process::Command;
use thiserror::Error;
use crate::models::FrameworkType;

/// AWS service specific errors
#[derive(Error, Debug)]
pub enum AwsServiceError {
    #[error("Failed to initialize AWS SDK: {0}")]
    InitializationFailed(String),
    
    #[error("ECR operation failed: {0}")]
    EcrOperationFailed(String),
    
    #[error("ECS operation failed: {0}")]
    EcsOperationFailed(String),
    
    #[error("CloudWatch operation failed: {0}")]
    CloudWatchOperationFailed(String),
    
    #[error("Docker operation failed: {0}")]
    DockerOperationFailed(String),
    
    #[error("Service health check failed: {0}")]
    HealthCheckFailed(String),
}

/// AWS service for deployment operations
pub struct AwsService {
    ecr_client: EcrClient,
    ecs_client: EcsClient,
    cloudwatch_client: CloudWatchClient,
    region: String,
}

/// ECS deployment configuration
#[derive(Debug, Clone)]
pub struct EcsDeploymentConfig {
    pub cluster_name: String,
    pub service_name: String,
    pub task_family: String,
    pub container_name: String,
    pub image_uri: String,
    pub cpu: String,
    pub memory: String,
    pub port: i32,
    pub desired_count: i32,
}

/// Service health status
#[derive(Debug, Clone)]
pub struct ServiceHealth {
    pub running_count: i32,
    pub desired_count: i32,
    pub pending_count: i32,
    pub is_healthy: bool,
}

impl AwsService {
    /// Create a new AwsService with AWS SDK clients
    pub async fn new(region: Option<String>) -> Result<Self, AwsServiceError> {
        // Load AWS configuration from environment
        let region_provider = RegionProviderChain::default_provider()
            .or_else(region.as_deref().unwrap_or("us-east-1"));
        
        let config = aws_config::from_env()
            .region(region_provider)
            .load()
            .await;
        
        let actual_region = config.region()
            .map(|r| r.as_ref().to_string())
            .unwrap_or_else(|| "us-east-1".to_string());
        
        Ok(Self {
            ecr_client: EcrClient::new(&config),
            ecs_client: EcsClient::new(&config),
            cloudwatch_client: CloudWatchClient::new(&config),
            region: actual_region,
        })
    }
    
    // ===== ECR Operations =====
    
    /// Create ECR repository if it doesn't exist
    pub async fn ensure_ecr_repository(&self, repository_name: &str) -> Result<String, AwsServiceError> {
        // Check if repository exists
        match self.ecr_client
            .describe_repositories()
            .repository_names(repository_name)
            .send()
            .await
        {
            Ok(output) => {
                // Repository exists, return URI
                if let Some(repo) = output.repositories().first() {
                    return Ok(repo.repository_uri().unwrap_or("").to_string());
                }
            }
            Err(_) => {
                // Repository doesn't exist, create it
            }
        }
        
        // Create repository
        let output = self.ecr_client
            .create_repository()
            .repository_name(repository_name)
            .send()
            .await
            .map_err(|e| AwsServiceError::EcrOperationFailed(e.to_string()))?;
        
        let repository_uri = output.repository()
            .and_then(|r| r.repository_uri())
            .ok_or_else(|| AwsServiceError::EcrOperationFailed("No repository URI returned".to_string()))?
            .to_string();
        
        Ok(repository_uri)
    }
    
    /// Get ECR login credentials and authenticate Docker
    pub async fn docker_login_ecr(&self) -> Result<(), AwsServiceError> {
        // Get authorization token
        let output = self.ecr_client
            .get_authorization_token()
            .send()
            .await
            .map_err(|e| AwsServiceError::EcrOperationFailed(e.to_string()))?;
        
        let auth_data = output.authorization_data()
            .first()
            .ok_or_else(|| AwsServiceError::EcrOperationFailed("No authorization data".to_string()))?;
        
        let token = auth_data.authorization_token()
            .ok_or_else(|| AwsServiceError::EcrOperationFailed("No authorization token".to_string()))?;
        
        let proxy_endpoint = auth_data.proxy_endpoint()
            .ok_or_else(|| AwsServiceError::EcrOperationFailed("No proxy endpoint".to_string()))?;
        
        // Decode token (base64 encoded "AWS:password")
        let decoded = base64::decode(token)
            .map_err(|e| AwsServiceError::EcrOperationFailed(format!("Failed to decode token: {}", e)))?;
        
        let token_str = String::from_utf8(decoded)
            .map_err(|e| AwsServiceError::EcrOperationFailed(format!("Invalid token format: {}", e)))?;
        
        let password = token_str.split(':').nth(1)
            .ok_or_else(|| AwsServiceError::EcrOperationFailed("Invalid token format".to_string()))?;
        
        // Execute docker login command
        let output = Command::new("docker")
            .args(&["login", "--username", "AWS", "--password-stdin", proxy_endpoint])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| AwsServiceError::DockerOperationFailed(format!("Failed to spawn docker: {}", e)))?
            .stdin
            .ok_or_else(|| AwsServiceError::DockerOperationFailed("No stdin".to_string()))?;
        
        use std::io::Write;
        let mut stdin = output;
        stdin.write_all(password.as_bytes())
            .map_err(|e| AwsServiceError::DockerOperationFailed(format!("Failed to write password: {}", e)))?;
        
        Ok(())
    }
    
    /// Build Docker image from source directory
    pub async fn build_docker_image(&self, source_dir: &str, image_tag: &str, framework: &FrameworkType) -> Result<(), AwsServiceError> {
        // Generate Dockerfile if it doesn't exist
        let dockerfile_path = format!("{}/Dockerfile", source_dir);
        if !std::path::Path::new(&dockerfile_path).exists() {
            self.generate_dockerfile(source_dir, framework)?;
        }
        
        // Build Docker image
        let output = Command::new("docker")
            .args(&["build", "-t", image_tag, source_dir])
            .output()
            .await
            .map_err(|e| AwsServiceError::DockerOperationFailed(format!("Failed to build: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AwsServiceError::DockerOperationFailed(format!("Build failed: {}", stderr)));
        }
        
        Ok(())
    }
    
    /// Push Docker image to ECR
    pub async fn push_docker_image(&self, local_tag: &str, ecr_uri: &str) -> Result<(), AwsServiceError> {
        // Tag image for ECR
        let output = Command::new("docker")
            .args(&["tag", local_tag, ecr_uri])
            .output()
            .await
            .map_err(|e| AwsServiceError::DockerOperationFailed(format!("Failed to tag: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AwsServiceError::DockerOperationFailed(format!("Tag failed: {}", stderr)));
        }
        
        // Push image to ECR
        let output = Command::new("docker")
            .args(&["push", ecr_uri])
            .output()
            .await
            .map_err(|e| AwsServiceError::DockerOperationFailed(format!("Failed to push: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AwsServiceError::DockerOperationFailed(format!("Push failed: {}", stderr)));
        }
        
        Ok(())
    }
    
    // ===== ECS Operations =====
    
    /// Register ECS task definition
    pub async fn register_task_definition(&self, config: &EcsDeploymentConfig) -> Result<String, AwsServiceError> {
        // Create container definition
        let container_def = ContainerDefinition::builder()
            .name(&config.container_name)
            .image(&config.image_uri)
            .cpu(0)
            .memory(512)
            .essential(true)
            .port_mappings(
                PortMapping::builder()
                    .container_port(config.port)
                    .host_port(config.port)
                    .protocol("tcp")
                    .build()
            )
            .log_configuration(
                LogConfiguration::builder()
                    .log_driver("awslogs")
                    .options("awslogs-group", format!("/ecs/{}", config.task_family))
                    .options("awslogs-region", &self.region)
                    .options("awslogs-stream-prefix", "ecs")
                    .build()
            )
            .build();
        
        // Register task definition
        let output = self.ecs_client
            .register_task_definition()
            .family(&config.task_family)
            .network_mode("awsvpc")
            .requires_compatibilities("FARGATE")
            .cpu(&config.cpu)
            .memory(&config.memory)
            .container_definitions(container_def)
            .send()
            .await
            .map_err(|e| AwsServiceError::EcsOperationFailed(e.to_string()))?;
        
        let task_def_arn = output.task_definition()
            .and_then(|td| td.task_definition_arn())
            .ok_or_else(|| AwsServiceError::EcsOperationFailed("No task definition ARN".to_string()))?
            .to_string();
        
        Ok(task_def_arn)
    }
    
    /// Create or update ECS service
    pub async fn deploy_service(&self, config: &EcsDeploymentConfig, task_definition_arn: &str) -> Result<(), AwsServiceError> {
        // Check if service exists
        let service_exists = self.ecs_client
            .describe_services()
            .cluster(&config.cluster_name)
            .services(&config.service_name)
            .send()
            .await
            .ok()
            .and_then(|output| output.services().first().cloned())
            .is_some();
        
        if service_exists {
            // Update existing service
            self.ecs_client
                .update_service()
                .cluster(&config.cluster_name)
                .service(&config.service_name)
                .task_definition(task_definition_arn)
                .desired_count(config.desired_count)
                .force_new_deployment(true)
                .send()
                .await
                .map_err(|e| AwsServiceError::EcsOperationFailed(e.to_string()))?;
        } else {
            // Create new service (simplified - would need VPC config in production)
            return Err(AwsServiceError::EcsOperationFailed(
                "Service creation not implemented - service must exist".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Get service health status
    pub async fn get_service_health(&self, cluster_name: &str, service_name: &str) -> Result<ServiceHealth, AwsServiceError> {
        let output = self.ecs_client
            .describe_services()
            .cluster(cluster_name)
            .services(service_name)
            .send()
            .await
            .map_err(|e| AwsServiceError::EcsOperationFailed(e.to_string()))?;
        
        let service = output.services()
            .first()
            .ok_or_else(|| AwsServiceError::EcsOperationFailed("Service not found".to_string()))?;
        
        let running_count = service.running_count();
        let desired_count = service.desired_count();
        let pending_count = service.pending_count();
        
        Ok(ServiceHealth {
            running_count,
            desired_count,
            pending_count,
            is_healthy: running_count == desired_count && pending_count == 0,
        })
    }
    
    // ===== CloudWatch Operations =====
    
    /// Fetch recent logs from CloudWatch
    pub async fn fetch_logs(&self, log_group: &str, log_stream: &str, limit: i32) -> Result<Vec<String>, AwsServiceError> {
        let output = self.cloudwatch_client
            .get_log_events()
            .log_group_name(log_group)
            .log_stream_name(log_stream)
            .limit(limit)
            .start_from_head(false)
            .send()
            .await
            .map_err(|e| AwsServiceError::CloudWatchOperationFailed(e.to_string()))?;
        
        let logs = output.events()
            .iter()
            .filter_map(|event| event.message().map(|s| s.to_string()))
            .collect();
        
        Ok(logs)
    }
    
    // ===== Helper Functions =====
    
    /// Generate basic Dockerfile based on framework
    fn generate_dockerfile(&self, source_dir: &str, framework: &FrameworkType) -> Result<(), AwsServiceError> {
        let dockerfile_content = match framework {
            FrameworkType::NextJs => {
                r#"FROM node:18-alpine
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
                r#"FROM node:18-alpine
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
                r#"FROM node:18-alpine
WORKDIR /app
COPY package*.json ./
RUN npm ci --only=production
COPY . .
EXPOSE 3000
CMD ["node", "index.js"]
"#
            }
            FrameworkType::Python => {
                r#"FROM python:3.11-slim
WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt
COPY . .
EXPOSE 8000
CMD ["python", "main.py"]
"#
            }
            _ => {
                return Err(AwsServiceError::DockerOperationFailed(
                    format!("No Dockerfile template for framework: {:?}", framework)
                ));
            }
        };
        
        let dockerfile_path = format!("{}/Dockerfile", source_dir);
        std::fs::write(&dockerfile_path, dockerfile_content)
            .map_err(|e| AwsServiceError::DockerOperationFailed(format!("Failed to write Dockerfile: {}", e)))?;
        
        Ok(())
    }
    
    /// Get default port for framework
    pub fn get_framework_port(framework: &FrameworkType) -> i32 {
        match framework {
            FrameworkType::NextJs | FrameworkType::React | FrameworkType::Node => 3000,
            FrameworkType::Python => 8000,
            FrameworkType::Ruby => 3000,
            FrameworkType::Go => 8080,
            FrameworkType::Rust => 8080,
            _ => 8080,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_get_framework_port() {
        assert_eq!(AwsService::get_framework_port(&FrameworkType::NextJs), 3000);
        assert_eq!(AwsService::get_framework_port(&FrameworkType::Python), 8000);
        assert_eq!(AwsService::get_framework_port(&FrameworkType::Go), 8080);
    }
}
