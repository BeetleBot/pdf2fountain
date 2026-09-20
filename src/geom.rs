use lopdf::content::Content;
use lopdf::{Document, Object};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct RawTextItem {
    pub x: f32,
    pub y: f32,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct PageData {
    pub page_number: usize,
    pub width: f32,
    pub height: f32,
    pub lines: Vec<RawTextItem>,
}

pub fn extract_pages(doc: &Document) -> Result<Vec<PageData>, String> {
    let mut pages = Vec::new();

    for (page_num, page_id) in doc.get_pages() {
        let (width, height) = get_page_dimensions(doc, page_id);
        let fonts = doc.get_page_fonts(page_id).unwrap_or_default();
        let content_data = doc.get_page_content(page_id);
        
        let content = match Content::decode(&content_data) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let mut current_font = String::new();
        let mut current_x = 0.0f32;
        let mut current_y = 0.0f32;
        let mut raw_items = Vec::new();

        // cm matrix stack
        let mut ctm_stack: Vec<[f32; 6]> = Vec::new();
        let mut current_ctm: [f32; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

        for op in &content.operations {
            match op.operator.as_str() {
                "q" => ctm_stack.push(current_ctm),
                "Q" => {
                    if let Some(prev) = ctm_stack.pop() {
                        current_ctm = prev;
                    }
                }
                "cm" => {
                    if op.operands.len() >= 6 {
                        let m = [
                            op.operands[0].as_float().unwrap_or(1.0),
                            op.operands[1].as_float().unwrap_or(0.0),
                            op.operands[2].as_float().unwrap_or(0.0),
                            op.operands[3].as_float().unwrap_or(1.0),
                            op.operands[4].as_float().unwrap_or(0.0),
                            op.operands[5].as_float().unwrap_or(0.0),
                        ];
                        current_ctm = multiply_matrix(&m, &current_ctm);
                    }
                }
                "Tf" => {
                    if let Some(Object::Name(name)) = op.operands.get(0) {
                        current_font = String::from_utf8_lossy(name).to_string();
                    }
                }
                "Tm" => {
                    if op.operands.len() >= 6 {
                        let tx = op.operands[4].as_float().unwrap_or(0.0);
                        let ty = op.operands[5].as_float().unwrap_or(0.0);
                        let (mapped_x, mapped_y) = apply_matrix(tx, ty, &current_ctm);
                        current_x = mapped_x;
                        current_y = mapped_y;
                    }
                }
                "TJ" | "Tj" => {
                    let text = if let Some(font_dict) = fonts.get(current_font.as_bytes()) {
                        decode_tj(doc, font_dict, &op.operands)
                    } else {
                        decode_plain_tj(&op.operands)
                    };

                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        let leading_spaces = text.chars().take_while(|c| c.is_whitespace()).count() as f32;
                        let adjusted_x = current_x + leading_spaces * 7.2;
                        raw_items.push(RawTextItem {
                            x: adjusted_x,
                            y: current_y,
                            text: trimmed.to_string(),
                        });
                    }
                }
                _ => {}
            }
        }

        // group into lines if on same horizontal baseline (within 3pt)
        let merged_lines = merge_same_line_items(raw_items);

        pages.push(PageData {
            page_number: page_num as usize,
            width,
            height,
            lines: merged_lines,
        });
    }

    Ok(pages)
}

fn get_page_dimensions(doc: &Document, page_id: lopdf::ObjectId) -> (f32, f32) {
    let mut current_id = page_id;
    while let Ok(page_dict) = doc.get_object(current_id).and_then(|o| o.as_dict()) {
        if let Ok(box_obj) = page_dict.get(b"MediaBox") {
            if let Ok(arr) = box_obj.as_array() {
                if arr.len() >= 4 {
                    let w = arr[2].as_float().unwrap_or(595.0);
                    let h = arr[3].as_float().unwrap_or(842.0);
                    return (w, h);
                }
            }
        }
        // climb parent if not in leaf
        if let Ok(parent_ref) = page_dict.get(b"Parent").and_then(|p| p.as_reference()) {
            current_id = parent_ref;
        } else {
            break;
        }
    }
    (595.0, 842.0)
}

fn multiply_matrix(a: &[f32; 6], b: &[f32; 6]) -> [f32; 6] {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4],
        a[4] * b[1] + a[5] * b[3] + b[5],
    ]
}

