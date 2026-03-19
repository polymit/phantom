//! MutationObserver binding for QuickJS.
//!
//! Tracks DOM mutations and batches them for callback delivery.
//! This is how agents know React/Vue has finished rendering.

use std::collections::HashMap;

/// A single mutation record describing one DOM change.
#[derive(Debug, Clone)]
pub struct MutationRecord {
    /// Type of mutation: "childList", "attributes", or "characterData".
    pub mutation_type: String,
    /// Arena ID of the target node.
    pub target_arena_id: u64,
    /// Name of the changed attribute (for "attributes" type).
    pub attribute_name: Option<String>,
    /// Old value of the attribute (if requested).
    pub old_value: Option<String>,
}

/// Observer configuration matching the MutationObserverInit dictionary.
#[derive(Debug, Clone, Default)]
pub struct ObserveConfig {
    /// Observe child list changes.
    pub child_list: bool,
    /// Observe attribute changes.
    pub attributes: bool,
    /// Observe text content changes.
    pub character_data: bool,
    /// Monitor the entire subtree.
    pub subtree: bool,
    /// Record old attribute values.
    pub attribute_old_value: bool,
}

/// MutationObserver state for a single observer instance.
pub struct MutationObserverState {
    /// Targets being observed, keyed by arena_id.
    pub targets: HashMap<u64, ObserveConfig>,
    /// Pending mutation records not yet delivered.
    pub pending_records: Vec<MutationRecord>,
    /// Whether this observer is connected.
    pub connected: bool,
}

impl MutationObserverState {
    /// Create a new disconnected observer.
    pub fn new() -> Self {
        Self {
            targets: HashMap::new(),
            pending_records: Vec::new(),
            connected: false,
        }
    }

    /// Start observing a target with the given configuration.
    pub fn observe(&mut self, target_arena_id: u64, config: ObserveConfig) {
        self.targets.insert(target_arena_id, config);
        self.connected = true;
    }

    /// Stop observing all targets.
    pub fn disconnect(&mut self) {
        self.targets.clear();
        self.connected = false;
    }

    /// Record a mutation if the target is being observed.
    pub fn record_mutation(&mut self, record: MutationRecord) {
        if self.connected && self.targets.contains_key(&record.target_arena_id) {
            self.pending_records.push(record);
        }
    }

    /// Take all pending records, clearing the queue.
    pub fn take_records(&mut self) -> Vec<MutationRecord> {
        std::mem::take(&mut self.pending_records)
    }
}

impl Default for MutationObserverState {
    fn default() -> Self {
        Self::new()
    }
}
