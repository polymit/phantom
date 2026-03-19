use phantom_core::dom::{ComputedStyle, Display, DomTree, PointerEvents, Position, Visibility};
use phantom_core::layout::ViewportBounds;
use phantom_serializer::{coalesce_mutations, HeadlessSerializer, Mutation};
use std::collections::HashMap;

fn create_test_dom() -> (DomTree, HashMap<indextree::NodeId, ViewportBounds>) {
    let mut tree = DomTree::new();
    let root = tree.create_element("div", HashMap::new());
    tree.document_root = Some(root);

    let mut bounds = HashMap::new();
    bounds.insert(
        root,
        ViewportBounds {
            x: 0.0,
            y: 0.0,
            width: 1000.0,
            height: 1000.0,
        },
    );

    (tree, bounds)
}

#[test]
fn test_cct_format_correct() {
    let (mut tree, mut bounds) = create_test_dom();
    let root = tree.document_root.unwrap();

    let mut attrs = HashMap::new();
    attrs.insert("data-agent-id".to_string(), "login_btn".to_string());
    attrs.insert("onclick".to_string(), "true".to_string());
    let btn = tree.create_element("button", attrs);
    tree.get_node_mut(btn).unwrap().computed_style = ComputedStyle {
        display: Display::Block,
        visibility: Visibility::Visible,
        opacity: 1.0,
        position: Position::Static,
        z_index: None,
        pointer_events: PointerEvents::Auto,
    };
    bounds.insert(
        btn,
        ViewportBounds {
            x: 120.0,
            y: 340.0,
            width: 140.0,
            height: 36.0,
        },
    );
    tree.append_child(root, btn);

    let text = tree.create_text("Submit Form");
    tree.append_child(btn, text);

    let serializer = HeadlessSerializer::default();
    let cct = serializer.serialize(&tree, &bounds);

    println!("CCT Output:\n{}", cct);

    let lines: Vec<&str> = cct.trim().split('\n').collect();
    for line in &lines {
        let parts: Vec<&str> = line.split('|').collect();
        assert!(parts.len() >= 10, "Each line must have at least 10 fields");
        assert!(parts[0].starts_with("n_") || parts[0] == "login_btn" || parts[0] == "root");
        let bounds_parts: Vec<&str> = parts[3].split(',').collect();
        assert_eq!(
            bounds_parts.len(),
            4,
            "Bounds must be 4 comma-separated values"
        );
    }

    // The button's parent is the root div (n_0), not the document root itself
    assert!(cct.contains("login_btn|btn|btn|120,340,140,36|b,v,1.0,a|-|Submit Form|c|n_0|0"));
}

#[test]
fn test_hidden_elements_excluded() {
    let (mut tree, mut bounds) = create_test_dom();
    let root = tree.document_root.unwrap();

    let btn1 = tree.create_element("button", HashMap::new());
    bounds.insert(
        btn1,
        ViewportBounds {
            x: 10.0,
            y: 10.0,
            width: 100.0,
            height: 20.0,
        },
    );
    tree.get_node_mut(btn1).unwrap().computed_style.display = Display::None;
    tree.append_child(root, btn1);

    let btn2 = tree.create_element("button", HashMap::new());
    bounds.insert(
        btn2,
        ViewportBounds {
            x: 10.0,
            y: 40.0,
            width: 100.0,
            height: 20.0,
        },
    );
    tree.get_node_mut(btn2).unwrap().computed_style.visibility = Visibility::Hidden;
    tree.append_child(root, btn2);

    let btn3 = tree.create_element("button", HashMap::new());
    bounds.insert(
        btn3,
        ViewportBounds {
            x: 10.0,
            y: 70.0,
            width: 100.0,
            height: 20.0,
        },
    );
    tree.get_node_mut(btn3).unwrap().computed_style.opacity = 0.0;
    tree.append_child(root, btn3);

    let serializer = HeadlessSerializer::default();
    let cct = serializer.serialize(&tree, &bounds);

    println!("Hidden check output:\n{}", cct);
    assert_eq!(cct.trim().lines().count(), 1);
}

#[test]
fn test_delta_coalescing() {
    let mut tree = DomTree::new();
    let node_id = tree.create_element("div", HashMap::new());

    let m1 = Mutation::AttrChanged {
        node_id,
        attr: "class".to_string(),
        old: None,
        new: Some("a".to_string()),
    };
    let m2 = Mutation::AttrChanged {
        node_id,
        attr: "class".to_string(),
        old: Some("a".to_string()),
        new: Some("b".to_string()),
    };

    let coalesced = coalesce_mutations(vec![m1, m2]);
    assert_eq!(coalesced.len(), 1);
}

#[test]
fn test_accessible_name_priority() {
    let (mut tree, mut bounds) = create_test_dom();
    let root = tree.document_root.unwrap();

    let mut attrs = HashMap::new();
    attrs.insert("aria-label".to_string(), "Primary Label".to_string());
    attrs.insert("title".to_string(), "Secondary Title".to_string());

    let div = tree.create_element("div", attrs);
    bounds.insert(
        div,
        ViewportBounds {
            x: 10.0,
            y: 10.0,
            width: 100.0,
            height: 20.0,
        },
    );
    tree.append_child(root, div);

    let serializer = HeadlessSerializer::default();
    let cct = serializer.serialize(&tree, &bounds);

    assert!(cct.contains("Primary Label"));
    assert!(!cct.contains("Secondary Title"));
}
