//! KaosNet Rust Client SDK
//!
//! Official Rust client library for connecting to KaosNet game servers.
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
//!     // Create real-time socket
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

mod client;
mod error;
mod session;
mod socket;
mod types;

pub use client::{KaosClient, KaosClientBuilder};
pub use error::{Error, Result};
pub use session::Session;
pub use socket::KaosSocket;
pub use types::*;
