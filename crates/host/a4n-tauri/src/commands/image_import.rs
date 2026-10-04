use base64::{Engine, engine::general_purpose::STANDARD};
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;

const MAX_PICKED_IMAGE_BYTES: u64 = 50 * 1024 * 1024;

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
    let mime_type = image_mime_type(&path)?.to_owned();
    let mut bytes = Vec::new();
    tokio::fs::File::open(&path)
        .await
        .map_err(|error| format!("打开图片失败：{error}"))?
        .take(MAX_PICKED_IMAGE_BYTES + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|error| format!("读取图片失败：{error}"))?;
    if bytes.len() as u64 > MAX_PICKED_IMAGE_BYTES {
        return Err("图片文件不能超过 50 MiB".to_owned());
    }

    Ok(PickedImageFile {
        name,
        mime_type,
        size: bytes.len() as u64,
        data_base64: STANDARD.encode(bytes),
    })
}

fn image_mime_type(path: &Path) -> Result<&'static str, String> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .ok_or_else(|| "仅支持 PNG、JPG、JPEG 和 WebP 图片".to_owned())?;
    if extension.eq_ignore_ascii_case("png") {
        Ok("image/png")
    } else if extension.eq_ignore_ascii_case("jpg") || extension.eq_ignore_ascii_case("jpeg") {
        Ok("image/jpeg")
    } else if extension.eq_ignore_ascii_case("webp") {
        Ok("image/webp")
    } else {
        Err("仅支持 PNG、JPG、JPEG 和 WebP 图片".to_owned())
    }
}
