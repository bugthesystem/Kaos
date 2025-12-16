//! Leaderboard system for KaosNet.
//!
//! Supports multiple leaderboards with different sorting and reset strategies.
//!
//! ## Features
//!
//! - Multiple leaderboards per game
//! - Score submission with metadata
//! - Rank queries (top N, around player)
//! - Time-based resets (daily, weekly, monthly)
//! - Subsetting (by region, platform, etc.)

use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Leaderboard configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardConfig {
    /// Unique identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Sort order.
    pub sort_order: SortOrder,
    /// Score operator (best, latest, sum, etc.).
    pub operator: ScoreOperator,
    /// Reset schedule.
    pub reset_schedule: ResetSchedule,
    /// Maximum entries to keep.
    pub max_entries: usize,
    /// Metadata schema (optional).
    pub metadata_schema: Option<serde_json::Value>,
}

impl Default for LeaderboardConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            sort_order: SortOrder::Descending,
            operator: ScoreOperator::Best,
            reset_schedule: ResetSchedule::Never,
            max_entries: 10000,
            metadata_schema: None,
        }
    }
}

/// Sort order for leaderboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    /// Higher scores are better.
    Descending,
    /// Lower scores are better (e.g., speedruns).
    Ascending,
}

/// How to handle multiple score submissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScoreOperator {
    /// Keep the best score.
    Best,
    /// Always use the latest score.
    Latest,
    /// Sum all scores.
    Sum,
    /// Increment score.
    Increment,
}

/// When to reset the leaderboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResetSchedule {
    /// Never reset.
    Never,
    /// Reset daily at midnight UTC.
    Daily,
    /// Reset weekly on Monday midnight UTC.
    Weekly,
    /// Reset monthly on the 1st midnight UTC.
    Monthly,
}

/// A leaderboard entry/record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardRecord {
    /// User ID.
    pub user_id: String,
    /// Username (cached for display).
    pub username: String,
    /// Score value.
    pub score: i64,
    /// Number of submissions.
    pub num_submissions: u32,
    /// Optional metadata.
    pub metadata: Option<serde_json::Value>,
    /// Rank (1-indexed, set when queried).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<u64>,
    /// When this record was created.
    pub created_at: i64,
    /// When this record was last updated.
    pub updated_at: i64,
}

/// Leaderboard service.
pub struct Leaderboards {
    /// Leaderboard configs.
    configs: DashMap<String, LeaderboardConfig>,
    /// Leaderboard data: leaderboard_id -> records (sorted).
    boards: DashMap<String, Arc<RwLock<LeaderboardData>>>,
}

struct LeaderboardData {
    config: LeaderboardConfig,
    /// Records indexed by user_id.
    by_user: DashMap<String, LeaderboardRecord>,
    /// Sorted scores for ranking (score -> user_ids).
    sorted: RwLock<BTreeMap<SortKey, String>>,
    /// Last reset time (for scheduled resets).
    #[allow(dead_code)]
    last_reset: i64,
}

/// Sort key for ordered storage.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SortKey {
    score: i64,
    tiebreaker: i64, // update time for stable ordering
    user_id: String, // final tiebreaker
}

impl Leaderboards {
    pub fn new() -> Self {
        Self {
            configs: DashMap::new(),
            boards: DashMap::new(),
        }
    }

    /// Create a new leaderboard.
    pub fn create(&self, config: LeaderboardConfig) -> Result<(), LeaderboardError> {
        if config.id.is_empty() {
            return Err(LeaderboardError::InvalidConfig("id cannot be empty".into()));
        }

        if self.configs.contains_key(&config.id) {
            return Err(LeaderboardError::AlreadyExists(config.id.clone()));
        }

        let data = LeaderboardData {
            config: config.clone(),
            by_user: DashMap::new(),
            sorted: RwLock::new(BTreeMap::new()),
            last_reset: now_millis(),
        };

        self.boards.insert(config.id.clone(), Arc::new(RwLock::new(data)));
        self.configs.insert(config.id.clone(), config);
        Ok(())
    }

