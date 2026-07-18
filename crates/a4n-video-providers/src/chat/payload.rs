/// chat-completions 视频响应中提取出的视频负载。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ChatVideoPayload {
    /// `data:video/<ext>;base64,XXX`，扩展名已知。
    DataUri { format: String, bytes: Vec<u8> },
    /// 纯 base64 字符串，未带 data URI 头，格式可能由响应字段额外指明。
    Base64 {
        format: Option<String>,
        bytes: Vec<u8>,
    },
    /// HTTP(S) URL，需要后续下载；`hint_format` 来自响应中显式的 format/content_type。
    Url {
        url: String,
        hint_format: Option<String>,
    },
}
