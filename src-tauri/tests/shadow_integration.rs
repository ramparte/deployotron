//! Integration tests for shadow world testing system
//!
//! Tests the complete deployment workflow using mock services without
//! requiring real AWS, Docker, or Git infrastructure.

use std::sync::Arc;
use deployotron::shadow::{ShadowConfig, ShadowState};
use deployotron::services::{factory, AwsOperations, GitOperations, EcsDeploymentConfig};
use deployotron::models::FrameworkType;

/// Test complete ECR + Docker workflow
#[tokio::test]
async fn test_ecr_docker_workflow() {
    let config = ShadowConfig {
        enabled: true,
        failure_rate: 0.0,
        simulate_delays: false,
    };
    let state = Arc::new(ShadowState::new());
    
    let aws = factory::create_aws_operations(Some("us-east-1".into()), &config, state.clone())
        .await
        .unwrap();
    
    // Step 1: Ensure ECR repository
    let repo_uri = aws.ensure_ecr_repository("test-app").await.unwrap();
    assert!(repo_uri.contains("test-app"));
    assert!(repo_uri.contains("us-east-1"));
    
    // Step 2: Docker login
    aws.docker_login_ecr().await.unwrap();
    
    // Step 3: Build Docker image
    let temp_dir = std::env::temp_dir().join("test_build");
    std::fs::create_dir_all(&temp_dir).unwrap();
    
    aws.build_docker_image(
        temp_dir.to_str().unwrap(),
        "test-app:v1",
        &FrameworkType::NextJs
    ).await.unwrap();
    
    // Verify image was tracked
    assert!(state.has_docker_image("test-app:v1"));
    
    // Step 4: Push to ECR
    let ecr_uri = format!("{}:v1", repo_uri);
    aws.push_docker_image("test-app:v1", &ecr_uri).await.unwrap();
    
    // Verify pushed image was tracked
    assert!(state.has_docker_image(&ecr_uri));
    
    // Cleanup
    std::fs::remove_dir_all(&temp_dir).ok();
}

/// Test complete Git workflow
#[tokio::test]
async fn test_git_workflow() {
    let config = ShadowConfig {
        enabled: true,
        failure_rate: 0.0,
        simulate_delays: false,
    };
    let state = Arc::new(ShadowState::new());
    
    let git = factory::create_git_operations(&config, state.clone());
    
    // Step 1: Clone repository
    let repo_path = git.clone_repository(
        "https://github.com/test/nextjs-app",
        "main"
    ).await.unwrap();
    
    assert!(repo_path.exists());
    assert!(repo_path.join("package.json").exists());
    
    // Verify tracked in state
    assert!(state.get_cloned_repo("https://github.com/test/nextjs-app").is_some());
    
    // Step 2: Detect framework
    let framework = git.detect_framework(&repo_path).await.unwrap();
    assert_eq!(framework, FrameworkType::NextJs);
    
    // Step 3: Get commit info
    let commit_info = git.get_commit_info(&repo_path, None).await.unwrap();
    assert!(!commit_info.sha.is_empty());
    assert!(!commit_info.message.is_empty());
    assert_eq!(commit_info.author, "Mock Developer");
    
    // Step 4: Get commit SHA
    let sha = git.get_latest_commit_sha(&repo_path).await.unwrap();
    assert!(!sha.is_empty());
    
    // Step 5: Cleanup
    git.cleanup_repository(&repo_path).await.unwrap();
    assert!(!repo_path.exists());
}