    /// Submit a score.
    pub fn submit(
        &self,
        leaderboard_id: &str,
        user_id: &str,
        username: &str,
        score: i64,
        metadata: Option<serde_json::Value>,
    ) -> Result<LeaderboardRecord, LeaderboardError> {
        let board = self.boards.get(leaderboard_id)
            .ok_or_else(|| LeaderboardError::NotFound(leaderboard_id.to_string()))?;

        let board = board.read();
        let now = now_millis();

        // Check if user has existing record
        let existing = board.by_user.get(user_id).map(|r| r.clone());

        // Calculate new score based on operator
        let new_score = match board.config.operator {
            ScoreOperator::Best => {
                if let Some(ref existing) = existing {
                    match board.config.sort_order {
                        SortOrder::Descending => score.max(existing.score),
                        SortOrder::Ascending => score.min(existing.score),
                    }
                } else {
                    score
                }
            }
            ScoreOperator::Latest => score,
            ScoreOperator::Sum => existing.as_ref().map(|e| e.score).unwrap_or(0) + score,
            ScoreOperator::Increment => existing.as_ref().map(|e| e.score).unwrap_or(0) + score,
        };

        // Create or update record
        let record = LeaderboardRecord {
            user_id: user_id.to_string(),
            username: username.to_string(),
            score: new_score,
            num_submissions: existing.as_ref().map(|e| e.num_submissions + 1).unwrap_or(1),
            metadata,
            rank: None,
            created_at: existing.as_ref().map(|e| e.created_at).unwrap_or(now),
            updated_at: now,
        };

        // Remove old entry from sorted index
        if let Some(existing) = existing {
            let mut sorted = board.sorted.write();
            let old_key = SortKey {
                score: match board.config.sort_order {
                    SortOrder::Descending => -existing.score,
                    SortOrder::Ascending => existing.score,
                },
                tiebreaker: existing.updated_at,
                user_id: existing.user_id.clone(),
            };
            sorted.remove(&old_key);
        }

        // Add new entry to sorted index
        {
            let mut sorted = board.sorted.write();
            let key = SortKey {
                score: match board.config.sort_order {
                    SortOrder::Descending => -new_score,
                    SortOrder::Ascending => new_score,
                },
                tiebreaker: now,
                user_id: user_id.to_string(),
            };
            sorted.insert(key, user_id.to_string());
        }

        // Store record
        board.by_user.insert(user_id.to_string(), record.clone());

        Ok(record)
    }

    /// Get a user's record.
    pub fn get_record(&self, leaderboard_id: &str, user_id: &str) -> Result<Option<LeaderboardRecord>, LeaderboardError> {
        let board = self.boards.get(leaderboard_id)
            .ok_or_else(|| LeaderboardError::NotFound(leaderboard_id.to_string()))?;

        let board = board.read();
        let record = board.by_user.get(user_id).map(|r| {
            let mut rec = r.clone();
            rec.rank = Some(self.get_rank_internal(&board, user_id));
            rec
        });

        Ok(record)
    }

    /// Get top N records.
    pub fn get_top(&self, leaderboard_id: &str, limit: usize) -> Result<Vec<LeaderboardRecord>, LeaderboardError> {
        let board = self.boards.get(leaderboard_id)
            .ok_or_else(|| LeaderboardError::NotFound(leaderboard_id.to_string()))?;

        let board = board.read();
        let sorted = board.sorted.read();

        let mut records = Vec::with_capacity(limit);
        for (rank, (_, user_id)) in sorted.iter().take(limit).enumerate() {
            if let Some(entry) = board.by_user.get(user_id) {
                let mut record = entry.clone();
                record.rank = Some((rank + 1) as u64);
                records.push(record);
            }
        }

        Ok(records)
    }

    /// Get records around a user (for "show my rank" UI).
    pub fn get_around(
        &self,
        leaderboard_id: &str,
        user_id: &str,
        count: usize,
    ) -> Result<Vec<LeaderboardRecord>, LeaderboardError> {
        let board = self.boards.get(leaderboard_id)
            .ok_or_else(|| LeaderboardError::NotFound(leaderboard_id.to_string()))?;

        let board = board.read();

        // Find user's rank
        let user_rank = self.get_rank_internal(&board, user_id);
        if user_rank == 0 {
            return Ok(vec![]);
        }

        let sorted = board.sorted.read();
        let total = sorted.len();

        // Calculate range around user
        let half = count / 2;
        let start = (user_rank as usize).saturating_sub(half + 1);
        let end = (start + count).min(total);
        let start = if end == total { end.saturating_sub(count) } else { start };

        let mut records = Vec::with_capacity(count);
        for (rank, (_, uid)) in sorted.iter().enumerate().skip(start).take(end - start) {
            if let Some(entry) = board.by_user.get(uid) {
                let mut record = entry.clone();
                record.rank = Some((rank + 1) as u64);
                records.push(record);
            }
        }

        Ok(records)
    }

    /// Get total number of records.
    pub fn count(&self, leaderboard_id: &str) -> Result<usize, LeaderboardError> {
        let board = self.boards.get(leaderboard_id)
            .ok_or_else(|| LeaderboardError::NotFound(leaderboard_id.to_string()))?;

        let count = board.read().by_user.len();
        Ok(count)
    }

