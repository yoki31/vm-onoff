use std::time::Instant;

use tokio::sync::Mutex;

use super::TokenProvider;

pub struct TokenManager<Provider>
where
    Provider: TokenProvider,
{
    provider: Provider,
    cached_token: Mutex<Option<Record>>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error<RenewalError> {
    #[error("token provider: {0}")]
    Provider(#[source] RenewalError),
}

#[derive(Debug, Clone)]
pub struct Record {
    pub access_token: String,
    pub expires_at: Instant,
}

impl Record {
    pub fn from_expiring_token<T: super::ExpiringToken>(token: T) -> Self {
        Self {
            access_token: token.access_token().to_owned(),
            expires_at: token.expires_at(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at >= Instant::now()
    }
}

impl super::Token for Record {
    fn access_token(&self) -> &str {
        &self.access_token
    }
}

impl<Provider> TokenManager<Provider>
where
    Provider: TokenProvider,
    <Provider as TokenProvider>::Token: super::ExpiringToken,
{
    pub fn new(provider: Provider) -> Self {
        let cached_token = Mutex::const_new(None);
        Self {
            provider,
            cached_token,
        }
    }

    async fn fetch_new_token(&self) -> Result<Record, Error<Provider::Error>> {
        let token = self
            .provider
            .get_auth_token()
            .await
            .map_err(Error::Provider)?;
        let record = Record::from_expiring_token(token);
        Ok(record)
    }

    pub async fn get_token(&self) -> Result<Record, Error<Provider::Error>> {
        let mut cached_token = self.cached_token.lock().await;

        if let Some(ref mut cached_token) = &mut *cached_token {
            if !cached_token.is_expired() {
                return Ok(cached_token.clone());
            }
        }

        let new_record = self.fetch_new_token().await?;
        cached_token.replace(new_record.clone());

        Ok(new_record)
    }
}

#[async_trait::async_trait]
impl<Provider> super::TokenProvider for TokenManager<Provider>
where
    Provider: TokenProvider,
    <Provider as TokenProvider>::Token: super::ExpiringToken,
{
    type Token = Record;
    type Error = Error<Provider::Error>;

    async fn get_auth_token(&self) -> Result<Self::Token, Self::Error> {
        let token = self.get_token().await?;
        Ok(token)
    }
}
