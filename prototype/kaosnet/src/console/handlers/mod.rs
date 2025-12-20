//! API handlers for console endpoints.

mod auth;
mod client_auth;
mod status;
mod sessions;
mod rooms;
mod accounts;
mod api_keys;
mod players;
mod leaderboards;
mod chat;
mod social;
mod storage;
mod tournaments;
mod matchmaker;
mod notifications;
mod lua;
mod audit_logs;

pub use auth::*;
pub use client_auth::*;
pub use status::*;
pub use sessions::*;
pub use rooms::*;
pub use accounts::*;
pub use api_keys::*;
pub use players::*;
pub use leaderboards::*;
pub use chat::*;
pub use social::*;
pub use storage::*;
pub use tournaments::*;
pub use matchmaker::*;
pub use notifications::*;
pub use lua::*;
pub use audit_logs::*;
