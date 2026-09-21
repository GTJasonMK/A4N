use base64::Engine as _;
use std::path::Path;

use a4n_api_contract::import::MAX_PROJECT_TEXT_UPLOAD_BYTES;

use super::ImportProjectTextUploadInput;

#[derive(Debug)]
pub(super) struct DecodedImportUpload {
    pub(super) project_id: i64,
    pub(super) file_name: String,
    pub(super) file_type: String,
    pub(super) file_bytes: Vec<u8>,
}

pub(super) fn decode_import_upload(
    input: ImportProjectTextUploadInput,
) -> Result<DecodedImportUpload, String> {
    if input.project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    if input.file_name.trim().is_empty() {
        return Err("上传导入文件缺少文件名".to_string());
    }
    let file_type = import_file_type_from_name(&input.file_name)?;
    let raw = input.data_base64.trim();
    if raw.is_empty() {
        return Err("上传导入文件数据不能为空".to_string());
    }
    let max_base64_len = MAX_PROJECT_TEXT_UPLOAD_BYTES.div_ceil(3) * 4;
    if raw.len() > max_base64_len {
        return Err(format!(
            "上传导入文件超过 {}MB",
            MAX_PROJECT_TEXT_UPLOAD_BYTES / 1024 / 1024
        ));
    }
    let file_bytes = base64::engine::general_purpose::STANDARD
        .decode(raw)
        .map_err(|err| format!("上传导入文件 base64 解码失败：{err}"))?;
    if file_bytes.is_empty() {
        return Err("上传导入文件不能为空".to_string());
    }
    if file_bytes.len() > MAX_PROJECT_TEXT_UPLOAD_BYTES {
        return Err(format!(
            "上传导入文件超过 {}MB",
            MAX_PROJECT_TEXT_UPLOAD_BYTES / 1024 / 1024
        ));
    }
    Ok(DecodedImportUpload {
        project_id: input.project_id,
        file_name: input.file_name,
        file_type,
        file_bytes,
    })
}

fn import_file_type_from_name(file_name: &str) -> Result<String, String> {
    let extension = Path::new(file_name.trim())
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| "导入文件缺少受支持的扩展名".to_string())?;
    match extension.as_str() {
        "txt" | "epub" | "docx" => Ok(extension),
        "md" | "markdown" => Ok("markdown".to_string()),
        _ => Err(format!("不支持的导入文件扩展名: .{extension}")),
    }
}

#[cfg(test)]
mod tests {
    use base64::Engine as _;

    use super::*;

    fn upload(file_name: &str, data_base64: String) -> ImportProjectTextUploadInput {
        ImportProjectTextUploadInput {
            project_id: 1,
            file_name: file_name.to_string(),
            data_base64,
        }
    }

    #[test]
    fn decode_import_upload_rejects_blank_file_name_before_decoding() {
        let err = decode_import_upload(upload("  ", "not-base64".to_string()))
            .expect_err("blank filename should be the first error");

        assert_eq!(err, "上传导入文件缺少文件名");
    }

    #[test]
    fn decode_import_upload_rejects_invalid_base64() {
        let err = decode_import_upload(upload("novel.txt", "not-base64".to_string()))
            .expect_err("invalid base64 should fail");

        assert!(err.starts_with("上传导入文件 base64 解码失败："));
    }

    #[test]
    fn decode_import_upload_infers_markdown_from_extension() {
        let body = base64::engine::general_purpose::STANDARD.encode(b"chapter");
        let decoded = decode_import_upload(upload("novel.MD", body)).unwrap();

        assert_eq!(decoded.file_type, "markdown");
    }
}
