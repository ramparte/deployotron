//! Deployment orchestrator
//!
//! Coordinates the full deployment workflow from git clone to ECS service running.
//! Emits progress events to the frontend via Tauri events.

use crate::infrastructure::Database;
use crate::models::{Deployment, DeploymentStatus, Project};
use crate::services::{AwsOperations, AwsService, EcsDeploymentConfig, GitOperations, TerraformService, TerraformConfig};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use tauri::Window;
use thiserror::Error;

/// Deployment orchestrator errors
#[derive(Error, Debug)]
pub enum OrchestratorError {
    #[error("Database operation failed: {0}")]
    DatabaseError(String),
    
    #[error("Git operation failed: {0}")]
    GitError(String),
    
    #[error("AWS operation failed: {0}")]
    AwsError(String),
    
    #[error("Terraform operation failed: {0}")]
    TerraformError(String),
    
    #[error("Event emission failed: {0}")]
    EventError(String),
}

/// Deployment progress event payload
#[derive(Debug, Clone, serde::Serialize)]
struct ProgressEvent {
    pub deployment_id: String,
    pub step: String,
    pub progress: u8,
    pub message: String,
}

/// Deployment orchestrator that coordinates the full workflow
pub struct DeploymentOrchestrator {
    database: Arc<Mutex<Database>>,
    git_service: Arc<dyn GitOperations>,
    aws_service: Arc<dyn AwsOperations>,
    terraform_service: Arc<TerraformService>,
    window: Window,
}

impl DeploymentOrchestrator {
    /// Create a new deployment orchestrator
    pub fn new(
        database: Arc<Mutex<Database>>,
        git_service: Arc<dyn GitOperations>,
        aws_service: Arc<dyn AwsOperations>,
        terraform_service: Arc<TerraformService>,
        window: Window,
    ) -> Self {
        Self {
            database,
            git_service,
            aws_service,
            terraform_service,
            window,
        }
    }
    
    /// Run the complete deployment workflow
    ///
    /// This orchestrates the 10-step deployment process:
    /// 1. Initialize deployment record
    /// 2. Clone git repository
    /// 3. Detect framework type
    /// 4. Get commit information
    /// 5. Build Docker image
    /// 6. Login to ECR
    /// 7. Push image to ECR
    /// 8. Register ECS task definition
    /// 9. Deploy to ECS service
    /// 10. Monitor until running
    pub async fn run_deployment(&self, project: Project) -> Result<String, OrchestratorError> {
        // Step 1: Initialize deployment record (0-10%)
        let mut deployment = self.initialize_deployment(&project).await?;
        
        self.emit_progress(&deployment.id, "Initializing deployment", 10).await?;
        
        // Step 2: Clone repository (10-20%)
        let repo_path = match self.clone_repository(&project, &deployment.id).await {
            Ok(path) => path,
            Err(e) => {
                self.fail_deployment(&mut deployment, &format!("Git clone failed: {}", e)).await?;
                return Err(e);
            }
        };
        
        self.emit_progress(&deployment.id, "Repository cloned", 20).await?;
        
        // Step 3: Detect framework (20-25%)
        let framework = match self.detect_framework(&repo_path, &deployment.id).await {
            Ok(fw) => fw,
            Err(e) => {
                self.cleanup_repository(&repo_path).await;
                self.fail_deployment(&mut deployment, &format!("Framework detection failed: {}", e)).await?;
                return Err(e);
            }
        };
        
        self.emit_progress(&deployment.id, &format!("Framework detected: {:?}", framework), 25).await?;
        
        // Step 4: Get commit information (25-30%)
        let commit_info = match self.get_commit_info(&repo_path, &deployment.id).await {
            Ok(info) => info,
            Err(e) => {
                self.cleanup_repository(&repo_path).await;
                self.fail_deployment(&mut deployment, &format!("Failed to get commit info: {}", e)).await?;
                return Err(e);
            }
        };
        
        // Update deployment with commit info
        deployment.commit_sha = commit_info.sha.clone();
        deployment.commit_message = Some(commit_info.message.clone());
        self.update_deployment(&deployment).await?;
        
        self.emit_progress(&deployment.id, &format!("Commit: {}", &commit_info.sha[..8]), 30).await?;
        
        // Step 5: Build Docker image (30-50%)
        let image_tag = format!("{}:{}", project.name, &commit_info.sha[..8]);
        match self.build_docker_image(&repo_path, &image_tag, &project, &deployment.id).await {
            Ok(_) => {},
            Err(e) => {
                self.cleanup_repository(&repo_path).await;
                self.fail_deployment(&mut deployment, &format!("Docker build failed: {}", e)).await?;
                return Err(e);
            }
        };
        
        self.emit_progress(&deployment.id, "Docker image built", 50).await?;
        
        // Step 6: Login to ECR (50-55%)
        match self.login_to_ecr(&deployment.id).await {
            Ok(_) => {},
            Err(e) => {
                self.cleanup_repository(&repo_path).await;
                self.fail_deployment(&mut deployment, &format!("ECR login failed: {}", e)).await?;
                return Err(e);
            }
        };
        
        self.emit_progress(&deployment.id, "Authenticated with ECR", 55).await?;
        
        // Step 7: Push image to ECR (55-70%)
        let ecr_image_uri = format!("{}:{}", project.ecr_repository, &commit_info.sha[..8]);
        match self.push_to_ecr(&image_tag, &ecr_image_uri, &deployment.id).await {
            Ok(_) => {},
            Err(e) => {
                self.cleanup_repository(&repo_path).await;
                self.fail_deployment(&mut deployment, &format!("ECR push failed: {}", e)).await?;
                return Err(e);
            }
        };
        
        self.emit_progress(&deployment.id, "Image pushed to ECR", 70).await?;
        
        // Step 8: Register ECS task definition (70-80%)
        let task_arn = match self.register_task_definition(&project, &ecr_image_uri, &deployment.id).await {
            Ok(arn) => arn,
            Err(e) => {
                self.cleanup_repository(&repo_path).await;
                self.fail_deployment(&mut deployment, &format!("Task registration failed: {}", e)).await?;
                return Err(e);
            }
        };
        
        self.emit_progress(&deployment.id, "ECS task definition registered", 80).await?;
        
        // Step 9: Deploy to ECS service (80-90%)
        match self.deploy_to_ecs(&project, &task_arn, &deployment.id).await {
            Ok(_) => {},
            Err(e) => {
                self.cleanup_repository(&repo_path).await;
                self.fail_deployment(&mut deployment, &format!("ECS deployment failed: {}", e)).await?;
                return Err(e);
            }
        };
        
        self.emit_progress(&deployment.id, "Deployment initiated on ECS", 90).await?;
        
        // Step 10: Monitor until running (90-100%)
        match self.monitor_deployment(&project, &deployment.id).await {
            Ok(_) => {},
            Err(e) => {
                self.cleanup_repository(&repo_path).await;
                self.fail_deployment(&mut deployment, &format!("Service failed to become healthy: {}", e)).await?;
                return Err(e);
            }
        };
        
        // Cleanup repository
        self.cleanup_repository(&repo_path).await;
        
        // Mark deployment as successful
        deployment.status = DeploymentStatus::Success;
        deployment.completed_at = Some(chrono::Utc::now().timestamp());
        self.update_deployment(&deployment).await?;
        
        self.emit_progress(&deployment.id, "Deployment successful", 100).await?;
        
        Ok(deployment.id)
    }
    