fn apply_matrix(x: f32, y: f32, m: &[f32; 6]) -> (f32, f32) {
    (
        x * m[0] + y * m[2] + m[4],
        x * m[1] + y * m[3] + m[5],
    )
}

fn merge_same_line_items(mut items: Vec<RawTextItem>) -> Vec<RawTextItem> {
    if items.is_empty() {
        return items;
    }

    // sort top to bottom (higher y to lower y in standard pdf coordinates), then left to right
    items.sort_by(|a, b| {
        if (a.y - b.y).abs() < 2.0 {
            a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal)
        }
    });

    let mut result: Vec<RawTextItem> = Vec::new();
    let mut last_end_x: f32 = 0.0;

    for item in items {
        if let Some(last) = result.last_mut() {
            if (last.y - item.y).abs() < 2.5 {
                let gap = item.x - last_end_x;
                // If gap is moderate (< 25pt), merge onto the same line.
                // If gap is large (>= 25pt), keep separated (e.g. scene numbers in margins)
                if gap < 25.0 {
                    // In Courier 12pt, character pitch is ~7.2pt.
                    // If gap <= 2.5pt, the characters are immediately adjacent in the same word (no space).
                    // If gap > 2.5pt, it represents a word space.
                    if gap > 2.5 && !last.text.ends_with(' ') && !item.text.starts_with(' ') {
                        last.text.push(' ');
                    }
                    last.text.push_str(&item.text);
                    let char_count = item.text.chars().count() as f32;
                    last_end_x = item.x + char_count * 7.2;
                    continue;
                }
            }
        }
        let char_count = item.text.chars().count() as f32;
        last_end_x = item.x + char_count * 7.2;
        result.push(item);
    }
    result
}

pub fn decode_tj(doc: &Document, font: &lopdf::Dictionary, operands: &[Object]) -> String {
    let to_unicode = font.get(b"ToUnicode").ok()
        .and_then(|obj| match obj {
            Object::Reference(id) => doc.get_object(*id).ok(),
            _ => None,
        })
        .and_then(|obj| obj.as_stream().ok());

    let cmap = to_unicode.map(|stream| {
        let data = stream.decompressed_content().unwrap_or_default();
        parse_cmap(&data)
    }).unwrap_or_default();

    // Detect CID fonts (Type0 with Identity-H encoding) which use 2-byte character codes
    let is_cid = font.get(b"Encoding").ok()
        .and_then(|e| if let Object::Name(n) = e { Some(n.as_slice()) } else { None })
        .is_some_and(|n| n == b"Identity-H" || n == b"Identity-V");

    let mut out = String::new();
    for op in operands {
        match op {
            Object::Array(arr) => {
                for item in arr {
                    if let Object::String(bytes, _) = item {
                        out.push_str(&decode_bytes(bytes, &cmap, is_cid));
                    }
                }
            }
            Object::String(bytes, _) => {
                out.push_str(&decode_bytes(bytes, &cmap, is_cid));
            }
            _ => {}
        }
    }
    out
}

pub fn decode_plain_tj(operands: &[Object]) -> String {
    let mut out = String::new();
    for op in operands {
        match op {
            Object::Array(arr) => {
                for item in arr {
                    if let Object::String(bytes, _) = item {
                        out.push_str(&String::from_utf8_lossy(bytes));
                    }
                }
            }
            Object::String(bytes, _) => {
                out.push_str(&String::from_utf8_lossy(bytes));
            }
            _ => {}
        }
    }
    out
}

