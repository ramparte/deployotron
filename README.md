# Deployotron

A desktop application for deploying web applications to AWS with AI assistance.

## Overview

Deployotron is a Tauri-based (Rust + React) desktop application that simplifies deploying web applications to AWS with features like:

- 🚀 One-click deployments to AWS ECS
- 🤖 AI-powered deployment assistance with Claude
- 🔐 Secure credential management
- 📊 Real-time deployment monitoring
- 🏗️ Automatic infrastructure generation with Terraform

## Technology Stack

**Backend (Rust)**
- Tauri: Native desktop app framework
- SQLite: Local database
- AWS SDK: Direct integration with ECS, ECR, CloudWatch
- keyring: Secure credential storage
- ring: Cryptography

**Frontend (React)**
- React + TypeScript
- Tailwind CSS (planned)
- React Query (planned)

## Status

**Current Phase**: Infrastructure Layer Implementation

See [TASKS.md](./TASKS.md) for detailed task list and progress.

## Architecture

```
┌─────────────────────────────────────────┐
│           React Frontend                │
│  (Dashboard, Deployments, Chat UI)      │
└────────────────┬────────────────────────┘
                 │ Tauri IPC
┌────────────────┴────────────────────────┐
│      Application Layer (Rust)           │
│  - Deployment Orchestrator              │
│  - Tauri Commands                       │
└──┬──────────┬──────────┬────────────────┘
   │          │          │
┌──┴───┐  ┌──┴───┐  ┌───┴──┐
│ AWS  │  │ Git  │  │Claude│
│Service│  │Service│  │Service│
└──┬───┘  └──┬───┘  └───┬──┘
   │          │          │
┌──┴──────────┴──────────┴────────────────┐
│      Infrastructure Layer (Rust)         │
│  - SQLite Database                       │
│  - Keychain/Encrypted Credentials        │
└──────────────────────────────────────────┘
```

## Features (Planned MVP)

1. **Project Management**: Import from Git, auto-detect framework, store configs
2. **Deployment Pipeline**: Build Docker images, push to ECR, deploy to ECS, monitor progress
3. **AI Assistant**: Chat with Claude, generate Terraform configs, troubleshoot issues
4. **Credential Management**: Secure AWS credentials, SSH keys, API keys

## Security

- Credentials stored in OS keychain when available
- Fallback to AES-256-GCM encrypted storage
- No credentials in logs or UI
- Secure IPC between frontend and backend
