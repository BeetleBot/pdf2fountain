pub mod calibrate;
pub mod fountain;
pub mod fsm;
pub mod geom;

use lopdf::Document;
use std::path::Path;

pub fn parse_pdf_to_fountain(pdf_bytes: &[u8]) -> Result<String, String> {
    let doc = Document::load_mem(pdf_bytes).map_err(|e| format!("failed reading pdf bytes: {e}"))?;
    parse_document(&doc, None)
}

pub fn parse_pdf_file_to_fountain<P: AsRef<Path>>(path: P) -> Result<String, String> {
    let doc = Document::load(path.as_ref()).map_err(|e| format!("failed loading pdf file: {e}"))?;
    parse_document(&doc, None)
}

pub fn parse_pdf_file_to_fountain_pages<P: AsRef<Path>>(path: P, page_range: (usize, usize)) -> Result<String, String> {
    let doc = Document::load(path.as_ref()).map_err(|e| format!("failed loading pdf file: {e}"))?;
    parse_document(&doc, Some(page_range))
}

fn parse_document(doc: &Document, range: Option<(usize, usize)>) -> Result<String, String> {
    let mut pages = geom::extract_pages(doc)?;
    if let Some((start, end)) = range {
        pages.retain(|p| p.page_number >= start && p.page_number <= end);
    }
    let profile = calibrate::calibrate_layout(&pages);
    let (title_page, blocks) = fsm::parse_screenplay(&pages, &profile);
    Ok(fountain::format_fountain(title_page.as_ref(), &blocks))
}
