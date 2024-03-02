use futures_util::{stream::{SplitStream, SplitSink}, StreamExt, SinkExt, Stream};
use prost::Message as _;
use tokio::net::TcpStream;
use tokio_tungstenite::{WebSocketStream, MaybeTlsStream, tungstenite::Message};
use tracing::error;
use crate::proto::{models, packet};

#[derive(Debug)]
pub struct TAConnection {
    pub ws_rx: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    pub ws_tx: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    pub ws_user: models::User,
}

impl TAConnection {
    pub async fn connect<T: Into<String>, U: Into<String>>(uri: T, name: U) -> anyhow::Result<Self> {
        let uri = uri.into();
        let name = name.into();
        let (ws_stream, _) = match tokio_tungstenite::connect_async(uri).await {
            Ok(c) => c,
            Err(_) => {
                error!("Failed to connect to server. Are you sure you're connecting to the overlay websocket?");
                return Err(anyhow::anyhow!("Failed to connect to server. Are you sure you're connecting to the overlay websocket?"));
            },
        };
        let (mut ws_tx, ws_rx) = ws_stream.split();
        let ws_user = models::User { 
            guid: uuid::Uuid::new_v4().to_string(),
            client_type: models::user::ClientTypes::WebsocketConnection.into(),
            name, 
            ..Default::default()
        };

        let connect = packet::Packet {
            id: uuid::Uuid::new_v4().to_string(),
            from: "".to_string(),
            packet: Some(packet::packet::Packet::Request(packet::Request {
                r#type: Some(packet::request::Type::Connect(packet::request::Connect {
                    user: Some(ws_user.clone()),
                    password: "".to_string(),
                    client_version: 74
                }))
            }))
        };

        match ws_tx.send(Message::Binary(connect.encode_to_vec())).await {
            Ok(_) => {},
            Err(e) => {
                error!("Failed to send connect packet. {:#?}", e);
                return Err(anyhow::anyhow!("Failed to send connect packet. {:#?}", e));
            },
        };

        Ok(TAConnection {
            ws_rx,
            ws_tx,
            ws_user,
        })
    }  

    pub async fn send(&mut self, packet: packet::Packet) -> anyhow::Result<()> {
        self.ws_tx.send(Message::Binary(packet.encode_to_vec())).await?;
        Ok(())
    }

    pub async fn close(self) {
        drop(self);
    } 
}

impl Stream for TAConnection {
    type Item = anyhow::Result<packet::Packet>;

    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
        let p = self.ws_rx.poll_next_unpin(cx);
        match p {
            std::task::Poll::Ready(Some(Ok(msg))) => {
                let packet = packet::Packet::decode(msg.into_data().as_slice())?;
                std::task::Poll::Ready(Some(Ok(packet)))
            },
            std::task::Poll::Ready(Some(Err(e))) => {
                std::task::Poll::Ready(Some(Err(anyhow::anyhow!(e))))
            },
            std::task::Poll::Ready(None) => {
                std::task::Poll::Ready(None)
            },
            std::task::Poll::Pending => {
                std::task::Poll::Pending
            }
        }
    }
}