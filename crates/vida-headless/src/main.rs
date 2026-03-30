use std::sync::Arc;

use tokio::sync::RwLock;
use vida_core::VidaEngine;

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

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl-C");

    eprintln!("\nShutting down…");
    server.stop();
}

#[cfg(not(feature = "remote"))]
async fn run_headless() {
    eprintln!("ERROR: Headless mode requires the 'remote' feature.");
    eprintln!("  Build with: cargo build --release -p vida-headless --features remote");
    std::process::exit(1);
}

fn main() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(run_headless());
}
