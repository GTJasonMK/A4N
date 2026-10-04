use base64::{Engine, engine::general_purpose::STANDARD};
use std::path::{Path, PathBuf};
use tauri::WebviewWindow;
use tauri_plugin_dialog::DialogExt;
use tokio::io::AsyncReadExt;

const MAX_PICKED_IMAGE_BYTES: u64 = 50 * 1024 * 1024;
const PICKED_IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];

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
pub async fn pick_image_file(window: WebviewWindow) -> Result<Option<PickedImageFile>, String> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    window
        .dialog()
        .file()
        .add_filter("Images", PICKED_IMAGE_EXTENSIONS)
        .pick_file(move |path| {
            let _ = sender.send(path);
        });

    let Some(file_path) = receiver
        .await
        .map_err(|error| format!("图片选择器未返回结果：{error}"))?
    else {
        return Ok(None);
    };
    let path = file_path
        .into_path()
        .map_err(|error| format!("无法读取所选图片路径：{error}"))?;

    read_image_file(path).await.map(Some)
}

async fn read_image_file(path: PathBuf) -> Result<PickedImageFile, String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "所选图片文件名无效".to_owned())?
        .to_owned();
    let mime_type = image_mime_type(&path)?.to_owned();
    let mut bytes = Vec::new();
    tokio::fs::File::open(&path)
        .await
        .map_err(|error| format!("打开所选图片失败：{error}"))?
        .take(MAX_PICKED_IMAGE_BYTES + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|error| format!("读取所选图片失败：{error}"))?;
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
        .ok_or_else(|| "所选文件不是支持的图片格式".to_owned())?;
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
