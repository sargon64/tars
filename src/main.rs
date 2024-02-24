#![allow(clippy::option_map_unit_fn)]
// #![forbid(clippy::unwrap_used)]



use async_graphql::{http::{playground_source, GraphQLPlaygroundConfig, GraphiQLSource}, EmptyMutation, EmptySubscription, Schema};
use async_graphql_poem::GraphQL;
// use actix_cors::Cors;
// use actix_web::{get, middleware::Logger, App, HttpServer, Responder, HttpResponse, Error, web::{self, Data}, http::header, HttpRequest};
use carboxyl::Sink;
use futures_util::StreamExt;
use log::{error, info};
use poem::{get, handler, http::StatusCode, listener::TcpListener, EndpointExt, IntoResponse, Response, Route};
// use juniper_graphql_ws::ConnectionConfig;
// use juniper_warp::subscriptions::serve_graphql_ws;
use structs::GQLOverState;
use text_to_ascii_art::convert;
use tokio::sync::RwLock;

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
    pub static ref TA_CON: RwLock<Option<connection::TAConnection>> = {
        RwLock::new(None)
    };
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
    env_logger::init();
    // show a pretty ascii banner
    log::info!(
        "{} v{}\n Created by {}",
        convert("TARS".to_string()).unwrap(),
        env!("CARGO_PKG_VERSION"),
        &env!("CARGO_PKG_AUTHORS").replace(':', " & ")
    );

    safety_checks();

    log::info!("Connecting to Server...");
    *TA_CON.write().await = Some(
        connection::TAConnection::connect(std::env::var("TA_WS_URI").unwrap(), "TA-Relay-TX")
            .await
            .unwrap(),
    );
    let mut ta_con =
        connection::TAConnection::connect(std::env::var("TA_WS_URI").unwrap(), "TA-Relay-RX")
            .await
            .unwrap();

    std::thread::spawn(move || {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                while let Some(msg) = ta_con.next().await {
                    let msg = match msg {
                        Ok(msg) => msg,
                        Err(e) => {
                            log::error!("Error receiving message: {}", e);
                            continue;
                        }
                    };
                    tokio::spawn(async {
                        packets::route_packet(&mut *TA_STATE.write().await, msg)
                            .await
                            .unwrap();

                        TA_UPDATE_SINK.send(TAUpdates::NewState);
                    });
                }
            });
    });

    // let state = warp::any().map(move || Context {});

    // let graphql_filter = juniper_warp::make_graphql_filter(create_schema(), state.boxed());
    // let log = warp::log("ta_relay_rs");
    // let root_node = Arc::new(create_schema());

    // let cors = warp::cors()
    //     .allow_any_origin()
    //     .allow_methods(vec!["POST", "GET", "OPTIONS"])
    //     .allow_headers(vec![
    //         "User-Agent",
    //         "Sec-Fetch-Mode",
    //         "Referer",
    //         "Origin",
    //         "Content-Type",
    //         "Access-Control-Request-Method",
    //         "Access-Control-Request-Headers",
    //         "Sec-WebSocket-Protocol",
    //     ])
    //     .allow_credentials(true)
    //     .max_age(3600);

    // warp::serve(
    //     warp::get()
    //         .and(warp::path("graphiql").and(juniper_warp::graphiql_filter("/graphql", None)))
    //         .or(warp::path("playground").and(juniper_warp::playground_filter("/graphql", None)))
    //         .or(warp::path("graphql")
    //             .and(warp::ws())
    //             .map(move |ws: warp::ws::Ws| {
    //                 let root_node = root_node.clone();
    //                 let config = ConnectionConfig::new(Context {});
    //                 let config = config.with_keep_alive_interval(Duration::from_secs(15));
    //                 ws.on_upgrade(move |websocket| async move {
    //                     serve_graphql_ws(websocket, root_node, config)
    //                         .map(|r| {
    //                             if let Err(e) = r {
    //                                 println!("Websocket error: {e}");
    //                             }
    //                         })
    //                         .await
    //                 })
    //             })
    //             .map(|reply| {
    //                 // this is todo in the example, but it's required for the magic websocket magic to work!
    //                 warp::reply::with_header(reply, "Sec-WebSocket-Protocol", "graphql-ws")
    //             })
    //             .map(|reply| warp::reply::with_header(reply, "Access-Control-Allow-Origin", "*")))
    //         .or(warp::path("graphql").and(graphql_filter))
    //         .with(log)
    //         .with(cors),
    // )
    // .run(([0, 0, 0, 0], 8080))
    // .await;
    let schema = Schema::build(Query, EmptyMutation, EmptySubscription)
        .finish();

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
