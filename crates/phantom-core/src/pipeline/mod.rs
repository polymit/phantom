use crate::css::CascadeEngine;
use crate::dom::DomTree;
use crate::errors::CoreError;
use crate::layout::{LayoutEngine, ViewportBounds};
use crate::parser;
use indextree::NodeId;
use phantom_net::NetworkClient;
use std::collections::HashMap;

pub struct PagePipeline {
    network: NetworkClient,
    cascade: CascadeEngine,
}

pub struct ProcessedPage {
    pub url: String,
    pub dom: DomTree,
    pub bounds: HashMap<NodeId, ViewportBounds>,
    pub viewport: ViewportBounds,
    pub title: Option<String>,
}

impl PagePipeline {
    #[tracing::instrument]
    pub fn new() -> Result<Self, CoreError> {
        Ok(Self {
            network: NetworkClient::new()?,
            cascade: CascadeEngine::new(),
        })
    }

    #[tracing::instrument(skip(self))]
    pub async fn process_url(
        &mut self,
        url: &str,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Result<ProcessedPage, CoreError> {
        let fetch_response = self.network.fetch(url).await?;
        let body = fetch_response.body_as_str().map_err(CoreError::Utf8)?;
        let html_string = body.to_string();
        self.process_html(&html_string, url, viewport_width, viewport_height)
    }

    #[tracing::instrument(skip(self, html))]
    pub fn process_html(
        &mut self,
        html: &str,
        base_url: &str,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Result<ProcessedPage, CoreError> {
        let mut dom = parser::parse_html(html);

        self.cascade.apply_cascade(&mut dom);

        let mut layout = LayoutEngine::new();
        let bounds = layout.build_layout_tree(&dom, viewport_width, viewport_height);

        let title = dom.query_selector("title").and_then(|id| {
            if let Some(_node) = dom.get_node(id) {
                // If the Title element has a text child or we just query for texts
                // Simplified title extraction
                let children = id.children(&dom.arena);
                for child_id in children {
                    if let Some(child_node) = dom.get_node(child_id) {
                        if let crate::dom::NodeData::Text { content } = &child_node.data {
                            return Some(content.clone());
                        }
                    }
                }
            }
            None
        });

        let viewport = ViewportBounds {
            x: 0.0,
            y: 0.0,
            width: viewport_width,
            height: viewport_height,
        };

        Ok(ProcessedPage {
            url: base_url.to_string(),
            dom,
            bounds,
            viewport,
            title,
        })
    }
}
