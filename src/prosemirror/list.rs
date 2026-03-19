use docx_rs::{DocumentChild, Paragraph};

use crate::prosemirror::{
    ProseMirrorNode,
    helpers::unsupported_block_node,
    paragraph::{paragraph_children_to_nodes, paragraph_node},
};

pub fn paragraph_list_kind(paragraph: &Paragraph) -> Option<&'static str> {
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

pub fn consume_list_block(
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