    // ===== Step Implementations =====
    
    /// Initialize deployment record in database
    async fn initialize_deployment(&self, project: &Project) -> Result<Deployment, OrchestratorError> {
        let deployment = Deployment::new(
            project.id.clone(),
            "pending".to_string(), // Will be updated with actual commit SHA
            None,
            format!("{}:latest", project.name),
        );
        
        let db = self.database.lock()
            .map_err(|e| OrchestratorError::DatabaseError(format!("Lock failed: {}", e)))?;
        
        db.create_deployment(&deployment)
            .map_err(|e| OrchestratorError::DatabaseError(e.to_string()))?;
        
        Ok(deployment)
    }
    
    /// Clone git repository
    async fn clone_repository(&self, project: &Project, deployment_id: &str) -> Result<PathBuf, OrchestratorError> {
        let path = self.git_service
            .clone_repository(&project.repository_url, &project.branch)
            .await
            .map_err(|e| OrchestratorError::GitError(e.to_string()))?;
        
        Ok(path)
    }
    
    /// Detect framework from repository
    async fn detect_framework(&self, repo_path: &PathBuf, deployment_id: &str) -> Result<crate::models::FrameworkType, OrchestratorError> {
        let framework = self.git_service
            .detect_framework(repo_path)
            .await
            .map_err(|e| OrchestratorError::GitError(e.to_string()))?;
        
        Ok(framework)
    }
    
    /// Get commit information
    async fn get_commit_info(&self, repo_path: &PathBuf, deployment_id: &str) -> Result<crate::services::CommitInfo, OrchestratorError> {
        let commit_info = self.git_service
            .get_commit_info(repo_path, None)
            .await
            .map_err(|e| OrchestratorError::GitError(e.to_string()))?;
        
        Ok(commit_info)
    }
    
