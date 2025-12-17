//! # KaosNet
//!
//! A high-performance game server framework with Lua scripting.
//!
//! ## Features
//!
//! - **Session Management**: Client connections with heartbeat tracking
//! - **Rooms/Matches**: Multiplayer coordination with tick-based updates
//! - **RPC**: Custom server functions callable from clients
//! - **Lua Scripting**: Server-side logic via sandboxed Lua runtime
//! - **Storage**: Persistent key-value and document storage
//! - **Leaderboards**: Score tracking with rankings
//! - **Matchmaking**: Skill-based matchmaking with parties
//! - **Social**: Friends, groups, and presence
//! - **Notifications**: Real-time and persistent push notifications
//! - **Tournaments**: Scheduled competitions with rankings
//! - **Chat**: Real-time messaging with channels and DMs

// Core modules (always available)
pub mod error;
pub mod peer;
pub mod protocol;
pub mod room;
pub mod server;
pub mod session;
pub mod transport;

// Game services (always available)
pub mod chat;
pub mod leaderboard;
pub mod matchmaker;
pub mod notifications;
pub mod ratelimit;
pub mod social;
pub mod storage;
pub mod tournament;

// Hooks system (always available)
pub mod hooks;

// Hooked service wrappers (always available)
pub mod services;

// Match handler (always available)
pub mod match_handler;

// Auth (requires feature)
#[cfg(feature = "auth")]
pub mod auth;

// Lua scripting (requires feature)
#[cfg(feature = "lua")]
pub mod lua;

// Console admin (requires feature)
#[cfg(feature = "console")]
pub mod console;

// Prometheus metrics (requires feature)
#[cfg(feature = "metrics")]
pub mod metrics;

// OpenTelemetry tracing (requires feature)
#[cfg(feature = "telemetry")]
pub mod telemetry;

// Re-exports - Core
pub use error::{KaosNetError, Result};
pub use protocol::{Message, Op};
pub use room::{Room, RoomConfig, RoomInfo, RoomRegistry, RoomSnapshot, RoomState};
pub use server::{Server, ServerBuilder, ServerConfig};
pub use session::{Presence, Session, SessionRegistry, SessionSnapshot, SessionState};
pub use transport::{
    ClientId, ClientTransport, TransportServer,
    WsServerTransport, WsClientTransport,
};

// Re-exports - Game Services
pub use chat::{Chat, Channel, ChannelType, ChatMessage};
pub use leaderboard::{Leaderboards, LeaderboardConfig, LeaderboardRecord, SortOrder, ScoreOperator, ResetSchedule};
pub use matchmaker::{Matchmaker, MatchmakerConfig, MatchmakerTicket, MatchmakerMatch};
pub use notifications::{Notifications, Notification, NotificationCode};
pub use ratelimit::{RateLimiter, RateLimitConfig, RateLimitResult, OperationRateLimiter, RateLimitPresets};
pub use social::{Social, Friend, FriendState, Group, GroupRole, PresenceStatus};
pub use storage::{Storage, StorageObject, ObjectPermission, Query, StorageBackend, MemoryBackend};

// Re-exports - PostgreSQL storage (conditional)
#[cfg(feature = "postgres")]
pub use storage::{PostgresBackend, PostgresSyncBackend, AsyncStorageBackend};
pub use tournament::{Tournaments, Tournament, TournamentConfig, TournamentRecord, TournamentState};

// Re-exports - Hooks
pub use hooks::{
    HookRegistry, HookExecutor, HookError,
    HookOperation, HookContext,
    BeforeHookResult, BeforeHook, AfterHook,
};

// Re-exports - Hooked Services
pub use services::{
    HookedStorage, HookedLeaderboards, HookedSocial,
    ServiceError,
};

// Re-exports - Match Handler
pub use match_handler::{
    MatchHandler, MatchHandlerRegistry, MatchRegistry, MatchError,
    MatchContext, MatchState, MatchInit, MatchMessage, MatchPresence,
    MatchDispatcher, MatchLifecycle, Match,
};

// Re-exports - Auth (conditional)
#[cfg(feature = "auth")]
pub use auth::{
    AuthService, AuthConfig, AuthError,
    DeviceAuthRequest, EmailAuthRequest, CustomAuthRequest,
    AuthResponse, UserAccountInfo,
    LinkDeviceRequest, LinkEmailRequest,
    UserAccount, AccountId, DeviceLink, AuthProvider,
    AccountStore, MemoryAccountStore,
    ClientToken, ClientClaims, RefreshToken,
};

// Re-exports - Lua (conditional)
#[cfg(feature = "lua")]
pub use lua::{LuaConfig, LuaContext, LuaRuntime, LuaMatchHandler, LuaServices};

// Re-exports - Console (conditional)
#[cfg(feature = "console")]
pub use console::{ConsoleServer, ConsoleConfig};

// Re-exports - Metrics (conditional)
#[cfg(feature = "metrics")]
pub use metrics::Metrics;

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
