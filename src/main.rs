use std::sync::Arc;

use axum::{Router, Server};
use tracing::info;
use vm_onoff::{
    api::http::{axum::GraphQL, graphql},
    azure,
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let reqwest_client = reqwest::Client::builder()
        .connection_verbose(true)
        .build()
        .unwrap();

    let azure_client_id = getenv("AZURE_CLIENT_ID");
    let azure_client_secret = getenv("AZURE_CLIENT_SECRET");
    let azure_tenant_id = getenv("AZURE_TENANT_ID");
    let azure_subscription_id = getenv("AZURE_SUBSCRIPTION_ID");

    let azure_auth_provider = azure::auth::client_credentials::ClientCredentials {
        client: reqwest_client.clone(),
        client_id: azure_client_id,
        client_secret: azure_client_secret,
        tenant_id: azure_tenant_id,
        scopes: vec!["https://management.azure.com/.default".into()],
    };

    let azure_auth_provider = azure::auth::token_manager::TokenManager::new(azure_auth_provider);

    let azure_provider = azure::Provider {
        client: reqwest_client,
        subscription_id: azure_subscription_id,
        auth_token_provider: azure_auth_provider,
    };

    let providers = vec![("azure".to_owned(), Box::new(azure_provider) as _)]
        .into_iter()
        .collect();
    let core = Arc::new(vm_onoff::core::Core { providers });
    let instance_loader = graphql::loader::InstanceLoader {
        core: Arc::clone(&core),
    };
    let schema = graphql::schema().data(core).data(instance_loader).finish();

    let app = Router::new();
    let app = GraphQL::routes(app, schema);

    info!("Playground: http://localhost:8000");

    Server::bind(&"0.0.0.0:8000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn getenv(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("env var {} is not set", key))
}
