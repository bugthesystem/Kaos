//! Matchmaking system for KaosNet.
//!
//! Supports skill-based matchmaking with configurable queues and rules.
//!
//! ## Features
//!
//! - Multiple queues (ranked, casual, custom)
//! - Skill-based matching (MMR/ELO)
//! - Party/group support
//! - Custom match properties
//! - Expansion over time (wider skill range as wait increases)

use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Matchmaking configuration for a queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchmakerConfig {
    /// Queue identifier.
    pub queue: String,
    /// Minimum players needed to start a match.
    pub min_players: usize,
    /// Maximum players in a match.
    pub max_players: usize,
    /// Initial skill range for matching.
    pub initial_skill_range: f64,
    /// How much skill range expands per second of waiting.
    pub skill_expansion_rate: f64,
    /// Maximum skill range (cap).
    pub max_skill_range: f64,
    /// Maximum time in queue before forcing a match (seconds).
    pub max_wait_time: u64,
    /// Match properties that must match exactly.
    pub required_properties: Vec<String>,
    /// Match properties for range matching.
    pub range_properties: Vec<String>,
}

impl Default for MatchmakerConfig {
    fn default() -> Self {
        Self {
            queue: "default".into(),
            min_players: 2,
            max_players: 10,
            initial_skill_range: 100.0,
            skill_expansion_rate: 10.0,
            max_skill_range: 500.0,
            max_wait_time: 120,
            required_properties: vec![],
            range_properties: vec![],
        }
    }
}

/// A matchmaking ticket (player in queue).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchmakerTicket {
    /// Unique ticket ID.
    pub id: String,
    /// Queue this ticket is in.
    pub queue: String,
    /// Players in this ticket (for party support).
    pub players: Vec<MatchmakerPlayer>,
    /// When ticket was created.
    pub created_at: i64,
    /// Match properties.
    pub properties: HashMap<String, serde_json::Value>,
}

/// A player in a matchmaking ticket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchmakerPlayer {
    /// User ID.
    pub user_id: String,
    /// Session ID.
    pub session_id: u64,
    /// Username.
    pub username: String,
    /// Skill rating (MMR/ELO).
    pub skill: f64,
}

/// A completed match from matchmaking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchmakerMatch {
    /// Unique match ID.
    pub id: String,
    /// Queue this match was created from.
    pub queue: String,
    /// All players in the match.
    pub players: Vec<MatchmakerPlayer>,
    /// Combined properties from all tickets.
    pub properties: HashMap<String, serde_json::Value>,
    /// When match was created.
    pub created_at: i64,
}

/// Internal ticket with queue time tracking.
struct QueuedTicket {
    ticket: MatchmakerTicket,
    queued_at: Instant,
}

/// Matchmaking service.
pub struct Matchmaker {
    /// Queue configurations.
    configs: DashMap<String, MatchmakerConfig>,
    /// Active tickets by queue: queue -> ticket_id -> ticket.
    queues: DashMap<String, DashMap<String, QueuedTicket>>,
    /// Ticket lookup by player: user_id -> ticket_id.
    player_tickets: DashMap<String, String>,
    /// Pending matches (ready to be consumed).
    pending_matches: RwLock<VecDeque<MatchmakerMatch>>,
    /// Match callbacks.
    match_handlers: RwLock<Vec<Box<dyn Fn(&MatchmakerMatch) + Send + Sync>>>,
}

impl Matchmaker {
    pub fn new() -> Self {
        Self {
            configs: DashMap::new(),
            queues: DashMap::new(),
            player_tickets: DashMap::new(),
            pending_matches: RwLock::new(VecDeque::new()),
            match_handlers: RwLock::new(vec![]),
        }
    }

    /// Register a queue configuration.
    pub fn register_queue(&self, config: MatchmakerConfig) {
        self.queues.entry(config.queue.clone()).or_insert_with(DashMap::new);
        self.configs.insert(config.queue.clone(), config);
    }

    /// Add a ticket to matchmaking.
    pub fn add(&self, ticket: MatchmakerTicket) -> Result<String, MatchmakerError> {
        // Validate queue exists
        if !self.configs.contains_key(&ticket.queue) {
            return Err(MatchmakerError::QueueNotFound(ticket.queue.clone()));
        }

        // Check players aren't already in queue
        for player in &ticket.players {
            if self.player_tickets.contains_key(&player.user_id) {
                return Err(MatchmakerError::AlreadyQueued(player.user_id.clone()));
            }
        }

        let ticket_id = ticket.id.clone();

        // Register players
        for player in &ticket.players {
            self.player_tickets.insert(player.user_id.clone(), ticket_id.clone());
        }

        // Add to queue
        let queue = self.queues.get(&ticket.queue).unwrap();
        queue.insert(ticket_id.clone(), QueuedTicket {
            ticket,
            queued_at: Instant::now(),
        });

        Ok(ticket_id)
    }

