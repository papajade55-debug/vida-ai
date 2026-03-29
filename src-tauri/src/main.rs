#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;
use vida_core::VidaEngine;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir().expect("Failed to get app data dir");
            let rt = tokio::runtime::Runtime::new().unwrap();
            let engine = rt
                .block_on(VidaEngine::init(&data_dir))
                .expect("Failed to initialize VidaEngine");
            app.manage(Arc::new(RwLock::new(engine)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth::is_pin_configured,
            commands::auth::store_api_key,
            commands::auth::remove_api_key,
            commands::providers::list_providers,
            commands::providers::list_models,
            commands::providers::health_check,
            commands::chat::send_message,
            commands::chat::stream_completion,
            commands::chat::create_session,
            commands::chat::list_sessions,
            commands::chat::get_messages,
            commands::chat::delete_session,
            commands::chat::send_vision_message,
            commands::config::get_config,
            commands::teams::create_team,
            commands::teams::list_teams,
            commands::teams::get_team,
            commands::teams::delete_team,
            commands::teams::create_team_session,
            commands::teams::stream_team_completion,
        ])
        .run(tauri::generate_context!())
        .expect("error while running vida-ai");
}
