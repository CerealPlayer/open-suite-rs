use docx_rs::read_docx;
use serde::Serialize;
use serde_json::Value;
use std::fmt;

#[derive(Debug)]
pub enum ParseError {
    Read(docx_rs::ReaderError),
    Serialize(serde_json::Error),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(err) => write!(f, "failed to read DOCX: {err}"),
            Self::Serialize(err) => write!(f, "failed to serialize parsed DOCX: {err}"),
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
    let document_json = serde_json::to_value(&docx.document).map_err(ParseError::Serialize)?;
    Ok(from_document_json(&document_json))
}

fn from_document_json(document_json: &Value) -> ProseMirrorDoc {
    let mut paragraphs = Vec::new();

    if let Some(children) = document_json.get("children").and_then(Value::as_array) {
        for child in children {
            if child
                .get("type")
                .and_then(Value::as_str)
                .is_some_and(|value| value == "paragraph")
            {
                let mut paragraph_content = Vec::new();
                collect_runs(child.get("data"), &mut paragraph_content);
                paragraphs.push(ProseMirrorNode {
                    node_type: "paragraph",
                    text: None,
                    content: Some(paragraph_content),
                    marks: None,
                });
            }
        }
    }

    ProseMirrorDoc {
        node_type: "doc",
        content: paragraphs,
    }
}

fn collect_runs(value: Option<&Value>, out: &mut Vec<ProseMirrorNode>) {
    let Some(value) = value else {
        return;
    };

    match value {
        Value::Array(items) => {
            for item in items {
                collect_runs(Some(item), out);
            }
        }
        Value::Object(object) => {
            if object
                .get("type")
                .and_then(Value::as_str)
                .is_some_and(|value| value == "run")
            {
                if let Some(data) = object.get("data") {
                    out.extend(run_to_text_nodes(data));
                }
            } else {
                collect_runs(object.get("data"), out);
                collect_runs(object.get("children"), out);
            }
        }
        _ => {}
    }
}

fn run_to_text_nodes(run_data: &Value) -> Vec<ProseMirrorNode> {
    let marks = run_marks(run_data);
    let mut content = Vec::new();

    if let Some(children) = run_data.get("children").and_then(Value::as_array) {
        for child in children {
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
    }

    content
}

fn run_marks(run_data: &Value) -> Vec<ProseMirrorMark> {
    let mut marks = Vec::new();
    let Some(run_property) = run_data.get("runProperty") else {
        return marks;
    };

    if run_property.get("bold").is_some() {
        marks.push(ProseMirrorMark { mark_type: "bold" });
    }
    if run_property.get("italic").is_some() {
        marks.push(ProseMirrorMark {
            mark_type: "italic",
        });
    }
    if run_property.get("underline").is_some() {
        marks.push(ProseMirrorMark {
            mark_type: "underline",
        });
    }

    marks
}

fn run_child_text(run_child: &Value) -> Option<String> {
    let kind = run_child.get("type").and_then(Value::as_str)?;
    let data = run_child.get("data");

    match kind {
        "text" => {
            if let Some(text) = data.and_then(Value::as_str) {
                return Some(text.to_owned());
            }
            let maybe_text = data
                .and_then(Value::as_object)
                .and_then(|value| value.get("text"))
                .and_then(Value::as_str);
            maybe_text.map(ToOwned::to_owned)
        }
        "tab" => Some("\t".to_owned()),
        "break" => Some("\n".to_owned()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maps_paragraph_run_text_and_marks() {
        let document = json!({
            "children": [
                {
                    "type": "paragraph",
                    "data": {
                        "children": [
                            {
                                "type": "run",
                                "data": {
                                    "runProperty": {
                                        "bold": {},
                                        "italic": {}
                                    },
                                    "children": [
                                        { "type": "text", "data": { "text": "Hello" } }
                                    ]
                                }
                            }
                        ]
                    }
                }
            ]
        });

        let mapped = from_document_json(&document);
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
        let document = json!({
            "children": [
                {
                    "type": "paragraph",
                    "data": {
                        "children": [
                            {
                                "type": "run",
                                "data": {
                                    "children": [
                                        { "type": "text", "data": { "text": "A" } },
                                        { "type": "tab", "data": {} },
                                        { "type": "text", "data": { "text": "B" } },
                                        { "type": "break", "data": {} },
                                        { "type": "text", "data": { "text": "C" } }
                                    ]
                                }
                            }
                        ]
                    }
                }
            ]
        });

        let mapped = from_document_json(&document);
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
}
