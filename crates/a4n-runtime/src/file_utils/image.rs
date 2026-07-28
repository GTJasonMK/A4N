use std::path::Path;

pub(crate) fn decoded_image_dimensions(bytes: &[u8], label: &str) -> Result<(i32, i32), String> {
    if bytes.is_empty() {
        return Err(format!("{label}为空，无法读取真实尺寸"));
    }
    let image = image::load_from_memory(bytes)
        .map_err(|error| format!("{label}不是可解码的图片：{error}"))?;
    checked_image_dimensions(image.width(), image.height(), label)
}

pub(crate) fn image_file_dimensions(path: &Path, label: &str) -> Result<(i32, i32), String> {
    let (width, height) = image::image_dimensions(path)
        .map_err(|error| format!("无法读取{label}真实尺寸：{error}"))?;
    checked_image_dimensions(width, height, label)
}

fn checked_image_dimensions(width: u32, height: u32, label: &str) -> Result<(i32, i32), String> {
    if width == 0 || height == 0 {
        return Err(format!("{label}真实尺寸无效：{width}x{height}"));
    }
    let width = i32::try_from(width).map_err(|_| format!("{label}宽度超过 i32 范围"))?;
    let height = i32::try_from(height).map_err(|_| format!("{label}高度超过 i32 范围"))?;
    Ok((width, height))
}
