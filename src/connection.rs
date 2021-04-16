use anyhow::{Error, Result, anyhow};
use serde_json::{self, Value};
use futures::{future::ready, select, sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tokio_stream::wrappers::ReceiverStream;
use async_tungstenite::tungstenite;
use slab::Slab;
use crate::protocol::{Message, NumericID};

#[derive(Debug)]
struct Method {
    name: String,
    params: Vec<Value>,
    result: oneshot::Sender<MethodResult>,
}

type MethodResult = std::result::Result<Value,Value>;

#[derive(Debug)]
pub struct Connection {
    stream: mpsc::Receiver<Message>,
    rpc: mpsc::Sender<Method>,
}

// this is cursed
type WSStream = async_tungstenite::WebSocketStream<
    async_tungstenite::stream::Stream<
        async_tungstenite::tokio::TokioAdapter<tokio::net::TcpStream>,
        async_tungstenite::tokio::TokioAdapter<
            tokio_rustls::client::TlsStream<
                tokio::net::TcpStream
            >
        >
    >>;


impl Connection {

    pub async fn connect(url: &str) -> Result<Self> {

        let tlsconfig = {
            let mut tlsconfig = tokio_rustls::rustls::ClientConfig::new();
            tlsconfig.root_store = rustls_native_certs::load_native_certs()
                .map_err(|(_store, err)| err)?;
            Arc::new(tlsconfig)
        };

        let tls = tokio_rustls::TlsConnector::from(tlsconfig);

        let (stream, response) =
            async_tungstenite::tokio::connect_async_with_tls_connector(url, Some(tls)).await?;

        eprintln!("Got response from websocket: {:?}", response);


        Self::connect_with_websocket(stream).await
    }

    pub async fn connect_with_websocket(stream: WSStream) -> Result<Self> {
        

        let (ws_up, mut ws_down) = stream.split();

        let mut ws_up = ws_up.with(|m: Message| {
            let payload = serde_json::to_string(&m).unwrap();
            eprintln!("WS -> {}", payload);
            ready(Ok::<_,tungstenite::Error>(tungstenite::Message::Text(payload)))
        } );

        let connect_msg = Message::Connect { version: "1".to_string(),
                                                     support: vec!["1".to_string()],
                                                     session: None };

        ws_up.send(connect_msg).await?;

        //TODO actually check these
        let _server_version = ws_down.next().await.ok_or(anyhow!("no server version"))?;
        let _connected = ws_down.next().await.ok_or(anyhow!("no connected msg"))?;

        let mut ws_down = ws_down.map(|m| {
            match m {
                Ok(tungstenite::Message::Text(txt)) => {
                    eprintln!("WS <- {}", txt);
                    serde_json::from_str::<Message>(&txt)
                    .map_err(Error::from)
                },
                other => Err(anyhow!("unhandled down message: {:?}", other))
            }
        }).fuse();

        let (down_tx, down_rx) = mpsc::channel::<Message>(16);
        let (up_tx, up_rx) = mpsc::channel::<Method>(16);

        tokio::spawn(async move {

            let mut pending: Slab<oneshot::Sender<MethodResult>> = Slab::new();
            let mut up_rx = ReceiverStream::new(up_rx).fuse();

            loop {

                select! {
                    msg = ws_down.next() => {
                        eprintln!("Received message {:?}", msg);

                        let msg = msg.ok_or(anyhow!("end of ws stream"))??;

                        match msg {
                            Message::Ping => {
                                eprintln!("Sending pong");
                                ws_up.send(Message::Pong).await?;
                            },
                    
                            Message::Result(r) => {
                                let id = r.id();
                                if pending.contains(id) {
                                    pending.remove(id)
                                        .send(r.into())
                                        .map_err(|e| anyhow!("Could not deliver reply {:?}", e))?

                                } else {
                                    return Err::<(),Error>(anyhow!("Unknown call response ID"))
                                }

                            },
                            other => {
                                down_tx.send(other).await?;
                            }
                        }
                    },

                    msg = up_rx.next() => {
                        let Method { name, params, result } = msg.ok_or(anyhow!("end of method stream"))?;
                        let id = NumericID(pending.insert(result));

                        let message = Message::Method { id, method: name, params };
                        ws_up.send(message).await?
                    }
                }
            }

        });


        Ok(Self { stream: down_rx, rpc: up_tx })
    }

    pub async fn recv(&mut self) -> Option<Message> {
        self.stream.recv().await
    }

    pub async fn call(&mut self, name: String, params: Vec<Value>) -> Result<Value> {
        let (tx, rx) = oneshot::channel();
        let request = Method { name, params, result: tx };
        self.rpc.send(request).await?;
        rx.await?.map_err(|e| anyhow!("RPC Call returned an error: {}", e))
    }

}