/// Test complete ECS deployment workflow
#[tokio::test]
async fn test_ecs_deployment_workflow() {
    let config = ShadowConfig {
        enabled: true,
        failure_rate: 0.0,
        simulate_delays: false,
    };
    let state = Arc::new(ShadowState::new());
    
    let aws = factory::create_aws_operations(Some("us-east-1".into()), &config, state.clone())
        .await
        .unwrap();
    
    // Step 1: Register task definition
    let ecs_config = EcsDeploymentConfig {
        cluster_name: "test-cluster".to_string(),
        service_name: "test-service".to_string(),
        task_family: "test-task".to_string(),
        container_name: "test-container".to_string(),
        image_uri: "123456.dkr.ecr.us-east-1.amazonaws.com/test-app:v1".to_string(),
        cpu: "256".to_string(),
        memory: "512".to_string(),
        port: 3000,
        desired_count: 1,
    };
    
    let task_arn = aws.register_task_definition(&ecs_config).await.unwrap();
    assert!(task_arn.contains("test-task"));
    assert!(task_arn.starts_with("arn:aws:ecs:"));
    
    // Verify tracked in state
    assert!(state.get_task_definition("test-task").is_some());
    
    // Step 2: Deploy service
    aws.deploy_service(&ecs_config, &task_arn).await.unwrap();
    
    // Step 3: Check service health (should start with pending tasks)
    let health1 = aws.get_service_health("test-cluster", "test-service").await.unwrap();
    assert_eq!(health1.desired_count, 1);
    assert!(!health1.is_healthy); // Initially not healthy
    
    // Step 4: Check again - should progress towards healthy
    let health2 = aws.get_service_health("test-cluster", "test-service").await.unwrap();
    assert!(health2.running_count >= health1.running_count);
    
    // Eventually becomes healthy after polling
    let mut attempts = 0;
    let mut is_healthy = false;
    while attempts < 5 {
        let health = aws.get_service_health("test-cluster", "test-service").await.unwrap();
        if health.is_healthy {
            is_healthy = true;
            break;
        }
        attempts += 1;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
    assert!(is_healthy);
}

/// Test CloudWatch logs
#[tokio::test]
async fn test_cloudwatch_logs() {
    let config = ShadowConfig {
        enabled: true,
        failure_rate: 0.0,
        simulate_delays: false,
    };
    let state = Arc::new(ShadowState::new());
    
    let aws = factory::create_aws_operations(Some("us-east-1".into()), &config, state.clone())
        .await
        .unwrap();
    
    // Fetch logs (should return mock logs)
    let logs = aws.fetch_logs("/ecs/test-task", "stream-1", 10).await.unwrap();
    
    assert!(!logs.is_empty());
    assert!(logs.iter().any(|log| log.contains("Container started") || log.contains("Server listening")));
}

/// Test failure injection
#[tokio::test]
async fn test_failure_injection() {
    let config = ShadowConfig {
        enabled: true,
        failure_rate: 1.0, // Always fail
        simulate_delays: false,
    };
    let state = Arc::new(ShadowState::new());
    
    let aws = factory::create_aws_operations(Some("us-east-1".into()), &config, state.clone())
        .await
        .unwrap();
    
    // Should fail due to failure injection
    let result = aws.ensure_ecr_repository("test-repo").await;
    assert!(result.is_err());
    
    let git = factory::create_git_operations(&config, state.clone());
    let result = git.clone_repository("https://github.com/test/app", "main").await;
    assert!(result.is_err());
}

/// Test full end-to-end deployment simulation
#[tokio::test]
async fn test_full_deployment_simulation() {
    let config = ShadowConfig {
        enabled: true,
        failure_rate: 0.0,
        simulate_delays: false,
    };
    let state = Arc::new(ShadowState::new());
    
    let aws = factory::create_aws_operations(Some("us-east-1".into()), &config, state.clone())
        .await
        .unwrap();
    let git = factory::create_git_operations(&config, state.clone());
    
    // Simulate full deployment workflow
    
    // 1. Clone repository
    let repo_path = git.clone_repository("https://github.com/test/nextjs-app", "main").await.unwrap();
    
    // 2. Detect framework
    let framework = git.detect_framework(&repo_path).await.unwrap();
    
    // 3. Get commit info
    let commit_info = git.get_commit_info(&repo_path, None).await.unwrap();
    let image_tag = format!("test-app:{}", &commit_info.sha[..8]);
    
    // 4. Build Docker image
    aws.build_docker_image(
        repo_path.to_str().unwrap(),
        &image_tag,
        &framework
    ).await.unwrap();
    
    // 5. Ensure ECR repository
    let repo_uri = aws.ensure_ecr_repository("test-app").await.unwrap();
    
    // 6. Login to ECR
    aws.docker_login_ecr().await.unwrap();
    
    // 7. Push to ECR
    let ecr_uri = format!("{}:{}", repo_uri, &commit_info.sha[..8]);
    aws.push_docker_image(&image_tag, &ecr_uri).await.unwrap();
    
    // 8. Register task definition
    let ecs_config = EcsDeploymentConfig {
        cluster_name: "prod-cluster".to_string(),
        service_name: "prod-service".to_string(),
        task_family: "test-app-task".to_string(),
        container_name: "test-app-container".to_string(),
        image_uri: ecr_uri,
        cpu: "512".to_string(),
        memory: "1024".to_string(),
        port: 3000,
        desired_count: 2,
    };
    
    let task_arn = aws.register_task_definition(&ecs_config).await.unwrap();
    
    // 9. Deploy service
    aws.deploy_service(&ecs_config, &task_arn).await.unwrap();
    
    // 10. Monitor until healthy
    let mut attempts = 0;
    let mut is_healthy = false;
    while attempts < 10 {
        let health = aws.get_service_health("prod-cluster", "prod-service").await.unwrap();
        if health.is_healthy {
            is_healthy = true;
            break;
        }
        attempts += 1;
    }
    assert!(is_healthy);
    
    // 11. Fetch logs
    let logs = aws.fetch_logs("/ecs/test-app-task", "stream-1", 50).await.unwrap();
    assert!(!logs.is_empty());
    
    // 12. Cleanup
    git.cleanup_repository(&repo_path).await.unwrap();
    
    // Verify state tracking
    assert!(state.has_docker_image(&image_tag));
    assert!(state.get_ecr_repository("test-app").is_some());
    assert!(state.get_task_definition("test-app-task").is_some());
}

/// Test state reset functionality
#[tokio::test]
async fn test_state_reset() {
    let config = ShadowConfig {
        enabled: true,
        failure_rate: 0.0,
        simulate_delays: false,
    };
    let state = Arc::new(ShadowState::new());
    
    let aws = factory::create_aws_operations(Some("us-east-1".into()), &config, state.clone())
        .await
        .unwrap();
    
    // Add some state
    aws.ensure_ecr_repository("test-repo").await.unwrap();
    assert!(state.get_ecr_repository("test-repo").is_some());
    
    // Reset state
    state.reset();
    
    // Verify state was cleared
    assert!(state.get_ecr_repository("test-repo").is_none());
}

/// Test multiple framework types
#[tokio::test]
async fn test_multiple_frameworks() {
    let config = ShadowConfig {
        enabled: true,
        failure_rate: 0.0,
        simulate_delays: false,
    };
    let state = Arc::new(ShadowState::new());
    
    let git = factory::create_git_operations(&config, state.clone());
    
    // Test different framework types
    let test_cases = vec![
        ("https://github.com/test/nextjs-app", FrameworkType::NextJs),
        ("https://github.com/test/react-app", FrameworkType::React),
        ("https://github.com/test/vue-app", FrameworkType::Vue),
        ("https://github.com/test/python-app", FrameworkType::Python),
        ("https://github.com/test/go-app", FrameworkType::Go),
    ];
    
    for (url, expected_framework) in test_cases {
        let repo_path = git.clone_repository(url, "main").await.unwrap();
        let framework = git.detect_framework(&repo_path).await.unwrap();
        assert_eq!(framework, expected_framework);
        git.cleanup_repository(&repo_path).await.unwrap();
    }
}
