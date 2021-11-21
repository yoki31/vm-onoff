//! Authorize using the client credentials flow.

use serde::Deserialize;
use serde_urlencoded;

use crate::azure::utils::{check_status, ServerError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("reqwest: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("server: {0}")]
    Server(#[from] ServerError),
}

pub struct ClientCredentials {
    pub client: reqwest::Client,
    pub client_id: String,
    pub client_secret: String,
    pub scopes: Vec<String>,
    pub tenant_id: String,
}

impl ClientCredentials {
    /// Perform the client credentials flow.
    pub async fn perform(&self) -> Result<AuthResponse, Error> {
        let params = &[
            ("grant_type", "client_credentials"),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("scope", &self.scopes.join(" ")),
        ];
        let params =
            serde_urlencoded::to_string(params).expect("what kind of failure is possible here?");

        let url = format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            self.tenant_id
        );

        let req = self
            .client
            .post(url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(params)
            .build()?;

        let res = self.client.execute(req).await?;
        check_status(&res)?;
        let login_response = res.json().await?;
        Ok(login_response)
    }
}

#[derive(Debug, Deserialize)]
pub struct AuthResponse {
    /// The requested access token.
    /// The app can use this token to authenticate to the secured resource, such as to a web API.
    access_token: String,
    /// The amount of time that an access token is valid (in seconds).
    expires_in: u64,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub access_token: String,
    pub expires_at: std::time::Instant,
}

impl From<AuthResponse> for Token {
    fn from(auth: AuthResponse) -> Self {
        let AuthResponse {
            access_token,
            expires_in,
        } = auth;
        let expires_in = std::time::Duration::from_secs(expires_in);
        let expires_at = std::time::Instant::now() + expires_in;
        Self {
            access_token,
            expires_at,
        }
    }
}

#[async_trait::async_trait]
impl super::TokenProvider for ClientCredentials {
    type Token = Token;
    type Error = Error;

    async fn get_auth_token(&self) -> Result<Self::Token, Self::Error> {
        let auth_response = self.perform().await?;
        let token = auth_response.into();
        Ok(token)
    }
}

impl super::Token for Token {
    fn access_token(&self) -> &str {
        self.access_token.as_str()
    }
}

impl super::ExpiringToken for Token {
    fn expires_at(&self) -> std::time::Instant {
        self.expires_at
    }
}
