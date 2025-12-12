//! Claude AI assistance service
//!
//! Provides functionality for:
//! - Initializing Anthropic API client
//! - Sending questions with deployment context
//! - Analyzing logs and suggesting fixes
//! - Using Claude 3.5 Sonnet model

use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Claude service specific errors
#[derive(Error, Debug)]
pub enum ClaudeServiceError {
    #[error("Failed to initialize Claude client: {0}")]
    InitializationFailed(String),
    
    #[error("API request failed: {0}")]
    RequestFailed(String),
    
    #[error("Invalid API response: {0}")]
    InvalidResponse(String),
    
    #[error("API key not configured")]
    ApiKeyMissing,
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
}

impl From<reqwest::Error> for ClaudeServiceError {
    fn from(err: reqwest::Error) -> Self {
        ClaudeServiceError::RequestFailed(err.to_string())
    }
}

/// Claude AI service for deployment assistance
pub struct ClaudeService {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
}

/// Request to Claude API
#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
    system: Option<String>,
}

/// Message in Claude conversation
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Message {
    role: String,
    content: String,
}

/// Response from Claude API
#[derive(Debug, Deserialize)]
struct ClaudeApiResponse {
    id: String,
    #[serde(rename = "type")]
    response_type: String,
    role: String,
    content: Vec<ContentBlock>,
    model: String,
    stop_reason: Option<String>,
}

/// Content block in Claude response
#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

/// Deployment context for Claude
#[derive(Debug, Clone)]
pub struct DeploymentContext {
    pub project_name: String,
    pub framework: String,
    pub environment: String,
    pub cluster_name: String,
    pub service_name: String,
    pub commit_sha: String,
    pub error_message: Option<String>,
    pub logs: Option<Vec<String>>,
}

/// Claude response with suggestion
#[derive(Debug, Clone)]
pub struct ClaudeApiResponse {
    pub answer: String,
    pub suggestions: Vec<String>,
}

impl ClaudeService {
    /// Create a new ClaudeService instance
    pub fn new(api_key: String) -> Result<Self, ClaudeServiceError> {
        if api_key.is_empty() {
            return Err(ClaudeServiceError::ApiKeyMissing);
        }
        
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| ClaudeServiceError::InitializationFailed(e.to_string()))?;
        
