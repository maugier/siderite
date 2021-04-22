use anyhow::{Error, Result, anyhow};
use serde_json::{self, Value};
use futures::{Stream, channel::{mpsc, oneshot}, future::ready, select, sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use async_tungstenite::tungstenite;
use crate::randomslab::Slab;
use crate::protocol::{ClientMessage, ServerMessage, MethodResponse};
use log::{debug, trace, error};

#[derive(Debug, PartialEq, Eq)]
pub struct RPCError(Value);

impl std::fmt::Display for RPCError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RPC Error: {}", self.0)
    }
}

impl std::error::Error for RPCError {}

pub type MethodResult = std::result::Result<Value,RPCError>;

impl Into<MethodResult> for MethodResponse {
    fn into(self) -> MethodResult {
        match self {
            MethodResponse { error: Some(error), .. } => Err(RPCError(error)),
            MethodResponse { result,.. } => Ok(result.unwrap_or(Value::Null)),
        }
    }
}

#[derive(Debug)]
enum Request {
    Method {
        name: String,
        params: Vec<Value>,
        result: oneshot::Sender<MethodResult>,
    },
    Subscribe {
        name: String,
        id: String,
        params: Vec<Value>,
    },
    Unsubscribe {
        id: String,
    }

}

#[derive(Debug)]
pub struct Connection {
    stream: mpsc::Receiver<ServerMessage>,
    handle: Handle,
}

#[derive(Clone, Debug)]
pub struct Handle {
    rpc: mpsc::Sender<Request>,
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

        debug!(target: "websocket", "Got HTTP response: {:?}", response);


        Self::connect_with_websocket(stream).await
    }

    pub async fn connect_with_websocket(stream: WSStream) -> Result<Self> {
        

        let (ws_up, mut ws_down) = stream.split();

        let mut ws_up = ws_up.with(|m: ClientMessage| {
            let payload = serde_json::to_string(&m).unwrap();
            trace!("=> {}", payload);
            ready(Ok::<_,tungstenite::Error>(tungstenite::Message::Text(payload)))
        } );

        let connect_msg = ClientMessage::Connect { version: "1".to_string(),
                                                     support: vec!["1".to_string()],
                                                     session: None };

        ws_up.send(connect_msg).await?;

        //TODO actually check these
        let _server_version = ws_down.next().await.ok_or(anyhow!("no server version"))?;
        let _connected = ws_down.next().await.ok_or(anyhow!("no connected msg"))?;

        let mut ws_down = ws_down.map(|m| {
            match m {
                Ok(tungstenite::Message::Text(txt)) => {
                    trace!("<= {}", txt);
                    serde_json::from_str::<ServerMessage>(&txt)
                    .map_err(Error::from)
                },
                other => Err(anyhow!("unhandled down message: {:?}", other))
            }
        }).fuse();

        let (mut down_tx, down_rx) = mpsc::channel::<ServerMessage>(16);
        let (up_tx, mut up_rx) = mpsc::channel::<Request>(16);

        let actor = tokio::spawn(async move {

            let mut pending: Slab<oneshot::Sender<MethodResult>> = Slab::new();
            //let mut up_rx = ReceiverStream::new(up_rx).fuse();

            loop {

                select! {
                    msg = ws_down.next() => {

                        let msg = msg.ok_or(anyhow!("end of ws stream"))??;

                        match msg {
                            ServerMessage::Ping { id } => {
                                debug!("Answering ping request");
                                ws_up.send(ClientMessage::Pong { id }).await?;
                            },
                    
                            ServerMessage::Result(r) => {
                                if let Some(chan) = pending.remove(&r.id) {
                                    // Our caller dropped, what're we gonna do?
                                    let _ = chan.send(r.into());
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
                        match msg.ok_or(anyhow!("end of method stream"))? {
                            Request::Method { name, params, result } => {
                                let id = pending.insert(result);
                                let message = ClientMessage::Method { id, method: name, params };
                                ws_up.send(message).await?
                            },
                            Request::Subscribe { name, id, params } => {
                                let message = ClientMessage::Sub { id, name, params };
                                ws_up.send(message).await?
                            },
                            Request::Unsubscribe { id } => {
                                let message = ClientMessage::Unsub { id };
                                ws_up.send(message).await?
                            }
                        }
                    }
                }
            }

        });

        tokio::spawn(async move {
            let res = actor.await;
            error!("Siderite worker has terminated: {:?}", res);
        });

        Ok(Self { stream: down_rx, handle: Handle { rpc: up_tx } })
    }

    pub fn stream(&mut self) -> &mut impl Stream<Item = ServerMessage> {
        &mut self.stream
    }

    pub async fn recv(&mut self) -> Option<ServerMessage> {
        self.stream.next().await
    }

    pub fn handle(&self) -> Handle {
        self.handle.clone()
    }

    pub async fn call(&mut self, name: String, params: Vec<Value>) -> Result<MethodResult> {
        self.handle.call(name, params).await
    }

    pub async fn subscribe(&mut self, id: String, name: String, params: Vec<Value>) -> Result<()> {
        self.handle.subscribe(id, name, params).await
    }

    pub async fn unsubscribe(&mut self, id: String) -> Result<()> {
        self.handle.unsubscribe(id).await
    }

}

impl Handle {

    pub async fn call(&mut self, name: String, params: Vec<Value>) -> Result<MethodResult> {
        let (tx, rx) = oneshot::channel();
        let request = Request::Method { name, params, result: tx };
        self.rpc.send(request).await?;
        Ok(rx.await?)
    }

    pub async fn subscribe(&mut self, id: String, name: String, params: Vec<Value>) -> Result<()> {
        let request = Request::Subscribe { name, id, params };
        self.rpc.send(request).await?;
        Ok(())
    }

    pub async fn unsubscribe(&mut self, id: String) -> Result<()> {
        let request = Request::Unsubscribe { id };
        self.rpc.send(request).await?;
        Ok(())
    }

} 