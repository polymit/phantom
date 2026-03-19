//! HTMLElement binding for QuickJS.
//!
//! Decision D-09: JS wrapper stores `arena_id: u64` ONLY.
//! Never stores Rust references, Arc, or &DomNode.
//! All DOM access goes through the thread-local `ENGINE_CONTEXT`.

use crate::quickjs::bindings::mutation_observer::MutationRecord;
use phantom_core::dom::{DomTree, NodeData};
use phantom_core::layout::ViewportBounds;
use std::collections::HashMap;

/// Engine context stored in thread-local storage.
///
/// This is the bridge between JS wrappers (which only hold `arena_id: u64`)
/// and the actual Rust DOM tree. Set before JS execution, cleared after.
pub struct EngineContext {
    /// The live DOM tree for this task.
    pub dom_tree: DomTree,
    /// Layout bounds computed by taffy.
    pub bounds: HashMap<indextree::NodeId, ViewportBounds>,
    /// Forward map: arena_id (u64) → NodeId.
    pub id_to_node: HashMap<u64, indextree::NodeId>,
    /// Reverse map: NodeId → arena_id (u64).
    pub node_to_id: HashMap<indextree::NodeId, u64>,
    /// Counter for generating new arena IDs.
    pub next_id: u64,
    /// Current page URL for location bindings.
    pub current_url: String,
    /// Page title.
    pub page_title: String,
    /// Timers registered by page scripts (Bug 4).
    pub timers: crate::quickjs::bindings::timers::TimerRegistry,
    /// Active mutation observers (Bug 6).
    pub observers: Vec<crate::quickjs::bindings::mutation_observer::MutationObserverState>,
}

impl EngineContext {
    /// Create a new engine context from a parsed DOM tree and layout bounds.
    pub fn new(
        dom_tree: DomTree,
        bounds: HashMap<indextree::NodeId, ViewportBounds>,
        url: &str,
    ) -> Self {
        let mut id_to_node = HashMap::new();
        let mut node_to_id = HashMap::new();
        let mut next_id: u64 = 0;

        // Assign sequential arena IDs to all existing nodes
        for node in dom_tree.arena.iter() {
            let node_id = dom_tree
                .arena
                .get_node_id(node)
                .expect("node in arena must have id");
            id_to_node.insert(next_id, node_id);
            node_to_id.insert(node_id, next_id);
            next_id += 1;
        }

        Self {
            dom_tree,
            bounds,
            id_to_node,
            node_to_id,
            next_id,
            current_url: url.to_string(),
            page_title: String::new(),
            timers: crate::quickjs::bindings::timers::TimerRegistry::new(),
            observers: Vec::new(),
        }
    }

    /// Allocate a new arena ID for a freshly created node.
    pub fn register_node(&mut self, node_id: indextree::NodeId) -> u64 {
        let arena_id = self.next_id;
        self.next_id += 1;
        self.id_to_node.insert(arena_id, node_id);
        self.node_to_id.insert(node_id, arena_id);
        arena_id
    }
}

tokio::task_local! {
    /// Task-local engine context for JS-DOM bridge (Bug 2).
    pub static ENGINE_CONTEXT: Option<std::sync::Arc<parking_lot::RwLock<EngineContext>>>;
}

/// Take the engine context from the current task.
pub fn take_engine_context() -> Option<std::sync::Arc<parking_lot::RwLock<EngineContext>>> {
    ENGINE_CONTEXT.with(|ctx| ctx.clone())
}

/// Execute a closure with read access to the engine context.
pub fn with_engine_context<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&EngineContext) -> R,
{
    ENGINE_CONTEXT.with(|ctx| {
        ctx.as_ref().map(|arc| {
            let lock = arc.read();
            f(&lock)
        })
    })
}

/// Execute a closure with mutable access to the engine context.
pub fn with_engine_context_mut<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut EngineContext) -> R,
{
    ENGINE_CONTEXT.with(|ctx| {
        ctx.as_ref().map(|arc| {
            let mut lock = arc.write();
            f(&mut lock)
        })
    })
}

/// HTMLElement wrapper exposed to JavaScript.
///
/// Decision D-09: stores `arena_id: u64` ONLY.
/// All DOM access goes through the thread-local `ENGINE_CONTEXT`.
#[derive(Clone, Debug)]
pub struct HTMLElement {
    /// Arena index into the DomTree — NOT a Rust reference.
    pub arena_id: u64,
}

