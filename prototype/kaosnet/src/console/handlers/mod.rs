//! API handlers for console endpoints.

mod auth;
mod status;
mod sessions;
mod rooms;
mod accounts;
mod api_keys;

pub use auth::*;
pub use status::*;
pub use sessions::*;
pub use rooms::*;
pub use accounts::*;
pub use api_keys::*;
