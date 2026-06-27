// 作用：定义 AI Provider trait 接口
// 关联：被 openai/anthropic/fetch 客户端实现
// 预期结果：提供统一的 AiProvider trait

use async_trait::async_trait;
use futures::stream::Stream;
use std::pin::Pin;

use crate::error::AiError;
use crate::types::{ChatRequest, ChatResponse};

/// 流式响应的 chunk 类型
pub type StreamChunk = Result<ChatResponse, AiError>;
/// 流式响应的 Stream 类型
pub type ChatStream = Pin<Box<dyn Stream<Item = StreamChunk> + Send>>;

/// AI Provider 核心 trait
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// 非流式聊天请求
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AiError>;

    /// 流式聊天请求
    async fn chat_stream(&self, request: ChatRequest) -> Result<ChatStream, AiError>;

    /// 返回 provider 类型
    fn provider_type(&self) -> crate::types::ProviderType;
}