impl HTMLElement {
    /// Get the tag name of this element (e.g., "DIV", "BUTTON").
    pub fn tag_name(&self) -> String {
        with_engine_context(|ctx| {
            if let Some(&node_id) = ctx.id_to_node.get(&self.arena_id) {
                if let Some(node) = ctx.dom_tree.get_node(node_id) {
                    if let NodeData::Element { tag_name, .. } = &node.data {
                        return tag_name.to_uppercase();
                    }
                }
            }
            String::new()
        })
        .unwrap_or_default()
    }

    /// Get the text content of this element and all descendants.
    pub fn text_content(&self) -> String {
        with_engine_context(|ctx| {
            if let Some(&node_id) = ctx.id_to_node.get(&self.arena_id) {
                return collect_text_content(&ctx.dom_tree, node_id);
            }
            String::new()
        })
        .unwrap_or_default()
    }

    /// Set the text content, replacing all children with a single text node.
    pub fn set_text_content(&self, text: String) {
        with_engine_context_mut(|ctx| {
            if let Some(&node_id) = ctx.id_to_node.get(&self.arena_id) {
                // Remove existing children
                while let Some(child) = node_id.children(&ctx.dom_tree.arena).next() {
                    child.detach(&mut ctx.dom_tree.arena);
                }
                // Add new text node
                let text_node = ctx.dom_tree.create_text(&text);
                ctx.register_node(text_node);
                ctx.dom_tree.append_child(node_id, text_node);
            }
        });
    }

    /// Get an attribute value by name.
    pub fn get_attribute(&self, name: &str) -> Option<String> {
        with_engine_context(|ctx| {
            if let Some(&node_id) = ctx.id_to_node.get(&self.arena_id) {
                if let Some(node) = ctx.dom_tree.get_node(node_id) {
                    if let NodeData::Element { attributes, .. } = &node.data {
                        return attributes.get(name).cloned();
                    }
                }
            }
            None
        })
        .flatten()
    }

    /// Set an attribute value.
    pub fn set_attribute(&self, name: String, value: String) {
        with_engine_context_mut(|ctx| {
            if let Some(&node_id) = ctx.id_to_node.get(&self.arena_id) {
                if let Some(node) = ctx.dom_tree.get_node_mut(node_id) {
                    if let NodeData::Element { attributes, .. } = &mut node.data {
                        let old = attributes.insert(name.clone(), value.clone());

                        // Notify observers (Bug 6)
                        let record = MutationRecord {
                            mutation_type: "attributes".to_string(),
                            target_arena_id: self.arena_id,
                            attribute_name: Some(name),
                            old_value: old,
                        };
                        for obs in &mut ctx.observers {
                            obs.record_mutation(record.clone());
                        }
                    }
                }
            }
        });
    }

    /// Check if an attribute exists.
    pub fn has_attribute(&self, name: &str) -> bool {
        with_engine_context(|ctx| {
            if let Some(&node_id) = ctx.id_to_node.get(&self.arena_id) {
                if let Some(node) = ctx.dom_tree.get_node(node_id) {
                    if let NodeData::Element { attributes, .. } = &node.data {
                        return attributes.contains_key(name);
                    }
                }
            }
            false
        })
        .unwrap_or(false)
    }

    /// Remove an attribute.
    pub fn remove_attribute(&self, name: String) {
        with_engine_context_mut(|ctx| {
            if let Some(&node_id) = ctx.id_to_node.get(&self.arena_id) {
                if let Some(node) = ctx.dom_tree.get_node_mut(node_id) {
                    if let NodeData::Element { attributes, .. } = &mut node.data {
                        let old_value = attributes.remove(&name);

                        // Notify observers (Bug 6)
                        let record = MutationRecord {
                            mutation_type: "attributes".to_string(),
                            target_arena_id: self.arena_id,
                            attribute_name: Some(name),
                            old_value,
                        };
                        for obs in &mut ctx.observers {
                            obs.record_mutation(record.clone());
                        }
                    }
                }
            }
        });
    }

    /// Get bounding client rect from taffy-computed layout bounds.
    pub fn get_bounding_client_rect(&self) -> DomRect {
        with_engine_context(|ctx| {
            if let Some(&node_id) = ctx.id_to_node.get(&self.arena_id) {
                if let Some(b) = ctx.bounds.get(&node_id) {
                    return DomRect {
                        x: f64::from(b.x),
                        y: f64::from(b.y),
                        width: f64::from(b.width),
                        height: f64::from(b.height),
                        top: f64::from(b.y),
                        right: f64::from(b.x + b.width),
                        bottom: f64::from(b.y + b.height),
                        left: f64::from(b.x),
                    };
                }
            }
            DomRect::default()
        })
        .unwrap_or_default()
    }

