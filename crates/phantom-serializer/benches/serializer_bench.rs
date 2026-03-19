use criterion::{black_box, criterion_group, criterion_main, Criterion};
use phantom_core::dom::DomTree;
use phantom_core::layout::ViewportBounds;
use phantom_serializer::HeadlessSerializer;
use std::collections::HashMap;

fn create_large_dom(size: usize) -> (DomTree, HashMap<indextree::NodeId, ViewportBounds>) {
    let mut tree = DomTree::new();
    let root = tree.create_element("div", HashMap::new());
    tree.document_root = Some(root);

    let mut bounds = HashMap::new();
    bounds.insert(
        root,
        ViewportBounds {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        },
    );

    for i in 0..size {
        let child = tree.create_element("div", HashMap::new());
        bounds.insert(
            child,
            ViewportBounds {
                x: (i % 100) as f32,
                y: (i / 100) as f32,
                width: 10.0,
                height: 10.0,
            },
        );
        tree.append_child(root, child);
    }

    (tree, bounds)
}

fn bench_serialization(c: &mut Criterion) {
    let (tree, bounds) = create_large_dom(1000);
    let serializer = HeadlessSerializer::default();

    c.bench_function("serialize 1000 nodes", |b| {
        b.iter(|| serializer.serialize(black_box(&tree), black_box(&bounds)))
    });
}

criterion_group!(benches, bench_serialization);
criterion_main!(benches);
