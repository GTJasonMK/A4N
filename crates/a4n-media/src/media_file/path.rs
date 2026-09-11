use std::path::PathBuf;

pub fn relative_media_path(raw: &str) -> Result<PathBuf, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err("媒体文件路径不能为空".to_string());
    }
    let path = PathBuf::from(value);
    if path.is_absolute() {
        return Err("媒体文件路径必须是应用数据目录内的相对路径".to_string());
    }
    Ok(path)
}

pub fn relative_media_uri_path(uri_path: &str) -> Result<PathBuf, String> {
    let encoded = uri_path
        .strip_prefix('/')
        .filter(|path| !path.is_empty())
        .ok_or_else(|| "媒体文件路径不能为空".to_string())?;
    relative_media_path(&percent_decode(encoded)?)
}

fn percent_decode(value: &str) -> Result<String, String> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                let hi = bytes.get(index + 1).copied();
                let lo = bytes.get(index + 2).copied();
                let (Some(hi), Some(lo)) = (hi, lo) else {
                    return Err("媒体文件路径包含无效转义".to_string());
                };
                let hi = hex_value(hi).ok_or_else(|| "媒体文件路径包含无效转义".to_string())?;
                let lo = hex_value(lo).ok_or_else(|| "媒体文件路径包含无效转义".to_string())?;
                output.push((hi << 4) | lo);
                index += 3;
            }
            b'+' => {
                output.push(b'+');
                index += 1;
            }
            byte => {
                output.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(output).map_err(|_| "媒体文件路径不是有效 UTF-8".to_string())
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
