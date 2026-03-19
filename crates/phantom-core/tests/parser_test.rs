use phantom_core::parser::parse_html;

#[test]
fn test_parse_simple_html() {
    let html = r#"<!DOCTYPE html>
    <html>
      <head><title>Test</title></head>
      <body>
        <h1 class="title">Hello World</h1>
        <p>Paragraph text</p>
        <a href="https://example.com">Link</a>
        <button disabled>Submit</button>
      </body>
    </html>"#;

    let tree = parse_html(html);
    assert!(tree.node_count() > 0, "Must parse at least 1 node");
    assert!(tree.document_root.is_some(), "Must have document root");
    println!("Parsed {} nodes successfully", tree.node_count());
}

#[test]
fn test_element_attributes() {
    let html = r#"<html><body>
        <input type="email" aria-label="Email" required placeholder="user@example.com">
    </body></html>"#;

    let tree = parse_html(html);
    let input = tree.query_selector("input").expect("input must exist");
    let node = tree.get_node(input).unwrap();
    assert_eq!(node.get_attr("type"), Some("email"));
    assert_eq!(node.get_attr("aria-label"), Some("Email"));
}

#[test]
fn test_nested_structure() {
    let html = r#"<html><body>
        <div id="parent">
            <div id="child1"><span>Text</span></div>
            <div id="child2"><button>Click</button></div>
        </div>
    </body></html>"#;

    let tree = parse_html(html);
    assert!(tree.node_count() >= 8, "Must have all nested nodes");
}
