//! Game server implementation.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use kaos_rudp::{RudpTransport, Transport};

use crate::error::{KaosNetError, Result};
use crate::peer::PeerManager;
use crate::protocol::{Message, Op, RoomDataPayload, RoomJoinPayload, RpcPayload, RpcResponsePayload, SessionStartPayload};
use crate::room::{RoomConfig, RoomRegistry};
use crate::session::{SessionRegistry, SessionState};

#[cfg(feature = "lua")]
use crate::lua::{LuaConfig, LuaContext, LuaRuntime};

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub tick_rate: u32,
    pub session_timeout_secs: u64,
    pub max_sessions: usize,
    #[cfg(feature = "lua")]
    pub lua: LuaConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:7350".to_string(),
            tick_rate: 60,
            session_timeout_secs: 30,
            max_sessions: 10000,
            #[cfg(feature = "lua")]
            lua: LuaConfig::default(),
        }
    }
}

/// Game server
pub struct Server {
    config: ServerConfig,
    transport: RudpTransport,
    sessions: Arc<SessionRegistry>,
    rooms: Arc<RoomRegistry>,
    peers: Arc<PeerManager>,
    #[cfg(feature = "lua")]
    lua: Arc<LuaRuntime>,
    last_cleanup: Instant,
}

impl Server {
    pub fn new(config: ServerConfig) -> Result<Self> {
        let bind_addr: SocketAddr = config.bind_addr.parse()
            .map_err(|_| KaosNetError::protocol("invalid bind address"))?;

        // Use a dummy remote for server (will receive from any)
        let dummy_remote: SocketAddr = "0.0.0.0:0".parse().unwrap();
        let transport = RudpTransport::new(bind_addr, dummy_remote, 4096)?;

        #[cfg(feature = "lua")]
        let lua = Arc::new(LuaRuntime::new(config.lua.clone())?);

        Ok(Self {
            config,
            transport,
            sessions: Arc::new(SessionRegistry::new()),
            rooms: Arc::new(RoomRegistry::new()),
            peers: Arc::new(PeerManager::new()),
            #[cfg(feature = "lua")]
            lua,
            last_cleanup: Instant::now(),
        })
    }

    /// Run the server main loop
    pub fn run(&mut self) -> Result<()> {
        let tick_duration = Duration::from_micros(1_000_000 / self.config.tick_rate as u64);
        let cleanup_interval = Duration::from_secs(5);

        loop {
            let tick_start = Instant::now();

            // Receive and process messages
            self.process_incoming()?;

            // Send queued outgoing messages
            self.flush_outgoing()?;

            // Periodic cleanup
            if self.last_cleanup.elapsed() >= cleanup_interval {
                self.cleanup()?;
                self.last_cleanup = Instant::now();
            }

            // Sleep for remaining tick time
            let elapsed = tick_start.elapsed();
            if elapsed < tick_duration {
                std::thread::sleep(tick_duration - elapsed);
            }
        }
    }

    /// Process incoming messages
    fn process_incoming(&mut self) -> Result<()> {
        // Collect messages first to avoid borrow conflict
        let mut messages = Vec::new();
        self.transport.receive(|data| {
            if let Some((msg, _)) = Message::decode(data) {
                messages.push(msg);
            }
        });

        // Process collected messages
        for msg in messages {
            let _ = self.handle_message(msg);
        }
        Ok(())
    }

    /// Handle a single message
    fn handle_message(&self, msg: Message) -> Result<()> {
        match msg.op {
            Op::Heartbeat => self.handle_heartbeat(&msg),
            Op::SessionStart => self.handle_session_start(&msg),
            Op::SessionEnd => self.handle_session_end(&msg),
            Op::RoomCreate => self.handle_room_create(&msg),
            Op::RoomJoin => self.handle_room_join(&msg),
            Op::RoomLeave => self.handle_room_leave(&msg),
            Op::RoomData => self.handle_room_data(&msg),
            Op::Rpc => self.handle_rpc(&msg),
            _ => Ok(()),
        }
    }

    fn handle_heartbeat(&self, _msg: &Message) -> Result<()> {
        // Extract session ID from context and touch
        Ok(())
    }

    fn handle_session_start(&self, msg: &Message) -> Result<()> {
        if msg.payload.len() < SessionStartPayload::SIZE {
            return Err(KaosNetError::protocol("invalid session start payload"));
        }

        // TODO: Get source address from transport context
        // For now, create session with placeholder address
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();

        let session_id = self.sessions.create(addr);
        self.peers.add(session_id, addr);

        if let Some(mut session) = self.sessions.get_mut(session_id) {
            session.state = SessionState::Connected;
        }

        // Send session ack
        self.peers.send(session_id, Message::session_ack(session_id));

        Ok(())
    }

    fn handle_session_end(&self, _msg: &Message) -> Result<()> {
        // Extract session ID and clean up
        Ok(())
    }