    /// Append a child element and return it.
    pub fn append_child(&self, child_arena_id: u64) -> u64 {
        with_engine_context_mut(|ctx| {
            let parent_node_id = ctx.id_to_node.get(&self.arena_id).copied();
            let child_node_id = ctx.id_to_node.get(&child_arena_id).copied();
            if let (Some(parent), Some(child)) = (parent_node_id, child_node_id) {
                ctx.dom_tree.append_child(parent, child);
            }
            child_arena_id
        })
        .unwrap_or(child_arena_id)
    }

    /// Remove a child element.
    pub fn remove_child(&self, child_arena_id: u64) {
        with_engine_context_mut(|ctx| {
            if let Some(&child_node_id) = ctx.id_to_node.get(&child_arena_id) {
                child_node_id.detach(&mut ctx.dom_tree.arena);
            }
        });
    }

    /// Simple querySelector — matches by tag name or #id.
    pub fn query_selector(&self, selector: &str) -> Option<u64> {
        with_engine_context(|ctx| {
            if let Some(&node_id) = ctx.id_to_node.get(&self.arena_id) {
                return find_matching_node(&ctx.dom_tree, &ctx.node_to_id, node_id, selector);
            }
            None
        })
        .flatten()
    }
}

/// DOMRect returned by `getBoundingClientRect()`.
#[derive(Clone, Debug, Default)]
pub struct DomRect {
    /// X coordinate of the element.
    pub x: f64,
    /// Y coordinate of the element.
    pub y: f64,
    /// Width of the element.
    pub width: f64,
    /// Height of the element.
    pub height: f64,
    /// Top edge (same as y).
    pub top: f64,
    /// Right edge (x + width).
    pub right: f64,
    /// Bottom edge (y + height).
    pub bottom: f64,
    /// Left edge (same as x).
    pub left: f64,
}

/// Recursively collect text content from a node and all its descendants.
fn collect_text_content(tree: &DomTree, node_id: indextree::NodeId) -> String {
    let mut result = String::new();
    if let Some(node) = tree.get_node(node_id) {
        if let NodeData::Text { content } = &node.data {
            result.push_str(content);
        }
    }
    for child in node_id.children(&tree.arena) {
        result.push_str(&collect_text_content(tree, child));
    }
    result
}

/// Simple selector matching — supports tag names and #id selectors.
pub fn find_matching_node(
    tree: &DomTree,
    node_to_id: &HashMap<indextree::NodeId, u64>,
    root: indextree::NodeId,
    selector: &str,
) -> Option<u64> {
    let selector = selector.trim();

    for descendant in root.descendants(&tree.arena).skip(1) {
        if let Some(node) = tree.get_node(descendant) {
            if let NodeData::Element {
                tag_name,
                attributes,
                ..
            } = &node.data
            {
                let matches = if let Some(id_sel) = selector.strip_prefix('#') {
                    attributes.get("id").is_some_and(|v| v == id_sel)
                } else if let Some(class_sel) = selector.strip_prefix('.') {
                    attributes
                        .get("class")
                        .is_some_and(|v| v.split_whitespace().any(|c| c == class_sel))
                } else {
                    tag_name.eq_ignore_ascii_case(selector)
                };

                if matches {
                    return node_to_id.get(&descendant).copied();
                }
            }
        }
    }
    None
}

/// Find all matching nodes for querySelectorAll.
pub fn find_all_matching_nodes(
    tree: &DomTree,
    node_to_id: &HashMap<indextree::NodeId, u64>,
    root: indextree::NodeId,
    selector: &str,
) -> Vec<u64> {
    let selector = selector.trim();
    let mut results = Vec::new();

    for descendant in root.descendants(&tree.arena).skip(1) {
        if let Some(node) = tree.get_node(descendant) {
            if let NodeData::Element {
                tag_name,
                attributes,
                ..
            } = &node.data
            {
                let matches = if let Some(id_sel) = selector.strip_prefix('#') {
                    attributes.get("id").is_some_and(|v| v == id_sel)
                } else if let Some(class_sel) = selector.strip_prefix('.') {
                    attributes
                        .get("class")
                        .is_some_and(|v| v.split_whitespace().any(|c| c == class_sel))
                } else {
                    tag_name.eq_ignore_ascii_case(selector)
                };

                if matches {
                    if let Some(&aid) = node_to_id.get(&descendant) {
                        results.push(aid);
                    }
                }
            }
        }
    }
    results
}
