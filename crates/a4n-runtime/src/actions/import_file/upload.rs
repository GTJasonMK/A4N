use base64::Engine as _;

use super::ImportUploadedFileInput;

const MAX_UPLOADED_IMPORT_FILE_BYTES: usize = 100 * 1024 * 1024;

#[derive(Debug)]
pub(super) struct DecodedImportUpload {
    pub(super) file_type: String,
    pub(super) file_bytes: Vec<u8>,
}

pub(super) fn decode_import_upload(
    input: ImportUploadedFileInput,
) -> Result<DecodedImportUpload, String> {
    if input.file_name.trim().is_empty() {
        return Err("上传导入文件缺少文件名".to_string());
    }
    let raw = strip_data_url_prefix(&input.data_base64).trim();
    if raw.is_empty() {
        return Err("上传导入文件数据不能为空".to_string());
    }
    let file_bytes = base64::engine::general_purpose::STANDARD
        .decode(raw)
        .map_err(|err| format!("上传导入文件 base64 解码失败：{err}"))?;
    if file_bytes.is_empty() {
        return Err("上传导入文件不能为空".to_string());
    }
    if file_bytes.len() > MAX_UPLOADED_IMPORT_FILE_BYTES {
        return Err(format!(
            "上传导入文件超过 {}MB",
            MAX_UPLOADED_IMPORT_FILE_BYTES / 1024 / 1024
        ));
    }
    Ok(DecodedImportUpload {
        file_type: input.file_type,
        file_bytes,
    })
}

fn strip_data_url_prefix(value: &str) -> &str {
    value
        .split_once(";base64,")
        .map(|(_, body)| body)
        .unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use base64::Engine as _;

    use super::*;

    fn upload(file_name: &str, data_base64: String) -> ImportUploadedFileInput {
        ImportUploadedFileInput {
            file_name: file_name.to_string(),
            file_type: "txt".to_string(),
            data_base64,
        }
    }

    #[test]
    fn decode_import_upload_accepts_data_url_prefix() {
        let body = base64::engine::general_purpose::STANDARD.encode(b"chapter");
        let decoded = decode_import_upload(upload(
            "novel.txt",
            format!("data:text/plain;base64,{body}"),
        ))
        .expect("data URL upload should decode");

        assert_eq!(decoded.file_type, "txt");
        assert_eq!(decoded.file_bytes, b"chapter");
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
}
