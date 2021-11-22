use std::{convert::Infallible, marker::PhantomData};

use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    ObjectType, Schema, SubscriptionType,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::{
    body::Bytes,
    extract,
    handler::Handler,
    response::{self, IntoResponse},
    routing::{get, post},
    AddExtensionLayer, Router,
};

pub mod config {
    use std::collections::BTreeMap;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Config {
        pub graphql: GrqphQL,
        pub playground: Playground,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct GrqphQL {
        pub http_mount: String,
        pub ws_mount: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Playground {
        pub mount: String,
        pub extra_settings: BTreeMap<String, String>,
    }
}

pub struct GraphQL<Query, Mutation, Subscription>(
    PhantomData<(Query, Mutation, Subscription)>,
    Infallible,
);

impl<Query, Mutation, Subscription> GraphQL<Query, Mutation, Subscription>
where
    Query: ObjectType + 'static,
    Mutation: ObjectType + 'static,
    Subscription: SubscriptionType + 'static,
{
    async fn handler(
        schema: extract::Extension<Schema<Query, Mutation, Subscription>>,
        req: GraphQLRequest,
    ) -> GraphQLResponse {
        schema.execute(req.into_inner()).await.into()
    }

    async fn playground(source: extract::Extension<PlaygroundSource>) -> impl IntoResponse {
        response::Html(source.bytes())
    }

    pub fn routes(
        router: Router,
        schema: Schema<Query, Mutation, Subscription>,
        config: config::Config,
    ) -> Router {
        let router = router.route(
            &config.graphql.ws_mount,
            GraphQLSubscription::new(schema.clone()),
        );

        let playground_source = PlaygroundSource::new(&config);
        let playground_handler = Self::playground.layer(AddExtensionLayer::new(playground_source));

        let router = if config.graphql.http_mount == config.playground.mount {
            router.route(
                &config.graphql.http_mount,
                get(playground_handler).post(Self::handler),
            )
        } else {
            router
                .route(&config.graphql.http_mount, post(Self::handler))
                .route(&config.playground.mount, get(playground_handler))
        };

        router.layer(AddExtensionLayer::new(schema))
    }
}

#[derive(Debug, Clone)]
struct PlaygroundSource(Bytes);

impl PlaygroundSource {
    pub fn new(config: &config::Config) -> Self {
        let playground_config = GraphQLPlaygroundConfig::new(&config.graphql.http_mount)
            .subscription_endpoint(&config.graphql.ws_mount);

        let playground_config = config.playground.extra_settings.iter().fold(
            playground_config,
            |playground_config, (key, val)| {
                playground_config.with_setting(key.as_str(), val.as_str())
            },
        );

        let source = playground_source(playground_config);
        Self(source.into())
    }

    pub fn bytes(&self) -> Bytes {
        self.0.clone()
    }
}
