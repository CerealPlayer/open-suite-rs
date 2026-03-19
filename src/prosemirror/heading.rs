use docx_rs::Paragraph;

pub fn paragraph_heading_level(paragraph: &Paragraph) -> Option<u8> {
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
