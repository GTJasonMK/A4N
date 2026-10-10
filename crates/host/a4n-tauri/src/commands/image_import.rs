use a4n_api_contract::image::MAX_UPLOAD_IMAGE_BYTES;
use a4n_domain::media::image_upload_format_from_extension;
use base64::{Engine, engine::general_purpose::STANDARD};
use std::path::PathBuf;
use tokio::io::AsyncReadExt;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickedImageFile {
    pub name: String,
    #[serde(rename = "type")]
    pub mime_type: String,
    pub size: u64,
    pub data_base64: String,
}

#[tauri::command]
pub async fn read_local_image_file(path: String) -> Result<PickedImageFile, String> {
    let path = PathBuf::from(path.trim());
    if path.as_os_str().is_empty() {
        return Err("图片路径不能为空".to_owned());
    }
    read_image_file(path).await
}

async fn read_image_file(path: PathBuf) -> Result<PickedImageFile, String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "图片文件名无效".to_owned())?
        .to_owned();
    let mime_type = path
        .extension()
        .and_then(|extension| extension.to_str())
        .and_then(image_upload_format_from_extension)
        .map(|format| format.mime_type)
        .ok_or_else(|| "仅支持 PNG、JPG、JPEG 和 WebP 图片".to_owned())?
        .to_owned();
    let mut bytes = Vec::new();
    tokio::fs::File::open(&path)
        .await
        .map_err(|error| format!("打开图片失败：{error}"))?
        .take(MAX_UPLOAD_IMAGE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|error| format!("读取图片失败：{error}"))?;
    if bytes.len() as u64 > MAX_UPLOAD_IMAGE_BYTES as u64 {
        return Err(format!(
            "图片文件不能超过 {} MiB",
            MAX_UPLOAD_IMAGE_BYTES / (1024 * 1024)
        ));
    }

    Ok(PickedImageFile {
        name,
        mime_type,
        size: bytes.len() as u64,
        data_base64: STANDARD.encode(bytes),
    })
}
