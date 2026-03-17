use docx_rs::{Document, DocumentChild, Paragraph, ParagraphChild, Run, RunChild, read_docx};
use serde::Serialize;
use std::fmt;

#[derive(Debug)]
pub enum ParseError {
    Read(docx_rs::ReaderError),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(err) => write!(f, "failed to read DOCX: {err}"),
        }
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone, Serialize)]
pub struct ProseMirrorDoc {
    #[serde(rename = "type")]
    node_type: &'static str,
    content: Vec<ProseMirrorNode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProseMirrorNode {
    #[serde(rename = "type")]
    node_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<Vec<ProseMirrorNode>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    marks: Option<Vec<ProseMirrorMark>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProseMirrorMark {
    #[serde(rename = "type")]
    mark_type: &'static str,
}

pub fn parse_docx_to_prosemirror(bytes: &[u8]) -> Result<ProseMirrorDoc, ParseError> {
    let docx = read_docx(bytes).map_err(ParseError::Read)?;
    Ok(from_document(&docx.document))
}

fn from_document(document: &Document) -> ProseMirrorDoc {
    let mut paragraphs = Vec::new();

    for child in &document.children {
        if let DocumentChild::Paragraph(paragraph) = child {
            paragraphs.push(paragraph_to_node(paragraph));
        }
    }

    ProseMirrorDoc {
        node_type: "doc",
        content: paragraphs,
    }
}

fn paragraph_to_node(paragraph: &Paragraph) -> ProseMirrorNode {
    let mut content = Vec::new();

    for child in paragraph.children() {
        if let ParagraphChild::Run(run) = child {
            content.extend(run_to_nodes(run));
        }
    }

    ProseMirrorNode {
        node_type: "paragraph",
        text: None,
        content: Some(content),
        marks: None,
    }
}

fn run_to_nodes(run: &Run) -> Vec<ProseMirrorNode> {
    println!("new run: {:?}", run);
    let marks = run_marks(run);
    let mut content = Vec::new();

    for child in &run.children {
        if let Some(text) = run_child_text(child) {
            let marks = if marks.is_empty() {
                None
            } else {
                Some(marks.clone())
            };
            content.push(ProseMirrorNode {
                node_type: "text",
                text: Some(text),
                content: None,
                marks,
            });
        }
    }

    content
}

fn run_marks(run: &Run) -> Vec<ProseMirrorMark> {
    let mut marks = Vec::new();

    if run.run_property.bold.is_some() {
        marks.push(ProseMirrorMark { mark_type: "bold" });
    }
    if run.run_property.italic.is_some() {
        marks.push(ProseMirrorMark {
            mark_type: "italic",
        });
    }
    if run.run_property.underline.is_some() {
        marks.push(ProseMirrorMark {
            mark_type: "underline",
        });
    }

    marks
}

fn run_child_text(run_child: &RunChild) -> Option<String> {
    match run_child {
        RunChild::Text(text) => Some(text.text.clone()),
        RunChild::Tab(_) => Some("\t".to_owned()),
        RunChild::Break(_) => Some("\n".to_owned()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_paragraph_run_text_and_marks() {
        let document = Document::new()
            .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Hello").bold().italic()));

        let mapped = from_document(&document);
        assert_eq!(mapped.content.len(), 1);
        assert_eq!(mapped.content[0].node_type, "paragraph");
        let paragraph_content = mapped.content[0]
            .content
            .as_ref()
            .expect("paragraph content");
        assert_eq!(paragraph_content.len(), 1);
        assert_eq!(paragraph_content[0].text.as_deref(), Some("Hello"));
        let marks = paragraph_content[0].marks.as_ref().expect("marks");
        assert_eq!(marks.len(), 2);
        assert_eq!(marks[0].mark_type, "bold");
        assert_eq!(marks[1].mark_type, "italic");
    }

    #[test]
    fn maps_tab_and_break_children() {
        let run = Run::new()
            .add_text("A")
            .add_tab()
            .add_text("B")
            .add_break(docx_rs::BreakType::TextWrapping)
            .add_text("C");
        let document = Document::new().add_paragraph(Paragraph::new().add_run(run));

        let mapped = from_document(&document);
        let paragraph_content = mapped.content[0]
            .content
            .as_ref()
            .expect("paragraph content");

        assert_eq!(paragraph_content[0].text.as_deref(), Some("A"));
        assert_eq!(paragraph_content[1].text.as_deref(), Some("\t"));
        assert_eq!(paragraph_content[2].text.as_deref(), Some("B"));
        assert_eq!(paragraph_content[3].text.as_deref(), Some("\n"));
        assert_eq!(paragraph_content[4].text.as_deref(), Some("C"));
    }

    #[test]
    fn ignores_unsupported_document_children() {
        let document = Document::new().add_table_of_contents(docx_rs::TableOfContents::new());
        let mapped = from_document(&document);
        assert!(mapped.content.is_empty());
    }
}
