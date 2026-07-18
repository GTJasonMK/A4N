use super::{
    contracts::{BatchExportInput, ExportMarkdownInput},
    epub::{
        add_export_chapters_to_epub, epub_chapter_from_export_chapter, export_chapter_count_total,
        new_epub_builder,
    },
    rules::normalize_batch_formats,
};
use a4n_orchestrator::export::ExportChapter;

#[test]
fn epub_chapter_uses_plain_content_without_html_wrapper() {
    let chapter = ExportChapter {
        id: 1,
        chapter_number: 3,
        title: Some("风起".to_string()),
        content: "正文第一段\n\n正文第二段".to_string(),
    };

    let epub_chapter = epub_chapter_from_export_chapter(&chapter, 1);

    assert_eq!(epub_chapter.title, "第3章 风起");
    assert_eq!(epub_chapter.content, "正文第一段\n\n正文第二段");
    assert!(!epub_chapter.content.contains("<html"));
    assert!(!epub_chapter.content.contains("<body"));
}

#[test]
fn epub_chapter_allows_missing_or_blank_title() {
    for title in [None, Some("  ".to_string())] {
        let chapter = ExportChapter {
            id: 1,
            chapter_number: 3,
            title,
            content: "正文".to_string(),
        };

        let epub_chapter = epub_chapter_from_export_chapter(&chapter, 1);

        assert_eq!(epub_chapter.title, "第3章");
        assert_eq!(epub_chapter.content, "正文");
    }
}

#[test]
fn epub_builder_accepts_export_chapters_for_batch_export() {
    let chapters = vec![
        ExportChapter {
            id: 1,
            chapter_number: 1,
            title: Some("开端".to_string()),
            content: "第一章正文".to_string(),
        },
        ExportChapter {
            id: 2,
            chapter_number: 2,
            title: Some("转折".to_string()),
            content: "第二章正文".to_string(),
        },
    ];
    let mut seen = Vec::new();
    let mut builder = new_epub_builder("测试书名", "测试作者");

    add_export_chapters_to_epub(&mut builder, &chapters, |index, chapter| {
        seen.push((index, chapter.chapter_number));
        Ok(())
    })
    .expect("chapters should be accepted");

    assert_eq!(seen, vec![(0, 1), (1, 2)]);
    let epub_data = builder.build().expect("epub should build");
    assert!(!epub_data.is_empty());
    assert_eq!(&epub_data[0..2], b"PK");
}

#[test]
fn export_chapter_count_rejects_progress_overflow() {
    let err = export_chapter_count_total(u32::MAX as usize, 1)
        .expect_err("export progress overflow must be visible");

    assert!(err.contains("导出总进度超过 u32 上限"));
}

#[test]
fn normalize_batch_formats_rejects_empty_unknown_and_duplicate_formats() {
    for formats in [
        Vec::<String>::new(),
        vec![" ".to_string()],
        vec!["docx".to_string()],
        vec!["md".to_string(), "markdown".to_string()],
        vec!["epub".to_string(), "EPUB".to_string()],
    ] {
        normalize_batch_formats(&formats).expect_err("formats should be rejected");
    }
}

#[test]
fn normalize_batch_formats_accepts_supported_formats() {
    let formats = vec![
        "txt".to_string(),
        "MD".to_string(),
        "epub".to_string(),
        "json".to_string(),
        "pdf".to_string(),
    ];

    assert_eq!(
        normalize_batch_formats(&formats),
        Ok(vec!["txt", "markdown", "epub", "json", "pdf"])
    );
}

#[test]
fn export_markdown_input_rejects_unknown_fields() {
    let err = serde_json::from_value::<ExportMarkdownInput>(serde_json::json!({
        "title": "测试书名",
        "author": "测试作者",
        "project_id": 42,
    }))
    .expect_err("unknown markdown input fields should be rejected");

    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn batch_export_input_rejects_unknown_fields() {
    let err = serde_json::from_value::<BatchExportInput>(serde_json::json!({
        "project_id": 42,
        "formats": ["txt"],
        "title": "测试书名",
        "author": "测试作者",
        "output_path": "/tmp/export.txt",
    }))
    .expect_err("unknown batch input fields should be rejected");

    assert!(err.to_string().contains("unknown field"));
}
