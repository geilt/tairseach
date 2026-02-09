mod auth;
mod config;
mod manifest;
mod mcp;
mod monitor;
mod permissions;
mod profiles;
mod proxy;
mod router;

use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(debug_assertions)]
use tauri::Manager;

/// Shared state for the proxy server
struct ProxyServerState {
    server: Option<Arc<proxy::ProxyServer>>,
    running: bool,
}

/// Global proxy server state
static PROXY_STATE: once_cell::sync::Lazy<Arc<RwLock<ProxyServerState>>> =
    once_cell::sync::Lazy::new(|| {
        Arc::new(RwLock::new(ProxyServerState {
            server: None,
            running: false,
        }))
    });

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to Tairseach.", name)
}

/// Internal function to start the proxy server
async fn start_proxy_server_internal() -> Result<(), String> {
    let mut state = PROXY_STATE.write().await;
    
    if state.running {
        return Ok(());
    }
    
    // Initialize manifest registry
    tracing::info!("Initializing manifest registry");
    let registry = Arc::new(manifest::ManifestRegistry::new());
    if let Err(e) = registry.load_from_disk().await {
        tracing::warn!("Failed to load manifests: {}", e);
        // Continue with empty registry
    }

    // Start hot-reload watcher
    let registry_clone = Arc::clone(&registry);
    tokio::spawn(async move {
        if let Err(e) = registry_clone.start_watcher().await {
            tracing::error!("Failed to start manifest watcher: {}", e);
        }
    });
    
    // Initialize auth broker
    tracing::info!("Initializing auth broker");
    let auth_broker = match auth::AuthBroker::new().await {
        Ok(broker) => {
            broker.spawn_refresh_daemon();
            broker
        }
        Err(e) => {
            tracing::error!("Failed to initialize auth broker: {}", e);
            return Err(format!("Auth broker initialization failed: {}", e));
        }
    };
    
    // Create capability router
    tracing::info!("Creating capability router");
    let router = Arc::new(router::CapabilityRouter::new(registry, auth_broker));
    
    // Create handler registry with router
    let handlers = Arc::new(proxy::handlers::HandlerRegistry::with_router(router));
    
    // Create proxy server with custom handlers
    let server = Arc::new(proxy::ProxyServer::with_handlers(None, handlers));
    state.server = Some(Arc::clone(&server));
    state.running = true;
    
    drop(state); // Release lock before blocking
    
    let socket_path = server.socket_path().clone();
    tracing::info!("Starting proxy server at {:?}", socket_path);
    
    if let Err(e) = server.start().await {
        tracing::error!("Proxy server error: {}", e);
        let mut state = PROXY_STATE.write().await;
        state.running = false;
        state.server = None;
        return Err(e.to_string());
    }
    
    Ok(())
}

/// Get proxy server status
#[tauri::command]
async fn get_proxy_status() -> Result<serde_json::Value, String> {
    let state = PROXY_STATE.read().await;
    
    Ok(serde_json::json!({
        "running": state.running,
        "socket_path": state.server.as_ref().map(|s| s.socket_path().display().to_string()),
    }))
}

/// Start the proxy server (if not already running)
#[tauri::command]
async fn start_proxy_server() -> Result<serde_json::Value, String> {
    start_proxy_server_internal().await?;
    get_proxy_status().await
}

/// Stop the proxy server
#[tauri::command]
async fn stop_proxy_server() -> Result<serde_json::Value, String> {
    let state = PROXY_STATE.read().await;
    
    if let Some(server) = &state.server {
        server.shutdown();
    }
    
    drop(state);
    
    let mut state = PROXY_STATE.write().await;
    state.running = false;
    state.server = None;
    
    Ok(serde_json::json!({
        "stopped": true,
    }))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            // Permissions - these are defined with #[tauri::command] in permissions/mod.rs
            permissions::get_permissions,
            permissions::grant_permission,
            permissions::revoke_permission,
            permissions::check_permission,
            permissions::check_all_permissions,
            permissions::request_permission,
            permissions::trigger_permission_registration,
            permissions::open_permission_settings,
            permissions::get_permission_definitions,
            // Config
            config::get_config,
            config::set_config,
            config::get_provider_models,
            // Monitor
            monitor::get_events,
            monitor::get_manifest_summary,
            // Profiles
            profiles::get_profiles,
            profiles::save_profile,
            // Auth
            auth::authenticate,
            auth::check_auth,
            // Proxy
            get_proxy_status,
            start_proxy_server,
            stop_proxy_server,
        ])
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }
            
            // Start the proxy server in a background task using Tauri's runtime
            tracing::info!("Spawning proxy server task");
            tauri::async_runtime::spawn(async {
                // Give the app a moment to initialize before starting proxy
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                if let Err(e) = start_proxy_server_internal().await {
                    tracing::error!("Failed to start proxy server: {}", e);
                }
            });
            
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
