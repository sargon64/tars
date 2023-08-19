use std::pin::Pin;

use futures_util::Stream;
use juniper::{GraphQLObject, RootNode, EmptyMutation, EmptySubscription, FieldError};
use uuid::Uuid;

use crate::{TA_STATE, structs::GQLTAState, TA_UPDATE_SINK, TAUpdates};

pub struct Query;

#[juniper::graphql_object(context = Context)]
impl Query {
    async fn state() ->  GQLTAState {
        (*TA_STATE.read().await).into_gql().await
    }
}

pub struct Subscription;

type GQLTAStateStream = Pin<Box<dyn Stream<Item = Result<GQLTAState, FieldError>> + Send>>;

#[juniper::graphql_subscription(context = Context)]
impl Subscription {
    async fn state() ->  GQLTAStateStream {
        let mut stream = TA_UPDATE_SINK.stream().events();

        // magic macro :)
        async_stream::stream! {            
            while let Some(update) = stream.next() {
                match update {
                    TAUpdates::NewState => {
                        yield Ok((*TA_STATE.read().await).into_gql().await);
                    },
                    _ => {}
                }
            }
        }.boxed()
    }
}

pub struct Context {}

impl juniper::Context for Context {}

pub type Schema = RootNode<'static, Query, EmptyMutation<Context>, Subscription>;

pub fn create_schema() -> Schema {
    Schema::new(Query {}, EmptyMutation::new(), Subscription {})
}