//! Pure document export data and text renderers.
//!
//! Database loading and authorization stay outside this crate so the same
//! renderer can run in the Host or in a replaceable Export Worker.

use a4n_document_export_core::{
    chapter_heading,
    epub::{EpubBuilder, EpubChapter, EpubMetadata, builtin_epub_stylesheet},
    render_markdown_document, render_txt_document,
};
use a4n_export_contract::{
    ExportDocumentFormat, ExportDocumentRenderer, ExportRenderDocumentInput,
    ExportRenderDocumentOutput, ExportRenderFailure, ExportRenderObserver,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use crate::ExportRenderError;

pub fn render_document(
    input: ExportRenderDocumentInput,
) -> Result<ExportRenderDocumentOutput, ExportRenderError> {
    render_with_progress(input, None)
}

pub struct LocalDocumentExportRenderer;

#[async_trait::async_trait]
impl ExportDocumentRenderer for LocalDocumentExportRenderer {
    async fn render_document(
        &self,
        input: ExportRenderDocumentInput,
        _project_id: Option<i64>,
        _root_project_id: Option<i64>,
        progress: Option<&dyn ExportRenderObserver>,
    ) -> Result<ExportRenderDocumentOutput, ExportRenderFailure> {
        render_with_progress(input, progress).map_err(|error| match error {
            ExportRenderError::Cancelled => ExportRenderFailure::Cancelled,
            error => ExportRenderFailure::Failed(error.to_string()),
        })
    }
}

fn render_with_progress(
    input: ExportRenderDocumentInput,
    observer: Option<&dyn ExportRenderObserver>,
) -> Result<ExportRenderDocumentOutput, ExportRenderError> {
    check_cancelled(observer)?;
    let (bytes, extension, content_type) = match input.format {
        ExportDocumentFormat::Txt => (
            render_txt_document(&input.document.chapters).into_bytes(),
            "txt",
            "text/plain;charset=utf-8",
        ),
        ExportDocumentFormat::Markdown => (
            render_markdown_document(
                &input.document.chapters,
                &input.document.title,
                required_author(input.author.as_deref())?,
            )
            .into_bytes(),
            "md",
            "text/markdown;charset=utf-8",
        ),
        ExportDocumentFormat::Epub => (
            render_epub(
                &input.document,
                required_author(input.author.as_deref())?,
                observer,
            )?,
            "epub",
            "application/epub+zip",
        ),
        ExportDocumentFormat::Pdf => (
            render_pdf(&input.document, observer)?,
            "pdf",
            "application/pdf",
        ),
    };
    Ok(ExportRenderDocumentOutput {
        bytes_base64: BASE64.encode(bytes),
        extension: extension.to_string(),
        content_type: content_type.to_string(),
    })
}

fn required_author(author: Option<&str>) -> Result<&str, ExportRenderError> {
    author
        .map(str::trim)
        .filter(|author| !author.is_empty())
        .ok_or_else(|| ExportRenderError::InvalidDocument("导出作者不能为空".to_string()))
}

fn render_epub(
    document: &a4n_export_contract::ExportDocument,
    author: &str,
    observer: Option<&dyn ExportRenderObserver>,
) -> Result<Vec<u8>, ExportRenderError> {
    let total = chapter_count_total(document.chapters.len(), 4)?;
    progress(
        observer,
        1,
        total,
        &format!("已读取 {} 章，正在生成 EPUB", document.chapters.len()),
    );
    let metadata = EpubMetadata {
        title: document.title.clone(),
        author: author.to_string(),
        language: "zh-CN".to_string(),
        publisher: None,
        date: None,
        description: None,
        cover_image: None,
        cover_mime: None,
    };
    let mut builder = EpubBuilder::new(metadata, builtin_epub_stylesheet().to_string());
    for (index, chapter) in document.chapters.iter().enumerate() {
        check_cancelled(observer)?;
        progress(
            observer,
            index as u32 + 2,
            total,
            &format!("正在写入 EPUB 第{}章", chapter.chapter_number),
        );
        builder.add_chapter(EpubChapter {
            title: chapter_heading(chapter.chapter_number, chapter.title.as_deref()),
            content: chapter.content.clone(),
            order: index + 1,
        });
    }
    check_cancelled(observer)?;
    progress(observer, total - 2, total, "正在打包 EPUB");
    let bytes = builder.build()?;
    progress(observer, total, total, "EPUB 导出完成");
    Ok(bytes)
}

fn render_pdf(
    document: &a4n_export_contract::ExportDocument,
    observer: Option<&dyn ExportRenderObserver>,
) -> Result<Vec<u8>, ExportRenderError> {
    let total = chapter_count_total(document.chapters.len(), 5)?;
    progress(
        observer,
        2,
        total,
        &format!("已读取 {} 章，正在排版 PDF", document.chapters.len()),
    );
    let mut paragraphs = vec![String::new()];
    for (index, chapter) in document.chapters.iter().enumerate() {
        check_cancelled(observer)?;
        progress(
            observer,
            index as u32 + 3,
            total,
            &format!("正在排版 PDF 第{}章", chapter.chapter_number),
        );
        paragraphs.push(chapter_heading(
            chapter.chapter_number,
            chapter.title.as_deref(),
        ));
        paragraphs.push(chapter.content.clone());
    }
    check_cancelled(observer)?;
    progress(observer, total - 1, total, "正在生成 PDF 文件");
    let bytes = a4n_document_export_core::pdf::render_text_pdf(&document.title, &paragraphs);
    progress(observer, total, total, "PDF 导出完成");
    Ok(bytes)
}

fn chapter_count_total(count: usize, extra: u32) -> Result<u32, ExportRenderError> {
    u32::try_from(count)
        .ok()
        .and_then(|count| count.checked_add(extra))
        .ok_or_else(|| {
            ExportRenderError::InvalidDocument(format!(
                "导出总进度超过 u32 上限：{count} + {extra}"
            ))
        })
}

fn check_cancelled(observer: Option<&dyn ExportRenderObserver>) -> Result<(), ExportRenderError> {
    if observer.is_some_and(|observer| observer.is_cancelled()) {
        return Err(ExportRenderError::Cancelled);
    }
    Ok(())
}

fn progress(observer: Option<&dyn ExportRenderObserver>, current: u32, total: u32, message: &str) {
    if let Some(observer) = observer {
        observer.progress(current, total, message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendering_stops_when_cancelled_between_chapters_and_checks_progress_overflow() {
        use std::sync::atomic::{AtomicBool, Ordering};
        struct CancelAfterChapter(AtomicBool);
        impl ExportRenderObserver for CancelAfterChapter {
            fn progress(&self, _current: u32, _total: u32, message: &str) {
                if message.starts_with("正在写入 EPUB 第") || message.starts_with("正在排版 PDF 第")
                {
                    self.0.store(true, Ordering::Relaxed);
                }
            }
            fn is_cancelled(&self) -> bool {
                self.0.load(Ordering::Relaxed)
            }
        }
        for format in [ExportDocumentFormat::Epub, ExportDocumentFormat::Pdf] {
            let observer = CancelAfterChapter(AtomicBool::new(false));
            let error = render_with_progress(
                ExportRenderDocumentInput {
                    document: a4n_export_contract::ExportDocument {
                        title: "测试书名".into(),
                        chapters: vec![a4n_export_contract::ExportChapter {
                            chapter_number: 1,
                            title: Some("开篇".into()),
                            content: "正文".into(),
                        }],
                    },
                    format,
                    author: Some("作者".into()),
                },
                Some(&observer),
            )
            .unwrap_err();
            assert!(matches!(error, ExportRenderError::Cancelled));
        }
        assert!(chapter_count_total(u32::MAX as usize, 1).is_err());
    }

    #[test]
    fn pdf_renderer_preserves_existing_document_bytes() {
        let document = a4n_export_contract::ExportDocument {
            title: "测试书名".into(),
            chapters: vec![a4n_export_contract::ExportChapter {
                chapter_number: 2,
                title: Some("风起".into()),
                content: "第一段\n\n第二段".into(),
            }],
        };
        assert_eq!(
            render_pdf(&document, None).unwrap(),
            a4n_document_export_core::render_pdf_document(&document)
        );
    }

    #[test]
    fn chapter_heading_omits_blank_title_without_padding() {
        assert_eq!(
            a4n_document_export_core::chapter_heading(8, Some("  \t ")),
            "第8章"
        );
    }

    #[test]
    fn document_renderers_preserve_empty_document_contract() {
        let chapters = [];
        assert_eq!(render_txt_document(&chapters), "");
        assert_eq!(
            render_markdown_document(&chapters, "测试书名", "测试作者"),
            "# 测试书名\n\n作者：测试作者\n\n---\n"
        );
    }

    #[test]
    fn document_renderer_returns_wire_metadata_and_rejects_missing_author() {
        let document = a4n_export_contract::ExportDocument {
            title: "测试".to_string(),
            chapters: Vec::new(),
        };
        let output = render_document(ExportRenderDocumentInput {
            document: document.clone(),
            format: ExportDocumentFormat::Txt,
            author: None,
        })
        .expect("txt document should render");
        assert_eq!(output.extension, "txt");
        assert_eq!(output.content_type, "text/plain;charset=utf-8");
        assert!(BASE64.decode(output.bytes_base64).is_ok());

        let error = render_document(ExportRenderDocumentInput {
            document,
            format: ExportDocumentFormat::Epub,
            author: Some(" ".to_string()),
        })
        .expect_err("epub author is required");
        assert!(error.to_string().contains("作者不能为空"));
    }
}
