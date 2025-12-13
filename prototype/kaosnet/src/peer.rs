//! Network peer abstraction.

use std::net::SocketAddr;
use std::sync::Arc;

use dashmap::DashMap;
use parking_lot::Mutex;

use crate::protocol::Message;

/// Outbound message queue per peer
pub struct PeerQueue {
    addr: SocketAddr,
    queue: Mutex<Vec<Message>>,
}

impl PeerQueue {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            queue: Mutex::new(Vec::with_capacity(64)),
        }
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn push(&self, msg: Message) {
        self.queue.lock().push(msg);
    }

    pub fn drain(&self) -> Vec<Message> {
        std::mem::take(&mut *self.queue.lock())
    }

    pub fn len(&self) -> usize {
        self.queue.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.lock().is_empty()
    }
}

/// Peer manager tracks connected peers
pub struct PeerManager {
    peers: DashMap<u64, Arc<PeerQueue>>,
    by_addr: DashMap<SocketAddr, u64>,
}

impl PeerManager {
    pub fn new() -> Self {
        Self {
            peers: DashMap::new(),
            by_addr: DashMap::new(),
        }
    }

    pub fn add(&self, session_id: u64, addr: SocketAddr) -> Arc<PeerQueue> {
        let queue = Arc::new(PeerQueue::new(addr));
        self.peers.insert(session_id, queue.clone());
        self.by_addr.insert(addr, session_id);
        queue
    }

    pub fn get(&self, session_id: u64) -> Option<Arc<PeerQueue>> {
        self.peers.get(&session_id).map(|r| r.clone())
    }

    pub fn get_by_addr(&self, addr: &SocketAddr) -> Option<Arc<PeerQueue>> {
        let session_id = self.by_addr.get(addr)?;
        self.get(*session_id)
    }

    pub fn session_for_addr(&self, addr: &SocketAddr) -> Option<u64> {
        self.by_addr.get(addr).map(|r| *r)
    }

    pub fn remove(&self, session_id: u64) -> Option<Arc<PeerQueue>> {
        if let Some((_, queue)) = self.peers.remove(&session_id) {
            self.by_addr.remove(&queue.addr());
            Some(queue)
        } else {
            None
        }
    }

    pub fn count(&self) -> usize {
        self.peers.len()
    }

    /// Send message to a session
    pub fn send(&self, session_id: u64, msg: Message) -> bool {
        if let Some(queue) = self.get(session_id) {
            queue.push(msg);
            true
        } else {
            false
        }
    }

    /// Send message to multiple sessions
    pub fn send_to(&self, session_ids: &[u64], msg: Message) {
        for &id in session_ids {
            if let Some(queue) = self.get(id) {
                queue.push(msg.clone());
            }
        }
    }

    /// Broadcast to all peers except one
    pub fn broadcast_except(&self, except: u64, msg: Message) {
        for r in self.peers.iter() {
            if *r.key() != except {
                r.value().push(msg.clone());
            }
        }
    }

    /// Broadcast to all peers
    pub fn broadcast(&self, msg: Message) {
        for r in self.peers.iter() {
            r.value().push(msg.clone());
        }
    }

    /// Drain all queued messages for transmission
    pub fn drain_all(&self) -> Vec<(SocketAddr, Vec<u8>)> {
        let mut out = Vec::new();
        for r in self.peers.iter() {
            let messages = r.value().drain();
            if !messages.is_empty() {
                let addr = r.value().addr();
                for msg in messages {
                    out.push((addr, msg.encode()));
                }
            }
        }
        out
    }
}

impl Default for PeerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_queue() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let queue = PeerQueue::new(addr);

        queue.push(Message::heartbeat());
        queue.push(Message::heartbeat());

        assert_eq!(queue.len(), 2);
        let drained = queue.drain();
        assert_eq!(drained.len(), 2);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_peer_manager() {
        let manager = PeerManager::new();
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();

        let queue = manager.add(1, addr);
        assert_eq!(manager.count(), 1);

        manager.send(1, Message::heartbeat());
        assert_eq!(queue.len(), 1);

        manager.remove(1);
        assert_eq!(manager.count(), 0);
    }
}
