use docx_rs::{Run, RunChild};

use crate::prosemirror::{
    ProseMirrorNode,
    marks::{ProseMirrorMark, deduplicate_marks, run_marks},
    text::{text_node, unsupported_text_node},
};

pub fn run_to_nodes(run: &Run, inherited_marks: &[ProseMirrorMark]) -> Vec<ProseMirrorNode> {
    let mut marks = inherited_marks.to_vec();
    marks.extend(run_marks(run));
    let marks = deduplicate_marks(marks);
    let mut content = Vec::new();

    for child in &run.children {
        if let Some(text) = run_child_text(child) {
            content.push(text_node(text, marks.clone()));
        } else {
            content.push(unsupported_text_node("RunChild", child));
        }
    }

    if content.is_empty() {
        content.push(unsupported_text_node("Run", run));
    }

    content
}

fn run_child_text(run_child: &RunChild) -> Option<String> {
    match run_child {
        RunChild::Text(text) => Some(text.text.clone()),
        RunChild::Tab(_) => Some("\t".to_owned()),
        RunChild::Break(_) => Some("\n".to_owned()),
        RunChild::InstrTextString(value) => Some(value.clone()),
        _ => None,
    }
}
