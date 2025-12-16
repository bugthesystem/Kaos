//! Room/match system.

use std::collections::HashSet;
use std::time::Instant;

use dashmap::DashMap;
use parking_lot::RwLock;
use uuid::Uuid;

use crate::session::Presence;

/// Room state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomState {
    Open,
    Closed,
    Running,
}

/// Room configuration
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct RoomConfig {
    pub max_players: usize,
    pub tick_rate: u32,
    pub label: String,
    pub module: String,
}

impl Default for RoomConfig {
    fn default() -> Self {
        Self {
            max_players: 16,
            tick_rate: 10,
            label: String::new(),
            module: String::new(),
        }
    }
}

/// Game room (match)
pub struct Room {
    pub id: String,
    pub config: RoomConfig,
    pub state: RoomState,
    pub players: RwLock<HashSet<u64>>,
    pub game_state: RwLock<Vec<u8>>,
    pub tick: u64,
    pub created_at: Instant,
    pub last_tick: Instant,
}

impl Room {
    pub fn new(config: RoomConfig) -> Self {
        let now = Instant::now();
        Self {
            id: Uuid::new_v4().to_string(),
            config,
            state: RoomState::Open,
            players: RwLock::new(HashSet::new()),
            game_state: RwLock::new(Vec::new()),
            tick: 0,
            created_at: now,
            last_tick: now,
        }
    }

    pub fn with_id(id: String, config: RoomConfig) -> Self {
        let now = Instant::now();
        Self {
            id,
            config,
            state: RoomState::Open,
            players: RwLock::new(HashSet::new()),
            game_state: RwLock::new(Vec::new()),
            tick: 0,
            created_at: now,
            last_tick: now,
        }
    }

    pub fn player_count(&self) -> usize {
        self.players.read().len()
    }

    pub fn is_full(&self) -> bool {
        self.player_count() >= self.config.max_players
    }

    pub fn add_player(&self, session_id: u64) -> bool {
        if self.is_full() {
            return false;
        }
        self.players.write().insert(session_id)
    }

    pub fn remove_player(&self, session_id: u64) -> bool {
        self.players.write().remove(&session_id)
    }

    pub fn has_player(&self, session_id: u64) -> bool {
        self.players.read().contains(&session_id)
    }

    pub fn player_ids(&self) -> Vec<u64> {
        self.players.read().iter().copied().collect()
    }

    pub fn set_state(&self, state: Vec<u8>) {
        *self.game_state.write() = state;
    }

    pub fn get_state(&self) -> Vec<u8> {
        self.game_state.read().clone()
    }

    pub fn is_empty(&self) -> bool {
        self.players.read().is_empty()
    }
}

/// Room registry
pub struct RoomRegistry {
    rooms: DashMap<String, Room>,
}

impl RoomRegistry {
    pub fn new() -> Self {
        Self {
            rooms: DashMap::new(),
        }
    }

    pub fn create(&self, config: RoomConfig) -> String {
        let room = Room::new(config);
        let id = room.id.clone();
        self.rooms.insert(id.clone(), room);
        id
    }

    pub fn create_with_id(&self, id: String, config: RoomConfig) -> bool {
        if self.rooms.contains_key(&id) {
            return false;
        }
        let room = Room::with_id(id.clone(), config);
        self.rooms.insert(id, room);
        true
    }

    pub fn get(&self, id: &str) -> Option<dashmap::mapref::one::Ref<'_, String, Room>> {
        self.rooms.get(id)
    }

    pub fn remove(&self, id: &str) -> Option<Room> {
        self.rooms.remove(id).map(|(_, r)| r)
    }

    pub fn count(&self) -> usize {
        self.rooms.len()
    }

    pub fn list(&self, limit: usize) -> Vec<RoomInfo> {
        self.rooms
            .iter()
            .take(limit)
            .map(|r| RoomInfo {
                id: r.id.clone(),
                label: r.config.label.clone(),
                player_count: r.player_count(),
                max_players: r.config.max_players,
                state: r.state,
            })
            .collect()
    }

    pub fn join(&self, room_id: &str, session_id: u64) -> Result<(), JoinError> {
        let room = self.rooms.get(room_id).ok_or(JoinError::NotFound)?;
        if room.state != RoomState::Open {
            return Err(JoinError::Closed);
        }
        if !room.add_player(session_id) {
            return Err(JoinError::Full);
        }
        Ok(())
    }

    pub fn leave(&self, room_id: &str, session_id: u64) {
        if let Some(room) = self.rooms.get(room_id) {
            room.remove_player(session_id);
        }
    }

    /// Remove empty rooms
    pub fn cleanup_empty(&self) -> Vec<String> {
        let empty: Vec<String> = self
            .rooms
            .iter()
            .filter(|r| r.is_empty())
            .map(|r| r.id.clone())
            .collect();

        for id in &empty {
            self.rooms.remove(id);
        }
        empty
    }
}

impl Default for RoomRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Room join error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinError {
    NotFound,
    Full,
    Closed,
}

/// Room listing info
#[derive(Debug, Clone)]
pub struct RoomInfo {
    pub id: String,
    pub label: String,
    pub player_count: usize,
    pub max_players: usize,
    pub state: RoomState,
}

/// Match message (from players during tick)
#[derive(Debug, Clone)]
pub struct MatchMessage {
    pub sender: u64,
    pub op: u8,
    pub data: Vec<u8>,
}

/// Match broadcast target
#[derive(Debug, Clone)]
pub enum BroadcastTarget {
    All,
    Except(u64),
    Only(Vec<u64>),
}

/// Match handler trait for Lua integration
pub trait MatchHandler: Send + Sync {
    fn init(&self, params: &[u8]) -> Vec<u8>;
    fn join(&self, state: &[u8], presences: &[Presence]) -> Vec<u8>;
    fn leave(&self, state: &[u8], presences: &[Presence]) -> Vec<u8>;
    fn tick(&self, state: &[u8], tick: u64, messages: &[MatchMessage]) -> (Vec<u8>, Vec<Broadcast>);
    fn terminate(&self, state: &[u8]);
}

/// Broadcast message from tick
#[derive(Debug, Clone)]
pub struct Broadcast {
    pub target: BroadcastTarget,
    pub data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_creation() {
        let room = Room::new(RoomConfig::default());
        assert_eq!(room.state, RoomState::Open);
        assert_eq!(room.player_count(), 0);
    }

    #[test]
    fn test_room_players() {
        let room = Room::new(RoomConfig {
            max_players: 2,
            ..Default::default()
        });

        assert!(room.add_player(1));
        assert!(room.add_player(2));
        assert!(!room.add_player(3)); // Full

        assert_eq!(room.player_count(), 2);
        assert!(room.is_full());

        room.remove_player(1);
        assert!(!room.is_full());
    }

    #[test]
    fn test_room_registry() {
        let registry = RoomRegistry::new();
        let id = registry.create(RoomConfig::default());

        assert_eq!(registry.count(), 1);
        assert!(registry.get(&id).is_some());

        registry.join(&id, 1).unwrap();
        assert_eq!(registry.get(&id).unwrap().player_count(), 1);

        registry.leave(&id, 1);
        assert_eq!(registry.get(&id).unwrap().player_count(), 0);
    }
}