    /// Delete a user's record.
    pub fn delete_record(&self, leaderboard_id: &str, user_id: &str) -> Result<bool, LeaderboardError> {
        let board = self.boards.get(leaderboard_id)
            .ok_or_else(|| LeaderboardError::NotFound(leaderboard_id.to_string()))?;

        let board = board.read();

        if let Some((_, record)) = board.by_user.remove(user_id) {
            let mut sorted = board.sorted.write();
            let key = SortKey {
                score: match board.config.sort_order {
                    SortOrder::Descending => -record.score,
                    SortOrder::Ascending => record.score,
                },
                tiebreaker: record.updated_at,
                user_id: user_id.to_string(),
            };
            sorted.remove(&key);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// List all leaderboards.
    pub fn list(&self) -> Vec<LeaderboardConfig> {
        self.configs.iter().map(|r| r.value().clone()).collect()
    }

    fn get_rank_internal(&self, board: &LeaderboardData, user_id: &str) -> u64 {
        let sorted = board.sorted.read();
        for (rank, (_, uid)) in sorted.iter().enumerate() {
            if uid == user_id {
                return (rank + 1) as u64;
            }
        }
        0
    }
}

impl Default for Leaderboards {
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

/// Leaderboard errors.
#[derive(Debug, thiserror::Error)]
pub enum LeaderboardError {
    #[error("leaderboard not found: {0}")]
    NotFound(String),

    #[error("leaderboard already exists: {0}")]
    AlreadyExists(String),

    #[error("invalid config: {0}")]
    InvalidConfig(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leaderboard_submit_and_rank() {
        let lb = Leaderboards::new();

        lb.create(LeaderboardConfig {
            id: "highscores".into(),
            name: "High Scores".into(),
            sort_order: SortOrder::Descending,
            operator: ScoreOperator::Best,
            ..Default::default()
        }).unwrap();

        // Submit scores
        lb.submit("highscores", "user1", "Alice", 100, None).unwrap();
        lb.submit("highscores", "user2", "Bob", 200, None).unwrap();
        lb.submit("highscores", "user3", "Charlie", 150, None).unwrap();

        // Check rankings
        let top = lb.get_top("highscores", 10).unwrap();
        assert_eq!(top.len(), 3);
        assert_eq!(top[0].username, "Bob");
        assert_eq!(top[0].rank, Some(1));
        assert_eq!(top[1].username, "Charlie");
        assert_eq!(top[1].rank, Some(2));
        assert_eq!(top[2].username, "Alice");
        assert_eq!(top[2].rank, Some(3));
    }

    #[test]
    fn test_best_score_operator() {
        let lb = Leaderboards::new();

        lb.create(LeaderboardConfig {
            id: "best".into(),
            name: "Best".into(),
            operator: ScoreOperator::Best,
            ..Default::default()
        }).unwrap();

        lb.submit("best", "user1", "Alice", 100, None).unwrap();
        let r = lb.submit("best", "user1", "Alice", 50, None).unwrap();
        assert_eq!(r.score, 100); // Kept best

        let r = lb.submit("best", "user1", "Alice", 150, None).unwrap();
        assert_eq!(r.score, 150); // New best
    }

    #[test]
    fn test_sum_operator() {
        let lb = Leaderboards::new();

        lb.create(LeaderboardConfig {
            id: "total".into(),
            name: "Total".into(),
            operator: ScoreOperator::Sum,
            ..Default::default()
        }).unwrap();

        lb.submit("total", "user1", "Alice", 100, None).unwrap();
        let r = lb.submit("total", "user1", "Alice", 50, None).unwrap();
        assert_eq!(r.score, 150);

        let r = lb.submit("total", "user1", "Alice", 25, None).unwrap();
        assert_eq!(r.score, 175);
    }

    #[test]
    fn test_ascending_order() {
        let lb = Leaderboards::new();

        lb.create(LeaderboardConfig {
            id: "speedrun".into(),
            name: "Speedrun".into(),
            sort_order: SortOrder::Ascending,
            operator: ScoreOperator::Best,
            ..Default::default()
        }).unwrap();

        lb.submit("speedrun", "user1", "Alice", 120, None).unwrap(); // 2 minutes
        lb.submit("speedrun", "user2", "Bob", 90, None).unwrap();    // 1.5 minutes
        lb.submit("speedrun", "user3", "Charlie", 150, None).unwrap(); // 2.5 minutes

        let top = lb.get_top("speedrun", 10).unwrap();
        assert_eq!(top[0].username, "Bob");     // Fastest
        assert_eq!(top[1].username, "Alice");
        assert_eq!(top[2].username, "Charlie"); // Slowest
    }
}