    fn handle_room_create(&self, msg: &Message) -> Result<()> {
        let config = if msg.payload.is_empty() {
            RoomConfig::default()
        } else {
            serde_json::from_slice(&msg.payload)?
        };

        let room_id = self.rooms.create(config);

        // TODO: Send room created response
        let _ = room_id;

        Ok(())
    }

    fn handle_room_join(&self, msg: &Message) -> Result<()> {
        let payload = RoomJoinPayload::decode(&msg.payload)
            .ok_or_else(|| KaosNetError::protocol("invalid room join payload"))?;

        // TODO: Get session ID from context
        let session_id = 0u64; // placeholder

        self.rooms.join(&payload.room_id, session_id)
            .map_err(|e| match e {
                crate::room::JoinError::NotFound => KaosNetError::room_not_found(&payload.room_id),
                crate::room::JoinError::Full => KaosNetError::room_full(&payload.room_id),
                crate::room::JoinError::Closed => KaosNetError::protocol("room closed"),
            })?;

        if let Some(mut session) = self.sessions.get_mut(session_id) {
            session.join_room(payload.room_id);
        }

        Ok(())
    }

    fn handle_room_leave(&self, msg: &Message) -> Result<()> {
        let payload = RoomJoinPayload::decode(&msg.payload)
            .ok_or_else(|| KaosNetError::protocol("invalid room leave payload"))?;

        // TODO: Get session ID from context
        let session_id = 0u64; // placeholder

        self.rooms.leave(&payload.room_id, session_id);

        if let Some(mut session) = self.sessions.get_mut(session_id) {
            session.leave_room();
        }

        Ok(())
    }

    fn handle_room_data(&self, msg: &Message) -> Result<()> {
        let payload = RoomDataPayload::decode(&msg.payload)
            .ok_or_else(|| KaosNetError::protocol("invalid room data payload"))?;

        // Get all players in room except sender
        if let Some(room) = self.rooms.get(&payload.room_id) {
            let players = room.player_ids();
            let broadcast = Message::new(Op::RoomData, msg.payload.clone());

            for &pid in &players {
                if pid != payload.sender {
                    self.peers.send(pid, broadcast.clone());
                }
            }
        }

        Ok(())
    }

    #[cfg(feature = "lua")]
    fn handle_rpc(&self, msg: &Message) -> Result<()> {
        let payload = RpcPayload::decode(&msg.payload)
            .ok_or_else(|| KaosNetError::protocol("invalid rpc payload"))?;

        // TODO: Get session ID from context
        let session_id = 0u64; // placeholder

        let ctx = LuaContext::new(session_id);

        match self.lua.call_rpc(&payload.method, &ctx, &payload.data) {
            Ok(result) => {
                let response = RpcResponsePayload {
                    id: payload.id,
                    success: true,
                    data: result,
                };
                self.peers.send(session_id, Message::new(Op::RpcResponse, response.encode()));
            }
            Err(e) => {
                let response = RpcResponsePayload {
                    id: payload.id,
                    success: false,
                    data: e.to_string().into_bytes(),
                };
                self.peers.send(session_id, Message::new(Op::RpcResponse, response.encode()));
            }
        }

        Ok(())
    }

    #[cfg(not(feature = "lua"))]
    fn handle_rpc(&self, _msg: &Message) -> Result<()> {
        Ok(())
    }

    /// Flush outgoing messages
    fn flush_outgoing(&mut self) -> Result<()> {
        let pending = self.peers.drain_all();
        for (_addr, data) in pending {
            // TODO: Send to specific address
            let _ = self.transport.send(&data);
        }
        Ok(())
    }

    /// Cleanup stale sessions and empty rooms
    fn cleanup(&self) -> Result<()> {
        let stale = self.sessions.cleanup_stale(self.config.session_timeout_secs);
        for id in stale {
            self.peers.remove(id);
        }

        self.rooms.cleanup_empty();

        Ok(())
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        self.sessions.count()
    }

    /// Get room count
    pub fn room_count(&self) -> usize {
        self.rooms.count()
    }
}

/// Server builder for configuration
pub struct ServerBuilder {
    config: ServerConfig,
}

impl ServerBuilder {
    pub fn new() -> Self {
        Self {
            config: ServerConfig::default(),
        }
    }

    pub fn bind(mut self, addr: impl Into<String>) -> Self {
        self.config.bind_addr = addr.into();
        self
    }

    pub fn tick_rate(mut self, rate: u32) -> Self {
        self.config.tick_rate = rate;
        self
    }

    pub fn session_timeout(mut self, secs: u64) -> Self {
        self.config.session_timeout_secs = secs;
        self
    }

    pub fn max_sessions(mut self, max: usize) -> Self {
        self.config.max_sessions = max;
        self
    }

    #[cfg(feature = "lua")]
    pub fn lua_scripts(mut self, path: impl Into<String>) -> Self {
        self.config.lua.script_path = path.into();
        self
    }

    pub fn build(self) -> Result<Server> {
        Server::new(self.config)
    }
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
