//! Authorization logic.

pub mod client_credentials;
pub mod token_manager;

#[async_trait::async_trait]
pub trait TokenProvider: Send + Sync {
    type Token: Token;
    type Error: Send + Sync;

    async fn get_auth_token(&self) -> Result<Self::Token, Self::Error>;
}

pub trait Token: Send {
    fn access_token(&self) -> &str;
}

pub trait ExpiringToken: Token {
    fn expires_at(&self) -> std::time::Instant;
}
