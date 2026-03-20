use serde_json::{Map, json};

use crate::prosemirror::marks::ProseMirrorMark;

pub fn link_mark(link: &docx_rs::Hyperlink) -> ProseMirrorMark {
    let mut attrs = Map::new();
    attrs.insert("href".to_owned(), json!(extract_hyperlink_target(link)));
    ProseMirrorMark {
        mark_type: "link",
        attrs: Some(attrs),
    }
}

pub fn extract_hyperlink_target(link: &docx_rs::Hyperlink) -> String {
    let snapshot = format!("{:?}", link.link);
    if let Some(url) = first_quoted_segment(&snapshot) {
        return url;
    }
    snapshot
}

pub fn first_quoted_segment(value: &str) -> Option<String> {
    let start = value.find('"')?;
    let rest = &value[start + 1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}
