//! Tauri command handlers for Deployotron
//!
//! This module provides the command interface between the React frontend and Rust backend.
//! All commands are exposed via Tauri's IPC mechanism and return Result<T, String> for
//! frontend compatibility.

use crate::infrastructure::{Database, KeychainService};
use crate::models::{AwsCredentials, Deployment, Environment, FrameworkType, GitCredentials, Project};
use crate::services::{
    AwsService, ClaudeService, DeploymentContext, GitService, TerraformService,
};
use crate::application::orchestrator::DeploymentOrchestrator;
use std::sync::{Arc, Mutex};
use tauri::State;

/// Shared application state accessible to all commands
pub struct AppState {
    pub database: Arc<Mutex<Database>>,
    pub keychain: Arc<Mutex<KeychainService>>,
    pub git_service: Arc<GitService>,
    pub terraform_service: Arc<TerraformService>,
}

impl AppState {
    /// Create a new AppState with initialized services
    pub fn new() -> Result<Self, String> {
        let database = Database::new()
            .map_err(|e| format!("Failed to initialize database: {}", e))?;
        
        let keychain = KeychainService::new();
        
        Ok(Self {
            database: Arc::new(Mutex::new(database)),
            keychain: Arc::new(Mutex::new(keychain)),
            git_service: Arc::new(GitService::new()),
            terraform_service: Arc::new(TerraformService::new()),
        })
    }
}

// ===== Project Commands =====

/// Create a new deployment project
#[tauri::command]
pub async fn create_project(
    state: State<'_, AppState>,
    name: String,
    repository_url: String,
    branch: String,
    framework: String,
    environment: String,
    aws_cluster: String,
    aws_service: String,
    ecr_repository: String,
) -> Result<Project, String> {
    // Parse framework and environment from strings
    let framework_type: FrameworkType = serde_json::from_str(&format!("\"{}\"", framework))
        .map_err(|e| format!("Invalid framework type: {}", e))?;
    
    let env_type: Environment = serde_json::from_str(&format!("\"{}\"", environment))
        .map_err(|e| format!("Invalid environment type: {}", e))?;
    
    // Create project model
    let project = Project::new(
        name,
        repository_url,
        branch,
        framework_type,
        env_type,
        aws_cluster,
        aws_service,
        ecr_repository,
    );
    
    // Save to database
    let db = state.database.lock()
        .map_err(|e| format!("Failed to acquire database lock: {}", e))?;
    
    db.create_project(&project)
        .map_err(|e| format!("Failed to create project: {}", e))?;
    
    Ok(project)
}

/// Get all deployment projects
#[tauri::command]
pub async fn get_projects(state: State<'_, AppState>) -> Result<Vec<Project>, String> {
    let db = state.database.lock()
        .map_err(|e| format!("Failed to acquire database lock: {}", e))?;
    
    db.get_all_projects()
        .map_err(|e| format!("Failed to get projects: {}", e))
}

/// Get a single project by ID
#[tauri::command]
pub async fn get_project(state: State<'_, AppState>, project_id: String) -> Result<Project, String> {
    let db = state.database.lock()
        .map_err(|e| format!("Failed to acquire database lock: {}", e))?;
    
    db.get_project(&project_id)
        .map_err(|e| format!("Failed to get project: {}", e))
}

/// Update an existing project
#[tauri::command]
pub async fn update_project(
    state: State<'_, AppState>,
    project: Project,
) -> Result<(), String> {
    let db = state.database.lock()
        .map_err(|e| format!("Failed to acquire database lock: {}", e))?;
    
    db.update_project(&project)
        .map_err(|e| format!("Failed to update project: {}", e))
}

/// Delete a project and all associated deployments
#[tauri::command]
pub async fn delete_project(state: State<'_, AppState>, project_id: String) -> Result<(), String> {
    let db = state.database.lock()
        .map_err(|e| format!("Failed to acquire database lock: {}", e))?;
    
    db.delete_project(&project_id)
        .map_err(|e| format!("Failed to delete project: {}", e))
}

// ===== Deployment Commands =====

