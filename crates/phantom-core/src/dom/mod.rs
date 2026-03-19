use indextree::{Arena, NodeId};
use std::collections::HashMap;

/// Display property — ONLY these values matter for agents
#[derive(Debug, Clone, PartialEq)]
pub enum Display {
    Block,
    None,
    Inline,
    Flex,
    Grid,
    InlineBlock,
    Table,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Visible,
    Hidden,
    Collapse,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PointerEvents {
    Auto,
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Position {
    Static,
    Relative,
    Absolute,
    Fixed,
    Sticky,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventListenerType {
    Click,
    Focus,
    Blur,
    Input,
    Submit,
    Keypress,
    Change,
    Mousemove,
}

/// The 6 CSS properties Phantom Engine tracks. Nothing else.
/// See Decision D-05 in phantom skill.
#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub display: Display,
    pub visibility: Visibility,
    pub opacity: f32,
    pub position: Position,
    pub z_index: Option<i32>,
    pub pointer_events: PointerEvents,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            display: Display::Block,
            visibility: Visibility::Visible,
            opacity: 1.0,
            position: Position::Static,
            z_index: None,
            pointer_events: PointerEvents::Auto,
        }
    }
}

/// Data held by each DOM node
#[derive(Debug, Clone)]
pub enum NodeData {
    Document,
    Element {
        tag_name: String,
        attributes: HashMap<String, String>,
        layout_id: Option<taffy::NodeId>,
    },
    Text {
        content: String,
    },
    Comment {
        content: String,
    },
    Doctype {
        name: String,
    },
}

/// A single node in the DOM tree
#[derive(Debug, Clone)]
pub struct DomNode {
    pub data: NodeData,
    pub computed_style: ComputedStyle,
    pub event_listeners: Vec<EventListenerType>,
    pub aria_role: Option<String>,
    pub aria_label: Option<String>,
    pub is_visible: bool,
}

impl DomNode {
    pub fn new_element(tag: &str, attrs: HashMap<String, String>) -> Self {
        Self {
            data: NodeData::Element {
                tag_name: tag.to_string(),
                attributes: attrs,
                layout_id: None,
            },
            computed_style: ComputedStyle::default(),
            event_listeners: Vec::new(),
            aria_role: None,
            aria_label: None,
            is_visible: true,
        }
    }

    pub fn new_text(content: &str) -> Self {
        Self {
            data: NodeData::Text {
                content: content.to_string(),
            },
            computed_style: ComputedStyle::default(),
            event_listeners: Vec::new(),
            aria_role: None,
            aria_label: None,
            is_visible: true,
        }
    }

    pub fn new_document() -> Self {
        Self {
            data: NodeData::Document,
            computed_style: ComputedStyle::default(),
            event_listeners: Vec::new(),
            aria_role: None,
            aria_label: None,
            is_visible: true,
        }
    }

    pub fn tag_name(&self) -> Option<&str> {
        match &self.data {
            NodeData::Element { tag_name, .. } => Some(tag_name),
            _ => None,
        }
    }

    pub fn get_attr(&self, name: &str) -> Option<&str> {
        match &self.data {
            NodeData::Element { attributes, .. } => attributes.get(name).map(|s| s.as_str()),
            _ => None,
        }
    }
}

/// The arena-allocated DOM tree. Core data structure of Phantom Engine.
pub struct DomTree {
    pub arena: Arena<DomNode>,
    pub document_root: Option<NodeId>,
}

impl DomTree {
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
            document_root: None,
        }
    }

    pub fn create_element(&mut self, tag: &str, attrs: HashMap<String, String>) -> NodeId {
        self.arena.new_node(DomNode::new_element(tag, attrs))
    }

    pub fn create_text(&mut self, content: &str) -> NodeId {
        self.arena.new_node(DomNode::new_text(content))
    }

    pub fn create_comment(&mut self, content: &str) -> NodeId {
        let mut node = DomNode::new_text(content); // using text struct but logic should be comment
        node.data = NodeData::Comment {
            content: content.to_string(),
        };
        self.arena.new_node(node)
    }

    pub fn append_child(&mut self, parent: NodeId, child: NodeId) {
        parent.append(child, &mut self.arena);
    }

    pub fn insert_before(&mut self, sibling: NodeId, new_node: NodeId) {
        sibling.insert_before(new_node, &mut self.arena);
    }

    pub fn remove_node(&mut self, node: NodeId) {
        node.remove(&mut self.arena);
    }

    pub fn get_node(&self, id: NodeId) -> Option<&DomNode> {
        self.arena.get(id).map(|n| n.get())
    }

    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut DomNode> {
        self.arena.get_mut(id).map(|n| n.get_mut())
    }

    pub fn node_count(&self) -> usize {
        self.arena.iter().count()
    }

    #[tracing::instrument(skip(self))]
    pub fn query_selector(&self, selector: &str) -> Option<NodeId> {
        let selector = selector.trim();
        if let Some(root) = self.document_root {
            for id in root.descendants(&self.arena) {
                if let Some(node) = self.get_node(id) {
                    if let crate::dom::NodeData::Element {
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
                            return Some(id);
                        }
                    }
                }
            }
        }
        None
    }

    #[tracing::instrument(skip(self))]
    pub fn query_selector_all(&self, selector: &str) -> Vec<NodeId> {
        let selector = selector.trim();
        let mut results = Vec::new();
        if let Some(root) = self.document_root {
            for id in root.descendants(&self.arena) {
                if let Some(node) = self.get_node(id) {
                    if let crate::dom::NodeData::Element {
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
                            results.push(id);
                        }
                    }
                }
            }
        }
        results
    }
}

impl Default for DomTree {
    fn default() -> Self {
        Self::new()
    }
}
