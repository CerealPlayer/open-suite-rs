use docx_rs::{Document, DocumentChild, Docx, Paragraph, ParagraphChild, Run, RunChild, read_docx};
use serde::Serialize;
use serde_json::{Map, Value, json};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    attrs: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProseMirrorMark {
    #[serde(rename = "type")]
    mark_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    attrs: Option<Map<String, Value>>,
}

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

fn paragraph_to_node(paragraph: &Paragraph) -> ProseMirrorNode {
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

fn paragraph_children_to_nodes(
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

fn run_to_nodes(run: &Run, inherited_marks: &[ProseMirrorMark]) -> Vec<ProseMirrorNode> {
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

fn run_marks(run: &Run) -> Vec<ProseMirrorMark> {
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

fn run_child_text(run_child: &RunChild) -> Option<String> {
    match run_child {
        RunChild::Text(text) => Some(text.text.clone()),
        RunChild::Tab(_) => Some("\t".to_owned()),
        RunChild::Break(_) => Some("\n".to_owned()),
        RunChild::InstrTextString(value) => Some(value.clone()),
        _ => None,
    }
}

fn paragraph_list_kind(paragraph: &Paragraph) -> Option<&'static str> {
    let snapshot = format!("{paragraph:?}");
    let lowered = snapshot.to_ascii_lowercase();
    let is_numbered = lowered.contains("numbering_property: some(")
        || lowered.contains("listnumber")
        || lowered.contains("list_number");
    let is_bulleted = lowered.contains("listbullet")
        || lowered.contains("list_bullet")
        || lowered.contains("bullet");

    if is_numbered || is_bulleted {
        return Some(if is_bulleted && !is_numbered {
            "bullet_list"
        } else {
            "ordered_list"
        });
    }

    None
}

fn paragraph_heading_level(paragraph: &Paragraph) -> Option<u8> {
    let snapshot = format!("{paragraph:?}");
    let lowered = snapshot.to_ascii_lowercase();

    for level in 1..=6 {
        let marker = format!("heading{level}");
        if lowered.contains(&marker) {
            return Some(level);
        }
    }

    find_outline_level(&snapshot).map(|value| value.clamp(1, 6) as u8)
}

fn find_outline_level(snapshot: &str) -> Option<usize> {
    let marker = "outline_lvl";
    let start = snapshot.find(marker)?;
    let tail = &snapshot[start..];
    let first_digit = tail.find(|ch: char| ch.is_ascii_digit())?;
    let digits: String = tail[first_digit..]
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

fn consume_list_block(
    children: &[DocumentChild],
    list_kind: &'static str,
) -> (ProseMirrorNode, usize) {
    let mut consumed = 0usize;
    let mut list_items = Vec::new();

    for child in children {
        let DocumentChild::Paragraph(paragraph) = child else {
            break;
        };

        if paragraph_list_kind(paragraph) != Some(list_kind) {
            break;
        }

        list_items.push(ProseMirrorNode {
            node_type: "list_item",
            text: None,
            content: Some(vec![paragraph_node(paragraph_children_to_nodes(
                paragraph.children(),
                &[],
            ))]),
            marks: None,
            attrs: None,
        });
        consumed += 1;
    }

    if list_items.is_empty() {
        return (
            ProseMirrorNode {
                node_type: "ordered_list",
                text: None,
                content: Some(vec![ProseMirrorNode {
                    node_type: "list_item",
                    text: None,
                    content: Some(vec![unsupported_block_node("DocumentChild", &children[0])]),
                    marks: None,
                    attrs: None,
                }]),
                marks: None,
                attrs: None,
            },
            1,
        );
    }

    (
        ProseMirrorNode {
            node_type: list_kind,
            text: None,
            content: Some(list_items),
            marks: None,
            attrs: None,
        },
        consumed,
    )
}

fn paragraph_node(content: Vec<ProseMirrorNode>) -> ProseMirrorNode {
    ProseMirrorNode {
        node_type: "paragraph",
        text: None,
        content: Some(content),
        marks: None,
        attrs: None,
    }
}

fn text_node(text: String, marks: Vec<ProseMirrorMark>) -> ProseMirrorNode {
    ProseMirrorNode {
        node_type: "text",
        text: Some(text),
        content: None,
        marks: if marks.is_empty() { None } else { Some(marks) },
        attrs: None,
    }
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

fn map_attrs(pairs: Vec<(&'static str, Value)>) -> Map<String, Value> {
    pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
}

fn link_mark(link: &docx_rs::Hyperlink) -> ProseMirrorMark {
    let mut attrs = Map::new();
    attrs.insert("href".to_owned(), json!(extract_hyperlink_target(link)));
    ProseMirrorMark {
        mark_type: "link",
        attrs: Some(attrs),
    }
}

fn extract_hyperlink_target(link: &docx_rs::Hyperlink) -> String {
    let snapshot = format!("{:?}", link.link);
    if let Some(url) = first_quoted_segment(&snapshot) {
        return url;
    }
    snapshot
}

fn first_quoted_segment(value: &str) -> Option<String> {
    let start = value.find('"')?;
    let rest = &value[start + 1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}

fn deduplicate_marks(marks: Vec<ProseMirrorMark>) -> Vec<ProseMirrorMark> {
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

fn unsupported_text_node<T: fmt::Debug>(kind: &'static str, value: &T) -> ProseMirrorNode {
    text_node(format_unsupported(kind, value), Vec::new())
}

fn unsupported_block_node<T: fmt::Debug>(kind: &'static str, value: &T) -> ProseMirrorNode {
    paragraph_node(vec![unsupported_text_node(kind, value)])
}

fn format_unsupported<T: fmt::Debug>(kind: &'static str, value: &T) -> String {
    format!(
        "[Unsupported {kind}: {} | properties: {:?}]",
        std::any::type_name::<T>(),
        value
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use docx_rs::{Hyperlink, HyperlinkType};

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
