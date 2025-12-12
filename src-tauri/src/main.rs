// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod models;
mod infrastructure;
mod services;
mod application;

use application::commands::*;

fn main() {
    // Initialize application state
    let app_state = AppState::new().expect("Failed to initialize application state");

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            // Project commands
            create_project,
            get_projects,
            get_project,
            update_project,
            delete_project,
            
            // Deployment commands
            start_deployment,
            get_deployment_status,
            get_project_deployments,
            get_deployment_logs,
            
            // Credential commands
            store_aws_credentials,
            store_git_credentials,
            get_credentials_status,
            delete_aws_credentials,
            delete_git_credentials,
            
            // AI chat commands
            ask_claude,
            analyze_deployment_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
