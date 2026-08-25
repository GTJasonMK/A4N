mod parsing;
mod upload;

use parsing::parse_import_file_bytes;
use upload::decode_import_upload;

use a4n_core::error::AppError;

use crate::{AppState, actions::import_review};

use a4n_api_contract::import::ImportProjectTextUploadInput;
use a4n_api_contract::import_review::ImportReviewSummary;

pub async fn import_project_text_upload(
    state: &AppState,
    user_id: i64,
    input: ImportProjectTextUploadInput,
) -> Result<Vec<ImportReviewSummary>, AppError> {
    let upload = decode_import_upload(input).map_err(AppError::InvalidArgument)?;
    let chapters =
        parse_import_file_bytes(&upload.file_type, &upload.file_bytes, &upload.file_name)
            .map_err(AppError::InvalidArgument)?;
    a4n_orchestrator::import_analysis::create_import_analysis_reviews_for_user_validated(
        state.orchestrator(),
        user_id,
        upload.project_id,
        &upload.file_name,
        &chapters,
    )
    .await?
    .iter()
    .map(import_review::import_review_summary)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_import_file_bytes_accepts_markdown_as_plain_text() {
        let markdown = "# 第1章 开始\n开篇正文。\n\n## 第2章 继续\n后续正文。";

        let result = parse_import_file_bytes("markdown", markdown.as_bytes(), "测试文档")
            .expect("markdown should be parsed as text");

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].number, 1);
        assert_eq!(result[0].title, "第1章 开始");
        assert_eq!(result[1].title, "第2章 继续");
    }

    #[test]
    fn parse_import_file_bytes_rejects_unknown_file_type() {
        let err = parse_import_file_bytes("pdf", b"content", "测试文档")
            .expect_err("unsupported file type should fail");

        assert!(err.contains("不支持的文件类型: pdf"));
    }

    #[test]
    fn parse_import_file_bytes_does_not_duplicate_epub_headings() {
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut archive = zip::ZipWriter::new(&mut buffer);
            let options = zip::write::SimpleFileOptions::default();
            archive.start_file("OEBPS/content.opf", options).unwrap();
            archive
                .write_all(
                    br#"<package><manifest>
<item id="c1" href="c1.xhtml" media-type="application/xhtml+xml"/>
<item id="c2" href="c2.xhtml" media-type="application/xhtml+xml"/>
</manifest><spine><itemref idref="c1"/><itemref idref="c2"/></spine></package>"#,
                )
                .unwrap();
            archive.start_file("OEBPS/c1.xhtml", options).unwrap();
            archive
                .write_all(
                    r#"<html><head><title>第1章 开始</title></head><body><h1>第1章 开始</h1><p>开篇正文。</p></body></html>"#
                        .as_bytes(),
                )
                .unwrap();
            archive.start_file("OEBPS/c2.xhtml", options).unwrap();
            archive
                .write_all(
                    r#"<html><head><title>第2章 继续</title></head><body><h1>第2章 继续</h1><p>后续正文。</p></body></html>"#
                        .as_bytes(),
                )
                .unwrap();
            archive.finish().unwrap();
        }

        let bytes = buffer.into_inner();
        let parsed = a4n_import_core::file_parsers::EpubParser
            .parse(&bytes, "测试文档")
            .unwrap();
        assert_eq!(
            parsed.chapters.len(),
            2,
            "direct parser titles: {:?}",
            parsed
                .chapters
                .iter()
                .map(|chapter| chapter.title.as_str())
                .collect::<Vec<_>>()
        );
        let result = parse_import_file_bytes("epub", &bytes, "测试文档").unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].title, "第1章 开始");
        assert_eq!(result[1].title, "第2章 继续");
    }
}
