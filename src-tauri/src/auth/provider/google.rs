//! Google OAuth2 Provider
//!
//! Implements PKCE Authorization Code flow for Google APIs.
//! Client credentials are placeholder — Geilt will provide real ones.

use std::collections::HashMap;
use tracing::{error, info};

use super::{OAuthProvider, OAuthTokens};

// ── Google OAuth endpoints ──────────────────────────────────────────────────

const AUTH_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const REVOKE_ENDPOINT: &str = "https://oauth2.googleapis.com/revoke";

// TODO: Replace with real credentials — Geilt will provide these.
// These must be registered in Google Cloud Console as a "Desktop app" OAuth client.
const DEFAULT_CLIENT_ID: &str = "PLACEHOLDER_CLIENT_ID.apps.googleusercontent.com";
const DEFAULT_CLIENT_SECRET: &str = "PLACEHOLDER_CLIENT_SECRET";

/// Google OAuth2 provider.
pub struct GoogleProvider {
    pub client_id: String,
    pub client_secret: String,
}

impl GoogleProvider {
    pub fn new() -> Self {
        Self {
            client_id: DEFAULT_CLIENT_ID.to_string(),
            client_secret: DEFAULT_CLIENT_SECRET.to_string(),
        }
    }

    pub fn with_credentials(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret,
        }
    }
}

impl OAuthProvider for GoogleProvider {
    fn name(&self) -> &str {
        "google"
    }

    fn authorize_url(
        &self,
        scopes: &[String],
        state: &str,
        code_challenge: &str,
        redirect_uri: &str,
    ) -> String {
        let scope_str = scopes.join(" ");
        format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&code_challenge={}&code_challenge_method=S256&access_type=offline&prompt=consent",
            AUTH_ENDPOINT,
            urlencoding::encode(&self.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(&scope_str),
            urlencoding::encode(state),
            urlencoding::encode(code_challenge),
        )
    }

    async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<OAuthTokens, String> {
        info!("Exchanging authorization code for tokens");

        let mut params = HashMap::new();
        params.insert("client_id", self.client_id.as_str());
        params.insert("client_secret", self.client_secret.as_str());
        params.insert("code", code);
        params.insert("code_verifier", code_verifier);
        params.insert("grant_type", "authorization_code");
        params.insert("redirect_uri", redirect_uri);

        let response = post_form(TOKEN_ENDPOINT, &params).await?;
        parse_token_response(&response)
    }

    async fn refresh_token(
        &self,
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
    ) -> Result<OAuthTokens, String> {
        info!("Refreshing access token");

        // Use the client_id/secret from the token record if available,
        // falling back to provider defaults
        let cid = if client_id.is_empty() {
            self.client_id.as_str()
        } else {
            client_id
        };
        let csec = if client_secret.is_empty() {
            self.client_secret.as_str()
        } else {
            client_secret
        };

        let mut params = HashMap::new();
        params.insert("client_id", cid);
        params.insert("client_secret", csec);
        params.insert("refresh_token", refresh_token);
        params.insert("grant_type", "refresh_token");

        let response = post_form(TOKEN_ENDPOINT, &params).await?;
        parse_token_response(&response)
    }

    async fn revoke_token(&self, token: &str) -> Result<(), String> {
        info!("Revoking token at Google");

        let mut params = HashMap::new();
        params.insert("token", token);

        let response = post_form(REVOKE_ENDPOINT, &params).await?;

        // Google returns 200 on success, with an empty body or { }
        // Any non-error response is considered success
        if response.contains("error") {
            let parsed: serde_json::Value = serde_json::from_str(&response)
                .unwrap_or_else(|_| serde_json::json!({"error": "Unknown error"}));
            let err_msg = parsed
                .get("error_description")
                .or_else(|| parsed.get("error"))
                .and_then(|v| v.as_str())
                .unwrap_or("Revocation failed");
            Err(err_msg.to_string())
        } else {
            Ok(())
        }
    }

