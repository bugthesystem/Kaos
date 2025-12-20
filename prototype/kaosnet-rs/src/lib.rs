//! KaosNet Rust Client SDK
//!
//! Official Rust client library for connecting to KaosNet game servers.
//!
//! ## Features
//!
//! - **HTTP Client**: REST API for authentication, storage, leaderboards, matchmaking
//! - **WebSocket**: Real-time communication for chat, notifications, presence
//! - **RUDP**: Low-latency UDP transport for game state synchronization
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use kaosnet_rs::{KaosClient, Result};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Create client
//!     let client = KaosClient::new("localhost", 7350);
//!
//!     // Authenticate
//!     let session = client.authenticate_device("my-device-id").await?;
//!     println!("Logged in as: {}", session.user_id);
//!
//!     // Create real-time socket (WebSocket)
//!     let socket = client.create_socket();
//!     socket.connect(&session).await?;
//!
//!     // Join matchmaker
//!     let ticket = client.add_matchmaker(&session, "ranked")
//!         .string_property("region", "us")
//!         .numeric_property("skill", 1500.0)
//!         .send()
//!         .await?;
//!
//!     println!("Matchmaker ticket: {}", ticket.id);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Using RUDP for Low-Latency Game Data
//!
//! ```rust,no_run
//! use kaosnet_rs::RudpClient;
//! use std::net::SocketAddr;
//!
//! fn game_loop() -> std::io::Result<()> {
//!     let server: SocketAddr = "127.0.0.1:7351".parse().unwrap();
//!     let mut rudp = RudpClient::connect(server)?;
//!
//!     loop {
//!         // Send player state (60 Hz)
//!         rudp.send(1, b"x:100,y:200,rot:45")?;
//!
//!         // Receive other players' states
//!         rudp.receive(|op_code, data| {
//!             // Handle game state update
//!         });
//!
//!         std::thread::sleep(std::time::Duration::from_millis(16));
//!     }
//! }
//! ```

mod client;
mod error;
mod rudp;
mod session;
mod socket;
mod types;

pub use client::{KaosClient, KaosClientBuilder};
pub use error::{Error, Result};
pub use rudp::{RudpClient, RudpConfig, MatchDataPacket};
pub use session::Session;
pub use socket::{KaosSocket, ReconnectConfig};
pub use types::*;
