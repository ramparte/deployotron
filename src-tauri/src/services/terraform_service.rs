//! Terraform IaC generation service
//!
//! Provides functionality for:
//! - Generating main.tf, variables.tf, outputs.tf
//! - Creating ECS cluster, service, and task definition templates
//! - Framework-specific port mappings
//! - Writing configurations to output directory

use crate::models::FrameworkType;
use std::path::Path;
use std::fs;
use thiserror::Error;

/// Terraform service specific errors
#[derive(Error, Debug)]
pub enum TerraformServiceError {
    #[error("Failed to write Terraform file: {0}")]
    FileWriteFailed(String),
    
    #[error("Failed to create output directory: {0}")]
    DirectoryCreationFailed(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

impl From<std::io::Error> for TerraformServiceError {
    fn from(err: std::io::Error) -> Self {
        TerraformServiceError::FileWriteFailed(err.to_string())
    }
}

/// Terraform service for IaC generation
pub struct TerraformService;

/// Terraform configuration parameters
#[derive(Debug, Clone)]
pub struct TerraformConfig {
    pub project_name: String,
    pub environment: String,
    pub region: String,
    pub vpc_id: Option<String>,
    pub subnet_ids: Vec<String>,
    pub ecr_repository_name: String,
    pub container_port: i32,
    pub cpu: String,
    pub memory: String,
    pub desired_count: i32,
    pub framework: FrameworkType,
}

impl TerraformService {
    /// Create a new TerraformService instance
    pub fn new() -> Self {
        TerraformService
    }
    
    /// Generate all Terraform configuration files
    pub async fn generate_terraform(&self, config: &TerraformConfig, output_dir: &Path) -> Result<(), TerraformServiceError> {
        // Create output directory if it doesn't exist
        fs::create_dir_all(output_dir)
            .map_err(|e| TerraformServiceError::DirectoryCreationFailed(e.to_string()))?;
        
        // Generate main.tf
        let main_tf = self.generate_main_tf(config);
        fs::write(output_dir.join("main.tf"), main_tf)?;
        
        // Generate variables.tf
        let variables_tf = self.generate_variables_tf(config);
        fs::write(output_dir.join("variables.tf"), variables_tf)?;
        
        // Generate outputs.tf
        let outputs_tf = self.generate_outputs_tf();
        fs::write(output_dir.join("outputs.tf"), outputs_tf)?;
        
        // Generate terraform.tfvars with default values
        let tfvars = self.generate_tfvars(config);
        fs::write(output_dir.join("terraform.tfvars"), tfvars)?;
        
        Ok(())
    }
    
    /// Generate main.tf with ECS resources
    fn generate_main_tf(&self, config: &TerraformConfig) -> String {
        format!(r#"terraform {{
  required_version = ">= 1.0"
  
  required_providers {{
    aws = {{
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }}
  }}
}}

provider "aws" {{
  region = var.aws_region
}}

# ECS Cluster
resource "aws_ecs_cluster" "{project_name}_cluster" {{
  name = "${{var.project_name}}-${{var.environment}}-cluster"
  
  setting {{
    name  = "containerInsights"
    value = "enabled"
  }}
  
  tags = {{
    Name        = "${{var.project_name}}-${{var.environment}}-cluster"
    Environment = var.environment
    ManagedBy   = "Terraform"
  }}
}}

# CloudWatch Log Group
resource "aws_cloudwatch_log_group" "{project_name}_logs" {{
  name              = "/ecs/${{var.project_name}}-${{var.environment}}"
  retention_in_days = 7
  
  tags = {{
    Name        = "${{var.project_name}}-${{var.environment}}-logs"
    Environment = var.environment
  }}
}}

# ECS Task Execution Role
resource "aws_iam_role" "{project_name}_execution_role" {{
  name = "${{var.project_name}}-${{var.environment}}-execution-role"
  
  assume_role_policy = jsonencode({{
    Version = "2012-10-17"
    Statement = [
      {{
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {{
          Service = "ecs-tasks.amazonaws.com"
        }}
      }}
    ]
  }})
  
  tags = {{
    Name        = "${{var.project_name}}-${{var.environment}}-execution-role"
    Environment = var.environment
  }}
}}

resource "aws_iam_role_policy_attachment" "{project_name}_execution_role_policy" {{
  role       = aws_iam_role.{project_name}_execution_role.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
}}

# ECS Task Role (for application permissions)
resource "aws_iam_role" "{project_name}_task_role" {{
  name = "${{var.project_name}}-${{var.environment}}-task-role"
  
  assume_role_policy = jsonencode({{
    Version = "2012-10-17"
    Statement = [
      {{
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {{
          Service = "ecs-tasks.amazonaws.com"
        }}
      }}
    ]
  }})
  
  tags = {{
    Name        = "${{var.project_name}}-${{var.environment}}-task-role"
    Environment = var.environment
  }}
}}

# Security Group for ECS Tasks
resource "aws_security_group" "{project_name}_sg" {{
  name        = "${{var.project_name}}-${{var.environment}}-sg"
  description = "Security group for ${{var.project_name}} ECS tasks"
  vpc_id      = var.vpc_id
  
  ingress {{
    from_port   = {port}
    to_port     = {port}
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
    description = "Allow inbound traffic on application port"
  }}
  
  egress {{
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
    description = "Allow all outbound traffic"
  }}
  
  tags = {{
    Name        = "${{var.project_name}}-${{var.environment}}-sg"
    Environment = var.environment
  }}
}}

# ECS Task Definition
resource "aws_ecs_task_definition" "{project_name}_task" {{
  family                   = "${{var.project_name}}-${{var.environment}}"
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  cpu                      = var.task_cpu
  memory                   = var.task_memory
  execution_role_arn       = aws_iam_role.{project_name}_execution_role.arn
  task_role_arn            = aws_iam_role.{project_name}_task_role.arn
  
  container_definitions = jsonencode([
    {{
      name      = "${{var.project_name}}-container"
      image     = "${{var.ecr_repository_url}}:${{var.image_tag}}"
      essential = true
      
      portMappings = [
        {{
          containerPort = {port}
          hostPort      = {port}
          protocol      = "tcp"
        }}
      ]
      
      environment = [
        {{
          name  = "ENVIRONMENT"
          value = var.environment
        }},
        {{
          name  = "PORT"
          value = "{port}"
        }}
      ]
      
      logConfiguration = {{
        logDriver = "awslogs"
        options = {{
          "awslogs-group"         = aws_cloudwatch_log_group.{project_name}_logs.name
          "awslogs-region"        = var.aws_region
          "awslogs-stream-prefix" = "ecs"
        }}
      }}
      
      healthCheck = {{
        command     = ["CMD-SHELL", "curl -f http://localhost:{port}/health || exit 1"]
        interval    = 30
        timeout     = 5
        retries     = 3
        startPeriod = 60
      }}
    }}
  ])
  
  tags = {{
    Name        = "${{var.project_name}}-${{var.environment}}-task"
    Environment = var.environment
  }}
}}

# ECS Service
resource "aws_ecs_service" "{project_name}_service" {{
  name            = "${{var.project_name}}-${{var.environment}}-service"
  cluster         = aws_ecs_cluster.{project_name}_cluster.id
  task_definition = aws_ecs_task_definition.{project_name}_task.arn
  desired_count   = var.desired_count
  launch_type     = "FARGATE"
  
  network_configuration {{
    subnets          = var.subnet_ids
    security_groups  = [aws_security_group.{project_name}_sg.id]
    assign_public_ip = true
  }}
  
  deployment_configuration {{
    maximum_percent         = 200
    minimum_healthy_percent = 100
  }}
  
  tags = {{
    Name        = "${{var.project_name}}-${{var.environment}}-service"
    Environment = var.environment
  }}
}}
"#,
            project_name = self.sanitize_name(&config.project_name),
            port = config.container_port,
        )
    }
    
