//! Application layer for Deployotron
//!
//! This module provides the application layer components:
//! - commands: Tauri command handlers for frontend communication
//! - orchestrator: Deployment workflow orchestration

pub mod commands;
pub mod orchestrator;

pub use commands::{AppState, CredentialsStatus, ClaudeResponseDto};
pub use orchestrator::{DeploymentOrchestrator, OrchestratorError};
