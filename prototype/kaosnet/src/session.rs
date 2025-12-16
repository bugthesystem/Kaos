//! Session management.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use dashmap::DashMap;

static SESSION_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generate unique session ID
#[inline]
pub fn generate_session_id() -> u64 {
    SESSION_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Session state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Connecting,
    Connected,
    Authenticated,
    Disconnecting,
}

/// Client session
#[derive(Debug)]
pub struct Session {
    pub id: u64,
    pub addr: SocketAddr,
    pub state: SessionState,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub room_id: Option<String>,
    pub created_at: Instant,
    pub last_heartbeat: Instant,
    pub vars: DashMap<String, String>,
}

impl Session {
    pub fn new(addr: SocketAddr) -> Self {
        let now = Instant::now();
        Self {
            id: generate_session_id(),
            addr,
            state: SessionState::Connecting,
            user_id: None,
            username: None,
            room_id: None,
            created_at: now,
            last_heartbeat: now,
            vars: DashMap::new(),
        }
    }

    pub fn touch(&mut self) {
        self.last_heartbeat = Instant::now();
    }

    pub fn is_stale(&self, timeout_secs: u64) -> bool {
        self.last_heartbeat.elapsed().as_secs() > timeout_secs
    }

    pub fn set_authenticated(&mut self, user_id: String, username: Option<String>) {
        self.state = SessionState::Authenticated;
        self.user_id = Some(user_id);
        self.username = username;
    }

    pub fn join_room(&mut self, room_id: String) {
        self.room_id = Some(room_id);
    }

    pub fn leave_room(&mut self) {
        self.room_id = None;
    }
}

/// Presence info for room broadcasts
#[derive(Debug, Clone)]
pub struct Presence {
    pub session_id: u64,
    pub user_id: String,
    pub username: String,
}

impl From<&Session> for Presence {
    fn from(session: &Session) -> Self {
        Self {
            session_id: session.id,
            user_id: session.user_id.clone().unwrap_or_default(),
            username: session.username.clone().unwrap_or_default(),
        }
    }
}

/// Session registry
pub struct SessionRegistry {
    sessions: DashMap<u64, Session>,
    by_addr: DashMap<SocketAddr, u64>,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            by_addr: DashMap::new(),
        }
    }

    pub fn create(&self, addr: SocketAddr) -> u64 {
        let session = Session::new(addr);
        let id = session.id;
        self.by_addr.insert(addr, id);
        self.sessions.insert(id, session);
        id
    }

    pub fn get(&self, id: u64) -> Option<dashmap::mapref::one::Ref<'_, u64, Session>> {
        self.sessions.get(&id)
    }

    pub fn get_mut(&self, id: u64) -> Option<dashmap::mapref::one::RefMut<'_, u64, Session>> {
        self.sessions.get_mut(&id)
    }

    pub fn get_by_addr(&self, addr: &SocketAddr) -> Option<u64> {
        self.by_addr.get(addr).map(|r| *r)
    }

    pub fn remove(&self, id: u64) -> Option<Session> {
        if let Some((_, session)) = self.sessions.remove(&id) {
            self.by_addr.remove(&session.addr);
            Some(session)
        } else {
            None
        }
    }

    pub fn touch(&self, id: u64) {
        if let Some(mut session) = self.sessions.get_mut(&id) {
            session.touch();
        }
    }

    pub fn count(&self) -> usize {
        self.sessions.len()
    }

    /// Remove stale sessions, returns removed IDs
    pub fn cleanup_stale(&self, timeout_secs: u64) -> Vec<u64> {
        let stale: Vec<u64> = self
            .sessions
            .iter()
            .filter(|r| r.is_stale(timeout_secs))
            .map(|r| *r.key())
            .collect();

        for id in &stale {
            self.remove(*id);
        }
        stale
    }

    /// Get all session IDs in a room
    pub fn in_room(&self, room_id: &str) -> Vec<u64> {
        self.sessions
            .iter()
            .filter(|r| r.room_id.as_deref() == Some(room_id))
            .map(|r| *r.key())
            .collect()
    }
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let session = Session::new(addr);
        assert_eq!(session.state, SessionState::Connecting);
        assert!(session.user_id.is_none());
    }

    #[test]
    fn test_session_registry() {
        let registry = SessionRegistry::new();
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();

        let id = registry.create(addr);
        assert_eq!(registry.count(), 1);
        assert!(registry.get(id).is_some());
        assert_eq!(registry.get_by_addr(&addr), Some(id));

        registry.remove(id);
        assert_eq!(registry.count(), 0);
    }
}
