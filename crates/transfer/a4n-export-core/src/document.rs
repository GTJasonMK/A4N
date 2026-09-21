//! Pure document export data and text renderers.
//!
//! Database loading and authorization stay outside this crate so the same
//! renderer can run in the Host or in a replaceable Export Worker.

use a4n_document_export_core::{
    chapter_heading,
    epub::{EpubBuilder, EpubChapter, EpubMetadata, builtin_epub_stylesheet},
    render_markdown_document, render_pdf_document, render_txt_document,
};
use a4n_export_contract::{
    ExportDocumentFormat, ExportRenderDocumentInput, ExportRenderDocumentOutput,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use crate::ExportRenderError;

pub fn render_document(
    input: ExportRenderDocumentInput,
) -> Result<ExportRenderDocumentOutput, ExportRenderError> {
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
            render_epub(&input.document, required_author(input.author.as_deref())?)?,
            "epub",
            "application/epub+zip",
        ),
        ExportDocumentFormat::Pdf => (
            render_pdf_document(&input.document),
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
) -> Result<Vec<u8>, ExportRenderError> {
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
        builder.add_chapter(EpubChapter {
            title: chapter_heading(chapter.chapter_number, chapter.title.as_deref()),
            content: chapter.content.clone(),
            order: index + 1,
        });
    }
    Ok(builder.build()?)
}

#[cfg(test)]
mod tests {
    use super::*;

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
