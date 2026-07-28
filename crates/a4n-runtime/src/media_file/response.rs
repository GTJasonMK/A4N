use super::range::requested_byte_range_header;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

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
        Some("m4a") => "audio/mp4",
        Some("wav") => "audio/wav",
        Some("ogg") => "audio/ogg",
        _ => "application/octet-stream",
    }
}
