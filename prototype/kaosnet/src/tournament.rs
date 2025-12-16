//! Tournament system for KaosNet.
//!
//! Provides scheduled competitions with time-based windows.
//!
//! ## Features
//!
//! - Scheduled start/end times
//! - Join windows
//! - Score tracking and rankings
//! - Recurring tournaments (daily, weekly, monthly)
//! - Prize metadata
//! - Max participants limit

use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Tournament state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TournamentState {
    /// Tournament is scheduled but not yet open.
    Upcoming,
    /// Tournament is open for joining.
    Open,
    /// Tournament is active (in progress).
    Active,
    /// Tournament has ended.
    Ended,
    /// Tournament was cancelled.
    Cancelled,
}

/// Tournament recurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TournamentReset {
    /// One-time tournament.
    Never,
    /// Resets daily.
    Daily,
    /// Resets weekly.
    Weekly,
    /// Resets monthly.
    Monthly,
}

impl Default for TournamentReset {
    fn default() -> Self {
        Self::Never
    }
}

/// Tournament sort order for ranking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TournamentSortOrder {
    /// Higher score is better (default).
    Descending,
    /// Lower score is better (e.g., speedruns).
    Ascending,
}

impl Default for TournamentSortOrder {
    fn default() -> Self {
        Self::Descending
    }
}

/// Tournament configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentConfig {
    /// Unique tournament ID.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Category/type for filtering.
    pub category: String,
    /// Sort order for ranking.
    pub sort_order: TournamentSortOrder,
    /// How to handle score submissions.
    pub operator: ScoreOperator,
    /// Maximum participants (0 = unlimited).
    pub max_participants: usize,
    /// Maximum score submissions per user.
    pub max_submissions: u32,
    /// Entry fee (game currency, 0 = free).
    pub entry_fee: i64,
    /// Metadata (prizes, rewards, etc.).
    pub metadata: Option<serde_json::Value>,
    /// Reset schedule.
    pub reset: TournamentReset,
    /// Duration in seconds.
    pub duration_secs: u64,
    /// Join window before start (seconds).
    pub join_window_secs: u64,
}

/// Score operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScoreOperator {
    /// Keep the best score.
    Best,
    /// Keep the latest score.
    Latest,
    /// Sum all scores.
    Sum,
    /// Increment score.
    Increment,
}

impl Default for ScoreOperator {
    fn default() -> Self {
        Self::Best
    }
}

/// A tournament instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tournament {
    /// Tournament ID.
    pub id: String,
    /// Configuration.
    pub config: TournamentConfig,
    /// Current state.
    pub state: TournamentState,
    /// Join window start time.
    pub join_start: i64,
    /// Tournament start time.
    pub start_time: i64,
    /// Tournament end time.
    pub end_time: i64,
    /// Current participant count.
    pub participant_count: usize,
    /// Created timestamp.
    pub created_at: i64,
}

/// A tournament record (participant entry).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentRecord {
    /// Tournament ID.
    pub tournament_id: String,
    /// User ID.
    pub user_id: String,
    /// Username.
    pub username: String,
    /// Score.
    pub score: i64,
    /// Number of submissions.
    pub num_submissions: u32,
    /// Rank (1-based).
    pub rank: u64,
    /// Custom metadata.
    pub metadata: Option<serde_json::Value>,
    /// When user joined.
    pub joined_at: i64,
    /// Last score update.
    pub updated_at: i64,
}

/// Sort key for ranking.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SortKey {
    score: i64,
    updated_at: i64,
    user_id: String,
}

impl SortKey {
    fn new(score: i64, updated_at: i64, user_id: &str, ascending: bool) -> Self {
        Self {
            score: if ascending { score } else { -score },
            updated_at,
            user_id: user_id.to_string(),
        }
    }
}

/// Internal tournament data.
struct TournamentData {
    tournament: Tournament,
    records: DashMap<String, TournamentRecord>,
    sorted: RwLock<BTreeMap<SortKey, String>>,
}

/// Tournaments service.
pub struct Tournaments {
    tournaments: DashMap<String, Arc<RwLock<TournamentData>>>,
    by_category: DashMap<String, Vec<String>>,
}

impl Tournaments {
    pub fn new() -> Self {
        Self {
            tournaments: DashMap::new(),
            by_category: DashMap::new(),
        }
    }

