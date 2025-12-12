# Deployotron - Development Context

## Last Session

**Date**: 2025-12-11 14:05  
**Last Command**: "go ahead and build the whole thing"  
**Transcript Location**: `../poetiq/.amplifier/transcripts/transcript_20251211_140555.json`

## What Was Built in Last Session

### ✅ Completed Components

1. **Domain Models** (`src-tauri/src/models.rs`)
   - Project, Deployment, DeploymentConfig structures
   - Environment enum (Staging, Production)
   - DeploymentStatus enum (Pending, Building, Deploying, Running, Failed, Stopped)
   - FrameworkType enum (React, NextJs, Vite, Vue, Angular, Unknown)
   - AWS/Git credentials structures

2. **Database Layer** (`src-tauri/src/infrastructure/database.rs`)
   - SQLite connection and schema creation
   - Projects table (id, name, git_url, framework, timestamps)
   - Deployments table (id, project_id, environment, status, image_uri, service_url, timestamps)
   - CRUD operations for projects and deployments

3. **Keychain Service** (`src-tauri/src/infrastructure/keychain.rs`)
   - OS keychain integration (using keyring crate)
   - Fallback to encrypted file storage
   - AES-256-GCM encryption/decryption
   - Secure credential storage and retrieval

4. **AWS Service (Started)** (`src-tauri/src/services/aws_service.rs`)
   - AWS SDK client initialization (ECS, ECR, CloudWatch)
   - ECR repository creation
   - Mock ECS deployment (needs completion)
   - Mock log retrieval (needs completion)

5. **Infrastructure Module** (`src-tauri/src/infrastructure/mod.rs`)
   - Module exports for database and keychain

## Design Decisions Made

### Why Tauri over Electron?
- Smaller bundle size (~600KB vs ~200MB)
- Better performance (Rust backend vs Node.js)
- Security (Rust memory safety, no Node.js vulnerabilities)
- Native feel (system WebView vs bundled Chromium)

### Why SQLite?
- Desktop app - no server to manage
- Portability - database travels with the app
- Simplicity - no installation needed
- Performance - fast enough for local data

### Why System Keychain?
- OS-level credential protection
- User expectations for security
- Encrypted file fallback for compatibility

### Why Direct AWS SDK vs CLI?
- Performance - no subprocess overhead
- Type safety - Rust catches errors at compile time
- Better error handling - programmatic control
- No dependencies - works without AWS CLI

## File Structure Created

```
deployotron/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs        (needs creation)
│   │   ├── models.rs      ✅ DONE
│   │   ├── infrastructure/
│   │   │   ├── mod.rs     ✅ DONE
│   │   │   ├── database.rs ✅ DONE
│   │   │   └── keychain.rs ✅ DONE
│   │   └── services/
│   │       └── aws_service.rs 🔄 STARTED
│   └── Cargo.toml         (needs creation)
├── src/                   ⏳ NOT STARTED
└── package.json           ⏳ NOT STARTED
```

## What Needs to Be Built Next

### Immediate (Service Layer)
1. Complete AWS service implementation
2. Git service (clone, pull, framework detection)
3. Terraform service (generate configs)
4. Claude service (API integration)

### Then (Application Layer)
1. Tauri commands (expose services to frontend)
2. Deployment orchestrator (coordinate steps)
3. Event system (real-time updates)

### Then (UI)
1. Dashboard with project list
2. Deployment view with logs
3. Chat interface for Claude
4. Settings for credentials

## Dependencies Needed

### Cargo.toml (Rust)
```toml
[dependencies]
tauri = "1.5"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
rusqlite = { version = "0.30", features = ["bundled"] }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.6", features = ["v4", "serde"] }
keyring = "2.0"
ring = "0.17"
base64 = "0.21"
dirs = "5.0"
aws-config = "1.0"
aws-sdk-ecs = "1.0"
aws-sdk-ecr = "1.0"
aws-sdk-cloudwatchlogs = "1.0"
git2 = "0.18"
reqwest = { version = "0.11", features = ["json"] }
```

## Known Issues / TODO

- Database doesn't parse framework/datetime on read (currently uses placeholder values)
- AWS service has mock implementations that need real logic
- No error recovery or retry logic yet
- No logging infrastructure
- No progress updates to UI

## How to Continue

1. Navigate to `/mnt/c/ANext/devops/deployotron`
2. Read TASKS.md for current status
3. Continue with service layer implementation
4. Follow vertical slice approach (complete one feature end-to-end)
5. Test manually after each major component
6. Use the todo tool to track progress within sessions
