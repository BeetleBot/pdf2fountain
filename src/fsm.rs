use crate::calibrate::LayoutProfile;
use crate::geom::PageData;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ElementType {
    SceneHeading,
    Action,
    Character,
    Parenthetical,
    Dialogue,
    Transition,
}

#[derive(Debug, Clone)]
pub struct ScriptBlock {
    pub element_type: ElementType,
    pub text: String,
    pub scene_number: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TitlePageInfo {
    pub title: Option<String>,
    pub author: Option<String>,
    pub credit: Option<String>,
}

pub fn parse_screenplay(pages: &[PageData], profile: &LayoutProfile) -> (Option<TitlePageInfo>, Vec<ScriptBlock>) {
    let mut blocks = Vec::new();
    let mut title_page = None;
    let mut current_state = ElementType::Action;

    let mut start_page = 0;
    if !pages.is_empty() && crate::calibrate::is_likely_title_page(&pages[0]) {
        title_page = Some(extract_title_page(&pages[0]));
        start_page = 1;
    }

    let mut last_action_y: Option<f32> = None;

    for page in &pages[start_page..] {
        if page.lines.is_empty() {
            continue;
        }

        let mut i = 0;
        while i < page.lines.len() {
            let item = &page.lines[i];

            let norm_x = item.x / page.width;

            // skip running page header numbers at the top right of the page (e.g. "2.", "10.")
            if (page.height - item.y) < 75.0 && norm_x > 0.70 && is_running_page_number(&item.text) {
                i += 1;
                continue;
            }

            // check scene number on left margin (e.g. "10" or "57A" followed by "INT. ...")
            if is_scene_number_token(&item.text) && norm_x < profile.action_x - 0.02 {
                if i + 1 < page.lines.len() {
                    let next_item = &page.lines[i + 1];
                    if (next_item.y - item.y).abs() < 5.0 && is_scene_heading(&next_item.text) {
                        let sc_num = item.text.trim_matches(|c| c == '#' || c == '.').to_string();
                        let (clean_heading, inline_sc) = extract_scene_number(&next_item.text);
                        blocks.push(ScriptBlock {
                            element_type: ElementType::SceneHeading,
                            text: clean_heading,
                            scene_number: inline_sc.or(Some(sc_num)),
                        });
                        current_state = ElementType::Action;
                        last_action_y = None;
                        i += 2;
                        continue;
                    }
                }
            }

            // check inline scene heading
            if is_scene_heading(&item.text) {
                let (mut clean_heading, mut sc_num) = extract_scene_number(&item.text);

                // Check if the next line continues this scene heading
                // e.g. Line 1: "INT. RING MODULE, ENDURANCE -"
                //      Line 2: "MOMENTS LATER #216#"
                // or   Line 1: "INT. COCKPIT, LANDER - CONTINUOUS"
                //      Line 2: "#202#"
                while i + 1 < page.lines.len() {
                    let next_item = &page.lines[i + 1];
                    let current_ref_item = &page.lines[i];
                    let y_gap = current_ref_item.y - next_item.y;
                    let next_norm_x = next_item.x / page.width;

                    if y_gap > 6.0 && y_gap < 20.0 {
                        let next_trimmed = next_item.text.trim();
                        if is_scene_number_token(next_trimmed) && next_norm_x < 0.95 {
                            if sc_num.is_none() {
                                sc_num = Some(next_trimmed.trim_matches(|c| c == '#' || c == '.').to_string());
                            }
                            i += 1;
                            continue;
                        }

                        if clean_heading.ends_with('-')
                            && next_norm_x < 0.45
                            && next_trimmed.chars().all(|c| !c.is_alphabetic() || c.is_uppercase())
                        {
                            let (next_clean, next_sc) = extract_scene_number(next_trimmed);
                            append_continuation_line(&mut clean_heading, &next_clean);
                            if sc_num.is_none() {
                                sc_num = next_sc;
                            }
                            i += 1;
                            continue;
                        }
                    }
                    break;
                }

                blocks.push(ScriptBlock {
                    element_type: ElementType::SceneHeading,
                    text: clean_heading,
                    scene_number: sc_num,
                });
                current_state = ElementType::Action;
                last_action_y = None;
                i += 1;
                continue;
            }

            // ignore page break dialogue split markers like (MORE)
            if is_more_marker(&item.text) {
                i += 1;
                continue;
            }

            // classify based on dynamic margin cluster distance
            let classified_type = classify_by_margin(norm_x, profile, &item.text, current_state);

            match classified_type {
                ElementType::Character => {
                    let clean_char = clean_character_name(&item.text);
                    // if dialogue continues across a page split with the same character, merge it
                    if let Some(last) = blocks.last() {
                        if (last.element_type == ElementType::Dialogue || last.element_type == ElementType::Parenthetical)
                            && find_last_character_name(&blocks) == Some(&clean_char)
                        {
                            current_state = ElementType::Dialogue;
                            last_action_y = None;
                            i += 1;
                            continue;
                        }
                    }
                    blocks.push(ScriptBlock {
                        element_type: ElementType::Character,
                        text: clean_char,
                        scene_number: None,
                    });
                    current_state = ElementType::Character;
                    last_action_y = None;
                }
                ElementType::Parenthetical => {
                    let clean = item.text.trim_matches(|c| c == '(' || c == ')').trim();
                    if current_state == ElementType::Parenthetical {
                        if let Some(last) = blocks.last_mut() {
                            if last.element_type == ElementType::Parenthetical {
                                let inner = last.text.trim_matches(|c| c == '(' || c == ')').trim();
                                last.text = format!("({} {})", inner, clean);
                            }
                        }
                    } else {
                        let formatted = ensure_parentheses(&item.text);
                        blocks.push(ScriptBlock {
                            element_type: ElementType::Parenthetical,
                            text: formatted,
                            scene_number: None,
                        });
                    }
                    current_state = ElementType::Parenthetical;
                    last_action_y = None;
                }
                ElementType::Dialogue => {
                    if current_state == ElementType::Dialogue {
                        if let Some(last) = blocks.last_mut() {
                            if last.element_type == ElementType::Dialogue {
                                append_continuation_line(&mut last.text, &item.text);
                            } else {
                                blocks.push(ScriptBlock {
                                    element_type: ElementType::Dialogue,
                                    text: item.text.clone(),
                                    scene_number: None,
                                });
                            }
                        }
                    } else {
                        blocks.push(ScriptBlock {
                            element_type: ElementType::Dialogue,
                            text: item.text.clone(),
                            scene_number: None,
                        });
                    }
                    current_state = ElementType::Dialogue;
                    last_action_y = None;
                }
                ElementType::Transition => {
                    blocks.push(ScriptBlock {
                        element_type: ElementType::Transition,
                        text: item.text.clone(),
                        scene_number: None,
                    });
                    current_state = ElementType::Action;
                    last_action_y = None;
                }
                ElementType::Action => {
                    let is_continuation = if let Some(prev_y) = last_action_y {
                        (prev_y - item.y).abs() < 18.0
                    } else {
                        false
                    };

                    if current_state == ElementType::Action && is_continuation {
                        if let Some(last) = blocks.last_mut() {
                            if last.element_type == ElementType::Action {
                                append_continuation_line(&mut last.text, &item.text);
                            } else {
                                blocks.push(ScriptBlock {
                                    element_type: ElementType::Action,
                                    text: item.text.clone(),
                                    scene_number: None,
                                });
                            }
                        } else {
                            blocks.push(ScriptBlock {
                                element_type: ElementType::Action,
                                text: item.text.clone(),
                                scene_number: None,
                            });
                        }
                    } else {
                        blocks.push(ScriptBlock {
                            element_type: ElementType::Action,
                            text: item.text.clone(),
                            scene_number: None,
                        });
                    }
                    current_state = ElementType::Action;
                    last_action_y = Some(item.y);
                }
                ElementType::SceneHeading => unreachable!(),
            }

            i += 1;
        }
    }

    (title_page, blocks)
}

fn classify_by_margin(norm_x: f32, profile: &LayoutProfile, text: &str, state: ElementType) -> ElementType {
    // semantic overrides first
    if text.starts_with('(') && text.ends_with(')') {
        if state == ElementType::Character || state == ElementType::Dialogue || state == ElementType::Parenthetical {
            return ElementType::Parenthetical;
        }
    }

    if (text.ends_with("TO:") || text == "FADE OUT." || text == "FADE IN:") && norm_x > 0.60 {
        return ElementType::Transition;
    }

    // calculate distance to learned margin clusters
    let dist_action = (norm_x - profile.action_x).abs();
    let dist_dialogue = (norm_x - profile.dialogue_x).abs();
    let dist_paren = (norm_x - profile.parenthetical_x).abs();
    let dist_char = (norm_x - profile.character_x).abs();

    // character test
    if is_character_name(text) && dist_char < 0.07 && norm_x > profile.dialogue_x + 0.05 {
        return ElementType::Character;
    }

    // if in dialogue flow and near paren or enclosed in parens
    if (state == ElementType::Character || state == ElementType::Dialogue || state == ElementType::Parenthetical)
        && (text.starts_with('(') || dist_paren < dist_dialogue)
    {
        if text.starts_with('(') || (dist_paren < 0.05 && text.len() < 35) {
            return ElementType::Parenthetical;
        }
    }

    // dialogue continuation
    if (state == ElementType::Character || state == ElementType::Parenthetical || state == ElementType::Dialogue)
        && norm_x > profile.action_x + 0.05
    {
        return ElementType::Dialogue;
    }

    // closest distance fallback
    let mut min_dist = dist_action;
    let mut choice = ElementType::Action;

    if dist_dialogue < min_dist && norm_x > profile.action_x + 0.06 {
        min_dist = dist_dialogue;
        choice = ElementType::Dialogue;
    }
    if dist_char < min_dist && is_character_name(text) {
        choice = ElementType::Character;
    }

    choice
}

fn is_scene_heading(s: &str) -> bool {
    let clean = s.trim_start_matches(|c: char| c.is_ascii_digit() || c.is_whitespace() || c == '.' || c == '#').trim();
    let up = clean.to_uppercase();
    if up.starts_with("INT.") || up.starts_with("EXT.") || up.starts_with("INT/EXT") || up.starts_with("I/E") {
        return true;
    }
    let scene_suffixes = [" - DAY", " - NIGHT", " - CONTINUOUS", " - LATER", " - MOMENTS LATER", " - SAME TIME", " - DAWN", " - DUSK"];
    scene_suffixes.iter().any(|suffix| up.contains(suffix)) && up.chars().all(|c| !c.is_alphabetic() || c.is_uppercase())
}

fn is_scene_number_token(s: &str) -> bool {
    let clean = s.trim_matches(|c: char| c.is_whitespace() || c == '#' || c == '.');
    if clean.is_empty() || clean.len() > 8 {
        return false;
    }
    let has_digit = clean.chars().any(|c| c.is_ascii_digit());
    let valid_chars = clean.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    has_digit && valid_chars
}

fn extract_scene_number(s: &str) -> (String, Option<String>) {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.is_empty() {
        return (s.to_string(), None);
    }

    // check if starts with scene number e.g. "10 INT. ..." or "57A EARTH ORBIT..."
    if is_scene_number_token(parts[0]) && parts.len() > 1 {
        let sc_num = parts[0].trim_matches(|c| c == '#' || c == '.').to_string();
        let rest = parts[1..].join(" ");
        return (rest, Some(sc_num));
    }

    // check trailing number e.g. "... #202#" or "... 202"
    if parts.len() > 1 && is_scene_number_token(parts.last().unwrap()) {
        let sc_num = parts.last().unwrap().trim_matches(|c| c == '#' || c == '.').to_string();
        let rest = parts[..parts.len() - 1].join(" ");
        return (rest, Some(sc_num));
    }

    (s.to_string(), None)
}

fn is_character_name(s: &str) -> bool {
    if s.len() > 45 || s.is_empty() {
        return false;
    }
    // Remove parenthetical extensions like (O.S.), (V.O.), (OVER RADIO), etc.
    let mut clean = s.to_string();
    while let Some(start) = clean.find('(') {
        if let Some(end) = clean[start..].find(')') {
            clean.replace_range(start..=start + end, "");
        } else {
            break;
        }
    }
    let clean = clean.replace('"', "");
    let clean = clean.trim();
    if clean.is_empty() {
        return false;
    }

    // must be largely uppercase
    let upper_count = clean.chars().filter(|c| c.is_ascii_uppercase()).count();
    let alpha_count = clean.chars().filter(|c| c.is_alphabetic()).count();

    alpha_count > 0 && (upper_count as f32 / alpha_count as f32) > 0.85
}

fn ensure_parentheses(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        trimmed.to_string()
    } else {
        format!("({})", trimmed.trim_matches(|c| c == '(' || c == ')'))
    }
}

