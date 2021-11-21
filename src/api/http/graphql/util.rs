use std::sync::Arc;

use async_graphql::Context;

use crate::core::Core;

pub fn load_core<'a>(ctx: &'a Context<'_>) -> &'a Arc<Core> {
    ctx.data_unchecked::<Arc<Core>>()
}
