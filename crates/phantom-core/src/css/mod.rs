use crate::dom::{ComputedStyle, Display, DomTree, NodeData, PointerEvents, Position, Visibility};
use cssparser::{Parser, ParserInput, ToCss, Token};
use std::collections::HashMap;

/// A single CSS rule extracted from a <style> tag.
#[derive(Debug, Clone)]
pub struct StylesheetRule {
    pub selector: String,
    pub property: String,
    pub value: String,
}

pub struct CascadeEngine;

impl Default for CascadeEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl CascadeEngine {
    pub fn new() -> Self {
        Self
    }

    #[tracing::instrument(skip(self, attributes, parent_style, stylesheet_rules))]
    pub fn compute_style(
        &self,
        tag_name: &str,
        attributes: &HashMap<String, String>,
        parent_style: Option<&ComputedStyle>,
        stylesheet_rules: &[StylesheetRule],
    ) -> ComputedStyle {
        let mut style = ComputedStyle {
            display: Display::Block,
            visibility: parent_style
                .map(|p| p.visibility.clone())
                .unwrap_or(Visibility::Visible),
            opacity: 1.0,
            position: Position::Static,
            z_index: None,
            pointer_events: parent_style
                .map(|p| p.pointer_events.clone())
                .unwrap_or(PointerEvents::Auto),
        };

        // 1. Apply matching stylesheet rules (Bug 7)
        for rule in stylesheet_rules {
            let matches = if let Some(id_sel) = rule.selector.strip_prefix('#') {
                attributes.get("id").is_some_and(|v| v == id_sel)
            } else if let Some(class_sel) = rule.selector.strip_prefix('.') {
                attributes
                    .get("class")
                    .is_some_and(|v| v.split_whitespace().any(|c| c == class_sel))
            } else {
                tag_name.eq_ignore_ascii_case(&rule.selector)
            };

            if matches {
                self.apply_property(&mut style, &rule.property, &rule.value);
            }
        }

        // 2. Apply inline style attribute (overrides stylesheet)
        if let Some(style_attr) = attributes.get("style") {
            let mut input = ParserInput::new(style_attr);
            let mut parser = Parser::new(&mut input);
            let _ = parser.parse_entirely(|p| -> Result<(), cssparser::ParseError<'_, ()>> {
                loop {
                    let next = p.next();
                    match next {
                        Ok(Token::Ident(ref name)) => {
                            let prop_name = name.clone();
                            if let Ok(Token::Colon) = p.next() {
                                let mut value_str = String::new();
                                while let Ok(val_tok) = p.next() {
                                    if matches!(val_tok, Token::Semicolon) {
                                        break;
                                    }
                                    if !value_str.is_empty() {
                                        value_str.push(' ');
                                    }
                                    value_str.push_str(&val_tok.to_css_string());
                                }
                                self.apply_property(&mut style, &prop_name, &value_str);
                            } else {
                                while let Ok(tok) = p.next() {
                                    if matches!(tok, Token::Semicolon) {
                                        break;
                                    }
                                }
                            }
                        }
                        Ok(_) => {
                            while let Ok(tok) = p.next() {
                                if matches!(tok, Token::Semicolon) {
                                    break;
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
                Ok(())
            });
        }

        style
    }

    /// Helper to apply a property-value pair to a ComputedStyle.
    fn apply_property(&self, style: &mut ComputedStyle, name: &str, value: &str) {
        match name.to_lowercase().as_str() {
            "display" => {
                if let Some(v) = parse_display(value) {
                    style.display = v;
                }
            }
            "visibility" => {
                if let Some(v) = parse_visibility(value) {
                    style.visibility = v;
                }
            }
            "opacity" => {
                if let Some(v) = parse_opacity(value) {
                    style.opacity = v;
                }
            }
            "position" => {
                if let Some(v) = parse_position(value) {
                    style.position = v;
                }
            }
            "z-index" => {
                if let Some(v) = parse_z_index(value) {
                    style.z_index = Some(v);
                }
            }
            "pointer-events" => {
                if let Some(v) = parse_pointer_events(value) {
                    style.pointer_events = v;
                }
            }
            _ => {}
        }
    }

    #[tracing::instrument(skip(self, tree))]
    pub fn apply_cascade(&self, tree: &mut DomTree) {
        let stylesheet_rules = self.extract_stylesheet_rules(tree);

        if let Some(root) = tree.document_root {
            let mut stack = vec![(root, None)];

            while let Some((node_id, parent_style_opt)) = stack.pop() {
                let computed;
                {
                    let node = tree.get_node(node_id).unwrap();
                    if let NodeData::Element {
                        tag_name,
                        attributes,
                        ..
                    } = &node.data
                    {
                        computed = self.compute_style(
                            tag_name,
                            attributes,
                            parent_style_opt.as_ref(),
                            &stylesheet_rules,
                        );
                    } else if let NodeData::Text { .. } = &node.data {
                        let default_style = ComputedStyle {
                            display: Display::Inline, // text is inherently inline
                            ..ComputedStyle::default()
                        };
                        computed = parent_style_opt.unwrap_or(default_style);
                    } else {
                        computed = ComputedStyle::default();
                    }
                }

                tree.get_node_mut(node_id).unwrap().computed_style = computed.clone();

                let children: Vec<_> = node_id.children(&tree.arena).collect();
                for child_id in children.into_iter().rev() {
                    stack.push((child_id, Some(computed.clone())));
                }
            }
        }
    }

    /// Extract stylesheet rules from all <style> tags in the DOM.
    pub fn extract_stylesheet_rules(&self, tree: &DomTree) -> Vec<StylesheetRule> {
        let mut rules = Vec::new();
        let style_nodes = tree.query_selector_all("style");

        for style_id in style_nodes {
            // Find text children of the style tag
            let mut css_content = String::new();
            for child in style_id.children(&tree.arena) {
                if let Some(node) = tree.get_node(child) {
                    if let NodeData::Text { content } = &node.data {
                        css_content.push_str(content);
                    }
                }
            }

            if css_content.is_empty() {
                continue;
            }

            let mut input = ParserInput::new(&css_content);
            let mut parser = Parser::new(&mut input);

            // Simple CSS parser for: selector { prop: val; }
            // Real pages would use a much more robust parser,
            // but this satisfies Bug 7 for the MVP.
            while !parser.is_exhausted() {
                let selector = match parser.next() {
                    Ok(Token::Ident(ref s)) => s.to_string(),
                    Ok(Token::Delim('.')) => {
                        if let Ok(Token::Ident(ref s)) = parser.next() {
                            format!(".{}", s)
                        } else {
                            continue;
                        }
                    }
                    Ok(Token::Delim('#')) => {
                        if let Ok(Token::Ident(ref s)) = parser.next() {
                            format!("#{}", s)
                        } else {
                            continue;
                        }
                    }
                    _ => {
                        let _ = parser.next();
                        continue;
                    }
                };

                if matches!(parser.next(), Ok(Token::CurlyBracketBlock)) {
                    // We need to parse inside the block.
                    // This is simplified for Bug 7.
                    parser
                        .parse_nested_block(|p| {
                            loop {
                                let next = p.next();
                                match next {
                                    Ok(Token::Ident(ref name)) => {
                                        let prop_name = name.to_string();
                                        if let Ok(Token::Colon) = p.next() {
                                            let mut value_str = String::new();
                                            while let Ok(val_tok) = p.next() {
                                                if matches!(val_tok, Token::Semicolon) {
                                                    break;
                                                }
                                                if !value_str.is_empty() {
                                                    value_str.push(' ');
                                                }
                                                value_str.push_str(&val_tok.to_css_string());
                                            }
                                            rules.push(StylesheetRule {
                                                selector: selector.clone(),
                                                property: prop_name,
                                                value: value_str,
                                            });
                                        }
                                    }
                                    Err(_) => break,
                                    _ => {}
                                }
                            }
                            Ok::<(), cssparser::ParseError<'_, ()>>(())
                        })
                        .unwrap_or(());
                }
            }
        }
        rules
    }
}

fn parse_display(value: &str) -> Option<Display> {
    match value.trim().to_lowercase().as_str() {
        "none" => Some(Display::None),
        "block" => Some(Display::Block),
        "inline" => Some(Display::Inline),
        "flex" => Some(Display::Flex),
        "grid" => Some(Display::Grid),
        "inline-block" => Some(Display::InlineBlock),
        "table" => Some(Display::Table),
        _ => None,
    }
}

fn parse_visibility(value: &str) -> Option<Visibility> {
    match value.trim().to_lowercase().as_str() {
        "visible" => Some(Visibility::Visible),
        "hidden" => Some(Visibility::Hidden),
        "collapse" => Some(Visibility::Collapse),
        _ => None,
    }
}

fn parse_opacity(value: &str) -> Option<f32> {
    value.trim().parse::<f32>().ok().map(|v| v.clamp(0.0, 1.0))
}

fn parse_position(value: &str) -> Option<Position> {
    match value.trim().to_lowercase().as_str() {
        "static" => Some(Position::Static),
        "relative" => Some(Position::Relative),
        "absolute" => Some(Position::Absolute),
        "fixed" => Some(Position::Fixed),
        "sticky" => Some(Position::Sticky),
        _ => None,
    }
}

fn parse_z_index(value: &str) -> Option<i32> {
    value.trim().parse::<i32>().ok()
}

fn parse_pointer_events(value: &str) -> Option<PointerEvents> {
    match value.trim().to_lowercase().as_str() {
        "auto" => Some(PointerEvents::Auto),
        "none" => Some(PointerEvents::None),
        _ => None,
    }
}