        Ok(Self {
            client,
            api_key,
            base_url: "https://api.anthropic.com/v1".to_string(),
            model: "claude-3-5-sonnet-20241022".to_string(),
        })
    }
    
    /// Ask Claude a question about deployment
    pub async fn ask_question(&self, question: &str, context: Option<&DeploymentContext>) -> Result<ClaudeResponse, ClaudeServiceError> {
        let system_prompt = self.build_system_prompt();
        let user_message = self.build_user_message(question, context);
        
        let response_text = self.send_request(&system_prompt, &user_message).await?;
        
        Ok(ClaudeResponse {
            answer: response_text.clone(),
            suggestions: self.extract_suggestions(&response_text),
        })
    }
    
    /// Analyze deployment logs and suggest fixes
    pub async fn analyze_logs(&self, logs: &[String], error_message: Option<&str>, context: &DeploymentContext) -> Result<ClaudeResponse, ClaudeServiceError> {
        let system_prompt = "You are an expert DevOps engineer helping debug deployment issues. \
                            Analyze the provided logs and error messages, then suggest specific fixes.";
        
        let mut user_message = format!(
            "Project: {}\nFramework: {}\nEnvironment: {}\nCluster: {}\nService: {}\nCommit: {}\n\n",
            context.project_name,
            context.framework,
            context.environment,
            context.cluster_name,
            context.service_name,
            context.commit_sha
        );
        
        if let Some(error) = error_message {
            user_message.push_str(&format!("Error Message:\n{}\n\n", error));
        }
        
        user_message.push_str("Recent Logs:\n");
        for (i, log) in logs.iter().take(50).enumerate() {
            user_message.push_str(&format!("{}: {}\n", i + 1, log));
        }
        
        user_message.push_str("\nPlease analyze these logs and:\n");
        user_message.push_str("1. Identify the root cause of the issue\n");
        user_message.push_str("2. Suggest specific fixes or configuration changes\n");
        user_message.push_str("3. Provide step-by-step remediation instructions\n");
        
        let response_text = self.send_request(system_prompt, &user_message).await?;
        
        Ok(ClaudeResponse {
            answer: response_text.clone(),
            suggestions: self.extract_suggestions(&response_text),
        })
    }
    
    /// Get deployment recommendations for a framework
    pub async fn get_deployment_recommendations(&self, framework: &str, environment: &str) -> Result<ClaudeResponse, ClaudeServiceError> {
        let system_prompt = "You are an expert DevOps consultant providing deployment best practices.";
        
        let user_message = format!(
            "I'm deploying a {} application to AWS ECS in the {} environment. \
            What are the recommended configuration settings for:\n\
            - CPU and memory allocation\n\
            - Health check configuration\n\
            - Scaling policies\n\
            - Environment variables\n\
            - Security best practices\n\n\
            Please provide specific, actionable recommendations.",
            framework, environment
        );
        
        let response_text = self.send_request(system_prompt, &user_message).await?;
        
        Ok(ClaudeResponse {
            answer: response_text.clone(),
            suggestions: self.extract_suggestions(&response_text),
        })
    }
    
    /// Explain a deployment error
    pub async fn explain_error(&self, error_message: &str, context: &DeploymentContext) -> Result<ClaudeResponse, ClaudeServiceError> {
        let system_prompt = "You are a helpful assistant explaining deployment errors in simple terms.";
        
        let user_message = format!(
            "I encountered this error while deploying a {} application:\n\n\
            Error: {}\n\n\
            Context:\n\
            - Project: {}\n\
            - Environment: {}\n\
            - ECS Cluster: {}\n\
            - ECS Service: {}\n\n\
            Please explain what this error means and how to fix it.",
            context.framework,
            error_message,
            context.project_name,
            context.environment,
            context.cluster_name,
            context.service_name
        );
        
        let response_text = self.send_request(system_prompt, &user_message).await?;
        
        Ok(ClaudeResponse {
            answer: response_text,
            suggestions: Vec::new(),
        })
    }
    
    // ===== Helper Methods =====
    
    /// Send request to Claude API
    async fn send_request(&self, system_prompt: &str, user_message: &str) -> Result<String, ClaudeServiceError> {
        let request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 4096,
            messages: vec![Message {
                role: "user".to_string(),
                content: user_message.to_string(),
            }],
            system: Some(system_prompt.to_string()),
        };
        
        let response = self.client
            .post(&format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;
        
        // Check for rate limiting
        if response.status() == 429 {
            return Err(ClaudeServiceError::RateLimitExceeded);
        }
        
        // Check for success
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ClaudeServiceError::RequestFailed(format!(
                "Status {}: {}",
                response.status(),
                error_text
            )));
        }
        
        let claude_response: ClaudeApiResponse = response.json().await
            .map_err(|e| ClaudeServiceError::InvalidResponse(e.to_string()))?;
        
        // Extract text from first content block
        let text = claude_response.content
            .first()
            .map(|block| block.text.clone())
            .ok_or_else(|| ClaudeServiceError::InvalidResponse("No content in response".to_string()))?;
        
        Ok(text)
    }
    
    /// Build system prompt for general questions
    fn build_system_prompt(&self) -> String {
        "You are Deployotron AI, an expert DevOps assistant specializing in AWS ECS deployments. \
        You help users deploy applications, troubleshoot issues, and optimize their infrastructure. \
        Provide clear, actionable advice with specific commands and configuration examples when relevant."
            .to_string()
    }
    
    /// Build user message with context
    fn build_user_message(&self, question: &str, context: Option<&DeploymentContext>) -> String {
        let mut message = String::new();
        
        if let Some(ctx) = context {
            message.push_str(&format!(
                "Deployment Context:\n\
                - Project: {}\n\
                - Framework: {}\n\
                - Environment: {}\n\
                - Cluster: {}\n\
                - Service: {}\n\
                - Commit: {}\n\n",
                ctx.project_name,
                ctx.framework,
                ctx.environment,
                ctx.cluster_name,
                ctx.service_name,
                ctx.commit_sha
            ));
            
            if let Some(error) = &ctx.error_message {
                message.push_str(&format!("Current Error: {}\n\n", error));
            }
        }
        
        message.push_str("Question: ");
        message.push_str(question);
        
        message
    }
    
    /// Extract action suggestions from Claude's response
    fn extract_suggestions(&self, response: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        // Simple extraction: look for numbered lists or bullet points
        for line in response.lines() {
            let trimmed = line.trim();
            
            // Match patterns like "1.", "2.", "-", "*", "•"
            if trimmed.starts_with(char::is_numeric) && trimmed.contains('.') {
                if let Some(suggestion) = trimmed.split_once('.') {
                    suggestions.push(suggestion.1.trim().to_string());
                }
            } else if trimmed.starts_with('-') || trimmed.starts_with('*') || trimmed.starts_with('•') {
                suggestions.push(trimmed[1..].trim().to_string());
            }
        }
        
        suggestions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_suggestions() {
        let service = ClaudeService::new("test_key".to_string()).unwrap();
        
        let response = "Here are some suggestions:\n\
                       1. Increase memory allocation\n\
                       2. Check environment variables\n\
                       - Review logs\n\
                       - Restart service";
        
        let suggestions = service.extract_suggestions(response);
        assert_eq!(suggestions.len(), 4);
        assert_eq!(suggestions[0], "Increase memory allocation");
        assert_eq!(suggestions[1], "Check environment variables");
    }
    
    #[test]
    fn test_new_service_without_api_key() {
        let result = ClaudeService::new("".to_string());
        assert!(matches!(result, Err(ClaudeServiceError::ApiKeyMissing)));
    }
    
    #[test]
    fn test_build_user_message() {
        let service = ClaudeService::new("test_key".to_string()).unwrap();
        let context = DeploymentContext {
            project_name: "test-project".to_string(),
            framework: "nextjs".to_string(),
            environment: "production".to_string(),
            cluster_name: "test-cluster".to_string(),
            service_name: "test-service".to_string(),
            commit_sha: "abc123".to_string(),
            error_message: Some("Connection timeout".to_string()),
            logs: None,
        };
        
        let message = service.build_user_message("How do I fix this?", Some(&context));
        assert!(message.contains("test-project"));
        assert!(message.contains("nextjs"));
        assert!(message.contains("Connection timeout"));
    }
}
