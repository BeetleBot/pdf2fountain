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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_title_page() {
        let title_info = TitlePageInfo {
            title: Some("MY SCREENPLAY".to_string()),
            credit: Some("Written by".to_string()),
            author: Some("Jane Doe".to_string()),
        };
        let out = format_fountain(Some(&title_info), &[]);
        assert!(out.contains("Title: MY SCREENPLAY"));
        assert!(out.contains("Credit: Written by"));
        assert!(out.contains("Author: Jane Doe"));
    }

    #[test]
    fn test_format_scene_heading_with_and_without_scene_number() {
        let blocks = vec![
            ScriptBlock {
                element_type: ElementType::SceneHeading,
                text: "INT. COFFEE SHOP - DAY".to_string(),
                scene_number: Some("1".to_string()),
            },
            ScriptBlock {
                element_type: ElementType::SceneHeading,
                text: ".EXT. STREET - NIGHT".to_string(),
                scene_number: None,
            },
        ];
        let out = format_fountain(None, &blocks);
        assert!(out.contains(".INT. COFFEE SHOP - DAY #1#"));
        assert!(out.contains(".EXT. STREET - NIGHT\n\n"));
    }

    #[test]
    fn test_format_action_block() {
        let blocks = vec![
            ScriptBlock {
                element_type: ElementType::Action,
                text: "John enters the room.".to_string(),
                scene_number: None,
            },
            ScriptBlock {
                element_type: ElementType::Action,
                text: "!Already exclamation prefixed.".to_string(),
                scene_number: None,
            },
        ];
        let out = format_fountain(None, &blocks);
        assert!(out.contains("!John enters the room.\n\n"));
        assert!(out.contains("!Already exclamation prefixed.\n\n"));
    }

    #[test]
    fn test_format_dialogue_block_and_character() {
        let blocks = vec![
            ScriptBlock {
                element_type: ElementType::Character,
                text: "JOHN".to_string(),
                scene_number: None,
            },
            ScriptBlock {
                element_type: ElementType::Parenthetical,
                text: "(whispering)".to_string(),
                scene_number: None,
            },
            ScriptBlock {
                element_type: ElementType::Dialogue,
                text: "Did you hear that?".to_string(),
                scene_number: None,
            },
        ];
        let out = format_fountain(None, &blocks);
        assert!(out.contains("@JOHN\n(whispering)\nDid you hear that?\n"));
    }

    #[test]
    fn test_format_transition() {
        let blocks = vec![
            ScriptBlock {
                element_type: ElementType::Transition,
                text: "CUT TO:".to_string(),
                scene_number: None,
            },
            ScriptBlock {
                element_type: ElementType::Transition,
                text: "> FADE OUT.".to_string(),
                scene_number: None,
            },
        ];
        let out = format_fountain(None, &blocks);
        assert!(out.contains("> CUT TO:\n\n"));
        assert!(out.contains("> FADE OUT.\n\n"));
    }
}
