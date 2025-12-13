//! # KaosNet
//!
//! Game server built on kaos-rudp with Lua scripting.
//!
//! ## Features
//!
//! - **Session Management**: Client connections with heartbeat tracking
//! - **Rooms/Matches**: Multiplayer coordination with tick-based updates
//! - **RPC**: Custom server functions callable from clients
//! - **Lua Scripting**: Server-side logic via sandboxed Lua runtime
//!
//! ## Example
//!
//! ```rust,ignore
//! use kaosnet::{Server, ServerBuilder};
//!
//! let server = ServerBuilder::new()
//!     .bind("0.0.0.0:7350")
//!     .tick_rate(60)
//!     .lua_scripts("scripts")
//!     .build()?;
//!
//! server.run()?;
//! ```

pub mod error;
pub mod peer;
pub mod protocol;
pub mod room;
pub mod server;
pub mod session;

#[cfg(feature = "lua")]
pub mod lua;

// Re-exports
pub use error::{KaosNetError, Result};
pub use protocol::{Message, Op};
pub use room::{Room, RoomConfig, RoomRegistry, RoomState};
pub use server::{Server, ServerBuilder, ServerConfig};
pub use session::{Presence, Session, SessionRegistry, SessionState};

#[cfg(feature = "lua")]
pub use lua::{LuaConfig, LuaContext, LuaRuntime};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_roundtrip() {
        let msg = Message::new(Op::Heartbeat, vec![1, 2, 3]);
        let encoded = msg.encode();
        let (decoded, _) = Message::decode(&encoded).unwrap();
        assert_eq!(decoded.op, Op::Heartbeat);
        assert_eq!(decoded.payload, vec![1, 2, 3]);
    }

    #[test]
    fn test_session_lifecycle() {
        let registry = SessionRegistry::new();
        let addr = "127.0.0.1:8080".parse().unwrap();

        let id = registry.create(addr);
        assert_eq!(registry.count(), 1);

        {
            let session = registry.get(id).unwrap();
            assert_eq!(session.state, SessionState::Connecting);
        }

        registry.remove(id);
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_room_lifecycle() {
        let registry = RoomRegistry::new();

        let id = registry.create(RoomConfig::default());
        assert_eq!(registry.count(), 1);

        registry.join(&id, 1).unwrap();
        {
            let room = registry.get(&id).unwrap();
            assert_eq!(room.player_count(), 1);
        }

        registry.leave(&id, 1);
        {
            let room = registry.get(&id).unwrap();
            assert_eq!(room.player_count(), 0);
        }
    }
}
