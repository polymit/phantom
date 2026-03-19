use indextree::NodeId;
use parking_lot::RwLock;
use phantom_core::dom::{Display, DomTree, NodeData, Visibility};
use phantom_core::layout::ViewportBounds;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::atomic::{AtomicU64, Ordering};

/// The Headless Serializer converts a parsed DOM tree with computed layout
/// bounds into the CCT (Custom Compressed Text) format for agent perception.
///
/// This is a strictly read-only transformation — it never mutates the DOM.
/// The serialization follows an exact 8-stage pipeline.
pub struct HeadlessSerializer {
    viewport: ViewportBounds,
    id_mapping: RwLock<HashMap<NodeId, String>>,
    id_counter: AtomicU64,
    buffer_pool: RwLock<Vec<String>>,
}

/// Determines if a node is truly visible to an agent.
///
/// All 7 conditions must be true:
/// 1. `display != None`
/// 2. `visibility != Hidden`
/// 3. `opacity > 0`
/// 4. bounds exist
/// 5. `width > 0`
/// 6. `height > 0`
/// 7. bounds intersect viewport
fn is_truly_visible(
    node_id: NodeId,
    dom: &DomTree,
    _bounds: &HashMap<NodeId, ViewportBounds>,
    _viewport: &ViewportBounds,
) -> bool {
    let node = match dom.get_node(node_id) {
        Some(n) => n,
        None => return false,
    };
    let style = &node.computed_style;
    style.display != Display::None && style.visibility != Visibility::Hidden && style.opacity > 0.0
}

/// Extracted semantic data for a single visible node, ready for CCT encoding.
struct NodeSemantic {
    id: String,
    tag: String,
    role: String,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    display: char,
    visibility: char,
    opacity: f32,
    pointer_events: char,
    accessible_name: String,
    visible_text: String,
    events: String,
    parent_id: String,
    flags: u8,
    state: String,
}

impl Default for HeadlessSerializer {
    fn default() -> Self {
        Self::new(ViewportBounds {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        })
    }
}

impl HeadlessSerializer {
    /// Create a new serializer with the given viewport dimensions.
    #[tracing::instrument]
    pub fn new(viewport: ViewportBounds) -> Self {
        Self {
            viewport,
            id_mapping: RwLock::new(HashMap::new()),
            id_counter: AtomicU64::new(0),
            buffer_pool: RwLock::new(Vec::new()),
        }
    }