    /// Create a new tournament.
    pub fn create(&self, config: TournamentConfig, start_time: i64) -> Result<Tournament, TournamentError> {
        if self.tournaments.contains_key(&config.id) {
            return Err(TournamentError::AlreadyExists(config.id.clone()));
        }

        let now = now_millis();
        let join_start = start_time - (config.join_window_secs as i64 * 1000);
        let end_time = start_time + (config.duration_secs as i64 * 1000);

        let state = if now < join_start {
            TournamentState::Upcoming
        } else if now < start_time {
            TournamentState::Open
        } else if now < end_time {
            TournamentState::Active
        } else {
            TournamentState::Ended
        };

        let tournament = Tournament {
            id: config.id.clone(),
            config: config.clone(),
            state,
            join_start,
            start_time,
            end_time,
            participant_count: 0,
            created_at: now,
        };

        let data = TournamentData {
            tournament: tournament.clone(),
            records: DashMap::new(),
            sorted: RwLock::new(BTreeMap::new()),
        };

        self.tournaments.insert(config.id.clone(), Arc::new(RwLock::new(data)));

        // Index by category
        self.by_category
            .entry(config.category.clone())
            .or_insert_with(Vec::new)
            .push(config.id);

        Ok(tournament)
    }

    /// Create a quick tournament (starts immediately).
    pub fn create_quick(&self, name: &str, duration_secs: u64) -> Result<Tournament, TournamentError> {
        let config = TournamentConfig {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: String::new(),
            category: "quick".to_string(),
            sort_order: TournamentSortOrder::Descending,
            operator: ScoreOperator::Best,
            max_participants: 0,
            max_submissions: 0,
            entry_fee: 0,
            metadata: None,
            reset: TournamentReset::Never,
            duration_secs,
            join_window_secs: 0,
        };

        self.create(config, now_millis())
    }

    /// Get a tournament.
    pub fn get(&self, tournament_id: &str) -> Option<Tournament> {
        self.tournaments.get(tournament_id)
            .map(|data| data.read().tournament.clone())
    }

    /// List tournaments by category.
    pub fn list_by_category(&self, category: &str, limit: usize) -> Vec<Tournament> {
        self.by_category.get(category)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.get(id))
                    .take(limit)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// List active tournaments.
    pub fn list_active(&self, limit: usize) -> Vec<Tournament> {
        self.tournaments.iter()
            .filter_map(|entry| {
                let t = entry.value().read().tournament.clone();
                if t.state == TournamentState::Active || t.state == TournamentState::Open {
                    Some(t)
                } else {
                    None
                }
            })
            .take(limit)
            .collect()
    }

    /// Join a tournament.
    pub fn join(&self, tournament_id: &str, user_id: &str, username: &str) -> Result<TournamentRecord, TournamentError> {
        let data = self.tournaments.get(tournament_id)
            .ok_or_else(|| TournamentError::NotFound(tournament_id.to_string()))?;

        let mut data = data.write();
        let now = now_millis();

        // Update state
        self.update_state(&mut data.tournament, now);

        // Check if can join
        match data.tournament.state {
            TournamentState::Upcoming => return Err(TournamentError::NotOpen),
            TournamentState::Ended | TournamentState::Cancelled => return Err(TournamentError::Ended),
            TournamentState::Open | TournamentState::Active => {}
        }

        // Check max participants
        if data.tournament.config.max_participants > 0
            && data.records.len() >= data.tournament.config.max_participants {
            return Err(TournamentError::Full);
        }

        // Check if already joined
        if data.records.contains_key(user_id) {
            return Err(TournamentError::AlreadyJoined);
        }

        let record = TournamentRecord {
            tournament_id: tournament_id.to_string(),
            user_id: user_id.to_string(),
            username: username.to_string(),
            score: 0,
            num_submissions: 0,
            rank: 0,
            metadata: None,
            joined_at: now,
            updated_at: now,
        };

        data.records.insert(user_id.to_string(), record.clone());
        data.tournament.participant_count = data.records.len();

        // Add to sorted index
        let key = SortKey::new(0, now, user_id, data.tournament.config.sort_order == TournamentSortOrder::Ascending);
        data.sorted.write().insert(key, user_id.to_string());

        Ok(record)
    }

