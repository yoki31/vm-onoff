//! Azure provider implementation.

use reqwest::Method;

use self::{
    auth::Token,
    utils::{check_status, ServerError},
};

pub mod auth;
mod utils;

pub struct Provider<AuthTokenProvider> {
    pub client: reqwest::Client,
    pub subscription_id: String,
    pub auth_token_provider: AuthTokenProvider,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Id {
    pub resource_group_name: String,
    pub vm_name: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Unable to parse the Id from Azure into our Id kind")]
pub struct ModelIdParsingError;

impl Id {
    pub fn from_model(id: &str) -> Result<Self, ModelIdParsingError> {
        // /subscriptions/00000000-0000-0000-0000-000000000000/resourceGroups/myrg/providers/Microsoft.Compute/virtualMachines/vm0
        let mut split = id.split('/');

        let _leading = split.next();
        let _subscriptions_text = split.next();
        let _subscription_id = split.next();
        let _resource_groups_text = split.next();
        let resource_group = split.next();
        let _providers_text = split.next();
        let _provider_id_0 = split.next();
        let _provider_id_1 = split.next();
        let vm_name = split.next();

        let (resource_group, vm_name) = match (resource_group, vm_name) {
            (Some(resource_group), Some(vm_name)) => (resource_group, vm_name),
            _ => return Err(ModelIdParsingError),
        };

        Ok(Self {
            resource_group_name: resource_group.to_owned(),
            vm_name: vm_name.to_owned(),
        })
    }
}

impl From<Id> for crate::core::Id {
    fn from(id: Id) -> Self {
        format!("{}/{}", id.resource_group_name, id.vm_name)
    }
}

impl TryFrom<&crate::core::IdRef> for Id {
    type Error = crate::core::IdParsingError;

    fn try_from(value: &crate::core::IdRef) -> Result<Self, Self::Error> {
        let mut iter = value.split('/');
        let resource_group_name = iter.next().ok_or(crate::core::IdParsingError)?;
        let vm_name = iter.next().ok_or(crate::core::IdParsingError)?;
        if iter.next().is_some() {
            return Err(crate::core::IdParsingError);
        }
        Ok(Self {
            resource_group_name: resource_group_name.to_owned(),
            vm_name: vm_name.to_owned(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error<AuthError> {
    #[error("auth: {0}")]
    Auth(#[source] AuthError),
    #[error("reqwest: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("server: {0}")]
    Server(#[from] ServerError),
    #[error(transparent)]
    ModelIdParsing(#[from] ModelIdParsingError),
}

impl<AuthTokenProvider> Provider<AuthTokenProvider>
where
    AuthTokenProvider: auth::TokenProvider,
{
    fn build_vm_url(&self, id: Id, action: &str, query_extras: &str) -> String {
        format!(
            "https://management.azure.com/subscriptions/{subscriptionId}/resourceGroups/{resourceGroupName}/providers/Microsoft.Compute/virtualMachines/{vmName}{action}?api-version=2021-07-01{query_extras}",
            subscriptionId = self.subscription_id,
            resourceGroupName = id.resource_group_name,
            vmName = id.vm_name,
            action = action,
            query_extras = query_extras,
        )
    }

    fn build_all_vms_list_url(&self, subscription_id: &str) -> String {
        format!(
            "https://management.azure.com/subscriptions/{subscriptionId}/providers/Microsoft.Compute/virtualMachines?api-version=2021-07-01&statusOnly=true",
            subscriptionId = subscription_id,
        )
    }

    fn build_request(
        &self,
        auth_token: &str,
        method: Method,
        url: &str,
    ) -> Result<reqwest::Request, Error<AuthTokenProvider::Error>> {
        let builder = self
            .client
            .request(method.clone(), url)
            .bearer_auth(auth_token);

        let builder = if method == Method::POST {
            builder.header(reqwest::header::CONTENT_LENGTH, 0)
        } else {
            builder
        };

        builder.build().map_err(Error::Reqwest)
    }

    async fn get_auth_token(&self) -> Result<String, Error<AuthTokenProvider::Error>> {
        let token = self
            .auth_token_provider
            .get_auth_token()
            .await
            .map_err(Error::Auth)?;
        Ok(token.access_token().to_owned())
    }

    async fn start(&self, id: Id) -> Result<(), Error<AuthTokenProvider::Error>> {
        let auth_token = self.get_auth_token().await?;
        let url = self.build_vm_url(id, "/start", "");
        self.exec(self.build_request(&auth_token, Method::POST, &url)?)
            .await?;
        Ok(())
    }

    async fn stop(&self, id: Id) -> Result<(), Error<AuthTokenProvider::Error>> {
        let auth_token = self.get_auth_token().await?;
        let url = self.build_vm_url(id, "/deallocate", "");
        self.exec(self.build_request(&auth_token, Method::POST, &url)?)
            .await?;
        Ok(())
    }

    async fn get(&self, id: Id) -> Result<model::VirtualMachine, Error<AuthTokenProvider::Error>> {
        let auth_token = self.get_auth_token().await?;
        let url = self.build_vm_url(id, "", "&$expand=instanceView");
        let res = self
            .exec(self.build_request(&auth_token, Method::GET, &url)?)
            .await?;

        let vm = Self::parse_json(res).await?;
        Ok(vm)
    }

    async fn list_all_vms(
        &self,
    ) -> Result<model::List<model::VirtualMachine>, Error<AuthTokenProvider::Error>> {
        let url = self.build_all_vms_list_url(&self.subscription_id);
        self.list_by_url(&url).await
    }

    async fn list_by_url<T>(
        &self,
        url: &str,
    ) -> Result<model::List<T>, Error<AuthTokenProvider::Error>>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let auth_token = self.get_auth_token().await?;
        let res = self
            .exec(self.build_request(&auth_token, Method::GET, url)?)
            .await?;
        let vms = Self::parse_json(res).await?;
        Ok(vms)
    }

    async fn exec(
        &self,
        request: reqwest::Request,
    ) -> Result<reqwest::Response, Error<AuthTokenProvider::Error>> {
        let res = self.client.execute(request).await.map_err(Error::Reqwest)?;
        check_status(&res)?;
        Ok(res)
    }

    async fn parse_json<T>(res: reqwest::Response) -> Result<T, Error<AuthTokenProvider::Error>>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        res.json().await.map_err(Error::Reqwest)
    }

    fn model_to_instance(
        vm: model::VirtualMachine,
    ) -> Result<crate::core::Instance, Error<AuthTokenProvider::Error>> {
        let name = vm.name;
        let id = Id::from_model(&vm.id)?;
        let state = Self::detect_state(&vm.properties.instance_view.statuses);

        Ok(crate::core::Instance {
            display_name: name,
            id: id.into(),
            state,
        })
    }

    fn detect_state(statuses: &[model::InstanceViewStatus]) -> crate::core::State {
        let is_found = |code: &str| statuses.iter().any(|status| status.code == code);

        let is_stopped = is_found(model::STATUS_POWER_STATE_STOPPED);
        let is_deallocated = is_found(model::STATUS_POWER_STATE_DEALLOCATED);
        let is_off = is_stopped || is_deallocated;

        let is_running = is_found(model::STATUS_POWER_STATE_RUNNING);
        let is_on = is_running;

        let is_stopping = is_found(model::STATUS_POWER_STATE_STOPPING);
        let is_deallocing = is_found(model::STATUS_POWER_STATE_DEALLOCATING);
        let is_starting = is_found(model::STATUS_POWER_STATE_STARTING);
        let is_in_progress = is_stopping || is_deallocing || is_starting;

        match (is_on, is_off, is_in_progress) {
            (true, false, false) => crate::core::State::On,
            (false, true, false) => crate::core::State::Off,
            (false, false, true) => crate::core::State::InProgress,
            (_, _, _) => crate::core::State::Other,
        }
    }
}

mod model {
    use serde::{Deserialize, Serialize};

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct List<T> {
        pub value: Vec<T>,
        pub next_link: Option<String>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct VirtualMachine {
        /// Resource name.
        pub name: String,
        /// Resource Id.
        pub id: String,
        /// Properties.
        pub properties: VirtualMachineProperties,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct VirtualMachineProperties {
        /// The virtual machine instance view.
        pub instance_view: VirtualMachineInstanceView,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct VirtualMachineInstanceView {
        /// The resource status information.
        pub statuses: Vec<InstanceViewStatus>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct InstanceViewStatus {
        /// The status code.
        pub code: String,
    }

    pub const STATUS_POWER_STATE_STOPPING: &str = "PowerState/stopping";
    pub const STATUS_POWER_STATE_STOPPED: &str = "PowerState/stopped";
    pub const STATUS_POWER_STATE_DEALLOCATING: &str = "PowerState/deallocating";
    pub const STATUS_POWER_STATE_DEALLOCATED: &str = "PowerState/deallocated";
    pub const STATUS_POWER_STATE_STARTING: &str = "PowerState/starting";
    pub const STATUS_POWER_STATE_RUNNING: &str = "PowerState/running";
}

#[async_trait::async_trait]
impl<AuthTokenProvider> crate::core::Provider for Provider<AuthTokenProvider>
where
    AuthTokenProvider: auth::TokenProvider,
    <AuthTokenProvider as auth::TokenProvider>::Error: std::error::Error + 'static,
{
    async fn list(&self) -> Result<Vec<crate::core::Instance>, anyhow::Error> {
        let list = self.list_all_vms().await?;

        let collect_instances = |vms: Vec<_>| {
            vms.into_iter()
                .map(Self::model_to_instance)
                .collect::<Result<Vec<_>, _>>()
        };

        let mut next_link = list.next_link;
        let mut instances = collect_instances(list.value)?;
        while let Some(url) = next_link {
            let list = self.list_by_url(&url).await?;
            next_link = list.next_link;

            let new_instances = collect_instances(list.value)?;
            instances.extend(new_instances);
        }

        Ok(instances)
    }

    async fn get(
        &self,
        id: &crate::core::IdRef,
    ) -> Result<Option<crate::core::Instance>, anyhow::Error> {
        let vm = match self.get(Id::try_from(id)?).await {
            Ok(vm) => vm,
            Err(Error::Server(ServerError { status_code })) if status_code == 404 => {
                return Ok(None)
            }
            Err(err) => return Err(err.into()),
        };
        let instance = Self::model_to_instance(vm)?;
        Ok(Some(instance))
    }

    async fn start(&self, id: &crate::core::IdRef) -> Result<(), anyhow::Error> {
        self.start(Id::try_from(id)?).await?;
        Ok(())
    }

    async fn stop(&self, id: &crate::core::IdRef) -> Result<(), anyhow::Error> {
        self.stop(Id::try_from(id)?).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_from_model() {
        let sample_model_id = "/subscriptions/00000000-0000-0000-0000-000000000000/resourceGroups/myrg/providers/Microsoft.Compute/virtualMachines/vm0";
        let id = Id::from_model(sample_model_id).unwrap();
        assert_eq!(
            id,
            Id {
                resource_group_name: "myrg".into(),
                vm_name: "vm0".into(),
            }
        )
    }
}
