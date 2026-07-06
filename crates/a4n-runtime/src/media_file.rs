use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaResponseStatus {
    Ok,
    PartialContent,
    RangeNotSatisfiable,
}

impl MediaResponseStatus {
    pub fn as_u16(self) -> u16 {
        match self {
            Self::Ok => 200,
            Self::PartialContent => 206,
            Self::RangeNotSatisfiable => 416,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaFileResponse {
    pub status: MediaResponseStatus,
    pub content_type: &'static str,
    pub content_length: Option<usize>,
    pub content_range: Option<String>,
    pub body: Vec<u8>,
}

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

pub fn requested_byte_range(value: &str, size: u64) -> Option<Result<(u64, u64), ()>> {
    let value = value.trim();
    let range = value.strip_prefix("bytes=")?;
    if range.contains(',') {
        return Some(Err(()));
    }
    let Some((start, end)) = range.split_once('-') else {
        return Some(Err(()));
    };
    if size == 0 {
        return Some(Err(()));
    }
    if start.is_empty() {
        let suffix_len = match end.parse::<u64>() {
            Ok(value) if value > 0 => value,
            _ => return Some(Err(())),
        };
        let len = suffix_len.min(size);
        return Some(Ok((size - len, size - 1)));
    }

    let start = match start.parse::<u64>() {
        Ok(value) => value,
        Err(_) => return Some(Err(())),
    };
    let end = if end.is_empty() {
        size - 1
    } else {
        match end.parse::<u64>() {
            Ok(value) => value,
            Err(_) => return Some(Err(())),
        }
    };
    if start >= size || end < start {
        Some(Err(()))
    } else {
        Some(Ok((start, end.min(size - 1))))
    }
}

pub fn requested_byte_range_header<E>(
    value: Option<Result<&str, E>>,
    size: u64,
) -> Option<Result<(u64, u64), ()>> {
    match value {
        Some(Ok(value)) => requested_byte_range(value, size),
        Some(Err(_)) => Some(Err(())),
        None => None,
    }
}

pub fn read_media_file_response<E>(
    path: &Path,
    range_header: Option<Result<&str, E>>,
    label: &str,
) -> Result<MediaFileResponse, String> {
    let mut file = std::fs::File::open(path)
        .map_err(|err| format!("无法打开{label}：{}：{err}", path.display()))?;
    let size = file
        .metadata()
        .map_err(|err| format!("无法读取{label}信息：{}：{err}", path.display()))?
        .len();
    let content_type = content_type_for_path(path);

    if let Some(range) = requested_byte_range_header(range_header, size) {
        return match range {
            Ok((start, end)) => {
                let len = end - start + 1;
                file.seek(SeekFrom::Start(start))
                    .map_err(|err| format!("无法定位{label}：{}：{err}", path.display()))?;
                let mut body = vec![0; len as usize];
                file.read_exact(&mut body)
                    .map_err(|err| format!("无法读取{label}：{}：{err}", path.display()))?;
                Ok(MediaFileResponse {
                    status: MediaResponseStatus::PartialContent,
                    content_type,
                    content_length: Some(len as usize),
                    content_range: Some(format!("bytes {start}-{end}/{size}")),
                    body,
                })
            }
            Err(()) => Ok(MediaFileResponse {
                status: MediaResponseStatus::RangeNotSatisfiable,
                content_type: "text/plain; charset=utf-8",
                content_length: None,
                content_range: Some(format!("bytes */{size}")),
                body: "请求的媒体文件范围无效".as_bytes().to_vec(),
            }),
        };
    }

    let mut body = Vec::new();
    file.read_to_end(&mut body)
        .map_err(|err| format!("无法读取{label}：{}：{err}", path.display()))?;
    Ok(MediaFileResponse {
        status: MediaResponseStatus::Ok,
        content_type,
        content_length: Some(size as usize),
        content_range: None,
        body,
    })
}

pub fn content_type_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("apng") => "image/apng",
        Some("avif") => "image/avif",
        Some("gif") => "image/gif",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        Some("mp4" | "m4v") => "video/mp4",
        Some("webm") => "video/webm",
        Some("mov") => "video/quicktime",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("ogg") => "audio/ogg",
        _ => "application/octet-stream",
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_media_uri_path_decodes_convert_file_src_path() {
        let path = relative_media_uri_path("/projects%2F1%2Fimages%2Fchapter_7%2Fimage.png")
            .expect("path should decode");

        assert_eq!(path, PathBuf::from("projects/1/images/chapter_7/image.png"));
    }

    #[test]
    fn relative_media_uri_path_rejects_absolute_path() {
        let err = relative_media_uri_path("/%2Fhome%2Ffufu%2FCode%2FA4N%2Fdata%2Fimage.png")
            .expect_err("absolute media path should fail");

        assert!(err.contains("相对路径"));
    }

    #[test]
    fn relative_media_uri_path_rejects_blank_path() {
        let err = relative_media_uri_path("/").expect_err("blank path should fail");

        assert!(err.contains("不能为空"));
    }

    #[test]
    fn requested_byte_range_supports_open_and_suffix_ranges() {
        assert_eq!(
            requested_byte_range("bytes=10-", 100).expect("range"),
            Ok((10, 99))
        );
        assert_eq!(
            requested_byte_range("bytes=-20", 100).expect("range"),
            Ok((80, 99))
        );
    }

    #[test]
    fn requested_byte_range_rejects_invalid_ranges() {
        assert_eq!(requested_byte_range("bytes=10-5", 100), Some(Err(())));
        assert_eq!(requested_byte_range("bytes=100-120", 100), Some(Err(())));
        assert_eq!(requested_byte_range("bytes=0-1,2-3", 100), Some(Err(())));
        assert_eq!(requested_byte_range("bytes=10", 100), Some(Err(())));
        assert_eq!(requested_byte_range("bytes=abc-20", 100), Some(Err(())));
        assert_eq!(requested_byte_range("bytes=-abc", 100), Some(Err(())));
        assert_eq!(requested_byte_range("bytes=", 100), Some(Err(())));
        assert_eq!(requested_byte_range("items=0-1", 100), None);
    }

    #[test]
    fn requested_byte_range_header_maps_invalid_header_to_invalid_range() {
        let valid: Option<Result<&str, ()>> = Some(Ok("bytes=10-"));
        let invalid: Option<Result<&str, ()>> = Some(Err(()));
        let absent: Option<Result<&str, ()>> = None;

        assert_eq!(
            requested_byte_range_header(valid, 100).expect("range"),
            Ok((10, 99))
        );
        assert_eq!(requested_byte_range_header(invalid, 100), Some(Err(())));
        assert_eq!(requested_byte_range_header(absent, 100), None);
    }

    #[test]
    fn content_type_for_known_media_extensions() {
        assert_eq!(content_type_for_path(Path::new("image.PNG")), "image/png");
        assert_eq!(content_type_for_path(Path::new("clip.mp4")), "video/mp4");
        assert_eq!(content_type_for_path(Path::new("voice.ogg")), "audio/ogg");
        assert_eq!(
            content_type_for_path(Path::new("archive.bin")),
            "application/octet-stream"
        );
    }
}
