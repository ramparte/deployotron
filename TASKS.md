# Deployotron MVP - Task List

**Project**: DevOps deployment tool using Tauri (Rust + React) + AWS  
**Status**: In Progress - Infrastructure Layer  
**Last Updated**: 2025-12-11  
**Previous Session Transcript**: `../poetiq/.amplifier/transcripts/transcript_20251211_140555.json`

## Project Overview

Deployotron is a desktop application for deploying web applications to AWS with AI assistance using:
- **Backend**: Tauri (Rust) for native performance and security
- **Frontend**: React with TypeScript
- **Infrastructure**: AWS (ECS, ECR, CloudWatch)
- **AI**: Claude for deployment assistance
- **Storage**: SQLite for project data, system keychain/encrypted storage for credentials

## Current Task List

### ✅ 1. Create Tauri structure and models
**Status**: COMPLETED
- Core domain models (Project, Deployment, Environment, etc.)
- Framework detection types (React, Next.js, Vite, etc.)
- Status enums and data structures

### 🔄 2. Build infrastructure layer (SQLite, keychain, crypto)
**Status**: IN PROGRESS
- ✅ SQLite database implementation (projects, deployments tables)
- ✅ Keychain service with fallback to encrypted storage
- ✅ AES-256-GCM encryption for credential storage
- 🔄 Module exports and integration

### ⏳ 3. Build service layer (AWS, Git, Terraform, Claude, SSH)
**Status**: PENDING
- AWS service (ECS, ECR, CloudWatch integration)
- Git service (clone, pull, detect framework)
- Terraform service (generate and apply configurations)
- Claude AI service (deployment assistance)
- SSH service (remote server management)

### ⏳ 4. Build application layer (commands, orchestrator)
**Status**: PENDING
- Tauri commands (expose services to frontend)
- Deployment orchestrator (coordinate deployment steps)
- State management
- Event emitters for real-time updates

### ⏳ 5. Build React UI (dashboard, deployments, chat)
**Status**: PENDING
- Dashboard view (project list, quick actions)
- Deployment view (logs, status, controls)
- Chat interface (Claude AI assistance)
- Settings/credentials management

### ⏳ 6. Create configuration files and integrate all layers
**Status**: PENDING
- Cargo.toml with all dependencies
- tauri.conf.json
- package.json for React
- TypeScript configuration

### ⏳ 7. Test end-to-end and fix issues
**Status**: PENDING
- Unit tests for core services
- Integration tests
- End-to-end workflow testing

### ⏳ 8. Evaluate and analyze with quality tools
**Status**: PENDING
- Run solution-evaluator agent
- Run code-quality analysis
- Security review

## Key Features

1. **Project Management**: Add Git projects, auto-detect framework, store configs
2. **Deployment Pipeline**: Build Docker images, push to ECR, deploy to ECS, monitor status
3. **AI Assistance**: Chat with Claude, generate Terraform configs, troubleshoot issues
4. **Security**: Secure credential storage (keychain or encrypted)

## Next Immediate Steps

1. Complete AWS service implementation
2. Implement Git service
3. Create Terraform service
4. Set up Tauri commands
5. Build React components