/// Start a new deployment for a project
#[tauri::command]
pub async fn start_deployment(
    state: State<'_, AppState>,
    window: tauri::Window,
    project_id: String,
) -> Result<String, String> {
    // Get project details
    let project = {
        let db = state.database.lock()
            .map_err(|e| format!("Failed to acquire database lock: {}", e))?;
        db.get_project(&project_id)
            .map_err(|e| format!("Project not found: {}", e))?
    };
    
    // Get AWS credentials
    let aws_credentials = {
        let keychain = state.keychain.lock()
            .map_err(|e| format!("Failed to acquire keychain lock: {}", e))?;
        keychain.get_aws_credentials()
            .map_err(|e| format!("AWS credentials not configured: {}", e))?
    };
    
    // Create AWS service
    let aws_service = AwsService::new(Some(aws_credentials.region.clone()))
        .await
        .map_err(|e| format!("Failed to initialize AWS service: {}", e))?;
    
    // Create orchestrator
    let orchestrator = DeploymentOrchestrator::new(
        state.database.clone(),
        state.git_service.clone(),
        Arc::new(aws_service),
        state.terraform_service.clone(),
        window,
    );
    
    // Run deployment in background and return deployment ID
    let deployment_id = orchestrator.run_deployment(project).await
        .map_err(|e| format!("Deployment failed: {}", e))?;
    
    Ok(deployment_id)
}

/// Get deployment status and details
#[tauri::command]
pub async fn get_deployment_status(
    state: State<'_, AppState>,
    deployment_id: String,
) -> Result<Deployment, String> {
    let db = state.database.lock()
        .map_err(|e| format!("Failed to acquire database lock: {}", e))?;
    
    db.get_deployment(&deployment_id)
        .map_err(|e| format!("Failed to get deployment: {}", e))
}

