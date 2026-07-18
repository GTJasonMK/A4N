use std::collections::HashMap;

use crate::ImportTransformError;
use zip::result::ZipError;

use super::super::shared::extract_document_metadata;
use super::document::extract_docx_document_text;

pub(super) struct DocxArchiveContent {
    pub text_content: String,
    pub metadata: HashMap<String, String>,
}

pub(super) fn read_docx_archive(data: &[u8]) -> Result<DocxArchiveContent, ImportTransformError> {
    let reader = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| ImportTransformError::InvalidDocument(format!("无法打开 DOCX 文件: {e}")))?;

    let mut doc_file = archive.by_name("word/document.xml").map_err(|e| {
        ImportTransformError::InvalidDocument(format!(
            "DOCX 缺少 word/document.xml 或无法读取: {e}"
        ))
    })?;
    let mut xml_content = String::new();
    std::io::Read::read_to_string(&mut doc_file, &mut xml_content)
        .map_err(|e| ImportTransformError::InvalidDocument(format!("无法读取 DOCX 内容: {e}")))?;
    drop(doc_file);

    let text_content = extract_docx_document_text(&xml_content);
    let mut metadata = HashMap::new();

    match archive.by_name("docProps/core.xml") {
        Ok(mut meta_file) => {
            let mut xml = String::new();
            std::io::Read::read_to_string(&mut meta_file, &mut xml).map_err(|e| {
                ImportTransformError::InvalidDocument(format!("无法读取 DOCX 元数据: {e}"))
            })?;
            metadata.extend(extract_document_metadata(&xml));
        }
        Err(ZipError::FileNotFound) => {}
        Err(e) => {
            return Err(ImportTransformError::InvalidDocument(format!(
                "无法读取 DOCX 元数据条目: {e}"
            )));
        }
    }

    Ok(DocxArchiveContent {
        text_content,
        metadata,
    })
}
