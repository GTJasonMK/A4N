use a4n_export_contract::{ExportChapter, ExportDocument};

pub fn render_txt_document(chapters: &[ExportChapter]) -> String {
    let mut output = String::new();
    for chapter in chapters {
        output.push_str("\n\n");
        output.push_str(&chapter_heading(
            chapter.chapter_number,
            chapter.title.as_deref(),
        ));
        output.push_str("\n\n");
        output.push_str(&chapter.content);
        output.push('\n');
    }
    output
}

pub fn render_markdown_document(chapters: &[ExportChapter], title: &str, author: &str) -> String {
    let mut markdown = format!("# {title}\n\n作者：{author}\n\n---\n");
    for chapter in chapters {
        markdown.push_str("\n## ");
        markdown.push_str(&chapter_heading(
            chapter.chapter_number,
            chapter.title.as_deref(),
        ));
        markdown.push_str("\n\n");
        markdown.push_str(&chapter.content);
        markdown.push('\n');
    }
    markdown
}

pub fn chapter_heading(chapter_number: i32, title: Option<&str>) -> String {
    match title.map(str::trim).filter(|title| !title.is_empty()) {
        Some(title) => format!("第{chapter_number}章 {title}"),
        None => format!("第{chapter_number}章"),
    }
}

pub fn render_pdf_document(document: &ExportDocument) -> Vec<u8> {
    let mut paragraphs = vec![String::new()];
    for chapter in &document.chapters {
        paragraphs.push(chapter_heading(
            chapter.chapter_number,
            chapter.title.as_deref(),
        ));
        paragraphs.push(chapter.content.clone());
    }
    crate::pdf::render_text_pdf(&document.title, &paragraphs)
}
