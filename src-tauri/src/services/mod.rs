//! Service layer for Deployotron
//!
//! This module provides the service layer components:
//! - GitService: Repository cloning, framework detection, commit information
//! - AwsService: AWS ECS/ECR deployments, CloudWatch logs, health monitoring
//! - TerraformService: Infrastructure-as-Code generation for ECS deployments
//! - ClaudeService: AI-powered deployment assistance and troubleshooting

pub mod git_service;
pub mod aws_service;
pub mod terraform_service;
pub mod claude_service;

pub use git_service::{GitService, GitServiceError, CommitInfo};
pub use aws_service::{AwsService, AwsServiceError, EcsDeploymentConfig, ServiceHealth};
pub use terraform_service::{TerraformService, TerraformServiceError, TerraformConfig};
pub use claude_service::{ClaudeService, ClaudeServiceError, DeploymentContext, ClaudeResponse};
