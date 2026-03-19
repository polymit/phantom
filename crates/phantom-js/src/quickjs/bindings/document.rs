//! Document binding for QuickJS.
//!
//! Implements the DOM Document interface via the thread-local `ENGINE_CONTEXT`.

use super::element::{
    find_all_matching_nodes, with_engine_context, with_engine_context_mut, HTMLElement,
};
use phantom_core::dom::NodeData;
use std::collections::HashMap;

/// Document API exposed to JavaScript.
///
/// All operations go through the thread-local `ENGINE_CONTEXT`.
pub struct DocumentBinding;

impl DocumentBinding {
    /// Create a new element with the given tag name.
    ///
    /// Returns an `HTMLElement` with a fresh `arena_id`.
    pub fn create_element(tag: &str) -> Option<HTMLElement> {
        with_engine_context_mut(|ctx| {
            let node_id = ctx.dom_tree.create_element(tag, HashMap::new());
            let arena_id = ctx.register_node(node_id);
            HTMLElement { arena_id }
        })
    }

    /// Find an element by its `id` attribute.
    pub fn get_element_by_id(id: &str) -> Option<HTMLElement> {
        with_engine_context(|ctx| {
            let root = ctx.dom_tree.document_root?;
            for descendant in root.descendants(&ctx.dom_tree.arena) {
                if let Some(node) = ctx.dom_tree.get_node(descendant) {
                    if let NodeData::Element { attributes, .. } = &node.data {
                        if attributes.get("id").is_some_and(|v| v == id) {
                            let arena_id = ctx.node_to_id.get(&descendant).copied()?;
                            return Some(HTMLElement { arena_id });
                        }
                    }
                }
            }
            None
        })
        .flatten()
    }

    /// querySelector on the document root.
    pub fn query_selector(selector: &str) -> Option<HTMLElement> {
        with_engine_context(|ctx| {
            let root = ctx.dom_tree.document_root?;
            let arena_id =
                super::element::find_matching_node(&ctx.dom_tree, &ctx.node_to_id, root, selector)?;
            Some(HTMLElement { arena_id })
        })
        .flatten()
    }

    /// querySelectorAll on the document root.
    pub fn query_selector_all(selector: &str) -> Vec<HTMLElement> {
        with_engine_context(|ctx| {
            let root = match ctx.dom_tree.document_root {
                Some(r) => r,
                None => return Vec::new(),
            };
            find_all_matching_nodes(&ctx.dom_tree, &ctx.node_to_id, root, selector)
                .into_iter()
                .map(|arena_id| HTMLElement { arena_id })
                .collect()
        })
        .unwrap_or_default()
    }

    /// Get the document title.
    pub fn title() -> String {
        with_engine_context(|ctx| ctx.page_title.clone()).unwrap_or_default()
    }

    /// Set the document title.
    pub fn set_title(title: &str) {
        with_engine_context_mut(|ctx| {
            ctx.page_title = title.to_string();
        });
    }

    /// Get the document ready state.
    pub fn ready_state() -> String {
        "complete".to_string()
    }

    /// Append a child to the document root.
    pub fn append_child(child_arena_id: u64) -> u64 {
        with_engine_context_mut(|ctx| {
            let root = match ctx.dom_tree.document_root {
                Some(r) => r,
                None => return child_arena_id,
            };
            if let Some(&child_node_id) = ctx.id_to_node.get(&child_arena_id) {
                ctx.dom_tree.append_child(root, child_node_id);
            }
            child_arena_id
        })
        .unwrap_or(child_arena_id)
    }
}

/// Find the `<body>` element arena_id, if present.
pub fn find_body_arena_id() -> Option<u64> {
    with_engine_context(|ctx| {
        let root = ctx.dom_tree.document_root?;
        for descendant in root.descendants(&ctx.dom_tree.arena) {
            if let Some(node) = ctx.dom_tree.get_node(descendant) {
                if let NodeData::Element { tag_name, .. } = &node.data {
                    if tag_name.eq_ignore_ascii_case("body") {
                        return ctx.node_to_id.get(&descendant).copied();
                    }
                }
            }
        }
        None
    })
    .flatten()
}