fn parse_cmap(data: &[u8]) -> BTreeMap<u32, String> {
    let mut map = BTreeMap::new();
    let text = String::from_utf8_lossy(data);
    let mut in_bfrange = false;
    let mut in_bfchar = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.ends_with("beginbfchar") { in_bfchar = true; continue; }
        if trimmed.ends_with("endbfchar") { in_bfchar = false; continue; }
        if trimmed.ends_with("beginbfrange") { in_bfrange = true; continue; }
        if trimmed.ends_with("endbfrange") { in_bfrange = false; continue; }

        if in_bfchar {
            // split_hex_tokens handles both space-separated and concatenated hex tokens
            // e.g. "<0041> <0061>" or "<0041><0061>"
            let tokens = split_hex_tokens(trimmed);
            if tokens.len() >= 2 {
                if let (Some(c), Some(t)) = (parse_hex(&tokens[0]), parse_utf16_hex(&tokens[1])) {
                    map.insert(c, t);
                }
            }
        } else if in_bfrange {
            let tokens = split_hex_tokens(trimmed);
            if tokens.len() >= 3 {
                if let (Some(s), Some(e)) = (parse_hex(&tokens[0]), parse_hex(&tokens[1])) {
                    if tokens[2].starts_with('[') {
                        // Array notation: <start> <end> [<dest1> <dest2> ...]
                        let inner = tokens[2].trim_matches(|c| c == '[' || c == ']');
                        let dest_tokens = split_hex_tokens(inner);
                        for (idx, dt) in dest_tokens.iter().enumerate() {
                            let cid = s + idx as u32;
                            if cid > e {
                                break;
                            }
                            if let Some(text) = parse_utf16_hex(dt) {
                                map.insert(cid, text);
                            }
                        }
                    } else if let Some(d) = parse_hex(&tokens[2]) {
                        for i in 0..=(e - s) {
                            if let Some(ch) = std::char::from_u32(d + i) {
                                map.insert(s + i, ch.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    map
}

/// Splits a line of CMap hex tokens, handling both formats:
/// - Space-separated: `<0003> <0003> <0020>`
/// - Concatenated (Fade In style): `<0003><0003><0020>`
fn split_hex_tokens(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_bracket = false;

    for ch in s.chars() {
        match ch {
            '<' => {
                in_bracket = true;
                current.push(ch);
            }
            '>' => {
                current.push(ch);
                if in_bracket {
                    tokens.push(current.clone());
                    current.clear();
                    in_bracket = false;
                }
            }
            '[' if !in_bracket => {
                // Array notation like [<0041> <0042>], push the rest as-is
                let rest: String = s[s.find('[').unwrap_or(0)..].to_string();
                tokens.push(rest);
                return tokens;
            }
            _ if in_bracket => {
                current.push(ch);
            }
            _ => {} // skip whitespace between tokens
        }
    }

    tokens
}

fn parse_hex(s: &str) -> Option<u32> {
    let clean = s.trim_matches(|c| c == '<' || c == '>');
    u32::from_str_radix(clean, 16).ok()
}

fn parse_utf16_hex(s: &str) -> Option<String> {
    let clean = s.trim_matches(|c| c == '<' || c == '>');
    if clean.len() % 4 == 0 {
        let mut u16_vals = Vec::new();
        for i in (0..clean.len()).step_by(4) {
            let code = u16::from_str_radix(&clean[i..i+4], 16).ok()?;
            u16_vals.push(code);
        }
        String::from_utf16(&u16_vals).ok()
    } else {
        None
    }
}

fn decode_bytes(bytes: &[u8], cmap: &BTreeMap<u32, String>, is_cid: bool) -> String {
    let mut res = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() {
            let code = ((bytes[i] as u32) << 8) | (bytes[i + 1] as u32);
            if let Some(s) = cmap.get(&code) {
                res.push_str(s);
                i += 2;
                continue;
            }
            // For CID fonts, always consume 2 bytes — never fall back to 1-byte mode
            if is_cid {
                i += 2;
                continue;
            }
        }
        let single = bytes[i] as u32;
        if let Some(s) = cmap.get(&single) {
            res.push_str(s);
        } else {
            res.push(bytes[i] as char);
        }
        i += 1;
    }
    res
}