    /// Generate variables.tf
    fn generate_variables_tf(&self, config: &TerraformConfig) -> String {
        format!(r#"variable "project_name" {{
  description = "Name of the project"
  type        = string
  default     = "{project_name}"
}}

variable "environment" {{
  description = "Environment name (dev, staging, prod)"
  type        = string
  default     = "{environment}"
}}

variable "aws_region" {{
  description = "AWS region for deployment"
  type        = string
  default     = "{region}"
}}

variable "vpc_id" {{
  description = "VPC ID for ECS tasks"
  type        = string
}}

variable "subnet_ids" {{
  description = "List of subnet IDs for ECS tasks"
  type        = list(string)
}}

variable "ecr_repository_url" {{
  description = "ECR repository URL"
  type        = string
}}

variable "image_tag" {{
  description = "Docker image tag to deploy"
  type        = string
  default     = "latest"
}}

variable "task_cpu" {{
  description = "CPU units for ECS task"
  type        = string
  default     = "{cpu}"
}}

variable "task_memory" {{
  description = "Memory for ECS task (MB)"
  type        = string
  default     = "{memory}"
}}

variable "desired_count" {{
  description = "Desired number of ECS tasks"
  type        = number
  default     = {desired_count}
}}

variable "container_port" {{
  description = "Port exposed by the container"
  type        = number
  default     = {port}
}}
"#,
            project_name = config.project_name,
            environment = config.environment,
            region = config.region,
            cpu = config.cpu,
            memory = config.memory,
            desired_count = config.desired_count,
            port = config.container_port,
        )
    }
    
    /// Generate outputs.tf
    fn generate_outputs_tf(&self) -> String {
        r#"output "cluster_id" {
  description = "ID of the ECS cluster"
  value       = aws_ecs_cluster.{project_name}_cluster.id
}