    /// Submit a score.
    pub fn submit(&self, tournament_id: &str, user_id: &str, score: i64, metadata: Option<serde_json::Value>) -> Result<TournamentRecord, TournamentError> {
        let data = self.tournaments.get(tournament_id)
            .ok_or_else(|| TournamentError::NotFound(tournament_id.to_string()))?;

        let mut data = data.write();
        let now = now_millis();

        // Update state
        self.update_state(&mut data.tournament, now);

        // Check if tournament is active
        if data.tournament.state != TournamentState::Active {
            return Err(TournamentError::NotActive);
        }

        // Get existing record
        let mut record = data.records.get_mut(user_id)
            .ok_or(TournamentError::NotJoined)?;

        // Check max submissions
        if data.tournament.config.max_submissions > 0
            && record.num_submissions >= data.tournament.config.max_submissions {
            return Err(TournamentError::MaxSubmissions);
        }

        // Remove old sort key
        let ascending = data.tournament.config.sort_order == TournamentSortOrder::Ascending;
        let old_key = SortKey::new(record.score, record.updated_at, user_id, ascending);
        data.sorted.write().remove(&old_key);

        // Update score based on operator
        let new_score = match data.tournament.config.operator {
            ScoreOperator::Best => {
                if ascending {
                    if record.num_submissions == 0 || score < record.score { score } else { record.score }
                } else {
                    if score > record.score { score } else { record.score }
                }
            }
            ScoreOperator::Latest => score,
            ScoreOperator::Sum => record.score + score,
            ScoreOperator::Increment => record.score + score,
        };

        record.score = new_score;
        record.num_submissions += 1;
        record.updated_at = now;
        if metadata.is_some() {
            record.metadata = metadata;
        }

        // Add new sort key
        let new_key = SortKey::new(new_score, now, user_id, ascending);
        data.sorted.write().insert(new_key, user_id.to_string());

        Ok(record.clone())
    }

    /// Get user's record.
    pub fn get_record(&self, tournament_id: &str, user_id: &str) -> Result<TournamentRecord, TournamentError> {
        let data = self.tournaments.get(tournament_id)
            .ok_or_else(|| TournamentError::NotFound(tournament_id.to_string()))?;

        let data = data.read();

        let record = data.records.get(user_id)
            .ok_or(TournamentError::NotJoined)?;

        let mut record = record.clone();
        record.rank = self.calculate_rank(&data, user_id);
        Ok(record)
    }

    /// Get top records.
    pub fn get_top(&self, tournament_id: &str, limit: usize) -> Result<Vec<TournamentRecord>, TournamentError> {
        let data = self.tournaments.get(tournament_id)
            .ok_or_else(|| TournamentError::NotFound(tournament_id.to_string()))?;

        let data = data.read();
        let sorted = data.sorted.read();

        let mut results = Vec::new();
        for (rank, (_, user_id)) in sorted.iter().enumerate() {
            if rank >= limit {
                break;
            }
            if let Some(record) = data.records.get(user_id) {
                let mut r = record.clone();
                r.rank = (rank + 1) as u64;
                results.push(r);
            }
        }

        Ok(results)
    }

    /// Get records around a user.
    pub fn get_around(&self, tournament_id: &str, user_id: &str, count: usize) -> Result<Vec<TournamentRecord>, TournamentError> {
        let data = self.tournaments.get(tournament_id)
            .ok_or_else(|| TournamentError::NotFound(tournament_id.to_string()))?;

        let data = data.read();

        // Find user's position
        let record = data.records.get(user_id)
            .ok_or(TournamentError::NotJoined)?;

        let ascending = data.tournament.config.sort_order == TournamentSortOrder::Ascending;
        let user_key = SortKey::new(record.score, record.updated_at, user_id, ascending);

        let sorted = data.sorted.read();
        let all_keys: Vec<_> = sorted.iter().collect();

        let user_pos = all_keys.iter().position(|(k, _)| *k == &user_key).unwrap_or(0);

        let start = user_pos.saturating_sub(count / 2);
        let end = (start + count).min(all_keys.len());

        let mut results = Vec::new();
        for (rank, (_, uid)) in all_keys[start..end].iter().enumerate() {
            if let Some(rec) = data.records.get(*uid) {
                let mut r = rec.clone();
                r.rank = (start + rank + 1) as u64;
                results.push(r);
            }
        }

        Ok(results)
    }

    /// Update tournament states (call periodically).
    pub fn update_states(&self) {
        let now = now_millis();
        for entry in self.tournaments.iter() {
            let mut data = entry.value().write();
            self.update_state(&mut data.tournament, now);
        }
    }

    fn update_state(&self, tournament: &mut Tournament, now: i64) {
        let new_state = if tournament.state == TournamentState::Cancelled {
            TournamentState::Cancelled
        } else if now < tournament.join_start {
            TournamentState::Upcoming
        } else if now < tournament.start_time {
            TournamentState::Open
        } else if now < tournament.end_time {
            TournamentState::Active
        } else {
            TournamentState::Ended
        };
        tournament.state = new_state;
    }

