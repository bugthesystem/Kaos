//! Shared utility functions for console handlers and storage.

use crate::console::auth::{Identity, Permission};
use kaos_http::Request;

/// Get current Unix timestamp in seconds.
pub fn unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Convert an Instant to approximate Unix epoch timestamp.
///
/// Uses a reference Instant (typically server start time) to calculate
/// the approximate epoch time when the instant was created.
pub fn instant_to_epoch(instant: std::time::Instant, reference: std::time::Instant) -> i64 {
    let now = std::time::SystemTime::now();
    let elapsed = reference.elapsed();
    let item_elapsed = instant.elapsed();
    let diff = elapsed.as_secs() as i64 - item_elapsed.as_secs() as i64;
    now.duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64 - (elapsed.as_secs() as i64 - diff))
        .unwrap_or(0)
}

/// Check if the request has the required permission.
pub fn check_permission(req: &Request, permission: Permission) -> bool {
    req.ext::<Identity>()
        .map(|i| i.has_permission(permission))
        .unwrap_or(false)
}
