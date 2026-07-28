use std::pin::Pin;

use a4n_core::error::AppError;
use async_trait::async_trait;
use futures::stream::Stream;

use super::contracts::{ChatChunk, ChatMessage, ModelCapabilities};
use super::request::ChatRequest;

/// LLM Provider trait
///
/// 所有 LLM 后端必须实现此 trait。
/// 提供流式聊天、连接测试和能力查询。
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// 流式聊天：发送消息列表，返回流式块
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, AppError>> + Send>>, AppError>;

    /// 测试连接可用性
    async fn test_connection(&self) -> Result<bool, AppError>;

    /// 获取模型名称
    fn model_name(&self) -> &str;

    /// 获取模型能力
    fn capabilities(&self) -> ModelCapabilities;

    /// 计算文本的 Token 数量
    fn count_tokens(&self, text: &str) -> usize;

    /// 计算消息列表的总 Token 数
    fn count_messages_tokens(&self, messages: &[ChatMessage]) -> usize {
        messages.iter().map(|m| self.count_tokens(&m.content)).sum()
    }

    /// 从消息列表中截断以适应上下文窗口
    ///
    /// 保留最新的消息，直到超出 Token 限制。
    fn truncate_messages(&self, messages: Vec<ChatMessage>, max_tokens: usize) -> Vec<ChatMessage> {
        let mut kept = Vec::new();
        let mut token_count = 0;
        for msg in messages.into_iter().rev() {
            let msg_tokens = self.count_tokens(&msg.content);
            if token_count + msg_tokens <= max_tokens {
                token_count += msg_tokens;
                kept.push(msg);
            } else {
                break;
            }
        }
        kept.reverse();
        kept
    }
}
