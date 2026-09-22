//! Pure prompt diagnostics for video provider requests.

use super::VideoGenerationRequest;

pub fn video_request_prompt_length_context(request: &VideoGenerationRequest) -> String {
    format!(
        "实际发送 prompt（不含图片/媒体请求体）：{} 字符 / {} UTF-8 字节",
        request.prompt.chars().count(),
        request.prompt.len()
    )
}

pub fn append_video_request_prompt_length_context(error: String, context: &str) -> String {
    if error.contains(context) {
        error
    } else {
        format!("{error}；{context}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VideoMediaInput;

    #[test]
    fn prompt_length_context_is_stable_and_idempotent() {
        let request = VideoGenerationRequest {
            prompt: "你好abc".into(),
            negative_prompt: None,
            width: 1280,
            height: 720,
            duration_secs: 5,
            fps: 24,
            model: "model".into(),
            media: VideoMediaInput::None,
        };
        let context = video_request_prompt_length_context(&request);
        assert_eq!(
            context,
            "实际发送 prompt（不含图片/媒体请求体）：5 字符 / 9 UTF-8 字节"
        );
        assert_eq!(
            append_video_request_prompt_length_context("provider failed".into(), &context),
            format!("provider failed；{context}")
        );
        assert_eq!(
            append_video_request_prompt_length_context(
                format!("provider failed；{context}"),
                &context,
            ),
            format!("provider failed；{context}")
        );
    }
}
