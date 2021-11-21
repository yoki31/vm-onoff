use std::collections::HashMap;

pub type ProviderKey = String;
pub type ProviderKeyRef = str;

pub struct Core {
    pub providers: HashMap<ProviderKey, Box<dyn Provider>>,
}

impl Core {
    pub fn provider(&self, key: &ProviderKeyRef) -> Option<&dyn Provider> {
        self.providers
            .get(key)
            .map(|val| val.as_ref() as &dyn Provider)
    }

    pub fn has_provider(&self, key: &ProviderKeyRef) -> bool {
        self.providers.contains_key(key)
    }
}

pub type Id = String;
pub type IdRef = str;

#[derive(Debug, thiserror::Error)]
#[error("Unable to parse the ID")]
pub struct IdParsingError;

#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    async fn list(&self) -> Result<Vec<Instance>, anyhow::Error>;
    async fn get(&self, id: &IdRef) -> Result<Option<Instance>, anyhow::Error>;

    async fn start(&self, id: &IdRef) -> Result<(), anyhow::Error>;
    async fn stop(&self, id: &IdRef) -> Result<(), anyhow::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    On,
    Off,
    InProgress,
    Other,
}

pub struct Instance {
    pub id: Id,
    pub display_name: String,
    pub state: State,
}
