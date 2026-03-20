use std::fmt;

use docx_rs::{Document, DocumentChild, Docx, read_docx};
use serde::Serialize;

use marks::ProseMirrorMark;
use serde_json::{Map, Value};

use crate::prosemirror::{
    helpers::unsupported_block_node,
    list::{consume_list_block, paragraph_list_kind},
    paragraph::paragraph_to_node,
};

mod heading;
mod helpers;
mod link;
mod list;
mod marks;
mod paragraph;
mod run;
mod text;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    attrs: Option<Map<String, Value>>,
}

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

pub fn parse_docx_to_prosemirror(bytes: &[u8]) -> Result<(ProseMirrorDoc, Docx), ParseError> {
    let docx = read_docx(bytes).map_err(ParseError::Read)?;
    Ok((from_document(&docx.document), docx))
}

fn from_document(document: &Document) -> ProseMirrorDoc {
    let mut content = Vec::new();
    let mut index = 0;

    while index < document.children.len() {
        match &document.children[index] {
            DocumentChild::Paragraph(paragraph) => {
                if let Some(list_kind) = paragraph_list_kind(paragraph) {
                    let (list_node, consumed) =
                        consume_list_block(&document.children[index..], list_kind);
                    content.push(list_node);
                    index += consumed;
                    continue;
                }

                content.push(paragraph_to_node(paragraph));
            }
            unsupported => content.push(unsupported_block_node("DocumentChild", unsupported)),
        }

        index += 1;
    }

    ProseMirrorDoc {
        node_type: "doc",
        content,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use docx_rs::{Hyperlink, HyperlinkType, Paragraph, Run};

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
    fn maps_unsupported_document_children_to_diagnostic_text() {
        let document = Document::new().add_table_of_contents(docx_rs::TableOfContents::new());
        let mapped = from_document(&document);
        assert_eq!(mapped.content.len(), 1);
        let paragraph_content = mapped.content[0]
            .content
            .as_ref()
            .expect("paragraph content");
        assert_eq!(paragraph_content.len(), 1);
        let text = paragraph_content[0]
            .text
            .as_deref()
            .expect("diagnostic text");
        assert!(text.contains("Unsupported DocumentChild"));
    }

    #[test]
    fn maps_heading_by_style() {
        let document = Document::new().add_paragraph(
            Paragraph::new()
                .style("Heading1")
                .add_run(Run::new().add_text("Title")),
        );

        let mapped = from_document(&document);
        assert_eq!(mapped.content.len(), 1);
        assert_eq!(mapped.content[0].node_type, "heading");
        assert_eq!(
            mapped.content[0]
                .attrs
                .as_ref()
                .and_then(|attrs| attrs.get("level"))
                .and_then(Value::as_u64),
            Some(1)
        );
    }

    #[test]
    fn maps_list_paragraph_styles_to_list_nodes() {
        let document = Document::new()
            .add_paragraph(
                Paragraph::new()
                    .style("ListBullet")
                    .add_run(Run::new().add_text("One")),
            )
            .add_paragraph(
                Paragraph::new()
                    .style("ListBullet")
                    .add_run(Run::new().add_text("Two")),
            );

        let mapped = from_document(&document);
        assert_eq!(mapped.content.len(), 1);
        assert_eq!(mapped.content[0].node_type, "bullet_list");
        let items = mapped.content[0].content.as_ref().expect("list content");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].node_type, "list_item");
    }

    #[test]
    fn maps_hyperlink_to_link_mark() {
        let paragraph = Paragraph::new().add_hyperlink(
            Hyperlink::new("https://example.com", HyperlinkType::External)
                .add_run(Run::new().add_text("site")),
        );
        let document = Document::new().add_paragraph(paragraph);
        let mapped = from_document(&document);
        let paragraph_content = mapped.content[0]
            .content
            .as_ref()
            .expect("paragraph content");
        let marks = paragraph_content[0].marks.as_ref().expect("marks");
        assert_eq!(marks[0].mark_type, "link");
        assert_eq!(
            marks[0]
                .attrs
                .as_ref()
                .and_then(|attrs| attrs.get("href"))
                .and_then(Value::as_str),
            Some("rIdHyperlink1")
        );
    }
}
