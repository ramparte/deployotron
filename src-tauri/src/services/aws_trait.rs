//! AWS operations trait
//!
//! Defines the trait for AWS operations that can be implemented by both
//! real AWS service and mock service for testing.

use async_trait::async_trait;
use crate::services::{AwsServiceError, EcsDeploymentConfig, ServiceHealth};
use crate::models::FrameworkType;

/// Trait for AWS operations (ECS, ECR, CloudWatch)
///
/// This trait allows swapping between real AWS SDK operations and mock
/// implementations for testing without infrastructure.
#[async_trait]
pub trait AwsOperations: Send + Sync {
    /// Create ECR repository if it doesn't exist
    ///
    /// # Arguments
    /// * `repository_name` - Name of the ECR repository
    ///
    /// # Returns
    /// Repository URI on success
    async fn ensure_ecr_repository(&self, repository_name: &str) -> Result<String, AwsServiceError>;
    
    /// Get ECR login credentials and authenticate Docker
    ///
    /// Authenticates the local Docker daemon with ECR registry.
    async fn docker_login_ecr(&self) -> Result<(), AwsServiceError>;
    
    /// Build Docker image from source directory
    ///
    /// # Arguments
    /// * `source_dir` - Path to source code directory
    /// * `image_tag` - Tag for the Docker image
    /// * `framework` - Framework type for Dockerfile generation
    async fn build_docker_image(
        &self,
        source_dir: &str,
        image_tag: &str,
        framework: &FrameworkType
    ) -> Result<(), AwsServiceError>;
    
    /// Push Docker image to ECR
    ///
    /// # Arguments
    /// * `local_tag` - Local Docker image tag
    /// * `ecr_uri` - Full ECR URI with tag
    async fn push_docker_image(&self, local_tag: &str, ecr_uri: &str) -> Result<(), AwsServiceError>;
    
    /// Register ECS task definition
    ///
    /// # Arguments
    /// * `config` - ECS deployment configuration
    ///
    /// # Returns
    /// Task definition ARN on success
    async fn register_task_definition(&self, config: &EcsDeploymentConfig) -> Result<String, AwsServiceError>;
    
    /// Create or update ECS service
    ///
    /// # Arguments
    /// * `config` - ECS deployment configuration
    /// * `task_definition_arn` - ARN of task definition to deploy
    async fn deploy_service(
        &self,
        config: &EcsDeploymentConfig,
        task_definition_arn: &str
    ) -> Result<(), AwsServiceError>;
    
    /// Get service health status
    ///
    /// # Arguments
    /// * `cluster_name` - ECS cluster name
    /// * `service_name` - ECS service name
    ///
    /// # Returns
    /// Service health status with running/desired/pending counts
    async fn get_service_health(
        &self,
        cluster_name: &str,
        service_name: &str
    ) -> Result<ServiceHealth, AwsServiceError>;
    
    /// Fetch recent logs from CloudWatch
    ///
    /// # Arguments
    /// * `log_group` - CloudWatch log group name
    /// * `log_stream` - CloudWatch log stream name
    /// * `limit` - Maximum number of log messages to fetch
    ///
    /// # Returns
    /// Vector of log messages
    async fn fetch_logs(
        &self,
        log_group: &str,
        log_stream: &str,
        limit: i32
    ) -> Result<Vec<String>, AwsServiceError>;
}
