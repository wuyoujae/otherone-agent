// 作用：定义 Agent 循环模块的类型
// 关联：被 lib.rs 使用，定义 invoke_agent 的参数类型
// 预期结果：提供清晰的类型定义

use otherone_storage::types::DatabaseConfig;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Context 加载类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContextLoadType {
    Database,
    LocalFile,
}

/// 存储类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageType {
    LocalFile,
    Database,
}

/// Input 参数类型
#[derive(Debug, Clone)]
pub struct InputOptions {
    /// 会话 ID
    pub session_id: String,
    /// Context 加载类型
    pub context_load_type: ContextLoadType,
    /// 存储类型（可选，默认 localfile）
    pub storage_type: Option<StorageType>,
    /// 数据库存储配置（当 storage_type 为 database 或 context_load_type 为 database 时必填）
    pub database_config: Option<DatabaseConfig>,
    /// 模型的上下文窗口大小
    pub context_window: u32,
    /// 触发压缩的阈值百分比（默认 0.8，即 80%）
    pub threshold_percentage: Option<f32>,
    /// 最大循环次数（默认 999999）
    pub max_iterations: Option<u32>,
    /// 是否启用长期记忆分析和记忆工具调用
    pub enable_long_term_memory: Option<bool>,
    /// 长期记忆读取工具每次最多允许查询的 types 关键词数量（默认 5）
    pub long_term_memory_recall_max_types: Option<usize>,
}

/// AI 配置参数类型
pub struct AiOptions {
    /// AI 提供商类型
    pub provider: otherone_ai::types::ProviderType,
    /// API 密钥
    pub api_key: String,
    /// 基础 URL
    pub base_url: String,
    /// 模型名称
    pub model: String,
    /// 用户提示词（本次对话的用户输入）
    pub user_prompt: Option<String>,
    /// 系统提示词
    pub system_prompt: Option<String>,
    /// 消息列表
    pub messages: Option<Vec<otherone_ai::types::Message>>,
    /// 上下文长度限制
    pub context_length: Option<u32>,
    /// 采样温度
    pub temperature: Option<f32>,
    /// 核采样参数
    pub top_p: Option<f32>,
    /// 工具定义数组
    pub tools: Option<Vec<otherone_ai::types::Tool>>,
    /// 工具实现函数映射
    pub tools_realize: Option<
        std::collections::HashMap<String, Box<dyn Fn(serde_json::Value) -> String + Send + Sync>>,
    >,
    /// 控制工具调用行为
    pub tool_choice: Option<otherone_ai::types::ToolChoice>,
    /// 是否启用并行工具调用
    pub parallel_tool_calls: Option<bool>,
    /// 启用流式响应
    pub stream: Option<bool>,
    /// 其他兼容参数
    pub other: Option<serde_json::Value>,
}

/// 流式 Agent 事件类型
#[derive(Debug, Clone)]
pub struct StreamAgentEvent {
    /// 事件类型: "chunk" | "thinking" | "tool_calls" | "queued_user_prompts" | "complete" | "error"
    pub event_type: String,
    /// 事件内容
    pub content: String,
    /// 原始 chunk 数据（当 event_type 为 "chunk" 时）
    pub raw_chunk: Option<serde_json::Value>,
    /// 错误信息（当 event_type 为 "error" 时）
    pub error: Option<String>,
}

/// 运行中的流式 Agent 命令
#[derive(Debug, Clone)]
pub enum AgentStreamCommand {
    EnqueueUserPrompts(Vec<String>),
}

/// 可交互的流式 Agent 句柄
pub struct AgentStreamHandle {
    pub events: mpsc::Receiver<StreamAgentEvent>,
    pub commands: mpsc::Sender<AgentStreamCommand>,
}