    fn calculate_rank(&self, data: &TournamentData, user_id: &str) -> u64 {
        let record = match data.records.get(user_id) {
            Some(r) => r,
            None => return 0,
        };

        let ascending = data.tournament.config.sort_order == TournamentSortOrder::Ascending;
        let user_key = SortKey::new(record.score, record.updated_at, user_id, ascending);

        let sorted = data.sorted.read();
        sorted.iter().position(|(k, _)| k == &user_key)
            .map(|p| (p + 1) as u64)
            .unwrap_or(0)
    }

    /// Get tournament count.
    pub fn count(&self) -> usize {
        self.tournaments.len()
    }

    /// Cancel a tournament.
    pub fn cancel(&self, tournament_id: &str) -> Result<(), TournamentError> {
        let data = self.tournaments.get(tournament_id)
            .ok_or_else(|| TournamentError::NotFound(tournament_id.to_string()))?;

        let mut data = data.write();
        data.tournament.state = TournamentState::Cancelled;
        Ok(())
    }
}

impl Default for Tournaments {
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

/// Tournament errors.
#[derive(Debug, thiserror::Error)]
pub enum TournamentError {
    #[error("tournament not found: {0}")]
    NotFound(String),

    #[error("tournament already exists: {0}")]
    AlreadyExists(String),

    #[error("tournament not open for joining")]
    NotOpen,

    #[error("tournament has ended")]
    Ended,

    #[error("tournament is full")]
    Full,

    #[error("already joined tournament")]
    AlreadyJoined,

    #[error("not joined tournament")]
    NotJoined,

    #[error("tournament not active")]
    NotActive,

    #[error("max submissions reached")]
    MaxSubmissions,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tournament_lifecycle() {
        let tournaments = Tournaments::new();

        // Create a quick tournament
        let t = tournaments.create_quick("Test Tournament", 3600).unwrap();
        assert_eq!(t.state, TournamentState::Active);

        // Join
        let record = tournaments.join(&t.id, "user1", "Alice").unwrap();
        assert_eq!(record.score, 0);

        // Submit score
        let record = tournaments.submit(&t.id, "user1", 100, None).unwrap();
        assert_eq!(record.score, 100);

        // Submit better score
        let record = tournaments.submit(&t.id, "user1", 150, None).unwrap();
        assert_eq!(record.score, 150);

        // Get ranking
        let top = tournaments.get_top(&t.id, 10).unwrap();
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].rank, 1);
    }

    #[test]
    fn test_tournament_ranking() {
        let tournaments = Tournaments::new();
        let t = tournaments.create_quick("Ranking Test", 3600).unwrap();

        // Add multiple players
        tournaments.join(&t.id, "user1", "Alice").unwrap();
        tournaments.join(&t.id, "user2", "Bob").unwrap();
        tournaments.join(&t.id, "user3", "Charlie").unwrap();

        // Submit scores
        tournaments.submit(&t.id, "user1", 100, None).unwrap();
        tournaments.submit(&t.id, "user2", 200, None).unwrap();
        tournaments.submit(&t.id, "user3", 150, None).unwrap();

        // Check rankings (descending - higher is better)
        let top = tournaments.get_top(&t.id, 10).unwrap();
        assert_eq!(top[0].user_id, "user2"); // 200
        assert_eq!(top[1].user_id, "user3"); // 150
        assert_eq!(top[2].user_id, "user1"); // 100
    }

    #[test]
    fn test_scheduled_tournament() {
        let tournaments = Tournaments::new();

        let config = TournamentConfig {
            id: "scheduled".to_string(),
            name: "Scheduled".to_string(),
            description: String::new(),
            category: "test".to_string(),
            sort_order: TournamentSortOrder::Descending,
            operator: ScoreOperator::Best,
            max_participants: 10,
            max_submissions: 3,
            entry_fee: 0,
            metadata: None,
            reset: TournamentReset::Never,
            duration_secs: 3600,
            join_window_secs: 600,
        };

        // Start in the future
        let future = now_millis() + 60_000;
        let t = tournaments.create(config, future).unwrap();
        assert_eq!(t.state, TournamentState::Open); // In join window

        // Can join during open period
        tournaments.join(&t.id, "user1", "Alice").unwrap();

        // Can't submit until active
        let result = tournaments.submit(&t.id, "user1", 100, None);
        assert!(matches!(result, Err(TournamentError::NotActive)));
    }
}
