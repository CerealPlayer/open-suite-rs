use docx_rs::Run;
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::prosemirror::helpers::map_attrs;

#[derive(Debug, Clone, Serialize)]
pub struct ProseMirrorMark {
    #[serde(rename = "type")]
    pub mark_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attrs: Option<Map<String, Value>>,
}

pub fn run_marks(run: &Run) -> Vec<ProseMirrorMark> {
    let mut marks = Vec::new();

    if run.run_property.bold.is_some() {
        marks.push(mark("bold"));
    }
    if run.run_property.italic.is_some() {
        marks.push(mark("italic"));
    }
    if run.run_property.underline.is_some() {
        marks.push(mark("underline"));
    }
    if run.run_property.strike.is_some() || run.run_property.dstrike.is_some() {
        marks.push(mark("strike"));
    }
    if run.run_property.highlight.is_some() {
        marks.push(mark_with_attrs(
            "highlight",
            vec![("source", json!("docx_highlight"))],
        ));
    }
    if run.run_property.color.is_some() {
        marks.push(mark_with_attrs(
            "text_color",
            vec![("source", json!("docx_color"))],
        ));
    }
    if run.run_property.vert_align.is_some() {
        let vert_align = format!("{:?}", run.run_property.vert_align);
        let lowered = vert_align.to_ascii_lowercase();
        if lowered.contains("superscript") {
            marks.push(mark("superscript"));
        } else if lowered.contains("subscript") {
            marks.push(mark("subscript"));
        } else {
            marks.push(mark_with_attrs(
                "text_vertical_align",
                vec![("value", json!(vert_align))],
            ));
        }
    }

    marks
}

fn mark(mark_type: &'static str) -> ProseMirrorMark {
    ProseMirrorMark {
        mark_type,
        attrs: None,
    }
}

fn mark_with_attrs(mark_type: &'static str, pairs: Vec<(&'static str, Value)>) -> ProseMirrorMark {
    ProseMirrorMark {
        mark_type,
        attrs: Some(map_attrs(pairs)),
    }
}

pub fn deduplicate_marks(marks: Vec<ProseMirrorMark>) -> Vec<ProseMirrorMark> {
    let mut deduped: Vec<ProseMirrorMark> = Vec::new();

    for mark in marks {
        let is_duplicate = deduped
            .iter()
            .any(|existing| existing.mark_type == mark.mark_type && existing.attrs == mark.attrs);
        if !is_duplicate {
            deduped.push(mark);
        }
    }

    deduped
}
