use crate::models::{Deployment, DeploymentStatus, Environment, FrameworkType, Project};
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::PathBuf;
use thiserror::Error;

/// Database-specific errors
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Database initialization failed: {0}")]
    InitializationFailed(String),
    
    #[error("Project not found: {0}")]
    ProjectNotFound(String),
    
    #[error("Deployment not found: {0}")]
    DeploymentNotFound(String),
    
    #[error("Database query failed: {0}")]
    QueryFailed(String),
    
    #[error("Data serialization failed: {0}")]
    SerializationFailed(String),
}

impl From<rusqlite::Error> for DatabaseError {
    fn from(err: rusqlite::Error) -> Self {
        DatabaseError::QueryFailed(err.to_string())
    }
}

impl From<serde_json::Error> for DatabaseError {
    fn from(err: serde_json::Error) -> Self {
        DatabaseError::SerializationFailed(err.to_string())
    }
}

/// Database connection wrapper
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Create a new database connection and initialize schema
    pub fn new() -> Result<Self, DatabaseError> {
        let db_path = Self::get_database_path()?;
        
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| DatabaseError::InitializationFailed(e.to_string()))?;
        }
        
        let conn = Connection::open(&db_path)
            .map_err(|e| DatabaseError::ConnectionFailed(e.to_string()))?;
        
        let mut db = Database { conn };
        db.init_database()?;
        
        Ok(db)
    }
    
    /// Get the database file path
    fn get_database_path() -> Result<PathBuf, DatabaseError> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| DatabaseError::InitializationFailed(
                "Could not determine data directory".to_string()
            ))?;
        
        Ok(data_dir.join("deployotron").join("deployotron.db"))
    }
    
    /// Initialize database schema
    fn init_database(&mut self) -> Result<(), DatabaseError> {
        // Enable foreign key constraints
        self.conn.execute("PRAGMA foreign_keys = ON", [])
            .map_err(|e| DatabaseError::InitializationFailed(e.to_string()))?;
        
        // Create projects table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS projects (
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
            )",
            [],
        ).map_err(|e| DatabaseError::InitializationFailed(e.to_string()))?;
        
        // Create deployments table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS deployments (
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
            )",
            [],
        ).map_err(|e| DatabaseError::InitializationFailed(e.to_string()))?;
        
        // Create indexes for common queries
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_deployments_project_id 
             ON deployments(project_id)",
            [],
        ).map_err(|e| DatabaseError::InitializationFailed(e.to_string()))?;
        
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_deployments_status 
             ON deployments(status)",
            [],
        ).map_err(|e| DatabaseError::InitializationFailed(e.to_string()))?;
        
        Ok(())
    }
    
    // ===== Project CRUD Operations =====
    
    /// Create a new project
    pub fn create_project(&self, project: &Project) -> Result<(), DatabaseError> {
        self.conn.execute(
            "INSERT INTO projects (
                id, name, repository_url, branch, framework, environment,
                aws_cluster, aws_service, ecr_repository, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                project.id,
                project.name,
                project.repository_url,
                project.branch,
                serde_json::to_string(&project.framework)?,
                serde_json::to_string(&project.environment)?,
                project.aws_cluster,
                project.aws_service,
                project.ecr_repository,
                project.created_at,
                project.updated_at,
            ],
        )?;
        
        Ok(())
    }
    
    /// Get a project by ID
    pub fn get_project(&self, id: &str) -> Result<Project, DatabaseError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, repository_url, branch, framework, environment,
                    aws_cluster, aws_service, ecr_repository, created_at, updated_at
             FROM projects WHERE id = ?1"
        )?;
        
        let project = stmt.query_row(params![id], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                repository_url: row.get(2)?,
                branch: row.get(3)?,
                framework: serde_json::from_str(&row.get::<_, String>(4)?)
                    .map_err(|e| rusqlite::Error::InvalidColumnType(
                        4, "framework".to_string(), rusqlite::types::Type::Text
                    ))?,
                environment: serde_json::from_str(&row.get::<_, String>(5)?)
                    .map_err(|e| rusqlite::Error::InvalidColumnType(
                        5, "environment".to_string(), rusqlite::types::Type::Text
                    ))?,
                aws_cluster: row.get(6)?,
                aws_service: row.get(7)?,
                ecr_repository: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        }).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                DatabaseError::ProjectNotFound(id.to_string())
            }
            _ => DatabaseError::from(e),
        })?;
        
        Ok(project)
    }
    
    /// Get all projects
    pub fn get_all_projects(&self) -> Result<Vec<Project>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, repository_url, branch, framework, environment,
                    aws_cluster, aws_service, ecr_repository, created_at, updated_at
             FROM projects ORDER BY updated_at DESC"
        )?;
        
        let projects = stmt.query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                repository_url: row.get(2)?,
                branch: row.get(3)?,
                framework: serde_json::from_str(&row.get::<_, String>(4)?)
                    .map_err(|e| rusqlite::Error::InvalidColumnType(
                        4, "framework".to_string(), rusqlite::types::Type::Text
                    ))?,
                environment: serde_json::from_str(&row.get::<_, String>(5)?)
                    .map_err(|e| rusqlite::Error::InvalidColumnType(
                        5, "environment".to_string(), rusqlite::types::Type::Text
                    ))?,
                aws_cluster: row.get(6)?,
                aws_service: row.get(7)?,
                ecr_repository: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })?.collect::<SqliteResult<Vec<_>>>()?;
        
        Ok(projects)
    }
    
    /// Update an existing project
    pub fn update_project(&self, project: &Project) -> Result<(), DatabaseError> {
        let rows_affected = self.conn.execute(
            "UPDATE projects SET 
                name = ?1, repository_url = ?2, branch = ?3, framework = ?4,
                environment = ?5, aws_cluster = ?6, aws_service = ?7,
                ecr_repository = ?8, updated_at = ?9
             WHERE id = ?10",
            params![
                project.name,
                project.repository_url,
                project.branch,
                serde_json::to_string(&project.framework)?,
                serde_json::to_string(&project.environment)?,
                project.aws_cluster,
                project.aws_service,
                project.ecr_repository,
                project.updated_at,
                project.id,
            ],
        )?;
        
        if rows_affected == 0 {
            return Err(DatabaseError::ProjectNotFound(project.id.clone()));
        }
        
        Ok(())
    }
    
    /// Delete a project (and all associated deployments due to CASCADE)
    pub fn delete_project(&self, id: &str) -> Result<(), DatabaseError> {
        let rows_affected = self.conn.execute(
            "DELETE FROM projects WHERE id = ?1",
            params![id],
        )?;
        
        if rows_affected == 0 {
            return Err(DatabaseError::ProjectNotFound(id.to_string()));
        }
        
        Ok(())
    }
    
    // ===== Deployment CRUD Operations =====
    
    /// Create a new deployment
    pub fn create_deployment(&self, deployment: &Deployment) -> Result<(), DatabaseError> {
        self.conn.execute(
            "INSERT INTO deployments (
                id, project_id, status, commit_sha, commit_message,
                image_tag, started_at, completed_at, error_message, logs
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                deployment.id,
                deployment.project_id,
                serde_json::to_string(&deployment.status)?,
                deployment.commit_sha,
                deployment.commit_message,
                deployment.image_tag,
                deployment.started_at,
                deployment.completed_at,
                deployment.error_message,
                deployment.logs,
            ],
        )?;
        
        Ok(())
    }
    
    /// Get a deployment by ID
    pub fn get_deployment(&self, id: &str) -> Result<Deployment, DatabaseError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, status, commit_sha, commit_message,
                    image_tag, started_at, completed_at, error_message, logs
             FROM deployments WHERE id = ?1"
        )?;
        
        let deployment = stmt.query_row(params![id], |row| {
            Ok(Deployment {
                id: row.get(0)?,
                project_id: row.get(1)?,
                status: serde_json::from_str(&row.get::<_, String>(2)?)
                    .map_err(|e| rusqlite::Error::InvalidColumnType(
                        2, "status".to_string(), rusqlite::types::Type::Text
                    ))?,
                commit_sha: row.get(3)?,
                commit_message: row.get(4)?,
                image_tag: row.get(5)?,
                started_at: row.get(6)?,
                completed_at: row.get(7)?,
                error_message: row.get(8)?,
                logs: row.get(9)?,
            })
        }).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                DatabaseError::DeploymentNotFound(id.to_string())
            }
            _ => DatabaseError::from(e),
        })?;
        
        Ok(deployment)
    }
    
    /// Get all deployments for a project
    pub fn get_deployments_for_project(&self, project_id: &str) -> Result<Vec<Deployment>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, status, commit_sha, commit_message,
                    image_tag, started_at, completed_at, error_message, logs
             FROM deployments 
             WHERE project_id = ?1 
             ORDER BY started_at DESC"
        )?;
        
        let deployments = stmt.query_map(params![project_id], |row| {
            Ok(Deployment {
                id: row.get(0)?,
                project_id: row.get(1)?,
                status: serde_json::from_str(&row.get::<_, String>(2)?)
                    .map_err(|e| rusqlite::Error::InvalidColumnType(
                        2, "status".to_string(), rusqlite::types::Type::Text
                    ))?,
                commit_sha: row.get(3)?,
                commit_message: row.get(4)?,
                image_tag: row.get(5)?,
                started_at: row.get(6)?,
                completed_at: row.get(7)?,
                error_message: row.get(8)?,
                logs: row.get(9)?,
            })
        })?.collect::<SqliteResult<Vec<_>>>()?;
        
        Ok(deployments)
    }
    
    /// Get all deployments
    pub fn get_all_deployments(&self) -> Result<Vec<Deployment>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, status, commit_sha, commit_message,
                    image_tag, started_at, completed_at, error_message, logs
             FROM deployments 
             ORDER BY started_at DESC"
        )?;
        
        let deployments = stmt.query_map([], |row| {
            Ok(Deployment {
                id: row.get(0)?,
                project_id: row.get(1)?,
                status: serde_json::from_str(&row.get::<_, String>(2)?)
                    .map_err(|e| rusqlite::Error::InvalidColumnType(
                        2, "status".to_string(), rusqlite::types::Type::Text
                    ))?,
                commit_sha: row.get(3)?,
                commit_message: row.get(4)?,
                image_tag: row.get(5)?,
                started_at: row.get(6)?,
                completed_at: row.get(7)?,
                error_message: row.get(8)?,
                logs: row.get(9)?,
            })
        })?.collect::<SqliteResult<Vec<_>>>()?;
        
        Ok(deployments)
    }
    
    /// Update an existing deployment
    pub fn update_deployment(&self, deployment: &Deployment) -> Result<(), DatabaseError> {
        let rows_affected = self.conn.execute(
            "UPDATE deployments SET 
                status = ?1, commit_message = ?2, completed_at = ?3,
                error_message = ?4, logs = ?5
             WHERE id = ?6",
            params![
                serde_json::to_string(&deployment.status)?,
                deployment.commit_message,
                deployment.completed_at,
                deployment.error_message,
                deployment.logs,
                deployment.id,
            ],
        )?;
        
        if rows_affected == 0 {
            return Err(DatabaseError::DeploymentNotFound(deployment.id.clone()));
        }
        
        Ok(())
    }
    
    /// Delete a deployment
    pub fn delete_deployment(&self, id: &str) -> Result<(), DatabaseError> {
        let rows_affected = self.conn.execute(
            "DELETE FROM deployments WHERE id = ?1",
            params![id],
        )?;
        
        if rows_affected == 0 {
            return Err(DatabaseError::DeploymentNotFound(id.to_string()));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Environment, FrameworkType};

    fn create_test_db() -> Database {
        // Use in-memory database for tests
        let conn = Connection::open_in_memory().unwrap();
        let mut db = Database { conn };
        db.init_database().unwrap();
        db
    }

    #[test]
    fn test_create_and_get_project() {
        let db = create_test_db();
        let project = Project::new(
            "Test Project".to_string(),
            "https://github.com/test/repo".to_string(),
            "main".to_string(),
            FrameworkType::NextJs,
            Environment::Development,
            "test-cluster".to_string(),
            "test-service".to_string(),
            "test.ecr.repo".to_string(),
        );
        
        db.create_project(&project).unwrap();
        let retrieved = db.get_project(&project.id).unwrap();
        
        assert_eq!(retrieved.id, project.id);
        assert_eq!(retrieved.name, project.name);
    }

    #[test]
    fn test_update_project() {
        let db = create_test_db();
        let mut project = Project::new(
            "Test Project".to_string(),
            "https://github.com/test/repo".to_string(),
            "main".to_string(),
            FrameworkType::NextJs,
            Environment::Development,
            "test-cluster".to_string(),
            "test-service".to_string(),
            "test.ecr.repo".to_string(),
        );
        
        db.create_project(&project).unwrap();
        
        project.name = "Updated Project".to_string();
        project.touch();
        db.update_project(&project).unwrap();
        
        let retrieved = db.get_project(&project.id).unwrap();
        assert_eq!(retrieved.name, "Updated Project");
    }

    #[test]
    fn test_delete_project() {
        let db = create_test_db();
        let project = Project::new(
            "Test Project".to_string(),
            "https://github.com/test/repo".to_string(),
            "main".to_string(),
            FrameworkType::NextJs,
            Environment::Development,
            "test-cluster".to_string(),
            "test-service".to_string(),
            "test.ecr.repo".to_string(),
        );
        
        db.create_project(&project).unwrap();
        db.delete_project(&project.id).unwrap();
        
        let result = db.get_project(&project.id);
        assert!(matches!(result, Err(DatabaseError::ProjectNotFound(_))));
    }

    #[test]
    fn test_create_and_get_deployment() {
        let db = create_test_db();
        let project = Project::new(
            "Test Project".to_string(),
            "https://github.com/test/repo".to_string(),
            "main".to_string(),
            FrameworkType::NextJs,
            Environment::Development,
            "test-cluster".to_string(),
            "test-service".to_string(),
            "test.ecr.repo".to_string(),
        );
        
        db.create_project(&project).unwrap();
        
        let deployment = Deployment::new(
            project.id.clone(),
            "abc123".to_string(),
            Some("Test commit".to_string()),
            "v1.0.0".to_string(),
        );
        
        db.create_deployment(&deployment).unwrap();
        let retrieved = db.get_deployment(&deployment.id).unwrap();
        
        assert_eq!(retrieved.id, deployment.id);
        assert_eq!(retrieved.project_id, project.id);
    }

    #[test]
    fn test_get_deployments_for_project() {
        let db = create_test_db();
        let project = Project::new(
            "Test Project".to_string(),
            "https://github.com/test/repo".to_string(),
            "main".to_string(),
            FrameworkType::NextJs,
            Environment::Development,
            "test-cluster".to_string(),
            "test-service".to_string(),
            "test.ecr.repo".to_string(),
        );
        
        db.create_project(&project).unwrap();
        
        let deployment1 = Deployment::new(
            project.id.clone(),
            "abc123".to_string(),
            None,
            "v1.0.0".to_string(),
        );
        
        let deployment2 = Deployment::new(
            project.id.clone(),
            "def456".to_string(),
            None,
            "v1.0.1".to_string(),
        );
        
        db.create_deployment(&deployment1).unwrap();
        db.create_deployment(&deployment2).unwrap();
        
        let deployments = db.get_deployments_for_project(&project.id).unwrap();
        assert_eq!(deployments.len(), 2);
    }

    #[test]
    fn test_cascade_delete() {
        let db = create_test_db();
        let project = Project::new(
            "Test Project".to_string(),
            "https://github.com/test/repo".to_string(),
            "main".to_string(),
            FrameworkType::NextJs,
            Environment::Development,
            "test-cluster".to_string(),
            "test-service".to_string(),
            "test.ecr.repo".to_string(),
        );
        
        db.create_project(&project).unwrap();
        
        let deployment = Deployment::new(
            project.id.clone(),
            "abc123".to_string(),
            None,
            "v1.0.0".to_string(),
        );
        
        db.create_deployment(&deployment).unwrap();
        
        // Delete project should cascade to deployments
        db.delete_project(&project.id).unwrap();
        
        let result = db.get_deployment(&deployment.id);
        assert!(matches!(result, Err(DatabaseError::DeploymentNotFound(_))));
    }
}