/// Get all deployments for a project
#[tauri::command]
pub async fn get_project_deployments(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<Deployment>, String> {
    let db = state.database.lock()
        .map_err(|e| format!("Failed to acquire database lock: {}", e))?;
    
    db.get_deployments_for_project(&project_id)
        .map_err(|e| format!("Failed to get deployments: {}", e))
}

/// Get deployment logs
#[tauri::command]
pub async fn get_deployment_logs(
    state: State<'_, AppState>,
    deployment_id: String,
) -> Result<String, String> {
    let db = state.database.lock()
        .map_err(|e| format!("Failed to acquire database lock: {}", e))?;
    
    let deployment = db.get_deployment(&deployment_id)
        .map_err(|e| format!("Failed to get deployment: {}", e))?;
    
    Ok(deployment.logs.unwrap_or_else(|| "No logs available".to_string()))
}

// ===== Credential Commands =====

/// Store AWS credentials securely
#[tauri::command]
pub async fn store_aws_credentials(
    state: State<'_, AppState>,
    access_key_id: String,
    secret_access_key: String,
    region: String,
) -> Result<(), String> {
    let credentials = AwsCredentials {
        access_key_id,
        secret_access_key,
        region,
    };
    
    let keychain = state.keychain.lock()
        .map_err(|e| format!("Failed to acquire keychain lock: {}", e))?;
    
    keychain.store_aws_credentials(&credentials)
        .map_err(|e| format!("Failed to store AWS credentials: {}", e))
}

/// Store Git credentials securely
#[tauri::command]
pub async fn store_git_credentials(
    state: State<'_, AppState>,
    username: String,
    token: String,
    provider: String,
) -> Result<(), String> {
    let credentials = GitCredentials {
        username,
        token,
        provider,
    };
    
    let keychain = state.keychain.lock()
        .map_err(|e| format!("Failed to acquire keychain lock: {}", e))?;
    
    keychain.store_git_credentials(&credentials)
        .map_err(|e| format!("Failed to store Git credentials: {}", e))
}

/// Get credentials configuration status
#[tauri::command]
pub async fn get_credentials_status(
    state: State<'_, AppState>,
) -> Result<CredentialsStatus, String> {
    let keychain = state.keychain.lock()
        .map_err(|e| format!("Failed to acquire keychain lock: {}", e))?;
    
    let aws_configured = keychain.get_aws_credentials().is_ok();
    let git_configured = keychain.get_git_credentials().is_ok();
    
    Ok(CredentialsStatus {
        aws_configured,
        git_configured,
    })
}

/// Delete AWS credentials
#[tauri::command]
pub async fn delete_aws_credentials(state: State<'_, AppState>) -> Result<(), String> {
    let keychain = state.keychain.lock()
        .map_err(|e| format!("Failed to acquire keychain lock: {}", e))?;
    
    keychain.delete_aws_credentials()
        .map_err(|e| format!("Failed to delete AWS credentials: {}", e))
}

/// Delete Git credentials
#[tauri::command]
pub async fn delete_git_credentials(state: State<'_, AppState>) -> Result<(), String> {
    let keychain = state.keychain.lock()
        .map_err(|e| format!("Failed to acquire keychain lock: {}", e))?;
    
    keychain.delete_git_credentials()
        .map_err(|e| format!("Failed to delete Git credentials: {}", e))
}

// ===== AI Chat Commands =====

/// Ask Claude a question about deployments
#[tauri::command]
pub async fn ask_claude(
    state: State<'_, AppState>,
    question: String,
    project_id: Option<String>,
    api_key: String,
) -> Result<ClaudeResponseDto, String> {
    // Create Claude service
    let claude = ClaudeService::new(api_key)
        .map_err(|e| format!("Failed to initialize Claude service: {}", e))?;
    
    // Build context if project ID provided
    let context = if let Some(pid) = project_id {
        let db = state.database.lock()
            .map_err(|e| format!("Failed to acquire database lock: {}", e))?;
        
        let project = db.get_project(&pid)
            .map_err(|e| format!("Failed to get project: {}", e))?;
        
        // Get latest deployment for context
        let deployments = db.get_deployments_for_project(&pid)
            .map_err(|e| format!("Failed to get deployments: {}", e))?;
        
        deployments.first().map(|d| DeploymentContext {
            project_name: project.name.clone(),
            framework: format!("{:?}", project.framework),
            environment: format!("{:?}", project.environment),
            cluster_name: project.aws_cluster.clone(),
            service_name: project.aws_service.clone(),
            commit_sha: d.commit_sha.clone(),
            error_message: d.error_message.clone(),
            logs: d.logs.as_ref().map(|logs| logs.lines().map(|s| s.to_string()).collect()),
        })
    } else {
        None
    };
    
    // Ask Claude
    let response = claude.ask_question(&question, context.as_ref())
        .await
        .map_err(|e| format!("Claude request failed: {}", e))?;
    
    Ok(ClaudeResponseDto {
        answer: response.answer,
        suggestions: response.suggestions,
    })
}

/// Analyze deployment logs with Claude AI
#[tauri::command]
pub async fn analyze_deployment_logs(
    state: State<'_, AppState>,
    deployment_id: String,
    api_key: String,
) -> Result<ClaudeResponseDto, String> {
    // Get deployment and project details
    let (deployment, project) = {
        let db = state.database.lock()
            .map_err(|e| format!("Failed to acquire database lock: {}", e))?;
        
        let deployment = db.get_deployment(&deployment_id)
            .map_err(|e| format!("Failed to get deployment: {}", e))?;
        
        let project = db.get_project(&deployment.project_id)
            .map_err(|e| format!("Failed to get project: {}", e))?;
        
        (deployment, project)
    };
    
    // Create Claude service
    let claude = ClaudeService::new(api_key)
        .map_err(|e| format!("Failed to initialize Claude service: {}", e))?;
    
    // Build deployment context
    let logs: Vec<String> = deployment.logs
        .as_ref()
        .map(|logs| logs.lines().map(|s| s.to_string()).collect())
        .unwrap_or_default();
    
    let context = DeploymentContext {
        project_name: project.name.clone(),
        framework: format!("{:?}", project.framework),
        environment: format!("{:?}", project.environment),
        cluster_name: project.aws_cluster.clone(),
        service_name: project.aws_service.clone(),
        commit_sha: deployment.commit_sha.clone(),
        error_message: deployment.error_message.clone(),
        logs: Some(logs.clone()),
    };
    
    // Analyze logs
    let response = claude.analyze_logs(&logs, deployment.error_message.as_deref(), &context)
        .await
        .map_err(|e| format!("Log analysis failed: {}", e))?;
    
    Ok(ClaudeResponseDto {
        answer: response.answer,
        suggestions: response.suggestions,
    })
}

// ===== Response DTOs =====

/// Credentials configuration status
#[derive(Debug, Clone, serde::Serialize)]
pub struct CredentialsStatus {
    pub aws_configured: bool,
    pub git_configured: bool,
}

/// Claude response DTO for frontend
#[derive(Debug, Clone, serde::Serialize)]
pub struct ClaudeResponseDto {
    pub answer: String,
    pub suggestions: Vec<String>,
}
