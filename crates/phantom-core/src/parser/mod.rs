use crate::dom::{DomNode, DomTree, NodeData};
use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::{Attribute, ExpandedName, QualName};
use indextree::NodeId;
use parking_lot::RwLock;
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SinkHandle {
    pub id: NodeId,
    pub name: Option<QualName>,
}

pub struct DomSink {
    pub tree: RwLock<DomTree>,
    pub errors: RwLock<Vec<String>>,
    document_node: NodeId,
}

impl DomSink {
    pub fn new() -> Self {
        let mut tree = DomTree::new();
        let doc = tree.arena.new_node(DomNode::new_document());
        tree.document_root = Some(doc);
        Self {
            tree: RwLock::new(tree),
            errors: RwLock::new(Vec::new()),
            document_node: doc,
        }
    }

    pub fn finish(self) -> DomTree {
        self.tree.into_inner()
    }
}

impl Default for DomSink {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeSink for DomSink {
    type Handle = SinkHandle;
    type Output = Self;
    type ElemName<'a> = ExpandedName<'a>;

    fn finish(self) -> Self::Output {
        self
    }

    fn parse_error(&self, msg: Cow<'static, str>) {
        self.errors.write().push(msg.into_owned());
    }

    fn get_document(&self) -> Self::Handle {
        SinkHandle {
            id: self.document_node,
            name: None,
        }
    }

    fn elem_name<'a>(&'a self, target: &'a Self::Handle) -> ExpandedName<'a> {
        target
            .name
            .as_ref()
            .expect("elem_name called on non-element")
            .expanded()
    }

    fn create_element(
        &self,
        name: QualName,
        attrs: Vec<Attribute>,
        _flags: ElementFlags,
    ) -> Self::Handle {
        let mut attr_map = HashMap::new();
        for attr in attrs {
            attr_map.insert(attr.name.local.to_string(), attr.value.to_string());
        }
        let id = self
            .tree
            .write()
            .create_element(name.local.as_ref(), attr_map);
        SinkHandle {
            id,
            name: Some(name),
        }
    }

    fn create_comment(&self, text: StrTendril) -> Self::Handle {
        let id = self.tree.write().create_comment(&text);
        SinkHandle { id, name: None }
    }

    fn create_pi(&self, _target: StrTendril, _data: StrTendril) -> Self::Handle {
        let id = self.tree.write().create_comment("processing instruction");
        SinkHandle { id, name: None }
    }

    fn append(&self, parent: &Self::Handle, child: NodeOrText<Self::Handle>) {
        let mut tree = self.tree.write();
        match child {
            NodeOrText::AppendNode(node) => {
                tree.append_child(parent.id, node.id);
            }
            NodeOrText::AppendText(text) => {
                let id = tree.create_text(&text);
                tree.append_child(parent.id, id);
            }
        }
    }

    fn append_before_sibling(&self, sibling: &Self::Handle, new_node: NodeOrText<Self::Handle>) {
        let mut tree = self.tree.write();
        match new_node {
            NodeOrText::AppendNode(node) => {
                tree.insert_before(sibling.id, node.id);
            }
            NodeOrText::AppendText(text) => {
                let id = tree.create_text(&text);
                tree.insert_before(sibling.id, id);
            }
        }
    }

    fn append_based_on_parent_node(
        &self,
        element: &Self::Handle,
        _prev_element: &Self::Handle,
        child: NodeOrText<Self::Handle>,
    ) {
        let tree = self.tree.read();
        let has_parent = tree.get_node(element.id).is_some()
            && element.id.ancestors(&tree.arena).nth(1).is_some();
        drop(tree); // drop read lock before calling methods that take write lock

        if has_parent {
            self.append_before_sibling(element, child);
        } else {
            self.append(element, child);
        }
    }

    fn append_doctype_to_document(
        &self,
        name: StrTendril,
        _public_id: StrTendril,
        _system_id: StrTendril,
    ) {
        let mut tree = self.tree.write();
        let mut node = DomNode::new_text("");
        node.data = NodeData::Doctype {
            name: name.to_string(),
        };
        let id = tree.arena.new_node(node);
        tree.append_child(self.document_node, id);
    }

    fn get_template_contents(&self, target: &Self::Handle) -> Self::Handle {
        target.clone()
    }

    fn same_node(&self, x: &Self::Handle, y: &Self::Handle) -> bool {
        x.id == y.id
    }

    fn set_quirks_mode(&self, _mode: QuirksMode) {}

    fn add_attrs_if_missing(&self, target: &Self::Handle, attrs: Vec<Attribute>) {
        let mut tree = self.tree.write();
        if let Some(node) = tree.get_node_mut(target.id) {
            if let NodeData::Element {
                ref mut attributes, ..
            } = node.data
            {
                for attr in attrs {
                    let key = attr.name.local.to_string();
                    if let std::collections::hash_map::Entry::Vacant(e) = attributes.entry(key) {
                        e.insert(attr.value.to_string());
                    }
                }
            }
        }
    }

    fn remove_from_parent(&self, target: &Self::Handle) {
        self.tree.write().remove_node(target.id);
    }

    fn reparent_children(&self, node: &Self::Handle, new_parent: &Self::Handle) {
        let mut tree = self.tree.write();
        let children: Vec<NodeId> = node.id.children(&tree.arena).collect();
        for child in children {
            tree.append_child(new_parent.id, child);
        }
    }
}

#[tracing::instrument]
pub fn parse_html(html: &str) -> DomTree {
    parse_html_bytes(html.as_bytes())
}

#[tracing::instrument(skip(bytes))]
pub fn parse_html_bytes(bytes: &[u8]) -> DomTree {
    use html5ever::parse_document;
    let sink = DomSink::new();
    let parser = parse_document(sink, Default::default());
    let result = parser
        .from_utf8()
        .read_from(&mut std::io::Cursor::new(bytes))
        .unwrap();
    result.finish()
}
