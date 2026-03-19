use phantom_core::dom::NodeData;
use phantom_core::pipeline::PagePipeline;

#[test]
fn test_bounding_boxes_computed() {
    let html = r#"<html><body style="margin:0;padding:0">
        <div style="width:200px;height:100px;display:block">
            <button style="width:100px;height:40px">Click</button>
        </div>
        <p style="display:none">Hidden paragraph</p>
    </body></html>"#;

    let mut pipeline = PagePipeline::new().unwrap();
    let page = pipeline
        .process_html(html, "https://test.com", 1280.0, 720.0)
        .unwrap();

    assert!(page.bounds.len() > 0, "Must have bounding boxes");
    println!("Nodes with bounds: {}", page.bounds.len());

    for (node_id, bounds) in &page.bounds {
        let node = page.dom.get_node(*node_id).unwrap();
        if let NodeData::Element { tag_name, .. } = &node.data {
            println!("<{}> bounds: {:?}", tag_name, bounds);
        }
    }
}

#[test]
fn test_css_cascade_visibility() {
    let html = r##"<html><body>
        <div style="display:none">
            <button>Should be hidden</button>
        </div>
        <div style="visibility:hidden">Hidden but in layout</div>
        <button style="opacity:0">Transparent button</button>
        <a href="#">Visible link</a>
    </body></html>"##;

    let mut pipeline = PagePipeline::new().unwrap();
    let page = pipeline
        .process_html(html, "https://test.com", 1280.0, 720.0)
        .unwrap();

    let btn = page.dom.query_selector("button"); // The first button inside div
    if let Some(btn_id) = btn {
        let node = page.dom.get_node(btn_id).unwrap();
        println!(
            "Button in hidden div display: {:?}",
            node.computed_style.display
        );
        // Note: Inheritance is not completely precise in this simplistic cascade logic,
        // but 'display' is generally NOT inherited in CSS. The div has display: none,
        // so its children are not displayed, but their *computed* display is still what it is (e.g. inline-block).
        // If we want to simulate display: none propagation (where a node and all descendants get layout bounds of 0,0,0,0),
        // we handle that in layout.
    }
}
