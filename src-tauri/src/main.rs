#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;
use vida_core::VidaEngine;

/// Check if `--headless` was passed on the command line.
fn is_headless() -> bool {
    std::env::args().any(|arg| arg == "--headless")
}

/// Run Vida AI in headless mode (HTTP/WS server only, no GUI).
#[cfg(feature = "remote")]
async fn run_headless() {
    use vida_core::{generate_token, RemoteServer};

    let port: u16 = std::env::var("VIDA_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3690);
    let bind_addr = std::env::var("VIDA_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0".to_string());

    let data_dir = std::env::var("VIDA_DATA_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            std::path::PathBuf::from(home).join(".vida-ai")
        });

    eprintln!("Vida AI — Headless Mode");
    eprintln!("  Data dir: {}", data_dir.display());
    eprintln!("  Bind:     {}", bind_addr);
    eprintln!("  Port:     {}", port);

    let engine = VidaEngine::init(&data_dir)
        .await
        .expect("Failed to initialize VidaEngine");

    // Generate or retrieve token
    let token = engine
        .get_remote_token()
        .or_else(|_| engine.generate_remote_token())
        .unwrap_or_else(|_| {
            let t = generate_token();
            eprintln!("  Token:    {}", t);
            t
        });

    eprintln!("  Token:    {}…{}", &token[..8], &token[token.len() - 4..]);
    eprintln!("  Health:   http://{}:{}/api/health", bind_addr, port);
    eprintln!();

    let token_path = data_dir.join(".token");
    let _ = std::fs::create_dir_all(&data_dir);
    let _ = std::fs::write(&token_path, &token);

    let engine_arc = Arc::new(RwLock::new(engine));
    let mut server = RemoteServer::with_bind_addr(port, bind_addr);
    server
        .start(engine_arc.clone(), token)
        .await
        .expect("Failed to start remote server");

    // Keep running until SIGTERM / Ctrl-C
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl-C");

    eprintln!("\nShutting down…");
    server.stop();
}

#[cfg(not(feature = "remote"))]
async fn run_headless() {
    eprintln!("ERROR: Headless mode requires the 'remote' feature.");
    eprintln!("  Build with: cargo build --release -p vida-ai --features remote");
    std::process::exit(1);
}

fn main() {
    if is_headless() {
        // Headless mode: tokio runtime + HTTP server, no Tauri GUI
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(run_headless());
        return;
    }

    // Normal GUI mode
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");
            let rt = tokio::runtime::Runtime::new().unwrap();
            let engine = rt
                .block_on(VidaEngine::init(&data_dir))
                .expect("Failed to initialize VidaEngine");
            app.manage(Arc::new(RwLock::new(engine)));
            app.manage(commands::permissions::PermissionState::default());
            app.manage(commands::remote::RemoteState::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth::is_pin_configured,
            commands::auth::get_auth_status,
            commands::auth::bootstrap_local_admin,
            commands::auth::login_local,
            commands::auth::logout_local,
            commands::auth::list_users,
            commands::auth::create_user,
            commands::auth::change_password,
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
            commands::teams::set_team_member_role,
            commands::teams::delete_team,
            commands::teams::create_team_session,
            commands::teams::stream_team_completion,
            commands::workspace::open_workspace,
            commands::workspace::create_workspace,
            commands::workspace::list_recent_workspaces,
            commands::workspace::get_workspace_config,
            commands::workspace::set_workspace_config,
            commands::workspace::get_permission_mode,
            commands::workspace::set_permission_mode,
            commands::permissions::respond_permission,
            commands::mcp::start_mcp_server,
            commands::mcp::stop_mcp_server,
            commands::mcp::list_mcp_servers,
            commands::mcp::list_mcp_tools,
            commands::mcp::call_mcp_tool,
            commands::mcp::save_mcp_server_config,
            commands::mcp::delete_mcp_server_config,
            commands::remote::enable_remote,
            commands::remote::disable_remote,
            commands::remote::get_remote_status,
            commands::remote::get_remote_token,
            commands::remote::regenerate_remote_token,
        ])
        .run(tauri::generate_context!())
        .expect("error while running vida-ai");
}
