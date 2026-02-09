//! Capability Router
//!
//! Dynamically dispatches tool calls based on manifest-defined implementations.

pub mod dispatcher;
pub mod internal;
pub mod proxy;
pub mod script;

use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::auth::AuthBroker;
use crate::manifest::types::{Implementation, Manifest};
use crate::manifest::ManifestRegistry;
use crate::proxy::protocol::{JsonRpcRequest, JsonRpcResponse};
use serde_json::Value;

pub use dispatcher::CapabilityRouter;

impl CapabilityRouter {
    pub fn new(registry: Arc<ManifestRegistry>, auth_broker: Arc<AuthBroker>) -> Self {
        Self {
            registry,
            auth_broker,
        }
    }

    /// Route a JSON-RPC request to the appropriate implementation
    pub async fn route(&self, request: &JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone().unwrap_or(Value::Null);
        let tool_name = &request.method;

        debug!("Routing tool call: {}", tool_name);

        // Look up the tool in the manifest registry
        let (manifest, tool) = match self.registry.find_tool(tool_name).await {
            Some(result) => result,
            None => {
                // Not a manifest-registered tool, return method not found
                // (legacy routing is handled at a higher level)
                debug!("Tool not found in manifest registry: {}", tool_name);
                return JsonRpcResponse::method_not_found(id, tool_name);
            }
        };

        debug!(
            "Found tool {} in manifest {}",
            tool_name, manifest.name
        );

        // Check required permissions
        for perm_req in &manifest.requires.permissions {
            if let Err(response) = self.check_permission(&perm_req.name, &id).await {
                return response;
            }
        }

        // Tool-level requirements override manifest-level
        if let Some(ref tool_reqs) = tool.requires {
            for perm_req in &tool_reqs.permissions {
                if let Err(response) = self.check_permission(&perm_req.name, &id).await {
                    return response;
                }
            }
        }

        // Load required credentials
        let credentials = match self.load_credentials(&manifest, &tool, &request.params).await {
            Ok(creds) => creds,
            Err(response) => return response,
        };

        // Dispatch to implementation type
        match &manifest.implementation {
            Implementation::Internal { module, methods } => {
                internal::dispatch(
                    &manifest,
                    &tool,
                    &request.params,
                    id,
                    module,
                    methods,
                )
                .await
            }
            Implementation::Script {
                runtime,
                entrypoint,
                args,
                env,
                tool_bindings,
            } => {
                script::dispatch(
                    &manifest,
                    &tool,
                    &request.params,
                    id,
                    runtime,
                    entrypoint,
                    args,
                    env,
                    tool_bindings,
                    &credentials,
                )
                .await
            }
            Implementation::Proxy {
                base_url,
                auth,
                tool_bindings,
            } => {
                proxy::dispatch(
                    &manifest,
                    &tool,
                    &request.params,
                    id,
                    base_url,
                    auth,
                    tool_bindings,
                    &credentials,
                )
                .await
            }
        }
    }

    /// Check a single permission
    async fn check_permission(&self, permission: &str, id: &Value) -> Result<(), JsonRpcResponse> {
        use crate::permissions;

        let perm = permissions::check_permission(permission)
            .map_err(|e| JsonRpcResponse::error(id.clone(), -32000, e, None))?;

        if perm.status != permissions::PermissionStatus::Granted {
            return Err(JsonRpcResponse::permission_denied(
                id.clone(),
                permission,
                format!("{:?}", perm.status).as_str(),
            ));
        }

        Ok(())
    }

    /// Load required credentials from the auth broker
    async fn load_credentials(
        &self,
        manifest: &Manifest,
        tool: &crate::manifest::types::Tool,
        params: &Value,
    ) -> Result<HashMap<String, Value>, JsonRpcResponse> {
        let mut credentials = HashMap::new();

        // Manifest-level credentials
        for cred_req in &manifest.requires.credentials {
            if let Some(cred) = self
                .load_credential(&cred_req.id, &cred_req.provider, params)
                .await?
            {
                credentials.insert(cred_req.id.clone(), cred);
            }
        }

        // Tool-level credentials (override manifest-level)
        if let Some(ref tool_reqs) = tool.requires {
            for cred_req in &tool_reqs.credentials {
                if let Some(cred) = self
                    .load_credential(&cred_req.id, &cred_req.provider, params)
                    .await?
                {
                    credentials.insert(cred_req.id.clone(), cred);
                }
            }
        }

        Ok(credentials)
    }

    async fn load_credential(
        &self,
        credential_id: &str,
        provider_opt: &Option<String>,
        params: &Value,
    ) -> Result<Option<Value>, JsonRpcResponse> {
        // Parse provider from credential_id or use explicit provider
        let provider = if let Some(p) = provider_opt {
            p.as_str()
        } else {
            // Try to extract from credential_id (e.g., "google-oauth" → "google")
            credential_id
                .strip_suffix("-oauth")
                .or_else(|| credential_id.strip_suffix("-api"))
                .unwrap_or(credential_id)
        };

        // Get account from params (default: "me")
        let account = params
            .get("account")
            .and_then(|v| v.as_str())
            .unwrap_or("me");

        // Get token from auth broker
        match self.auth_broker.get_token(provider, account, None).await {
            Ok(token_info) => Ok(Some(token_info)),
            Err((code, msg)) => {
                // If credential is optional, return None instead of error
                // (for now, treat all as required)
                Err(JsonRpcResponse::error(Value::Null, code, msg, None))
            }
        }
    }
}