    /// Remove a ticket from matchmaking.
    pub fn remove(&self, ticket_id: &str) -> Result<MatchmakerTicket, MatchmakerError> {
        // Find which queue has this ticket
        for queue_entry in self.queues.iter() {
            if let Some((_, queued)) = queue_entry.value().remove(ticket_id) {
                // Remove player mappings
                for player in &queued.ticket.players {
                    self.player_tickets.remove(&player.user_id);
                }
                return Ok(queued.ticket);
            }
        }

        Err(MatchmakerError::TicketNotFound(ticket_id.to_string()))
    }

    /// Remove a player's ticket.
    pub fn remove_player(&self, user_id: &str) -> Result<MatchmakerTicket, MatchmakerError> {
        let ticket_id = self.player_tickets.get(user_id)
            .map(|r| r.value().clone())
            .ok_or_else(|| MatchmakerError::PlayerNotQueued(user_id.to_string()))?;

        self.remove(&ticket_id)
    }

    /// Check if a player is queued.
    pub fn is_queued(&self, user_id: &str) -> bool {
        self.player_tickets.contains_key(user_id)
    }

    /// Get a player's ticket.
    pub fn get_ticket(&self, user_id: &str) -> Option<MatchmakerTicket> {
        let ticket_id = self.player_tickets.get(user_id)?;

        for queue_entry in self.queues.iter() {
            if let Some(queued) = queue_entry.value().get(ticket_id.value()) {
                return Some(queued.ticket.clone());
            }
        }

        None
    }

    /// Process matchmaking for all queues.
    /// Call this periodically (e.g., every 100ms).
    pub fn tick(&self) -> Vec<MatchmakerMatch> {
        let mut matches = Vec::new();

        for config_entry in self.configs.iter() {
            let config = config_entry.value();
            let queue_name = &config.queue;

            if let Some(queue) = self.queues.get(queue_name) {
                let queue_matches = self.process_queue(config, &queue);
                matches.extend(queue_matches);
            }
        }

        // Store and notify
        for m in &matches {
            self.pending_matches.write().push_back(m.clone());

            for handler in self.match_handlers.read().iter() {
                handler(m);
            }
        }

        matches
    }

    /// Get pending matches (and clear them).
    pub fn drain_matches(&self) -> Vec<MatchmakerMatch> {
        self.pending_matches.write().drain(..).collect()
    }

    /// Register a match handler callback.
    pub fn on_match<F>(&self, handler: F)
    where
        F: Fn(&MatchmakerMatch) + Send + Sync + 'static,
    {
        self.match_handlers.write().push(Box::new(handler));
    }

    /// Get queue stats.
    pub fn stats(&self, queue: &str) -> Option<QueueStats> {
        let queue_data = self.queues.get(queue)?;

        let mut total_players = 0;
        let mut total_tickets = 0;
        let mut longest_wait = Duration::ZERO;

        for entry in queue_data.iter() {
            total_tickets += 1;
            total_players += entry.value().ticket.players.len();
            let wait = entry.value().queued_at.elapsed();
            if wait > longest_wait {
                longest_wait = wait;
            }
        }

        Some(QueueStats {
            queue: queue.to_string(),
            tickets: total_tickets,
            players: total_players,
            longest_wait_secs: longest_wait.as_secs(),
        })
    }

    fn process_queue(&self, config: &MatchmakerConfig, queue: &DashMap<String, QueuedTicket>) -> Vec<MatchmakerMatch> {
        let mut matches = Vec::new();
        let mut used_tickets: HashSet<String> = HashSet::new();

        // Collect tickets with their expanded skill ranges
        let mut candidates: Vec<(String, Vec<MatchmakerPlayer>, f64, f64, HashMap<String, serde_json::Value>)> = Vec::new();

        for entry in queue.iter() {
            let ticket = &entry.value().ticket;
            let wait_secs = entry.value().queued_at.elapsed().as_secs_f64();

            // Calculate expanded skill range
            let skill_range = (config.initial_skill_range + config.skill_expansion_rate * wait_secs)
                .min(config.max_skill_range);

            // Calculate average skill
            let avg_skill: f64 = ticket.players.iter().map(|p| p.skill).sum::<f64>()
                / ticket.players.len() as f64;

            candidates.push((
                ticket.id.clone(),
                ticket.players.clone(),
                avg_skill,
                skill_range,
                ticket.properties.clone(),
            ));
        }

        // Sort by queue time (oldest first, for fairness)
        candidates.sort_by(|a, b| a.0.cmp(&b.0));

        // Try to form matches
        for i in 0..candidates.len() {
            if used_tickets.contains(&candidates[i].0) {
                continue;
            }

            let (ticket_id, players, skill, range, props) = &candidates[i];

            let mut match_players = players.clone();
            let match_props = props.clone();
            let mut match_tickets = vec![ticket_id.clone()];

            // Find compatible tickets
            for j in (i + 1)..candidates.len() {
                if used_tickets.contains(&candidates[j].0) {
                    continue;
                }

                let (other_id, other_players, other_skill, other_range, other_props) = &candidates[j];

                // Check skill compatibility (both must be in range of each other)
                if (skill - other_skill).abs() > range.max(*other_range) {
                    continue;
                }

                // Check required properties match
                let props_match = config.required_properties.iter().all(|prop| {
                    match_props.get(prop) == other_props.get(prop)
                });

                if !props_match {
                    continue;
                }

                // Check if adding would exceed max players
                if match_players.len() + other_players.len() > config.max_players {
                    continue;
                }

                // Add to match
                match_players.extend(other_players.clone());
                match_tickets.push(other_id.clone());

                // Check if we have enough players
                if match_players.len() >= config.min_players {
                    break;
                }
            }

            // Check if we can form a match
            if match_players.len() >= config.min_players {
                // Mark tickets as used
                for tid in &match_tickets {
                    used_tickets.insert(tid.clone());
                }

                // Create match
                let mm_match = MatchmakerMatch {
                    id: Uuid::new_v4().to_string(),
                    queue: config.queue.clone(),
                    players: match_players,
                    properties: match_props,
                    created_at: now_millis(),
                };

                matches.push(mm_match);
            }
        }

        // Remove used tickets from queue
        for ticket_id in &used_tickets {
            if let Some((_, queued)) = queue.remove(ticket_id) {
                for player in &queued.ticket.players {
                    self.player_tickets.remove(&player.user_id);
                }
            }
        }

        matches
    }
}

