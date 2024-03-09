#![allow(clippy::option_map_unit_fn, clippy::module_inception)]
#![forbid(clippy::unwrap_used)]

use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig, GraphiQLSource},
    EmptyMutation, EmptySubscription, Schema,
};
use async_graphql_poem::GraphQL;
// use actix_cors::Cors;
// use actix_web::{get, middleware::Logger, App, HttpServer, Responder, HttpResponse, Error, web::{self, Data}, http::header, HttpRequest};
use carboxyl::Sink;
use futures_util::StreamExt;
use poem::{
    get, handler, http::StatusCode, listener::TcpListener, EndpointExt, IntoResponse, Response,
    Route,
};
use tracing::{debug, error, info, level_filters::LevelFilter, warn};
// use juniper_graphql_ws::ConnectionConfig;
// use juniper_warp::subscriptions::serve_graphql_ws;
use structs::GQLOverState;
use text_to_ascii_art::convert;
use tokio::sync::RwLock;
use tracing_subscriber::filter;

use crate::gql::Query;
// use warp::Filter;

// use crate::gql::{create_schema, Context};

pub mod connection;
pub mod gql;
pub mod packets;
pub mod structs;

#[allow(non_snake_case)]
pub mod proto {
    pub mod discord {
        include!(concat!(env!("OUT_DIR"), "/proto.discord.rs"));
    }

    pub mod models {
        include!(concat!(env!("OUT_DIR"), "/proto.models.rs"));
    }

    pub mod packet {
        include!(concat!(env!("OUT_DIR"), "/proto.packet.rs"));
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub enum TAUpdates {
    NewState,

    #[default]
    None,
}

#[derive(Debug, Default, Clone, Copy)]
pub enum OverUpdates {
    NewPage,

    #[default]
    None,
}

lazy_static::lazy_static! {
    pub static ref TA_STATE : RwLock<packets::TAState> = {
        RwLock::new(packets::TAState::new())
    };
    // pub static ref TA_CON: RwLock<Option<connection::TAConnection>> = {
    //     RwLock::new(None)
    // };
    pub static ref OVER_STATE : RwLock<GQLOverState> = {
        RwLock::new(GQLOverState::default())
    };

    pub static ref TA_UPDATE_SINK: Sink<TAUpdates> = {
        Sink::new()
    };
    pub static ref OVER_UPDATE_SINK: Sink<OverUpdates> = {
        Sink::new()
    };
}

#[handler]
async fn graphiql_route() -> Response {
    Response::builder()
        .content_type("text/html; charset=utf-8")
        .body(GraphiQLSource::build().endpoint("/graphql").finish())
}

#[handler]
async fn playground_route() -> Response {
    Response::builder()
        .content_type("text/html; charset=utf-8")
        .body(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
}

#[handler]
fn options() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::OK)
        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .header(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization, Access-Control-Allow-Origin",
        )
        .body("")
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(filter::EnvFilter::builder().with_default_directive(LevelFilter::INFO.into()).from_env()?)
        .init();
    // show a pretty ascii banner
    info!(
        "{} v{}\n Created by {}",
        convert("TARS".to_string()).expect("should not fail"),
        env!("CARGO_PKG_VERSION"),
        &env!("CARGO_PKG_AUTHORS").replace(':', " & ")
    );

    safety_checks();

    info!("Connecting to Server...");
    // *TA_CON.write().await = Some(
    //     match connection::TAConnection::connect(std::env::var("TA_WS_URI").expect("passed safety checks, should not fail"),"TA-Relay-TX").await {
    //         Ok(con) => con,
    //         Err(e) => {
    //             error!("Failed to connect to server (tx). Check your websocket uri.");
    //             debug!("Error: {}", e);
    //             std::process::exit(1);
    //         },
    //     },
    // );
    let mut ta_con = match connection::TAConnection::connect(
        std::env::var("TA_WS_URI").expect("passed safety checks, should not fail"),
        "TA-Relay-RX",
    )
    .await
    {
        Ok(con) => con,
        Err(e) => {
            error!("Failed to connect to server (rx). Check your websocket uri.");
            debug!("Error: {}", e);
            std::process::exit(1);
        }
    };

    std::thread::spawn(move || {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
            .block_on(async move {
                while let Some(msg) = ta_con.next().await {
                    let msg = match msg {
                        Ok(msg) => msg,
                        Err(e) => {
                            error!("Error receiving message: {}", e);
                            continue;
                        }
                    };
                    tokio::spawn(async move {
                        match packets::route_packet(&mut *TA_STATE.write().await, msg.clone()).await
                        {
                            Ok(_) => {}
                            Err(e) => {
                                warn!("Error routing packet. {:#?}", msg);
                                debug!("Error: {}", e);
                            }
                        };

                        TA_UPDATE_SINK.send(TAUpdates::NewState);
                    });
                }
            });
    });

    let schema = Schema::build(Query, EmptyMutation, EmptySubscription).finish();

    let app = Route::new()
        .at(
            "/graphql",
            get(GraphQL::new(schema.clone()))
                .post(GraphQL::new(schema))
                .options(options),
        )
        .at("/graphiql", graphiql_route)
        .at("/playground", playground_route)
        .with(poem::middleware::Tracing)
        .with(poem::middleware::SetHeader::new().appending("Access-Control-Allow-Origin", "*"));

    poem::Server::new(TcpListener::bind("0.0.0.0:8080"))
        .run(app)
        .await?;

    Ok(())
}

fn safety_checks() {
    let mut failed = false;

    if std::env::var("TA_WS_URI").is_err() {
        error!("TA_WS_URI not set in .env");
        failed = true;
    }

    if let Ok(uri) = std::env::var("TA_WS_URI") {
        let port = uri.split(':').last().unwrap_or("");
        if port == "2052" {
            info!("TA_WS_URI is set to use port 2052, but the default port is 2053. Are you using the correct port?");
        }
    }

    if failed {
        std::process::exit(1);
    }
}

pub fn get_ws_uri() -> String {
    std::env::var("TA_WS_URI").expect("TA_WS_URI not set in .env")
}

pub fn parse_uuid(uuid: &str) -> uuid::Uuid {
    uuid::Uuid::parse_str(uuid).expect("Failed to parse UUID")
}
