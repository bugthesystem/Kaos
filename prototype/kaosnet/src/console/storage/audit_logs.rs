//! In-memory audit log storage.

use crate::console::types::AuditLogEntry;
use parking_lot::RwLock;
use std::collections::VecDeque;
use uuid::Uuid;

/// Maximum number of audit log entries to keep in memory.
const MAX_ENTRIES: usize = 10_000;

/// In-memory audit log storage.
pub struct AuditLogStorage {
    entries: RwLock<VecDeque<AuditLogEntry>>,
}

impl AuditLogStorage {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(VecDeque::with_capacity(MAX_ENTRIES)),
        }
    }

    /// Record an audit log entry.
    pub fn record(
        &self,
        actor_id: Uuid,
        actor_name: String,
        actor_type: String,
        action: String,
        resource_type: String,
        resource_id: String,
        details: Option<String>,
        ip_address: Option<String>,
        success: bool,
    ) -> AuditLogEntry {
        let entry = AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
            actor_id,
            actor_name,
            actor_type,
            action,
            resource_type,
            resource_id,
            details,
            ip_address,
            success,
        };

        let mut entries = self.entries.write();
        if entries.len() >= MAX_ENTRIES {
            entries.pop_front();
        }
        entries.push_back(entry.clone());

        entry
    }

    /// List audit log entries with pagination and optional filters.
    pub fn list(
        &self,
        page: u32,
        page_size: u32,
        action_filter: Option<&str>,
        actor_filter: Option<&str>,
    ) -> Vec<AuditLogEntry> {
        let entries = self.entries.read();

        let filtered: Vec<_> = entries
            .iter()
            .filter(|e| {
                let action_match = action_filter
                    .map(|a| e.action.eq_ignore_ascii_case(a))
                    .unwrap_or(true);
                let actor_match = actor_filter
                    .map(|a| e.actor_name.to_lowercase().contains(&a.to_lowercase()))
                    .unwrap_or(true);
                action_match && actor_match
            })
            .cloned()
            .collect();

        let start = ((page - 1) * page_size) as usize;
        filtered
            .into_iter()
            .rev() // Most recent first
            .skip(start)
            .take(page_size as usize)
            .collect()
    }

    /// Get a single audit log entry by ID.
    pub fn get(&self, id: &str) -> Option<AuditLogEntry> {
        let uuid = Uuid::parse_str(id).ok()?;
        let entries = self.entries.read();
        entries.iter().find(|e| e.id == uuid).cloned()
    }

    /// Get total count of entries.
    pub fn count(&self) -> u32 {
        self.entries.read().len() as u32
    }
}

impl Default for AuditLogStorage {
    fn default() -> Self {
        Self::new()
    }
}
