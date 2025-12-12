//! Infrastructure layer for Deployotron
//! 
//! This module provides the foundational infrastructure services:
//! - Database: SQLite-based persistent storage for projects and deployments
//! - KeychainService: Secure credential storage using OS keychain with encrypted fallback

pub mod database;
pub mod keychain;

pub use database::{Database, DatabaseError};
pub use keychain::{KeychainService, KeychainError};