output "cluster_arn" {
  description = "ARN of the ECS cluster"
  value       = aws_ecs_cluster.{project_name}_cluster.arn
}

output "service_name" {
  description = "Name of the ECS service"
  value       = aws_ecs_service.{project_name}_service.name
}

output "task_definition_arn" {
  description = "ARN of the task definition"
  value       = aws_ecs_task_definition.{project_name}_task.arn
}

output "log_group_name" {
  description = "Name of the CloudWatch log group"
  value       = aws_cloudwatch_log_group.{project_name}_logs.name
}

output "security_group_id" {
  description = "ID of the security group"
  value       = aws_security_group.{project_name}_sg.id
}
"#.to_string()
    }
    
    /// Generate terraform.tfvars with sample values
    fn generate_tfvars(&self, config: &TerraformConfig) -> String {
        let subnet_ids = if config.subnet_ids.is_empty() {
            r#"["subnet-xxxxxxxx", "subnet-yyyyyyyy"]"#.to_string()
        } else {
            format!("[\"{}\"]", config.subnet_ids.join("\", \""))
        };
        
        let vpc_id = config.vpc_id.as_deref().unwrap_or("vpc-xxxxxxxx");
        
        format!(r#"# Terraform variable values
# Update these values with your actual AWS resources

project_name = "{project_name}"
environment  = "{environment}"
aws_region   = "{region}"

# Network Configuration
vpc_id     = "{vpc_id}"
subnet_ids = {subnet_ids}

# ECR Configuration
ecr_repository_url = "{ecr_repository}"
image_tag          = "latest"

# ECS Task Configuration
task_cpu       = "{cpu}"
task_memory    = "{memory}"
desired_count  = {desired_count}
container_port = {port}
"#,
            project_name = config.project_name,
            environment = config.environment,
            region = config.region,
            vpc_id = vpc_id,
            subnet_ids = subnet_ids,
            ecr_repository = config.ecr_repository_name,
            cpu = config.cpu,
            memory = config.memory,
            desired_count = config.desired_count,
            port = config.container_port,
        )
    }
    
    /// Sanitize project name for Terraform resource names
    fn sanitize_name(&self, name: &str) -> String {
        name.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect()
    }
    
    /// Get framework-specific defaults
    pub fn get_framework_defaults(framework: &FrameworkType) -> (i32, String, String) {
        match framework {
            FrameworkType::NextJs | FrameworkType::React | FrameworkType::Node => {
                (3000, "512".to_string(), "1024".to_string())
            }
            FrameworkType::Python => {
                (8000, "256".to_string(), "512".to_string())
            }
            FrameworkType::Ruby => {
                (3000, "512".to_string(), "1024".to_string())
            }
            FrameworkType::Go | FrameworkType::Rust => {
                (8080, "256".to_string(), "512".to_string())
            }
            _ => {
                (8080, "256".to_string(), "512".to_string())
            }
        }
    }
}

impl Default for TerraformService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sanitize_name() {
        let service = TerraformService::new();
        assert_eq!(service.sanitize_name("My-Project!"), "my_project_");
        assert_eq!(service.sanitize_name("test123"), "test123");
        assert_eq!(service.sanitize_name("Project@Name"), "project_name");
    }
    
    #[test]
    fn test_framework_defaults() {
        let (port, cpu, mem) = TerraformService::get_framework_defaults(&FrameworkType::NextJs);
        assert_eq!(port, 3000);
        assert_eq!(cpu, "512");
        assert_eq!(mem, "1024");
        
        let (port, cpu, mem) = TerraformService::get_framework_defaults(&FrameworkType::Python);
        assert_eq!(port, 8000);
        assert_eq!(cpu, "256");
        assert_eq!(mem, "512");
    }
}
