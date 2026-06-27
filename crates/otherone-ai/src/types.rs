// 作用：定义所有 AI 提供商通用的类型
// 关联：被各个 provider 子模块（openai、anthropic、fetch）导入使用
// 预期结果：提供统一的基础类型定义，各 provider 可以基于此扩展

use serde::{Deserialize, Serialize};

/// AI 提供商类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Fetch,
    OpenRouter,
    Local,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Anthropic => write!(f, "anthropic"),
            ProviderType::Fetch => write!(f, "fetch"),
            ProviderType::OpenRouter => write!(f, "openrouter"),
            ProviderType::Local => write!(f, "local"),
        }
    }
}

/// 消息角色
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
    Developer,
}

/// 通用消息结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// 消息内容（支持纯文本和多模态）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    MultiPart(Vec<ContentPart>),
}

/// 多模态内容的一个部分
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPart {
    #[serde(rename = "type")]
    pub content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<ImageUrl>,
}

/// 图片 URL 结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// 工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
    #[serde(default)]
    pub id: String,
    #[serde(rename = "type", default = "default_tool_call_type")]
    pub call_type: String,
    #[serde(default)]
    pub function: FunctionCall,
}

fn default_tool_call_type() -> String {
    "function".to_string()
}

/// 函数调用
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FunctionCall {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub arguments: String,
}

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

/// 函数定义（工具的 function 部分）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

/// 通用配置参数
#[derive(Debug, Clone)]
pub struct ConfigOptions {
    /// API 提供商类型
    pub provider: ProviderType,
    /// API 密钥
    pub api_key: String,
    /// 基础 URL
    pub base_url: String,
}

/// 通用聊天请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Map<String, serde_json::Value>>,
}

/// 工具选择策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    String(String),
    Object {
        #[serde(rename = "type")]
        choice_type: String,
        function: ToolChoiceFunction,
    },
}

/// 工具选择的函数指定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolChoiceFunction {
    pub name: String,
}

/// 通用聊天响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: Option<String>,
    pub object: Option<String>,
    pub created: Option<i64>,
    pub model: Option<String>,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

/// 响应中的单个选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Option<ResponseMessage>,
    pub delta: Option<ResponseDelta>,
    pub finish_reason: Option<String>,
}

/// 非流式响应消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMessage {
    pub role: Option<String>,
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// 流式响应的增量消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseDelta {
    pub role: Option<String>,
    pub content: Option<String>,
    #[serde(alias = "reasoningContent")]
    pub reasoning_content: Option<String>,
    pub reasoning: Option<String>,
    pub thinking: Option<String>,
    pub thought: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Token 使用量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

/// 统一解析后的响应（provider 无关）
#[derive(Debug, Clone)]
pub struct ParsedResponse {
    /// 消息内容
    pub content: String,
    /// 角色
    pub role: String,
    /// Token 消耗
    pub token_consumption: u32,
    /// 工具调用
    pub tools: Option<ToolCallsWrapper>,
    /// 思考内容
    pub thinking: Option<String>,
    /// 原始响应（保留供后续处理）
    pub raw_response: Option<serde_json::Value>,
}

/// 工具调用包装
#[derive(Debug, Clone)]
pub struct ToolCallsWrapper {
    pub tool_calls: Vec<ToolCall>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_delta_preserves_reasoning_content_fields() {
        let snake: ResponseDelta = serde_json::from_value(serde_json::json!({
            "role": "assistant",
            "reasoning_content": "thinking",
            "content": "answer"
        }))
        .unwrap();

        assert_eq!(snake.reasoning_content.as_deref(), Some("thinking"));
        assert_eq!(snake.content.as_deref(), Some("answer"));

        let camel: ResponseDelta = serde_json::from_value(serde_json::json!({
            "reasoningContent": "camel thinking"
        }))
        .unwrap();

        assert_eq!(camel.reasoning_content.as_deref(), Some("camel thinking"));

        let serialized = serde_json::to_value(&camel).unwrap();
        assert_eq!(serialized["reasoning_content"], "camel thinking");
    }
}