fn is_running_page_number(s: &str) -> bool {
    let clean = s.trim_matches(|c: char| c.is_whitespace() || c == '.');
    clean.parse::<u32>().is_ok()
}

fn append_continuation_line(target: &mut String, next: &str) {
    let trimmed_next = next.trim();
    // Only join without space if it's an intra-word hyphen (e.g. "student-" -> "student-ah", but not " - " or "--")
    let is_intra_word_hyphen = target.ends_with('-')
        && !target.ends_with("--")
        && !target.ends_with(" -")
        && target.chars().nth_back(1).map_or(false, |c| c.is_alphanumeric());

    if is_intra_word_hyphen {
        target.push_str(trimmed_next);
    } else {
        if !target.ends_with(' ') {
            target.push(' ');
        }
        target.push_str(trimmed_next);
    }
}

fn extract_title_page(page: &PageData) -> TitlePageInfo {
    let mut info = TitlePageInfo::default();
    let mut i = 0;

    while i < page.lines.len() {
        let line = &page.lines[i].text;
        let lower = line.to_lowercase();

        if lower.contains("written by") || lower.contains("story by") || lower.contains("screenplay by") {
            info.credit = Some(line.clone());
            if i + 1 < page.lines.len() {
                info.author = Some(page.lines[i + 1].text.clone());
                i += 2;
                continue;
            }
        } else if lower.starts_with("by ") || lower == "by" || lower.starts_with("by:") {
            if lower == "by" || lower == "by:" {
                info.credit = Some(line.clone());
                if i + 1 < page.lines.len() {
                    info.author = Some(page.lines[i + 1].text.clone());
                    i += 2;
                    continue;
                }
            } else {
                let author_part = line[2..].trim_start_matches(|c| c == ':' || c == ' ').trim();
                info.author = Some(author_part.to_string());
            }
        } else if info.title.is_none() {
            info.title = Some(line.clone());
        } else if info.author.is_none() && page.lines.len() <= 4 {
            let is_date_or_contact = lower.contains("draft")
                || lower.contains("@")
                || lower.contains("phone")
                || lower.chars().all(|c| c.is_ascii_digit() || c == '/' || c == '-' || c == '.');
            if !is_date_or_contact {
                info.author = Some(line.clone());
            }
        }
        i += 1;
    }

    info
}

fn is_more_marker(s: &str) -> bool {
    let clean = s.trim_matches(|c: char| c == '(' || c == ')' || c.is_whitespace());
    clean.eq_ignore_ascii_case("MORE")
}

fn clean_character_name(s: &str) -> String {
    let mut clean = s.trim().to_string();
    for suffix in &["(CONT'D)", "(cont'd)", "(CONT’D)", "(cont’d)"] {
        clean = clean.replace(suffix, "");
    }
    clean.trim().to_string()
}

fn find_last_character_name(blocks: &[ScriptBlock]) -> Option<&str> {
    for b in blocks.iter().rev() {
        if b.element_type == ElementType::Character {
            return Some(&b.text);
        }
        if b.element_type == ElementType::SceneHeading {
            return None;
        }
    }
    None
}
