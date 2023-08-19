use std::{convert::Infallible, sync::Arc, time::Duration};

// use actix_cors::Cors;
// use actix_web::{get, middleware::Logger, App, HttpServer, Responder, HttpResponse, Error, web::{self, Data}, http::header, HttpRequest};
use carboxyl::Sink;
use futures_util::{FutureExt, StreamExt};
use gql::Schema;
use juniper_graphql_ws::ConnectionConfig;
use juniper_warp::subscriptions::serve_graphql_ws;
use text_to_ascii_art::convert;
use tokio::sync::RwLock;
use warp::Filter;

use crate::gql::{create_schema, Context};

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

lazy_static::lazy_static! {
    pub static ref TA_STATE : RwLock<packets::TAState> = {
        RwLock::new(packets::TAState::new())
    };
    pub static ref TA_CON: RwLock<Option<connection::TAConnection>> = {
        RwLock::new(None)
    };

    pub static ref TA_UPDATE_SINK: Sink<TAUpdates> = {
        Sink::new()
    };
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();
    // show a pretty ascii banner
    log::info!(
        "{} v{}\n Created by {}",
        convert("TA-Relay-rs".to_string()).unwrap(),
        env!("CARGO_PKG_VERSION"),
        &env!("CARGO_PKG_AUTHORS").replace(":", " & ")
    );

    std::thread::spawn(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                log::info!("Connecting to Server...");
                *TA_CON.write().await = Some(
                    connection::TAConnection::connect(
                        std::env::var("TA_WS_URI").unwrap(),
                        "TA-Relay-TX",
                    )
                    .await
                    .unwrap(),
                );
                let mut ta_con = connection::TAConnection::connect(
                    std::env::var("TA_WS_URI").unwrap(),
                    "TA-Relay-RX",
                )
                .await
                .unwrap();

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

    // HttpServer::new(||
    //     App::new()
    //         .app_data(Data::new(create_schema()))
    //         .service(index)
    //         .service(
    //             web::resource("/graphql")
    //                 .route(web::post().to(graphql_route))
    //                 .route(web::get().to(graphql_route)),
    //         )
    //         .service(web::resource("/graphiql").route(web::get().to(graphiql_route)))
    //         .service(web::resource("/playground").route(web::get().to(playground_route)))
    //         .wrap(
    //             Cors::default()
    //                 .allow_any_origin()
    //                 .allowed_methods(vec!["POST", "GET"])
    //                 .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
    //                 .allowed_header(header::CONTENT_TYPE)
    //                 .supports_credentials()
    //                 .max_age(3600),
    //         )
    //         .wrap(Logger::default())
    //     )
    //     .bind(("0.0.0.0", 8080))?
    //     .run()
    //     .await?;
    let state = warp::any().map(move || Context {});

    let graphql_filter = juniper_warp::make_graphql_filter(create_schema(), state.boxed());
    let log = warp::log("ta_relay_rs");
    let root_node = Arc::new(create_schema());

    warp::serve(
        warp::get()
            .and(warp::path("graphiql").and(juniper_warp::graphiql_filter("/graphql", None)))
            .or(warp::path("playground").and(juniper_warp::playground_filter("/graphql", None)))
            .or(warp::path("graphql")
                .and(warp::ws())
                .map(move |ws: warp::ws::Ws| {
                    let root_node = root_node.clone();
                    let config = ConnectionConfig::new(Context {});
                    let config = config.with_keep_alive_interval(Duration::from_secs(15));
                    ws.on_upgrade(move |websocket| async move {
                        serve_graphql_ws(websocket, root_node, config)
                            .map(|r| {
                                if let Err(e) = r {
                                    println!("Websocket error: {e}");
                                }
                            })
                            .await
                    })
                })
                .map(|reply| {
                    // this is todo in the example, but it's required for the magic websocket magic to work!
                    warp::reply::with_header(reply, "Sec-WebSocket-Protocol", "graphql-ws")
                }))
            .or(warp::path("graphql").and(graphql_filter))
            .with(log),
    )
    .run(([0, 0, 0, 0], 8080))
    .await;
    Ok(())
}

// #[get("/")]
// async fn index() -> impl Responder {
//     ""
// }

// async fn graphiql_route() -> Result<HttpResponse, Error> {
//     juniper_actix::graphiql_handler("/graphql", None).await
// }

// async fn playground_route() -> Result<HttpResponse, Error> {
//     juniper_actix::playground_handler("/graphql", None).await
// }

// async fn graphql_route(
//     req: HttpRequest,
//     payload: web::Payload,
//     data: web::Data<Schema>,
// ) -> Result<HttpResponse, Error> {
//     juniper_actix::graphql_handler(&data, &gql::Context {  }, req, payload).await
// }

// async fn subscriptions_route(
//     req: HttpRequest,
//     stream: web::Payload,
//     schema: web::Data<Schema>,
// ) -> Result<HttpResponse, Error> {
//     let config = ConnectionConfig::new(schema.into_inner());
//     // set the keep alive interval to 15 secs so that it doesn't timeout in playground
//     // playground has a hard-coded timeout set to 20 secs
//     let config = config.with_keep_alive_interval(Duration::from_secs(15));

//     // juniper_actix::subscriptions_handler(req, stream, schema, config).await
// }
