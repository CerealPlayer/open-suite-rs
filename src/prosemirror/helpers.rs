use std::fmt;

use serde_json::{Map, Value};

use crate::prosemirror::{ProseMirrorNode, paragraph::paragraph_node, text::unsupported_text_node};

pub fn map_attrs(pairs: Vec<(&'static str, Value)>) -> Map<String, Value> {
    pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
}

pub fn format_unsupported<T: fmt::Debug>(kind: &'static str, value: &T) -> String {
    format!(
        "[Unsupported {kind}: {} | properties: {:?}]",
        std::any::type_name::<T>(),
        value
    )
}

pub fn unsupported_block_node<T: fmt::Debug>(kind: &'static str, value: &T) -> ProseMirrorNode {
    paragraph_node(vec![unsupported_text_node(kind, value)])
}