    /// Build Docker image
    async fn build_docker_image(&self, repo_path: &PathBuf, image_tag: &str, project: &Project, deployment_id: &str) -> Result<(), OrchestratorError> {
        self.aws_service
            .build_docker_image(
                repo_path.to_str().ok_or_else(|| OrchestratorError::AwsError("Invalid path".to_string()))?,
                image_tag,
                &project.framework,
            )
            .await
            .map_err(|e| OrchestratorError::AwsError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Login to ECR
    async fn login_to_ecr(&self, deployment_id: &str) -> Result<(), OrchestratorError> {
        self.aws_service
            .docker_login_ecr()
            .await
            .map_err(|e| OrchestratorError::AwsError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Push Docker image to ECR
    async fn push_to_ecr(&self, local_tag: &str, ecr_uri: &str, deployment_id: &str) -> Result<(), OrchestratorError> {
        self.aws_service
            .push_docker_image(local_tag, ecr_uri)
            .await
            .map_err(|e| OrchestratorError::AwsError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Register ECS task definition
    async fn register_task_definition(&self, project: &Project, image_uri: &str, deployment_id: &str) -> Result<String, OrchestratorError> {
        let port = AwsService::get_framework_port(&project.framework);
        
        let config = EcsDeploymentConfig {
            cluster_name: project.aws_cluster.clone(),
            service_name: project.aws_service.clone(),
            task_family: format!("{}-task", project.name),
            container_name: format!("{}-container", project.name),
            image_uri: image_uri.to_string(),
            cpu: "512".to_string(),
            memory: "1024".to_string(),
            port,
            desired_count: 1,
        };
        
        let task_arn = self.aws_service
            .register_task_definition(&config)
            .await
            .map_err(|e| OrchestratorError::AwsError(e.to_string()))?;
        
        Ok(task_arn)
    }
    
    /// Deploy to ECS service
    async fn deploy_to_ecs(&self, project: &Project, task_arn: &str, deployment_id: &str) -> Result<(), OrchestratorError> {
        let port = AwsService::get_framework_port(&project.framework);
        
        let config = EcsDeploymentConfig {
            cluster_name: project.aws_cluster.clone(),
            service_name: project.aws_service.clone(),
            task_family: format!("{}-task", project.name),
            container_name: format!("{}-container", project.name),
            image_uri: String::new(), // Not used in update
            cpu: "512".to_string(),
            memory: "1024".to_string(),
            port,
            desired_count: 1,
        };
        
        self.aws_service
            .deploy_service(&config, task_arn)
            .await
            .map_err(|e| OrchestratorError::AwsError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Monitor deployment until service is healthy
    async fn monitor_deployment(&self, project: &Project, deployment_id: &str) -> Result<(), OrchestratorError> {
        // Poll service health for up to 5 minutes
        let max_attempts = 30; // 30 attempts * 10 seconds = 5 minutes
        let mut attempts = 0;
        
        loop {
            attempts += 1;
            
            if attempts > max_attempts {
                return Err(OrchestratorError::AwsError(
                    "Deployment timeout: service did not become healthy".to_string()
                ));
            }
            
            // Check service health
            let health = self.aws_service
                .get_service_health(&project.aws_cluster, &project.aws_service)
                .await
                .map_err(|e| OrchestratorError::AwsError(e.to_string()))?;
            
            if health.is_healthy {
                // Service is healthy!
                return Ok(());
            }
            
            // Update progress based on running vs desired count
            let progress = 90 + (10 * health.running_count / health.desired_count.max(1)) as u8;
            self.emit_progress(
                deployment_id,
                &format!("Waiting for service to stabilize ({}/{})", health.running_count, health.desired_count),
                progress.min(99), // Cap at 99% until fully healthy
            ).await?;
            
            // Wait 10 seconds before next check
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    }
    
    /// Cleanup cloned repository
    async fn cleanup_repository(&self, repo_path: &PathBuf) {
        // Best effort cleanup - don't fail deployment if cleanup fails
        let _ = self.git_service.cleanup_repository(repo_path).await;
    }
    
    // ===== Helper Methods =====
    
    /// Emit progress event to frontend
    async fn emit_progress(&self, deployment_id: &str, message: &str, progress: u8) -> Result<(), OrchestratorError> {
        let event = ProgressEvent {
            deployment_id: deployment_id.to_string(),
            step: message.to_string(),
            progress,
            message: message.to_string(),
        };
        
        self.window
            .emit("deployment-progress", event)
            .map_err(|e| OrchestratorError::EventError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Update deployment record in database
    async fn update_deployment(&self, deployment: &Deployment) -> Result<(), OrchestratorError> {
        let db = self.database.lock()
            .map_err(|e| OrchestratorError::DatabaseError(format!("Lock failed: {}", e)))?;
        
        db.update_deployment(deployment)
            .map_err(|e| OrchestratorError::DatabaseError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Mark deployment as failed and update database
    async fn fail_deployment(&self, deployment: &mut Deployment, error: &str) -> Result<(), OrchestratorError> {
        deployment.status = DeploymentStatus::Failed;
        deployment.completed_at = Some(chrono::Utc::now().timestamp());
        deployment.error_message = Some(error.to_string());
        
        self.update_deployment(deployment).await?;
        
        // Emit failure event
        self.emit_progress(&deployment.id, &format!("Deployment failed: {}", error), 0).await?;
        
        Ok(())
    }
}
