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
            // ignore running page headers on very top
            if item.y < 60.0 && is_page_number(&item.text) {
                continue;
            }

            // normalized x ratio (0.0 .. 1.0)
            let norm_x = item.x / page.width;
            if norm_x < 0.05 || norm_x > 0.95 {
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

    // group nearby peaks within 3% width
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

    // assign learned peaks to screenplay roles
    let mut profile = LayoutProfile::default();

    if !distinct_peaks.is_empty() {
        profile.action_x = distinct_peaks[0];
    }
    if distinct_peaks.len() >= 2 {
        profile.dialogue_x = distinct_peaks[1];
    }
    if distinct_peaks.len() >= 3 {
        profile.parenthetical_x = distinct_peaks[2];
    }
    if distinct_peaks.len() >= 4 {
        profile.character_x = distinct_peaks[3];
    }
    if distinct_peaks.len() >= 5 {
        profile.transition_x = distinct_peaks[4];
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