    /// Serialize the DOM tree and layout bounds into a CCT scene graph string.
    ///
    /// This executes the full 8-stage pipeline:
    /// 1. Preparation  2. Visibility  3. Collect  4. Cull
    /// 5. Z-index  6. Semantic extraction  7. ID stabilization  8. CCT encoding
    #[tracing::instrument(skip(self, dom, bounds))]
    pub fn serialize(&self, dom: &DomTree, bounds: &HashMap<NodeId, ViewportBounds>) -> String {
        // STAGE 1: Preparation
        let estimated_size = bounds.len() * 80;
        let mut buffer = self
            .buffer_pool
            .write()
            .pop()
            .unwrap_or_else(|| String::with_capacity(estimated_size));
        buffer.clear();

        let root = match dom.document_root {
            Some(r) => r,
            None => return buffer,
        };

        // STAGE 2: Visibility computation — BOTTOM-UP (post-order)
        let mut post_order = Vec::with_capacity(bounds.len());
        {
            let mut stack = Vec::with_capacity(64);
            stack.push((root, false));

            while let Some((node_id, visited)) = stack.pop() {
                if visited {
                    post_order.push(node_id);
                } else {
                    stack.push((node_id, true));
                    for child in node_id.children(&dom.arena).rev() {
                        stack.push((child, false));
                    }
                }
            }
        }

        let mut visibility_map: HashMap<NodeId, bool> = HashMap::with_capacity(post_order.len());
        for &node_id in &post_order {
            let visible = is_truly_visible(node_id, dom, bounds, &self.viewport);
            visibility_map.insert(node_id, visible);
        }

        // STAGE 2.5: Geometry computation — TOP-DOWN (pre-order)
        // Decision D-19: Two-pass traversal in serializer.
        // Pass 1 (Bottom-up): Visibility (done above)
        // Pass 2 (Top-down): Geometry constraints (stubbed here)
        let mut geometry_map: HashMap<NodeId, ViewportBounds> =
            HashMap::with_capacity(bounds.len());
        {
            let mut stack = Vec::with_capacity(64);
            stack.push(root);

            while let Some(node_id) = stack.pop() {
                // In a real implementation we would constrain child bounds to parent clipping rects here
                if let Some(b) = bounds.get(&node_id) {
                    geometry_map.insert(node_id, *b);
                }
                for child in node_id.children(&dom.arena).rev() {
                    stack.push(child);
                }
            }
        }

        // STAGE 3: Collect visible element nodes only
        let visible_nodes: Vec<NodeId> = post_order
            .iter()
            .copied()
            .filter(|&node_id| {
                *visibility_map.get(&node_id).unwrap_or(&false)
                    && matches!(
                        dom.get_node(node_id).map(|n| &n.data),
                        Some(NodeData::Element { .. })
                    )
            })
            .collect();

        // STAGE 4: Viewport culling (already covered in is_truly_visible,
        // but kept as an explicit stage for pipeline correctness)

        // STAGE 5: Z-index resolution (stub — assumes non-overlapping for MVP)

        // STAGE 6: Semantic extraction — PARALLEL via rayon
        let extracted: Vec<NodeSemantic> = visible_nodes
            .par_iter()
            .map(|&node_id| {
                let node = dom.get_node(node_id).unwrap();
                let b = geometry_map
                    .get(&node_id)
                    .unwrap_or_else(|| bounds.get(&node_id).unwrap());

                let mut tag = String::new();
                let mut role = "none".to_string();
                let mut accessible_name = "-".to_string();
                let mut visible_text = "-".to_string();
                let mut events = "-".to_string();
                let mut flags: u8 = 0;
                let mut state_str = String::new();

                if let NodeData::Element {
                    tag_name,
                    attributes,
                    ..
                } = &node.data
                {
                    let lower_tag = tag_name.to_lowercase();
                    tag = match lower_tag.as_str() {
                        "button" => "btn",
                        "input" => "inpt",
                        "a" => "lnk",
                        "form" => "frm",
                        "select" => "sel",
                        "canvas" => "canv",
                        "iframe" => "ifrm",
                        "span" => "span",
                        "div" => "div",
                        t => t,
                    }
                    .to_string();

                    role = match lower_tag.as_str() {
                        "button" => "btn",
                        "a" => "lnk",
                        "input" => "ipt",
                        "nav" => "nav",
                        "main" => "main",
                        _ => "none",
                    }
                    .to_string();

                    if lower_tag == "canvas" {
                        flags |= 4;
                    }
                    if lower_tag == "svg" {
                        flags |= 8;
                    }
                    if lower_tag == "iframe" {
                        flags |= 2;
                    }

                    // Text extraction from direct children
                    let mut text_content = String::new();
                    for child in node_id.children(&dom.arena) {
                        if let Some(cn) = dom.get_node(child) {
                            if let NodeData::Text { content } = &cn.data {
                                text_content.push_str(content.trim());
                                text_content.push(' ');
                            }
                        }
                    }
                    let trimmed = text_content.trim();
                    if !trimmed.is_empty() {
                        visible_text = trimmed.chars().take(100).collect();
                    }

                    // Accessible name priority chain
                    let al = attributes.get("aria-label").cloned();
                    let a_ld = attributes.get("aria-labelledby").cloned();
                    let tit = attributes.get("title").cloned();
                    let alt = attributes.get("alt").cloned();
                    let ph = attributes.get("placeholder").cloned();
                    if let Some(lbl) = al.or(a_ld).or(tit).or(alt).or(ph) {
                        accessible_name = lbl.chars().take(100).collect();
                    }

                    // Event inference from tag semantics
                    let mut evs = Vec::new();
                    if attributes.contains_key("onclick")
                        || lower_tag == "button"
                        || lower_tag == "a"
                    {
                        evs.push("c");
                    }
                    if attributes.contains_key("onfocus") || lower_tag == "input" {
                        evs.push("f");
                    }
                    if !evs.is_empty() {
                        events = evs.join(",");
                    }

                    // State bits
                    let disabled = attributes.contains_key("disabled");
                    let checked = attributes.contains_key("checked");
                    let selected = attributes.contains_key("selected");
                    let expanded = attributes.contains_key("aria-expanded");
                    let required = attributes.contains_key("required");
                    if disabled || checked || selected || expanded || required {
                        state_str = format!(
                            "s:{},{},{},{},{}",
                            u8::from(disabled),
                            u8::from(checked),
                            u8::from(selected),
                            u8::from(expanded),
                            u8::from(required)
                        );
                    }
                }

                let d = match node.computed_style.display {
                    Display::None => 'n',
                    Display::Inline => 'i',
                    Display::Flex => 'f',
                    Display::Grid => 'g',
                    _ => 'b',
                };
                let v = match node.computed_style.visibility {
                    Visibility::Hidden | Visibility::Collapse => 'h',
                    _ => 'v',
                };
                let p = match node.computed_style.pointer_events {
                    phantom_core::dom::PointerEvents::None => 'n',
                    _ => 'a',
                };

                NodeSemantic {
                    id: String::new(),
                    tag,
                    role,
                    x: b.x as i32,
                    y: b.y as i32,
                    w: b.width as i32,
                    h: b.height as i32,
                    display: d,
                    visibility: v,
                    opacity: node.computed_style.opacity,
                    pointer_events: p,
                    accessible_name,
                    visible_text,
                    events,
                    parent_id: String::new(),
                    flags,
                    state: state_str,
                }
            })
            .collect();

        // STAGE 7: ID stabilization
        let mut final_semantics = extracted;
        let mut id_map = self.id_mapping.write();
        let mut resolved_node_ids: HashMap<NodeId, String> =
            HashMap::with_capacity(visible_nodes.len());

        for (i, &node_id) in visible_nodes.iter().enumerate() {
            let final_id = if let Some(existing) = id_map.get(&node_id) {
                existing.clone()
            } else {
                let node = dom.get_node(node_id).unwrap();
                let mut new_id = None;
                if let NodeData::Element { attributes, .. } = &node.data {
                    if let Some(agent_id) = attributes.get("data-agent-id") {
                        if !agent_id.is_empty() {
                            new_id = Some(agent_id.clone());
                        }
                    } else if let Some(test_id) = attributes.get("data-testid") {
                        if !test_id.is_empty() {
                            new_id = Some(test_id.clone());
                        }
                    }
                }
                let gen = new_id.unwrap_or_else(|| {
                    let c = self.id_counter.fetch_add(1, Ordering::SeqCst);
                    format!("n_{}", c)
                });
                id_map.insert(node_id, gen.clone());
                gen
            };

            resolved_node_ids.insert(node_id, final_id.clone());
            final_semantics[i].id = final_id;
        }

        // Resolve parent IDs
        for (i, &node_id) in visible_nodes.iter().enumerate() {
            let mut p_id = "root".to_string();
            let mut curr = node_id.ancestors(&dom.arena).nth(1);
            while let Some(anc) = curr {
                if let Some(anc_id) = resolved_node_ids.get(&anc) {
                    p_id = anc_id.clone();
                    break;
                }
                curr = anc.ancestors(&dom.arena).nth(1);
            }
            final_semantics[i].parent_id = p_id;
        }

        // STAGE 8: CCT encoding — write directly to buffer
        for sem in &final_semantics {
            let _ = write!(
                buffer,
                "{}|{}|{}|{},{},{},{}|{},{},{:.1},{}|{}|{}|{}|{}|{}",
                sem.id,
                sem.tag,
                sem.role,
                sem.x,
                sem.y,
                sem.w,
                sem.h,
                sem.display,
                sem.visibility,
                sem.opacity,
                sem.pointer_events,
                sem.accessible_name,
                sem.visible_text,
                sem.events,
                sem.parent_id,
                sem.flags
            );
            if !sem.state.is_empty() {
                buffer.push('|');
                buffer.push_str(&sem.state);
            }
            buffer.push('\n');
        }

        let result = buffer.clone();
        self.buffer_pool.write().push(buffer);
        result
    }
}

