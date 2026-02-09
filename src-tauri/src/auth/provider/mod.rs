//! OAuth Provider Abstraction
//!
//! Trait-based provider system allowing multiple OAuth providers (Google, Microsoft, etc.)

pub mod google;

use serde::{Deserialize, Serialize};

/// Tokens returned from an OAuth token exchange or refresh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub expiry: String,
    pub scopes: Vec<String>,
}

/// OAuth provider trait.
///
/// Each provider implements authorization URL construction, token exchange,
/// token refresh, and revocation.
#[allow(async_fn_in_trait)]
pub trait OAuthProvider {
    /// Provider name (e.g. "google")
    fn name(&self) -> &str;

    /// Build the authorization URL for the PKCE flow.
    fn authorize_url(
        &self,
        scopes: &[String],
        state: &str,
        code_challenge: &str,
        redirect_uri: &str,
    ) -> String;

    /// Exchange an authorization code for tokens.
    async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<OAuthTokens, String>;

    /// Refresh an access token using a refresh token.
    async fn refresh_token(
        &self,
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
    ) -> Result<OAuthTokens, String>;

    /// Revoke a token at the provider.
    async fn revoke_token(&self, token: &str) -> Result<(), String>;

    /// Default scopes for this provider.
    fn default_scopes(&self) -> Vec<String>;
}
