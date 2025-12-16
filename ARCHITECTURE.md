# Deployotron Architecture Documentation

**Version**: 0.1.0  
**Last Updated**: 2025-12-15  
**Status**: MVP Complete (~85%)

## Table of Contents

1. [Overview](#overview)
2. [System Architecture](#system-architecture)
3. [Technology Stack](#technology-stack)
4. [Core Components](#core-components)
5. [Data Models](#data-models)
6. [API Surface](#api-surface)
7. [Deployment Workflow](#deployment-workflow)
8. [Security Model](#security-model)
9. [Testing Strategy](#testing-strategy)
10. [Development Guidelines](#development-guidelines)

---

## Overview

Deployotron is a desktop application built with Tauri (Rust backend + React frontend) that simplifies deploying web applications to AWS ECS. It provides:

- **One-click deployments** to AWS ECS with real-time progress tracking
- **AI-powered assistance** using Claude for deployment troubleshooting
- **Secure credential management** using OS keychain with encrypted fallback
- **Framework detection** for 9+ web frameworks (React, Next.js, Vue, etc.)
- **Infrastructure as Code** generation (Terraform configurations)
- **Deployment history** and log management

### Design Philosophy

Deployotron follows the principles outlined in `IMPLEMENTATION_PHILOSOPHY.md`:

- **Ruthless Simplicity**: Every component does one thing well
- **Minimal Abstractions**: No unnecessary layers or indirection
- **Direct Integration**: Uses AWS SDK directly, not CLI wrappers
- **Pragmatic Error Handling**: Handles common cases, fails fast on edge cases
- **Modular Architecture**: Clear separation between layers

---

## System Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────┐
│         React Frontend (TypeScript)         │
│  - Dashboard UI                             │
│  - Deployment Monitor                       │
│  - Settings & Credentials                   │
│  - AI Chat Interface                        │
└───────────────────┬─────────────────────────┘
                    │ Tauri IPC (JSON-RPC)
┌───────────────────┴─────────────────────────┐
│      Application Layer (Rust)               │
│  - Tauri Commands (16 endpoints)            │
│  - Deployment Orchestrator                  │
│  - Event Emission System                    │
└─────┬────────────┬─────────────┬────────────┘
      │            │             │
┌─────┴─────┐ ┌───┴──────┐ ┌───┴──────────┐
│  Service  │ │ Service  │ │   Service    │
│   Layer   │ │  Layer   │ │    Layer     │
│           │ │          │ │              │
│ AWS       │ │ Git      │ │ Claude       │
│ Service   │ │ Service  │ │ Service      │
│           │ │          │ │              │
│ Terraform │ │          │ │              │
│ Service   │ │          │ │              │
└─────┬─────┘ └────┬─────┘ └──────┬───────┘
      │            │               │
┌─────┴────────────┴───────────────┴─────────┐
│     Infrastructure Layer (Rust)             │
│  - SQLite Database                          │
│  - Keychain Service (OS + Encrypted)        │
│  - File System Operations                   │
└─────────────────────────────────────────────┘
```

### Layer Responsibilities

#### 1. **Frontend Layer (React)**
- User interaction and visualization
- Form validation and state management
- Real-time deployment progress display
- Tauri command invocation via `@tauri-apps/api`

#### 2. **Application Layer (Rust)**
- **Tauri Commands**: 16 command handlers exposing backend to frontend
- **Orchestrator**: Coordinates 10-step deployment workflow
- **Event System**: Emits progress events to frontend via Tauri events
- **State Management**: Thread-safe shared state (`AppState`)

#### 3. **Service Layer (Rust)**
- **AwsService**: ECS/ECR operations, Docker build/push, health monitoring
- **GitService**: Repository cloning, framework detection, commit info
- **TerraformService**: Infrastructure-as-Code generation
- **ClaudeService**: AI assistance and log analysis

#### 4. **Infrastructure Layer (Rust)**
- **Database**: SQLite with thread-safe operations (projects, deployments)
- **KeychainService**: Secure credential storage (AWS, Git)
- **Models**: Domain models with validation and serialization

---

## Technology Stack

### Backend (Rust)

| Category | Technology | Version | Purpose |
|----------|-----------|---------|---------|
| Framework | Tauri | 1.5 | Desktop app framework |
| Database | rusqlite | 0.30 | Local SQLite database |
| Security | keyring | 2.0 | OS keychain integration |
| Security | ring | 0.17 | Cryptographic operations |
| AWS | aws-sdk-ecs | 1.0 | ECS service management |
| AWS | aws-sdk-ecr | 1.0 | Container registry |
| AWS | aws-sdk-cloudwatchlogs | 1.0 | Log retrieval |
| Git | git2 | 0.18 | Git operations |
| HTTP | reqwest | 0.11 | Claude API client |
| Async | tokio | 1.0 | Async runtime |
| Serialization | serde | 1.0 | JSON serialization |
| Error Handling | anyhow, thiserror | 1.0 | Error management |

### Frontend (React)

| Category | Technology | Version | Purpose |
|----------|-----------|---------|---------|
| Framework | React | 18.2 | UI framework |
| Language | TypeScript | 5.3 | Type safety |
| Bundler | Vite | 5.0 | Fast dev server & builds |
| Tauri API | @tauri-apps/api | 1.5 | Frontend-backend bridge |
| Styling | CSS | - | Custom dark theme |

---

## Core Components

### 1. Application State (`AppState`)

Thread-safe shared state accessible to all Tauri commands:

```rust
pub struct AppState {
    pub database: Arc<Mutex<Database>>,
    pub keychain: Arc<Mutex<KeychainService>>,
    pub git_service: Arc<GitService>,
    pub terraform_service: Arc<TerraformService>,
}
```

### 2. Deployment Orchestrator

Coordinates the 10-step deployment workflow:

```rust
pub struct DeploymentOrchestrator {
    database: Arc<Mutex<Database>>,
    git_service: Arc<GitService>,
    aws_service: Arc<AwsService>,
    terraform_service: Arc<TerraformService>,
    window: Window,  // For event emission
}
```

**10-Step Workflow:**
1. Initialize deployment record (0-10%)
2. Clone Git repository (10-20%)
3. Detect framework type (20-25%)
4. Get commit information (25-30%)
5. Build Docker image (30-50%)
6. ECR authentication (50-55%)
7. Push to ECR (55-70%)
8. Register ECS task definition (70-80%)
9. Deploy ECS service (80-90%)
10. Monitor health until running (90-100%)

### 3. Services

#### AWS Service (`AwsService`)

```rust
pub struct AwsService {
    ecs_client: aws_sdk_ecs::Client,
    ecr_client: aws_sdk_ecr::Client,
    logs_client: aws_sdk_cloudwatchlogs::Client,
}
```

**Key Methods:**
- `build_docker_image()` - Build image with framework-specific Dockerfile
- `docker_login_ecr()` - Authenticate Docker with ECR
- `push_docker_image()` - Tag and push to ECR
- `register_task_definition()` - Create ECS task definition
- `deploy_service()` - Update ECS service with new task
- `get_service_health()` - Poll service health status

#### Git Service (`GitService`)

```rust
pub struct GitService {}
```

**Key Methods:**
- `clone_repository()` - Clone repo to temp directory
- `detect_framework()` - Auto-detect framework from files
- `get_commit_info()` - Get latest commit SHA and message
- `cleanup_repository()` - Remove cloned directory

**Framework Detection:**
Checks for presence of:
- `next.config.js` → Next.js
- `package.json` with "react" → React
- `vue.config.js` → Vue
- `angular.json` → Angular
- `Cargo.toml` → Rust
- `go.mod` → Go
- etc.

#### Terraform Service (`TerraformService`)

```rust
pub struct TerraformService {}
```

**Key Methods:**
- `generate_config()` - Create complete Terraform configuration

**Generated Files:**
- `main.tf` - ECS cluster, service, task definition, security groups
- `variables.tf` - Configurable parameters
- `outputs.tf` - Service URL, task definition ARN

#### Claude Service (`ClaudeService`)

```rust
pub struct ClaudeService {
    api_key: String,
    client: reqwest::Client,
}
```

**Key Methods:**
- `ask_question()` - General deployment questions
- `analyze_logs()` - Analyze deployment logs for issues

---

## Data Models

### Project

```rust
pub struct Project {
    pub id: String,                    // UUID v4
    pub name: String,                  // Human-readable name
    pub repository_url: String,        // Git URL
    pub branch: String,                // Branch to deploy
    pub framework: FrameworkType,      // Detected framework
    pub environment: Environment,      // staging/production
    pub aws_cluster: String,           // ECS cluster name
    pub aws_service: String,           // ECS service name
    pub ecr_repository: String,        // ECR repo URI
    pub created_at: i64,               // Unix timestamp
    pub updated_at: i64,               // Unix timestamp
}
```

### Deployment

```rust
pub struct Deployment {
    pub id: String,                    // UUID v4
    pub project_id: String,            // Foreign key
    pub status: DeploymentStatus,      // pending/inprogress/success/failed
    pub commit_sha: String,            // Git commit SHA
    pub commit_message: Option<String>,// Commit message
    pub image_tag: String,             // Docker image tag
    pub started_at: i64,               // Unix timestamp
    pub completed_at: Option<i64>,     // Unix timestamp (if complete)
    pub error_message: Option<String>, // Error details
    pub logs: Option<String>,          // Deployment logs
}
```

### Enums

```rust
pub enum Environment {
    Development,
    Staging,
    Production,
}

pub enum DeploymentStatus {
    Pending,
    InProgress,
    Success,
    Failed,
    RolledBack,
}

pub enum FrameworkType {
    NextJs, React, Vue, Angular,
    Node, Python, Ruby, Go, Rust, Other,
}
```

---

## API Surface

### Tauri Commands (16 Total)

#### Project Commands (5)

```rust
create_project(name, repository_url, branch, framework, environment, 
              aws_cluster, aws_service, ecr_repository) -> Result<Project, String>

get_projects() -> Result<Vec<Project>, String>

get_project(project_id) -> Result<Project, String>

update_project(project) -> Result<(), String>

delete_project(project_id) -> Result<(), String>
```

#### Deployment Commands (4)

```rust
start_deployment(project_id) -> Result<String, String>  // Returns deployment_id

get_deployment_status(deployment_id) -> Result<Deployment, String>

get_project_deployments(project_id) -> Result<Vec<Deployment>, String>

get_deployment_logs(deployment_id) -> Result<String, String>
```

#### Credential Commands (5)

```rust
store_aws_credentials(access_key_id, secret_access_key, region) -> Result<(), String>

store_git_credentials(username, token, provider) -> Result<(), String>

get_credentials_status() -> Result<CredentialsStatus, String>

delete_aws_credentials() -> Result<(), String>

delete_git_credentials() -> Result<(), String>
```

#### AI Commands (2)

```rust
ask_claude(question, project_id?, api_key) -> Result<ClaudeResponseDto, String>

analyze_deployment_logs(deployment_id, api_key) -> Result<ClaudeResponseDto, String>
```

### Frontend API Usage

```typescript
import { invoke } from '@tauri-apps/api/tauri';

// Create project
const project = await invoke('create_project', {
  name: 'My App',
  repositoryUrl: 'https://github.com/user/repo',
  branch: 'main',
  framework: 'nextjs',
  environment: 'staging',
  awsCluster: 'my-cluster',
  awsService: 'my-service',
  ecrRepository: '123456.dkr.ecr.us-east-1.amazonaws.com/my-repo'
});

// Start deployment
const deploymentId = await invoke('start_deployment', {
  projectId: project.id
});

// Listen for progress events
import { listen } from '@tauri-apps/api/event';

await listen('deployment-progress', (event) => {
  const { deployment_id, step, progress, message } = event.payload;
  console.log(`${progress}%: ${message}`);
});
```

---

## Deployment Workflow

### Detailed Step-by-Step Flow

```
User Clicks "Deploy to Staging"
         │
         ▼
┌────────────────────────────────┐
│ 1. Initialize Deployment       │
│    - Create DB record          │
│    - Status: Pending           │
│    - Progress: 0-10%           │
└────────────┬───────────────────┘
             ▼
┌────────────────────────────────┐
│ 2. Clone Repository            │
│    - Git clone to temp dir     │
│    - Checkout specified branch │
│    - Progress: 10-20%          │
└────────────┬───────────────────┘
             ▼
┌────────────────────────────────┐
│ 3. Detect Framework            │
│    - Scan for config files     │
│    - Determine framework type  │
│    - Progress: 20-25%          │
└────────────┬───────────────────┘
             ▼
┌────────────────────────────────┐
│ 4. Get Commit Info             │
│    - Extract commit SHA        │
│    - Get commit message        │
│    - Update deployment record  │
│    - Progress: 25-30%          │
└────────────┬───────────────────┘
             ▼
┌────────────────────────────────┐
│ 5. Build Docker Image          │
│    - Generate Dockerfile       │
│    - Run docker build          │
│    - Tag: projectname:sha      │
│    - Progress: 30-50%          │
└────────────┬───────────────────┘
             ▼
┌────────────────────────────────┐
│ 6. Authenticate with ECR       │
│    - Get ECR login token       │
│    - Docker login              │
│    - Progress: 50-55%          │
└────────────┬───────────────────┘
             ▼
┌────────────────────────────────┐
│ 7. Push to ECR                 │
│    - Tag with ECR URI          │
│    - Docker push               │
│    - Progress: 55-70%          │
└────────────┬───────────────────┘
             ▼
┌────────────────────────────────┐
│ 8. Register Task Definition    │
│    - Create ECS task def       │
│    - Set CPU/memory/port       │
│    - Return task ARN           │
│    - Progress: 70-80%          │
└────────────┬───────────────────┘
             ▼
┌────────────────────────────────┐
│ 9. Deploy to ECS Service       │
│    - Update service            │
│    - Use new task definition   │
│    - Progress: 80-90%          │
└────────────┬───────────────────┘
             ▼
┌────────────────────────────────┐
│ 10. Monitor Health             │
│     - Poll service status      │
│     - Wait for healthy (5 min) │
│     - Track running tasks      │
│     - Progress: 90-100%        │
└────────────┬───────────────────┘
             ▼
┌────────────────────────────────┐
│ Cleanup & Complete             │
│ - Remove temp repository       │
│ - Mark deployment: Success     │
│ - Emit 100% progress event     │
└────────────────────────────────┘
```

### Error Handling

Each step has error handling:
- On error: Update deployment status to `Failed`
- Store error message in deployment record
- Cleanup temporary resources (repo, Docker images)
- Emit failure event to frontend
- Return error to user

---

## Security Model

### Credential Storage

**Priority Order:**
1. **OS Keychain** (macOS Keychain, Windows Credential Manager, Linux Secret Service)
2. **Encrypted File Fallback** (AES-256-GCM)

```rust
// Storage locations
- macOS: Keychain Access
- Windows: Credential Manager
- Linux: Secret Service API

// Fallback location
~/.deployotron/credentials.enc
```

### Credential Types

```rust
// AWS Credentials
{
  "access_key_id": "AKIA...",
  "secret_access_key": "...",
  "region": "us-east-1"
}

// Git Credentials
{
  "username": "user",
  "token": "ghp_...",
  "provider": "github"
}
```

### Security Best Practices

- ✅ No credentials in logs
- ✅ No credentials in UI (masked input fields)
- ✅ No credentials in git repository
- ✅ Secure IPC channel (Tauri built-in)
- ✅ Database file permissions (600)
- ✅ Encrypted credential file (if keychain unavailable)

---

## Testing Strategy

### Current Test Coverage

| Layer | Coverage | Test Types |
|-------|----------|------------|
| Models | 100% | Unit tests (4) |
| Database | 95% | Integration tests (7) |
| Keychain | 100% | Security tests (4) |
| Git Service | 80% | Unit tests (5) |
| Terraform Service | 70% | Unit tests (2) |
| Claude Service | 90% | Unit tests (3) |
| AWS Service | 0% | Manual testing only |
| Orchestrator | 0% | Manual testing only |
| Commands | 0% | Manual testing only |

### Testing Approach

```rust
// Unit tests
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_project_creation() { ... }
}

// Integration tests
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_database_crud() { ... }
}
```

### Manual Testing

Currently requires:
- Docker installed and running
- AWS credentials configured
- Git repository accessible
- ECS cluster and ECR repository pre-created

---

## Development Guidelines

### Code Organization

```
src-tauri/src/
├── main.rs              # Entry point, Tauri setup
├── models.rs            # Domain models
├── infrastructure/
│   ├── mod.rs
│   ├── database.rs      # SQLite operations
│   └── keychain.rs      # Credential storage
├── services/
│   ├── mod.rs
│   ├── aws_service.rs   # AWS SDK operations
│   ├── git_service.rs   # Git operations
│   ├── terraform_service.rs
│   └── claude_service.rs
└── application/
    ├── mod.rs
    ├── commands.rs      # Tauri command handlers
    └── orchestrator.rs  # Deployment workflow
```

### Adding New Features

1. **Define Model** (if needed) in `models.rs`
2. **Add Service Method** in appropriate service
3. **Create Tauri Command** in `commands.rs`
4. **Expose in main.rs** via `invoke_handler!`
5. **Add Frontend Call** in React components
6. **Write Tests** for new functionality

### Error Handling Pattern

```rust
// Services return Result with custom errors
pub enum ServiceError {
    #[error("...")]
    OperationFailed(String),
}

// Commands convert to String for Tauri IPC
#[tauri::command]
pub async fn do_something() -> Result<T, String> {
    service.operation()
        .map_err(|e| format!("Operation failed: {}", e))
}
```

### Async Operations

```rust
// All I/O operations are async
pub async fn clone_repository(&self, url: &str) -> Result<PathBuf> {
    tokio::task::spawn_blocking(move || {
        // Blocking git operation
    }).await?
}
```

---

## Database Schema

### Projects Table

```sql
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    repository_url TEXT NOT NULL,
    branch TEXT NOT NULL,
    framework TEXT NOT NULL,
    environment TEXT NOT NULL,
    aws_cluster TEXT NOT NULL,
    aws_service TEXT NOT NULL,
    ecr_repository TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
```

### Deployments Table

```sql
CREATE TABLE deployments (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    status TEXT NOT NULL,
    commit_sha TEXT NOT NULL,
    commit_message TEXT,
    image_tag TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    completed_at INTEGER,
    error_message TEXT,
    logs TEXT,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);
```

---

## Configuration Files

### Tauri Configuration (`tauri.conf.json`)

```json
{
  "build": {
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build",
    "devPath": "http://localhost:5173",
    "distDir": "../dist"
  },
  "package": {
    "productName": "Deployotron",
    "version": "0.1.0"
  },
  "tauri": {
    "allowlist": {
      "all": false,
      "shell": { "open": true }
    }
  }
}
```

### Cargo Dependencies (`Cargo.toml`)

See full dependency list in [Technology Stack](#technology-stack).

---

## Future Enhancements

### Planned Features

1. **Rollback Support** - Revert to previous deployment
2. **Multi-Region** - Deploy to multiple AWS regions
3. **Blue-Green Deployments** - Zero-downtime deployments
4. **Cost Monitoring** - Track AWS costs per project
5. **Team Collaboration** - Share projects with team members
6. **CI/CD Integration** - Trigger from GitHub Actions
7. **Kubernetes Support** - Deploy to EKS clusters
8. **Custom Docker Images** - Support custom Dockerfiles

### Technical Improvements

1. **Comprehensive Testing** - Unit + integration + E2E tests
2. **Performance Optimization** - Parallel deployments, caching
3. **Enhanced UI** - Real-time logs, deployment history timeline
4. **Better Error Recovery** - Auto-retry, partial rollback
5. **Observability** - Structured logging, metrics

---

## Appendix

### File Structure

```
deployotron/
├── src/                    # React frontend
│   ├── App.tsx
│   ├── Dashboard.tsx
│   └── main.tsx
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── main.rs
│   │   ├── models.rs
│   │   ├── infrastructure/
│   │   ├── services/
│   │   └── application/
│   ├── Cargo.toml
│   └── tauri.conf.json
├── .github/
│   └── workflows/
│       └── build-release.yml
├── ARCHITECTURE.md        # This file
├── README.md
├── PROJECT_SUMMARY.md
├── QUICKSTART.md
├── CHANGELOG.md
└── package.json
```

### Build Commands

```bash
# Development
npm run tauri:dev

# Production build
npm run tauri:build

# Run tests
cd src-tauri && cargo test

# Format code
cargo fmt
cd .. && npm run format
```

### Useful Resources

- [Tauri Documentation](https://tauri.app/)
- [AWS SDK for Rust](https://docs.aws.amazon.com/sdk-for-rust/)
- [git2-rs Documentation](https://docs.rs/git2/)
- [Anthropic Claude API](https://docs.anthropic.com/)

---

**End of Architecture Documentation**
