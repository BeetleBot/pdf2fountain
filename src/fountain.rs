use crate::fsm::{ElementType, ScriptBlock, TitlePageInfo};

pub fn format_fountain(title_page: Option<&TitlePageInfo>, blocks: &[ScriptBlock]) -> String {
    let mut out = String::new();

    if let Some(tp) = title_page {
        if let Some(title) = &tp.title {
            out.push_str(&format!("Title: {}\n", title.trim()));
        }
        if let Some(credit) = &tp.credit {
            out.push_str(&format!("Credit: {}\n", credit.trim()));
        }
        if let Some(author) = &tp.author {
            out.push_str(&format!("Author: {}\n", author.trim()));
        }
        if !out.is_empty() {
            out.push('\n');
        }
    }

    let is_dialogue_component = |t: Option<ElementType>| {
        matches!(t, Some(ElementType::Character) | Some(ElementType::Parenthetical) | Some(ElementType::Dialogue))
    };

    let mut prev_type = None;

    for block in blocks {
        match block.element_type {
            ElementType::SceneHeading => {
                if !out.is_empty() && !out.ends_with("\n\n") {
                    out.push('\n');
                }
                let heading = block.text.trim();
                let heading_clean = if let Some(stripped) = heading.strip_prefix('.') {
                    stripped.trim()
                } else {
                    heading
                };
                if let Some(num) = &block.scene_number {
                    out.push_str(&format!(".{} #{num}#\n\n", heading_clean.to_uppercase()));
                } else {
                    out.push_str(&format!(".{}\n\n", heading_clean.to_uppercase()));
                }
            }
            ElementType::Action => {
                if is_dialogue_component(prev_type) && !out.ends_with("\n\n") {
                    out.push('\n');
                }
                let action_text = block.text.trim();
                let formatted_action = if action_text.starts_with('!') {
                    action_text.to_string()
                } else {
                    format!("!{action_text}")
                };
                out.push_str(&format!("{}\n\n", formatted_action));
            }
            ElementType::Character => {
                if !out.is_empty() && !out.ends_with("\n\n") {
                    out.push('\n');
                }
                let char_name = block.text.trim();
                let clean_name = if let Some(stripped) = char_name.strip_prefix('@') {
                    stripped.trim()
                } else {
                    char_name
                };
                out.push_str(&format!("@{}\n", clean_name.to_uppercase()));
            }
            ElementType::Parenthetical => {
                out.push_str(&format!("{}\n", block.text.trim()));
            }
            ElementType::Dialogue => {
                out.push_str(&format!("{}\n", block.text.trim()));
            }
            ElementType::Transition => {
                if !out.is_empty() && !out.ends_with("\n\n") {
                    out.push('\n');
                }
                let trans_text = block.text.trim();
                let formatted_trans = if trans_text.starts_with('>') {
                    trans_text.to_string()
                } else {
                    format!("> {trans_text}")
                };
                out.push_str(&format!("{}\n\n", formatted_trans));
            }
        }
        prev_type = Some(block.element_type);
    }

    out
}
