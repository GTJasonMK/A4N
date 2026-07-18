const PAGE_WIDTH: f32 = 595.0;
const PAGE_HEIGHT: f32 = 842.0;
const LEFT_MARGIN: f32 = 50.0;
const TOP_MARGIN: f32 = 56.0;
const FONT_SIZE: f32 = 12.0;
const LINE_HEIGHT: f32 = 17.0;
const MAX_CHARS_PER_LINE: usize = 42;

pub(super) fn text_pdf_bytes(title: &str, paragraphs: &[String]) -> Vec<u8> {
    let mut lines = Vec::new();
    if !title.trim().is_empty() {
        lines.push(title.trim().to_string());
        lines.push(String::new());
    }
    for paragraph in paragraphs {
        append_wrapped_lines(&mut lines, paragraph, MAX_CHARS_PER_LINE);
    }
    if lines.is_empty() {
        lines.push("暂无内容".to_string());
    }

    let lines_per_page = ((PAGE_HEIGHT - TOP_MARGIN - 52.0) / LINE_HEIGHT).floor() as usize;
    let pages = lines
        .chunks(lines_per_page.max(1))
        .map(|chunk| chunk.to_vec())
        .collect::<Vec<_>>();
    build_pdf(&pages)
}

fn append_wrapped_lines(out: &mut Vec<String>, text: &str, max_chars: usize) {
    if text.trim().is_empty() {
        out.push(String::new());
        return;
    }
    for raw_line in text.replace("\r\n", "\n").split('\n') {
        let line = raw_line.trim_end();
        if line.is_empty() {
            out.push(String::new());
            continue;
        }
        let mut current = String::new();
        for ch in line.chars() {
            current.push(ch);
            if visual_len(&current) >= max_chars {
                out.push(current.trim_end().to_string());
                current.clear();
            }
        }
        if !current.trim().is_empty() {
            out.push(current.trim_end().to_string());
        }
    }
    out.push(String::new());
}

fn visual_len(text: &str) -> usize {
    text.chars()
        .map(|character| if character.is_ascii() { 1 } else { 2 })
        .sum()
}

fn build_pdf(pages: &[Vec<String>]) -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = Vec::new();
    let kids = (0..pages.len())
        .map(|index| format!("{} 0 R", 4 + index * 2))
        .collect::<Vec<_>>()
        .join(" ");
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(format!("<< /Type /Pages /Kids [{kids}] /Count {} >>", pages.len()).into_bytes());
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec());

    for (index, page_lines) in pages.iter().enumerate() {
        let page_id = 4 + index * 2;
        let content_id = page_id + 1;
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_WIDTH:.0} {PAGE_HEIGHT:.0}] /Resources << /Font << /F1 3 0 R >> >> /Contents {content_id} 0 R >>"
            )
            .into_bytes(),
        );
        objects.push(build_page_stream(page_lines));
    }

    let mut out = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    let mut offsets = vec![0usize; objects.len() + 1];
    for (index, object) in objects.iter().enumerate() {
        let id = index + 1;
        offsets[id] = out.len();
        out.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
        out.extend_from_slice(object);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_pos = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

fn build_page_stream(lines: &[String]) -> Vec<u8> {
    let mut content = String::new();
    content.push_str("BT\n");
    content.push_str(&format!("/F1 {FONT_SIZE:.0} Tf\n"));
    content.push_str(&format!("{LINE_HEIGHT:.0} TL\n"));
    content.push_str(&format!(
        "1 0 0 1 {LEFT_MARGIN:.0} {:.0} Tm\n",
        PAGE_HEIGHT - TOP_MARGIN
    ));
    for line in lines {
        if line.is_empty() {
            content.push_str("T*\n");
        } else {
            content.push('<');
            content.push_str(&utf16be_hex(line));
            content.push_str("> Tj\nT*\n");
        }
    }
    content.push_str("ET\n");
    let bytes = content.into_bytes();
    let mut stream = format!("<< /Length {} >>\nstream\n", bytes.len()).into_bytes();
    stream.extend_from_slice(&bytes);
    stream.extend_from_slice(b"endstream");
    stream
}

fn utf16be_hex(text: &str) -> String {
    let mut hex = String::with_capacity(4 + text.len() * 4);
    hex.push_str("FEFF");
    for unit in text.encode_utf16() {
        hex.push_str(&format!("{unit:04X}"));
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::{text_pdf_bytes, utf16be_hex};

    #[test]
    fn utf16be_hex_preserves_ascii_and_cjk_text() {
        assert_eq!(utf16be_hex("A中"), "FEFF00414E2D");
    }

    #[test]
    fn text_pdf_bytes_builds_catalog_xref_and_utf16_stream() {
        let pdf = text_pdf_bytes("标题", &["正文".to_string()]);
        let text = String::from_utf8_lossy(&pdf);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(text.contains("/Type /Catalog"));
        assert!(text.contains("FEFF68079898"));
        assert!(text.contains("xref\n"));
        assert!(text.ends_with("%%EOF\n"));
    }
}
