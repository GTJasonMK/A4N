use a4n_core::error::AppError;
use async_trait::async_trait;
use serde::Serialize;

/// 单次生成请求
#[derive(Debug, Clone, Serialize)]
pub struct ImageGenerationRequest {
    /// 完整 prompt（已合成）
    pub prompt: String,
    /// 输出宽度
    pub width: i32,
    /// 输出高度
    pub height: i32,
    /// 模型名（如 "dall-e-3"）
    pub model: String,
    /// 参考图。仅支持多模态图像后端；不支持的后端应显式报错。
    pub reference_images: Vec<ImageReference>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageReference {
    pub label: String,
    pub data_uri: String,
}

/// 单次生成结果
#[derive(Debug, Clone)]
pub struct ImageGenerationResult {
    /// 原始字节（PNG/JPEG/WEBP，由 provider 决定）
    pub bytes: Vec<u8>,
    /// 输出扩展名（不含点，如 "png"）
    pub format: String,
    /// 实际使用的模型
    pub model: String,
}

#[async_trait]
pub trait ImageProvider: Send + Sync {
    async fn generate(
        &self,
        req: ImageGenerationRequest,
    ) -> Result<ImageGenerationResult, AppError>;
}
