

use async_graphql::{Object, Context};


use uuid::Uuid;
use crate::{TA_STATE, structs::{GQLTAState, Match}};

pub struct Query;

#[Object]
impl Query {
    async fn state<'ctx>(&self, _ctx: &Context<'ctx>) ->  anyhow::Result<GQLTAState> {
        TA_STATE.read().await.into_gql().await
    }

    async fn match_by_id<'ctx>(&self, _ctx: &Context<'ctx>, id: Uuid) -> anyhow::Result<Option<Match>> {
        TA_STATE.read().await.get_single_match_gql(id).await
    }

    // async fn page<'ctx>(&self, _ctx: &Context<'ctx>) -> GQLOverState {
    //     OVER_STATE.read().await.clone()
    // }
}

// pub struct Mutation;

// #[juniper::graphql_object(context = Context)]
// impl Mutation {
//     async fn update_page(page: InputPage) -> GQLOverState {
//         OVER_STATE.write().await.page = page.into_page();
//         OVER_UPDATE_SINK.send(OverUpdates::NewPage);
//         OVER_STATE.read().await.clone()
//     }
// }

// pub struct Subscription;

// type GQLTAStateStream = Pin<Box<dyn Stream<Item = Result<GQLTAState, FieldError>> + Send>>;
// type GQLOverStateStream = Pin<Box<dyn Stream<Item = Result<GQLOverState, FieldError>> + Send>>;

// #[juniper::graphql_subscription(context = Context)]
// impl Subscription {
//     async fn state() ->  GQLTAStateStream {
//         let mut stream = TA_UPDATE_SINK.stream().events();

//         // magic macro :)
//         async_stream::stream! {
//             while let Some(update) = stream.next() {
//                 match update {
//                     TAUpdates::NewState => {
//                         yield Ok(TA_STATE.read().await.into_gql().await);
//                     },
//                     _ => {}
//                 }
//             }
//         }.boxed()
//     }

//     // async fn player_score(player: Uuid) 

//     async fn page() -> GQLOverStateStream {
//         let mut stream = OVER_UPDATE_SINK.stream().events();

//         async_stream::stream! {
//             while let Some(update) = stream.next() {
//                 match update {
//                     OverUpdates::NewPage => {
//                         yield Ok(OVER_STATE.read().await.clone());
//                     },
//                     _ => {}
//                 }
//             }
//         }.boxed()
//     }
// }

// pub struct Context {}

// impl juniper::Context for Context {}

// pub type Schema = RootNode<'static, Query, Mutation, EmptySubscription<Context>>;

// pub fn create_schema() -> Schema {
//     Schema::new(Query {}, Mutation {}, EmptySubscription::new())
// }