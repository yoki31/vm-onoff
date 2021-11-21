use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::Arc,
};

use async_graphql::dataloader::Loader;

use crate::core::{Core, Id, ProviderKey};

use super::model::Instance;

pub struct InstanceLoader {
    pub core: Arc<Core>,
}

#[async_trait::async_trait]
impl Loader<(ProviderKey, Id)> for InstanceLoader {
    type Value = Instance;
    type Error = Arc<anyhow::Error>;

    async fn load(
        &self,
        keys: &[(ProviderKey, Id)],
    ) -> Result<HashMap<(ProviderKey, Id), Self::Value>, Self::Error> {
        let mut ids_by_provider: BTreeMap<&ProviderKey, BTreeSet<&Id>> = BTreeMap::new();

        for (provider, id) in keys {
            let ids = ids_by_provider.entry(provider).or_default();
            ids.insert(id);
        }

        let mut all_instances = HashMap::with_capacity(keys.len());

        for (provider_key, ids) in ids_by_provider {
            let provider = self.core.provider(provider_key);
            let provider = if let Some(provider) = provider {
                provider
            } else {
                continue;
            };

            let instances = provider.list().await.map_err(Arc::new)?;
            for instance in instances
                .into_iter()
                .filter(|instance| ids.contains(&instance.id))
            {
                let key = (provider_key.clone(), instance.id.clone());
                all_instances.insert(key, Instance::from(instance));
            }
        }

        Ok(all_instances)
    }
}
