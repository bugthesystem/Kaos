//! Console admin interface for KaosNet.
//!
//! Provides HTTP API for server monitoring and management.

pub mod auth;
pub mod handlers;
pub mod server;
pub mod storage;
pub mod types;

pub use auth::{AuthService, Identity, Role};
pub use server::{ConsoleConfig, ConsoleServer, ConsoleServerBuilder, ServerContext};
pub use storage::{AccountStore, ApiKeyStore};
pub use types::*;
