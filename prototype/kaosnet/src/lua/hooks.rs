//! Hook registration and dispatch.

use std::collections::HashSet;

use dashmap::DashMap;
use parking_lot::RwLock;

/// Hook types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookType {
    BeforeRoomJoin,
    AfterRoomJoin,
    BeforeRoomLeave,
    AfterRoomLeave,
    BeforeRoomData,
    AfterRoomData,
    BeforeRpc,
    AfterRpc,
}

impl HookType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "room_join" => Some(Self::BeforeRoomJoin),
            "room_leave" => Some(Self::BeforeRoomLeave),
            "room_data" => Some(Self::BeforeRoomData),
            "rpc" => Some(Self::BeforeRpc),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BeforeRoomJoin => "before_room_join",
            Self::AfterRoomJoin => "after_room_join",
            Self::BeforeRoomLeave => "before_room_leave",
            Self::AfterRoomLeave => "after_room_leave",
            Self::BeforeRoomData => "before_room_data",
            Self::AfterRoomData => "after_room_data",
            Self::BeforeRpc => "before_rpc",
            Self::AfterRpc => "after_rpc",
        }
    }
}

/// Hook registry tracks which hooks are registered
pub struct HookRegistry {
    registered: RwLock<HashSet<HookType>>,
    rpc_names: DashMap<String, bool>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            registered: RwLock::new(HashSet::new()),
            rpc_names: DashMap::new(),
        }
    }

    pub fn register(&self, hook: HookType) {
        self.registered.write().insert(hook);
    }

    pub fn is_registered(&self, hook: HookType) -> bool {
        self.registered.read().contains(&hook)
    }

    pub fn register_rpc(&self, name: String) {
        self.rpc_names.insert(name, true);
    }

    pub fn has_rpc(&self, name: &str) -> bool {
        self.rpc_names.contains_key(name)
    }

    pub fn rpc_names(&self) -> Vec<String> {
        self.rpc_names.iter().map(|r| r.key().clone()).collect()
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_registry() {
        let registry = HookRegistry::new();

        assert!(!registry.is_registered(HookType::BeforeRoomJoin));

        registry.register(HookType::BeforeRoomJoin);
        assert!(registry.is_registered(HookType::BeforeRoomJoin));
    }

    #[test]
    fn test_rpc_registry() {
        let registry = HookRegistry::new();

        assert!(!registry.has_rpc("test_rpc"));

        registry.register_rpc("test_rpc".to_string());
        assert!(registry.has_rpc("test_rpc"));
    }
}
