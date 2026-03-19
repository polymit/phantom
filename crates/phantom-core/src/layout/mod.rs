use crate::dom::{ComputedStyle, Display, DomTree, Position};
use indextree::NodeId;
use std::collections::HashMap;
use taffy::prelude::*;

pub struct LayoutEngine {
    taffy: TaffyTree,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ViewportBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl ViewportBounds {
    pub fn intersects(&self, other: &ViewportBounds) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    pub fn contains_point(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.width && py >= self.y && py <= self.y + self.height
    }

    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0.0 || self.height == 0.0
    }

    pub fn area(&self) -> f32 {
        self.width * self.height
    }
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
        }
    }

    #[tracing::instrument(skip(self, dom))]
    pub fn build_layout_tree(
        &mut self,
        dom: &DomTree,
        viewport_width: f32,
        viewport_height: f32,
    ) -> HashMap<NodeId, ViewportBounds> {
        let mut taffy_map = HashMap::new();

        if let Some(root) = dom.document_root {
            // Build taffy tree (post-order so children exist before parent adds them)
            let mut stack = vec![(root, false)];
            while let Some((node_id, visited)) = stack.pop() {
                if visited {
                    let node = dom.get_node(node_id).unwrap();
                    let attrs = match &node.data {
                        crate::dom::NodeData::Element { attributes, .. } => Some(attributes),
                        _ => None,
                    };
                    let t_style = Self::node_to_taffy_style(&node.computed_style, attrs);

                    let children: Vec<_> = node_id.children(&dom.arena).collect();
                    let t_children: Vec<_> = children
                        .iter()
                        .filter_map(|child_id| taffy_map.get(child_id).copied())
                        .collect();

                    let t_node = self.taffy.new_with_children(t_style, &t_children).unwrap();
                    taffy_map.insert(node_id, t_node);
                } else {
                    stack.push((node_id, true));
                    let children: Vec<_> = node_id.children(&dom.arena).collect();
                    for child in children.into_iter().rev() {
                        stack.push((child, false));
                    }
                }
            }

            if let Some(&root_t_node) = taffy_map.get(&root) {
                let available_space = Size {
                    width: AvailableSpace::Definite(viewport_width),
                    height: AvailableSpace::Definite(viewport_height),
                };

                let _ = self.taffy.compute_layout(root_t_node, available_space);

                let mut bounds_map = HashMap::new();
                let mut layout_stack = vec![(root, 0.0, 0.0)];

                while let Some((node_id, parent_x, parent_y)) = layout_stack.pop() {
                    if let Some(&t_node) = taffy_map.get(&node_id) {
                        let result = self.taffy.layout(t_node).unwrap();
                        let abs_x = parent_x + result.location.x;
                        let abs_y = parent_y + result.location.y;

                        let bounds = ViewportBounds {
                            x: abs_x,
                            y: abs_y,
                            width: result.size.width,
                            height: result.size.height,
                        };
                        bounds_map.insert(node_id, bounds);

                        let children: Vec<_> = node_id.children(&dom.arena).collect();
                        for child in children.into_iter().rev() {
                            layout_stack.push((child, abs_x, abs_y));
                        }
                    }
                }
                return bounds_map;
            }
        }

        HashMap::new()
    }

    fn node_to_taffy_style(
        style: &ComputedStyle,
        attrs: Option<&HashMap<String, String>>,
    ) -> taffy::Style {
        let mut t_style = taffy::Style {
            display: match style.display {
                Display::None => taffy::style::Display::None,
                Display::Flex => taffy::style::Display::Flex,
                Display::Grid => taffy::style::Display::Grid,
                _ => taffy::style::Display::Block,
            },
            position: match style.position {
                Position::Absolute | Position::Fixed => taffy::style::Position::Absolute,
                _ => taffy::style::Position::Relative,
            },
            ..taffy::Style::default()
        };

        if let Some(attrs) = attrs {
            if let Some(w) = attrs.get("width").and_then(|v| v.parse::<f32>().ok()) {
                t_style.size.width = taffy::style::Dimension::length(w);
            }
            if let Some(h) = attrs.get("height").and_then(|v| v.parse::<f32>().ok()) {
                t_style.size.height = taffy::style::Dimension::length(h);
            }
        }

        t_style
    }
}