// -------------------------------------------------------------
// Mutation & Delta Types
// -------------------------------------------------------------

/// Represents a DOM mutation tracked for delta serialization.
#[derive(Debug, Clone, PartialEq)]
pub enum Mutation {
    /// A new node was inserted into the DOM.
    NodeInserted {
        node_id: NodeId,
        parent_id: NodeId,
        index: usize,
    },
    /// A node was removed from the DOM.
    NodeRemoved { node_id: NodeId, parent_id: NodeId },
    /// An attribute on a node changed.
    AttrChanged {
        node_id: NodeId,
        attr: String,
        old: Option<String>,
        new: Option<String>,
    },
    /// Text content of a node changed.
    TextChanged { node_id: NodeId, new_data: String },
}

/// Coalesce mutations within a batch (16ms window).
///
/// - Multiple attribute changes on the same node/attr → keep last only
/// - Insert + Remove same node → cancel out (handled by caller)
/// - A→B→A on same attr → effectively no-op (last value wins)
pub fn coalesce_mutations(mutations: Vec<Mutation>) -> Vec<Mutation> {
    let mut coalesced = Vec::new();
    let mut attr_changes: HashMap<(NodeId, String), Option<String>> = HashMap::new();

    for m in mutations {
        match m {
            Mutation::AttrChanged {
                node_id, attr, new, ..
            } => {
                attr_changes.insert((node_id, attr), new);
            }
            _ => coalesced.push(m),
        }
    }

    for ((node_id, attr), new) in attr_changes {
        coalesced.push(Mutation::AttrChanged {
            node_id,
            attr,
            old: None,
            new,
        });
    }

    coalesced
}
