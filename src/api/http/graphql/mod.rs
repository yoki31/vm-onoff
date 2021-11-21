mod error;
pub mod loader;
mod model;
mod util;

use async_graphql::{EmptySubscription, SchemaBuilder};

use self::model::{MutationRoot, QueryRoot};

pub type Schema = async_graphql::Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn schema() -> SchemaBuilder<QueryRoot, MutationRoot, EmptySubscription> {
    async_graphql::Schema::build(QueryRoot, MutationRoot, EmptySubscription)
}
