use crate::geom::PageData;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct LayoutProfile {
    pub action_x: f32,
    pub dialogue_x: f32,
    pub parenthetical_x: f32,
    pub character_x: f32,
    pub transition_x: f32,
}

impl Default for LayoutProfile {
    fn default() -> Self {
        Self {
            action_x: 0.18,
            dialogue_x: 0.30,
            parenthetical_x: 0.37,
            character_x: 0.44,
            transition_x: 0.80,
        }
    }
}

pub fn calibrate_layout(pages: &[PageData]) -> LayoutProfile {
    let mut histogram: BTreeMap<i32, usize> = BTreeMap::new();

    for page in pages {
        if page.lines.is_empty() || page.width <= 0.0 {
            continue;
        }

        // skip title page candidate when gathering clusters
        if is_likely_title_page(page) {
            continue;
        }

        for item in &page.lines {
            // ignore running page headers on very top (PDF y=0 is at bottom)
            if (page.height - item.y) < 75.0 && is_page_number(&item.text) {
                continue;
            }

            // ignore standalone numbers (scene numbers in margins, page numbers)
            if is_pure_number(&item.text) || is_page_number(&item.text) {
                continue;
            }

            // normalized x ratio (0.0 .. 1.0)
            let norm_x = item.x / page.width;
            // screenplay columns start at ~14% (1.2in+ margin) and end around 90%
            if norm_x < 0.13 || norm_x > 0.90 {
                continue;
            }

            // bucket by 1% intervals
            let bucket = (norm_x * 100.0).round() as i32;
            *histogram.entry(bucket).or_insert(0) += 1;
        }
    }

    if histogram.is_empty() {
        return LayoutProfile::default();
    }

    // find peaks
    let mut peaks: Vec<(i32, usize)> = Vec::new();
    for (&bucket, &count) in &histogram {
        if count >= 2 {
            peaks.push((bucket, count));
        }
    }

    peaks.sort_by(|a, b| b.1.cmp(&a.1));

    // group nearby peaks within 3.5% width
    let mut distinct_peaks: Vec<f32> = Vec::new();
    for (bucket, _) in peaks {
        let val = bucket as f32 / 100.0;
        let mut exists = false;
        for p in &distinct_peaks {
            if (p - val).abs() < 0.035 {
                exists = true;
                break;
            }
        }
        if !exists {
            distinct_peaks.push(val);
        }
        if distinct_peaks.len() >= 5 {
            break;
        }
    }

    distinct_peaks.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // assign learned peaks to screenplay roles by proximity
    let mut profile = LayoutProfile::default();

    for &p in &distinct_peaks {
        if (p - 0.18).abs() < 0.06 {
            profile.action_x = p;
        } else if (p - 0.30).abs() < 0.05 {
            profile.dialogue_x = p;
        } else if (p - 0.37).abs() < 0.04 {
            profile.parenthetical_x = p;
        } else if (p - 0.44).abs() < 0.06 {
            profile.character_x = p;
        } else if p > 0.70 {
            profile.transition_x = p;
        }
    }

    profile
}

pub fn is_likely_title_page(page: &PageData) -> bool {
    if page.page_number > 1 {
        return false;
    }
    if page.lines.is_empty() {
        return false;
    }

    // if page has scene heading, it cannot be title page
    for line in &page.lines {
        let up = line.text.to_uppercase();
        if up.starts_with("INT.") || up.starts_with("EXT.") || up.starts_with("INT/EXT") || up.starts_with("I/E") {
            return false;
        }
    }

    // title page has very few lines compared to script page
    if page.lines.len() <= 12 {
        // check if has title words
        for line in &page.lines {
            let low = line.text.to_lowercase();
            if low.contains("written by") || low.contains("draft") || low.contains("by:") || low.contains("author") {
                return true;
            }
        }
        // if few lines and vertically centered, consider it title page
        let avg_y: f32 = page.lines.iter().map(|l| l.y).sum::<f32>() / page.lines.len() as f32;
        if avg_y > 200.0 && avg_y < 600.0 {
            return true;
        }
    }

    false
}

fn is_page_number(s: &str) -> bool {
    let clean = s.trim_matches(|c: char| c.is_whitespace() || c == '.');
    clean.parse::<u32>().is_ok()
}

fn is_pure_number(s: &str) -> bool {
    let clean = s.trim_matches(|c: char| c.is_whitespace() || c == '.');
    !clean.is_empty() && clean.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::RawTextItem;

    #[test]
    fn test_is_page_number() {
        assert!(is_page_number("1"));
        assert!(is_page_number(" 42. "));
        assert!(is_page_number("123"));
        assert!(!is_page_number("PAGE 1"));
        assert!(!is_page_number("Scene 10"));
    }

    #[test]
    fn test_is_pure_number() {
        assert!(is_pure_number("12"));
        assert!(is_pure_number(".45."));
        assert!(!is_pure_number("12A"));
        assert!(!is_pure_number(""));
    }

    #[test]
    fn test_is_likely_title_page_true() {
        let page = PageData {
            page_number: 1,
            width: 612.0,
            height: 792.0,
            lines: vec![
                RawTextItem { x: 200.0, y: 500.0, text: "THE GREAT SCRIPT".to_string() },
                RawTextItem { x: 200.0, y: 480.0, text: "Written by".to_string() },
                RawTextItem { x: 200.0, y: 460.0, text: "John Doe".to_string() },
            ],
        };
        assert!(is_likely_title_page(&page));
    }

    #[test]
    fn test_is_likely_title_page_false_if_has_scene_heading() {
        let page = PageData {
            page_number: 1,
            width: 612.0,
            height: 792.0,
            lines: vec![
                RawTextItem { x: 100.0, y: 700.0, text: "INT. LIVING ROOM - DAY".to_string() },
                RawTextItem { x: 100.0, y: 680.0, text: "John sits on the couch.".to_string() },
            ],
        };
        assert!(!is_likely_title_page(&page));
    }

    #[test]
    fn test_is_likely_title_page_false_if_page_gt_1() {
        let page = PageData {
            page_number: 2,
            width: 612.0,
            height: 792.0,
            lines: vec![
                RawTextItem { x: 200.0, y: 500.0, text: "Written by".to_string() },
            ],
        };
        assert!(!is_likely_title_page(&page));
    }

    #[test]
    fn test_calibrate_layout_default() {
        let profile = calibrate_layout(&[]);
        assert_eq!(profile.action_x, 0.18);
        assert_eq!(profile.dialogue_x, 0.30);
        assert_eq!(profile.parenthetical_x, 0.37);
        assert_eq!(profile.character_x, 0.44);
        assert_eq!(profile.transition_x, 0.80);
    }
}