    fn default_scopes(&self) -> Vec<String> {
        vec![
            "https://www.googleapis.com/auth/gmail.modify".to_string(),
            "https://www.googleapis.com/auth/gmail.settings.basic".to_string(),
            "https://www.googleapis.com/auth/calendar".to_string(),
            "https://www.googleapis.com/auth/drive".to_string(),
            "https://www.googleapis.com/auth/contacts".to_string(),
            "https://www.googleapis.com/auth/tasks".to_string(),
            "https://www.googleapis.com/auth/documents".to_string(),
            "https://www.googleapis.com/auth/spreadsheets".to_string(),
        ]
    }
}

// ── HTTP utilities ──────────────────────────────────────────────────────────

/// POST a form-encoded request and return the response body.
///
/// **SECURITY:** Uses reqwest HTTP client. Secrets are passed in request body,
/// never in process arguments. Safe from ps/pgrep exposure.
async fn post_form(
    url: &str,
    params: &HashMap<&str, &str>,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = client
        .post(url)
        .form(params)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error response".to_string());
        error!("HTTP error {}: {}", status, body);
        return Err(format!("HTTP {} error: {}", status, body));
    }

    response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))
}

/// Parse a Google OAuth2 token response.
fn parse_token_response(body: &str) -> Result<OAuthTokens, String> {
    let parsed: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("Invalid JSON response: {}", e))?;

    // Check for error
    if let Some(err) = parsed.get("error").and_then(|v| v.as_str()) {
        let desc = parsed
            .get("error_description")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        return Err(format!("{}: {}", err, desc));
    }

    let access_token = parsed
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or("Missing access_token in response")?
        .to_string();

    let refresh_token = parsed
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(String::from);

    let token_type = parsed
        .get("token_type")
        .and_then(|v| v.as_str())
        .unwrap_or("Bearer")
        .to_string();

    let expires_in = parsed
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .unwrap_or(3600);

    let expiry = (chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64)).to_rfc3339();

    let scopes = parsed
        .get("scope")
        .and_then(|v| v.as_str())
        .map(|s| s.split(' ').map(String::from).collect())
        .unwrap_or_default();

    Ok(OAuthTokens {
        access_token,
        refresh_token,
        token_type,
        expiry,
        scopes,
    })
}

// ── PKCE Utilities ──────────────────────────────────────────────────────────

/// Generate a PKCE code verifier (43-128 characters of unreserved URI characters).
pub fn generate_code_verifier() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    base64_url_encode(&bytes)
}

/// Derive the PKCE code challenge from a code verifier using S256.
pub fn generate_code_challenge(verifier: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(verifier.as_bytes());
    base64_url_encode(&hash)
}

/// Base64url encoding (no padding) per RFC 4648 §5.
fn base64_url_encode(data: &[u8]) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    URL_SAFE_NO_PAD.encode(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_verifier_length() {
        let v = generate_code_verifier();
        assert!(v.len() >= 43);
        assert!(v.len() <= 128);
    }

    #[test]
    fn test_code_challenge_deterministic() {
        let verifier = "test_verifier_string_for_determinism";
        let c1 = generate_code_challenge(verifier);
        let c2 = generate_code_challenge(verifier);
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_parse_token_response_success() {
        let body = r#"{
            "access_token": "ya29.test",
            "refresh_token": "1//0e.test",
            "token_type": "Bearer",
            "expires_in": 3600,
            "scope": "https://www.googleapis.com/auth/gmail.modify"
        }"#;

        let tokens = parse_token_response(body).unwrap();
        assert_eq!(tokens.access_token, "ya29.test");
        assert_eq!(tokens.refresh_token.as_deref(), Some("1//0e.test"));
        assert_eq!(tokens.scopes.len(), 1);
    }

    #[test]
    fn test_parse_token_response_error() {
        let body = r#"{"error": "invalid_grant", "error_description": "Token has been revoked"}"#;
        assert!(parse_token_response(body).is_err());
    }
}
