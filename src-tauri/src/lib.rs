mod auth;
mod common;
mod config;
mod google;
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
async fn proxy_status_get() -> Result<serde_json::Value, String> {
    let state = PROXY_STATE.read().await;
    
    Ok(serde_json::json!({
        "running": state.running,
        "socket_path": state.server.as_ref().map(|s| s.socket_path().display().to_string()),
    }))
}

/// Start the proxy server (if not already running)
#[tauri::command]
async fn proxy_server_start() -> Result<serde_json::Value, String> {
    start_proxy_server_internal().await?;
    proxy_status_get().await
}

/// Stop the proxy server
#[tauri::command]
async fn proxy_server_stop() -> Result<serde_json::Value, String> {
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
            permissions::permissions_all_get,
            permissions::permissions_single_grant,
            permissions::permissions_single_revoke,
            permissions::permissions_single_check,
            permissions::check_all_permissions,
            permissions::permissions_single_request,
            permissions::permissions_registration_trigger,
            permissions::permissions_settings_open,
            permissions::permissions_definitions_get,
            // Config
            config::config_app_get,
            config::config_app_set,
            config::config_models_list,
            config::config_google_oauth_get,
            config::config_google_oauth_save,
            config::config_google_oauth_test,
            config::config_google_oauth_status_get,
            config::config_environment_get,
            config::config_node_get,
            config::config_node_set,
            config::config_exec_approvals_get,
            config::config_exec_approvals_set,
            // Monitor
            monitor::monitor_events_list,
            monitor::monitor_manifest_summary_get,
            monitor::manifests_all_list,
            monitor::monitor_socket_check,
            monitor::monitor_mcp_tool_test,
            monitor::monitor_namespace_statuses_get,
            monitor::monitor_openclaw_install,
            monitor::error_report_submit,
            // Profiles
            profiles::profiles_all_list,
            profiles::profiles_single_save,
            // Auth
            auth::auth_session_create,
            auth::auth_session_check,
            auth::auth_status_get,
            auth::auth_providers_list,
            auth::auth_accounts_list,
            auth::auth_get_token,
            auth::auth_refresh_token,
            auth::auth_revoke_token,
            auth::auth_store_token,
            auth::auth_start_google_oauth,
            // Credentials
            auth::auth_credential_types_list,
            auth::auth_credentials_store,
            auth::auth_credentials_list,
            auth::auth_credentials_get,
            auth::auth_credentials_delete,
            auth::auth_credentials_rename,
            auth::auth_credential_types_custom_create,
            auth::op_vaults_list,
            auth::op_config_default_vault_set,
            // Proxy
            proxy_status_get,
            proxy_server_start,
            proxy_server_stop,
        ])
        .setup(|_app| {
            #[cfg(debug_assertions)]
            {
                let window = _app.get_webview_window("main").unwrap();
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
