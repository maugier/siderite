//! A simple wrapper for the 
//! [Meteor DDP](https://github.com/meteor/meteor/blob/devel/packages/ddp/DDP.md) protocol.
//!
//! ```
//! let connection = siderite::Connection::connect("wss://example.com/websocket").await?;
//! 
//! // Make a RPC task in an independant task:
//! let handle = connection.handle();
//! tokio::spawn(async move {
//!     let r = handle.call("login", vec!["username".into(), "my-secret-token".into()])
//!                        .await?;  // this throws if the RPC call could not complete
//!                        .map_err(|e| eprintln!("Login failed with reason: {}", e))?
//!
//!     ...
//! });
//!
//! // Consume the stream
//! while let Some(msg) = connection.recv().await {
//!    match msg {
//!       ServerMessage::Added{..} => { ... }
//!    }
//! }
//! ```


/// This contains the message types defined in the DDP spec
pub mod protocol;

/// This offers an async interface for connecting to a DDP endpoint and exchange messages.
pub mod connection;

mod randomslab;

pub use connection::{Connection, Handle};
pub use protocol::{ClientMessage, ServerMessage, Timestamp};
