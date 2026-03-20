use docx_rs::{Paragraph, ParagraphChild};
use serde_json::json;

use crate::prosemirror::{
    ProseMirrorNode,
    heading::paragraph_heading_level,
    helpers::map_attrs,
    link::link_mark,
    marks::ProseMirrorMark,
    run::run_to_nodes,
    text::{text_node, unsupported_text_node},
};

pub fn paragraph_to_node(paragraph: &Paragraph) -> ProseMirrorNode {
    if let Some(level) = paragraph_heading_level(paragraph) {
        return ProseMirrorNode {
            node_type: "heading",
            text: None,
            content: Some(paragraph_children_to_nodes(paragraph.children(), &[])),
            marks: None,
            attrs: Some(map_attrs(vec![("level", json!(level))])),
        };
    }

    paragraph_node(paragraph_children_to_nodes(paragraph.children(), &[]))
}

pub fn paragraph_children_to_nodes(
    children: &[ParagraphChild],
    inherited_marks: &[ProseMirrorMark],
) -> Vec<ProseMirrorNode> {
    let mut content = Vec::new();

    for child in children {
        match child {
            ParagraphChild::Run(run) => content.extend(run_to_nodes(run, inherited_marks)),
            ParagraphChild::Hyperlink(link) => {
                let mut next_marks = inherited_marks.to_vec();
                next_marks.push(link_mark(link));
                content.extend(paragraph_children_to_nodes(&link.children, &next_marks));
            }
            unsupported => content.push(unsupported_text_node("ParagraphChild", unsupported)),
        }
    }

    if content.is_empty() {
        content.push(text_node(String::new(), inherited_marks.to_vec()));
    }

    content
}

pub fn paragraph_node(content: Vec<ProseMirrorNode>) -> ProseMirrorNode {
    ProseMirrorNode {
        node_type: "paragraph",
        text: None,
        content: Some(content),
        marks: None,
        attrs: None,
    }
}