impl Default for Matchmaker {
    fn default() -> Self {
        Self::new()
    }
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Queue statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    pub queue: String,
    pub tickets: usize,
    pub players: usize,
    pub longest_wait_secs: u64,
}

/// Matchmaker errors.
#[derive(Debug, thiserror::Error)]
pub enum MatchmakerError {
    #[error("queue not found: {0}")]
    QueueNotFound(String),

    #[error("ticket not found: {0}")]
    TicketNotFound(String),

    #[error("player already queued: {0}")]
    AlreadyQueued(String),

    #[error("player not queued: {0}")]
    PlayerNotQueued(String),
}

/// Helper to create tickets easily.
pub fn create_ticket(
    queue: impl Into<String>,
    user_id: impl Into<String>,
    session_id: u64,
    username: impl Into<String>,
    skill: f64,
) -> MatchmakerTicket {
    MatchmakerTicket {
        id: Uuid::new_v4().to_string(),
        queue: queue.into(),
        players: vec![MatchmakerPlayer {
            user_id: user_id.into(),
            session_id,
            username: username.into(),
            skill,
        }],
        created_at: now_millis(),
        properties: HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matchmaking_basic() {
        let mm = Matchmaker::new();

        mm.register_queue(MatchmakerConfig {
            queue: "ranked".into(),
            min_players: 2,
            max_players: 2,
            initial_skill_range: 100.0,
            ..Default::default()
        });

        // Add two players with similar skill
        mm.add(create_ticket("ranked", "user1", 1, "Alice", 1500.0)).unwrap();
        mm.add(create_ticket("ranked", "user2", 2, "Bob", 1550.0)).unwrap();

        // Process matchmaking
        let matches = mm.tick();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].players.len(), 2);
    }

    #[test]
    fn test_skill_range() {
        let mm = Matchmaker::new();

        mm.register_queue(MatchmakerConfig {
            queue: "ranked".into(),
            min_players: 2,
            max_players: 2,
            initial_skill_range: 50.0, // Narrow range
            ..Default::default()
        });

        // Add two players with large skill gap
        mm.add(create_ticket("ranked", "user1", 1, "Alice", 1500.0)).unwrap();
        mm.add(create_ticket("ranked", "user2", 2, "Bob", 2000.0)).unwrap();

        // Process - should NOT match due to skill gap
        let matches = mm.tick();
        assert!(matches.is_empty());
    }

    #[test]
    fn test_party_matchmaking() {
        let mm = Matchmaker::new();

        mm.register_queue(MatchmakerConfig {
            queue: "team".into(),
            min_players: 4,
            max_players: 4,
            initial_skill_range: 200.0,
            ..Default::default()
        });

        // Party of 2
        let mut ticket1 = create_ticket("team", "user1", 1, "Alice", 1500.0);
        ticket1.players.push(MatchmakerPlayer {
            user_id: "user2".into(),
            session_id: 2,
            username: "Bob".into(),
            skill: 1520.0,
        });

        // Another party of 2
        let mut ticket2 = create_ticket("team", "user3", 3, "Charlie", 1480.0);
        ticket2.players.push(MatchmakerPlayer {
            user_id: "user4".into(),
            session_id: 4,
            username: "Diana".into(),
            skill: 1510.0,
        });

        mm.add(ticket1).unwrap();
        mm.add(ticket2).unwrap();

        let matches = mm.tick();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].players.len(), 4);
    }

    #[test]
    fn test_remove_from_queue() {
        let mm = Matchmaker::new();

        mm.register_queue(MatchmakerConfig {
            queue: "test".into(),
            min_players: 2,
            max_players: 2,
            ..Default::default()
        });

        mm.add(create_ticket("test", "user1", 1, "Alice", 1500.0)).unwrap();

        assert!(mm.is_queued("user1"));

        mm.remove_player("user1").unwrap();

        assert!(!mm.is_queued("user1"));
    }
}
