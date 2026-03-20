use std::fmt;

use crate::prosemirror::{ProseMirrorNode, helpers::format_unsupported, marks::ProseMirrorMark};

pub fn text_node(text: String, marks: Vec<ProseMirrorMark>) -> ProseMirrorNode {
    ProseMirrorNode {
        node_type: "text",
        text: Some(text),
        content: None,
        marks: if marks.is_empty() { None } else { Some(marks) },
        attrs: None,
    }
}

pub fn unsupported_text_node<T: fmt::Debug>(kind: &'static str, value: &T) -> ProseMirrorNode {
    text_node(format_unsupported(kind, value), Vec::new())
}
