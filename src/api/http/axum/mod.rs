use std::{convert::Infallible, marker::PhantomData};

use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    ObjectType, Schema, SubscriptionType,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::{
    extract,
    response::{self, IntoResponse},
    routing::get,
    AddExtensionLayer, Router,
};

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

    async fn playground() -> impl IntoResponse {
        response::Html(playground_source(
            GraphQLPlaygroundConfig::new("/")
                .subscription_endpoint("/ws")
                .with_setting("request.credentials", "include"),
        ))
    }

    pub fn routes(router: Router, schema: Schema<Query, Mutation, Subscription>) -> Router {
        router
            .route("/", get(Self::playground).post(Self::handler))
            .route("/ws", GraphQLSubscription::new(schema.clone()))
            .layer(AddExtensionLayer::new(schema))
    }
}
