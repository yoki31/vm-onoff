use async_graphql::{ComplexObject, Context, Enum, Object, Result, SimpleObject, ID};

use super::{error, util::load_core};

#[derive(Debug, Enum, Clone, Copy, PartialEq, Eq)]
#[graphql(remote = "crate::core::State")]
pub enum State {
    On,
    Off,
    InProgress,
    Other,
}

#[derive(SimpleObject)]
#[graphql(complex)]
pub struct Provider {
    pub key: crate::core::ProviderKey,
}

#[ComplexObject]
impl Provider {
    async fn instances(&self, ctx: &Context<'_>) -> Result<Vec<Instance>> {
        let core = load_core(ctx);
        let provider = core.provider(&self.key).ok_or(error::UnknownProvider)?;
        let instances = provider.list().await?;
        let instances = instances.into_iter().map(Into::into).collect();
        Ok(instances)
    }

    async fn instance(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Instance>> {
        let core = load_core(ctx);
        let provider = core.provider(&self.key).ok_or(error::UnknownProvider)?;
        let instance = provider.get(&id).await?;
        let instance = instance.map(Into::into);
        Ok(instance)
    }
}

#[derive(SimpleObject, Clone)]
pub struct Instance {
    pub id: ID,
    pub name: String,
    pub state: State,
}

impl From<crate::core::Instance> for Instance {
    fn from(val: crate::core::Instance) -> Self {
        Self {
            id: val.id.into(),
            name: val.display_name,
            state: val.state.into(),
        }
    }
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn providers(&self, ctx: &Context<'_>) -> Result<Vec<Provider>> {
        let core = load_core(ctx);
        let providers = core
            .providers
            .keys()
            .cloned()
            .map(|key| Provider { key })
            .collect();
        Ok(providers)
    }

    async fn provider(&self, ctx: &Context<'_>, key: ID) -> Result<Option<Provider>> {
        let core = load_core(ctx);
        if core.has_provider(&key) {
            return Ok(Some(Provider { key: key.into() }));
        }
        Ok(None)
    }
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn start_instance(
        &self,
        ctx: &Context<'_>,
        provider: ID,
        instance: ID,
    ) -> Result<Instance> {
        let core = load_core(ctx);
        let provider = core.provider(&provider.0).ok_or(error::UnknownProvider)?;

        let id = instance.as_str();

        provider.start(id).await?;

        let instance = provider.get(id).await?;
        let instance = instance.ok_or(error::InstanceGone)?;
        Ok(instance.into())
    }

    async fn stop_instance(
        &self,
        ctx: &Context<'_>,
        provider: ID,
        instance: ID,
    ) -> Result<Instance> {
        let core = load_core(ctx);
        let provider = core.provider(&provider.0).ok_or(error::UnknownProvider)?;

        let id = instance.as_str();

        provider.stop(id).await?;

        let instance = provider.get(id).await?;
        let instance = instance.ok_or(error::InstanceGone)?;
        Ok(instance.into())
    }
}
