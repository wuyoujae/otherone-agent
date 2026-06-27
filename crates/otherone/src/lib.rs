// 作用：otherone 主 crate — 统一导出所有公开 API
// 关联：用户通过导入 otherone crate 来使用所有功能
// 预期结果：提供统一的 Otherone struct，封装所有 Agent 框架功能

use otherone_agent::types::{AiOptions, InputOptions};
use otherone_agent::{AgentStreamHandle, StreamAgentEvent};
use otherone_ai::types::ProviderType;
use tokio::sync::mpsc;

/// Otherone — 轻量级 AI Agent 框架的 Rust 实现
pub struct Otherone;

impl Otherone {
    /// Set the localfile storage root.
    ///
    /// By default localfile storage uses the current working directory. After
    /// setting a root, storage is written under
    /// `<root>/.otherone/storage/otherone-storage.json`.
    pub fn set_localfile_root(root: impl Into<std::path::PathBuf>) {
        otherone_storage::localfile::set_storage_root(root);
    }

    /// Clear the configured localfile storage root and restore current-dir behavior.
    pub fn clear_localfile_root() {
        otherone_storage::localfile::clear_storage_root();
    }

    /// Return the resolved localfile storage file path.
    pub fn localfile_storage_path() -> std::path::PathBuf {
        otherone_storage::localfile::get_storage_path()
    }

    /// 调用 Agent（非流式）
    pub async fn invoke_agent(
        input: &InputOptions,
        ai: &mut AiOptions,
        auxiliary_ai: Option<&mut AiOptions>,
    ) -> Result<otherone_ai::types::ParsedResponse, otherone_agent::error::AgentError> {
        otherone_agent::invoke_agent(input, ai, auxiliary_ai).await
    }

    /// 调用 Agent（流式）
    /// 作用：启动流式 Agent 循环，返回 mpsc Receiver 用于异步接收事件
    /// 关联：委托给 otherone_agent::invoke_agent_stream
    /// 预期结果：调用者通过 receiver 接收 StreamAgentEvent
    pub async fn invoke_agent_stream(
        input: InputOptions,
        ai: AiOptions,
        auxiliary_ai: Option<AiOptions>,
    ) -> Result<mpsc::Receiver<StreamAgentEvent>, otherone_agent::error::AgentError> {
        otherone_agent::invoke_agent_stream(input, ai, auxiliary_ai).await
    }

    /// 调用 Agent（可交互流式）
    /// 作用：返回事件接收器和运行中命令发送器，允许调用方在安全边界追加用户消息
    pub async fn invoke_agent_stream_interactive(
        input: InputOptions,
        ai: AiOptions,
        auxiliary_ai: Option<AiOptions>,
    ) -> Result<AgentStreamHandle, otherone_agent::error::AgentError> {
        otherone_agent::invoke_agent_stream_interactive(input, ai, auxiliary_ai).await
    }

    /// 调用 AI 模型（不经过 Agent 循环）
    pub async fn invoke_model(
        provider: ProviderType,
        api_key: &str,
        base_url: &str,
        config: serde_json::Value,
    ) -> Result<otherone_ai::types::ChatResponse, otherone_ai::error::AiError> {
        otherone_ai::invoke_model(provider, api_key, base_url, config).await
    }

    /// 处理工具调用
    pub fn process_tools(
        tool_calls: &[otherone_ai::types::ToolCall],
        tools_realize: &std::collections::HashMap<
            String,
            Box<dyn Fn(serde_json::Value) -> String + Send + Sync>,
        >,
    ) -> Result<Vec<otherone_tools::types::ToolResult>, String> {
        otherone_tools::process_tools(tool_calls, tools_realize)
    }

    /// 检查 Token 阈值
    pub fn check_threshold(
        context_tokens: u32,
        context_window: u32,
        threshold_percentage: Option<f32>,
    ) -> bool {
        otherone_context::compact::check_threshold::check_threshold(
            context_tokens,
            context_window,
            threshold_percentage,
        )
    }

    /// 估算 Token 数量
    pub fn estimate_tokens(messages: &[otherone_ai::types::Message]) -> u32 {
        otherone_context::compact::estimate_tokens::estimate_tokens(messages)
    }

    /// 创建新会话（本地文件）
    pub fn create_new_session() -> Result<String, otherone_storage::error::StorageError> {
        otherone_storage::localfile::writer::create_new_session()
    }

    /// 创建新会话（数据库）
    pub async fn create_new_session_in_database(
        config: &otherone_storage::types::DatabaseConfig,
    ) -> Result<String, otherone_storage::error::StorageError> {
        otherone_storage::database::writer::create_new_session_in_database(config).await
    }

    /// 获取所有会话（本地文件）
    pub fn get_all_sessions(
    ) -> Result<Vec<otherone_storage::types::Session>, otherone_storage::error::StorageError> {
        otherone_storage::localfile::reader::get_all_sessions()
    }

    /// 获取所有会话（数据库）
    pub async fn get_all_sessions_from_database(
        config: &otherone_storage::types::DatabaseConfig,
    ) -> Result<Vec<otherone_storage::types::Session>, otherone_storage::error::StorageError> {
        otherone_storage::database::reader::get_all_sessions_from_database(config).await
    }

    /// 读取会话数据（本地文件）
    pub fn read_session_data(
        session_id: &str,
    ) -> Result<otherone_storage::types::SessionData, otherone_storage::error::StorageError> {
        otherone_storage::localfile::reader::read_session_data(session_id)
    }

    /// 读取会话数据（数据库）
    pub async fn read_session_data_from_database(
        session_id: &str,
        config: &otherone_storage::types::DatabaseConfig,
    ) -> Result<otherone_storage::types::SessionData, otherone_storage::error::StorageError> {
        otherone_storage::database::reader::read_session_data_from_database(session_id, config)
            .await
    }

    /// 初始化数据库
    pub async fn init_database(
        config: &otherone_storage::types::DatabaseConfig,
    ) -> Result<(), otherone_storage::error::StorageError> {
        otherone_storage::database::init::init_database(config).await
    }

    /// 读取原始存储文件
    /// 作用：读取 .otherone/storage/otherone-storage.json 的完整内容
    /// 关联：被用户调用，用于调试或直接访问存储数据
    /// 预期结果：返回完整的 StorageFile 结构
    pub fn read_storage_file(
    ) -> Result<otherone_storage::types::StorageFile, otherone_storage::error::StorageError> {
        otherone_storage::localfile::reader::read_storage_file()
    }

    /// 组合上下文
    /// 作用：根据 session_id 加载历史消息，检查 token 阈值并触发压缩
    /// 关联：被 invoke_agent 内部调用，也可由用户手动调用
    /// 预期结果：返回 messages 数组（包含 system prompt）
    pub async fn combine_context(
        options: &otherone_context::types::CombineContextOptions,
    ) -> Result<Vec<otherone_ai::types::Message>, otherone_context::error::ContextError> {
        otherone_context::combine_context(options).await
    }

    /// 压缩上下文消息
    /// 作用：保留最新消息，压缩旧消息为摘要
    /// 关联：被 combine_context 内部调用，也可由用户手动调用
    /// 预期结果：返回压缩后的 messages 数组
    pub async fn compact_messages(
        messages: &[otherone_ai::types::Message],
        context_tokens: u32,
        context_window: u32,
        compact_ratio: Option<f32>,
        ai_config: Option<&serde_json::Value>,
        has_compacted_content: bool,
        session_id: Option<&str>,
        storage_type: Option<&otherone_storage::types::StorageType>,
        database_config: Option<&otherone_storage::types::DatabaseConfig>,
        original_entries: Option<&[otherone_storage::types::Entry]>,
    ) -> Result<Vec<otherone_ai::types::Message>, otherone_context::error::ContextError> {
        otherone_context::compact::compact_messages(
            messages,
            context_tokens,
            context_window,
            compact_ratio,
            ai_config,
            has_compacted_content,
            session_id,
            storage_type,
            database_config,
            original_entries,
        )
        .await
    }
}

// 重新导出常用类型
pub use otherone_agent::types::{ContextLoadType as AgentContextLoadType, StorageType};
pub use otherone_storage::database::mongodb;
pub use otherone_storage::database::mysql;
pub use otherone_storage::localfile::encrypt;
pub use otherone_storage::redis;
pub use otherone_storage::types::DatabaseConfig;

// 重新导出子模块
pub mod ai {
    pub use otherone_ai::*;
}
pub mod agent {
    pub use otherone_agent::*;
}
pub mod context {
    pub use otherone_context::*;
}
pub mod memory {
    pub use otherone_memory::*;
}
pub mod tools {
    pub use otherone_tools::*;
}
pub mod storage {
    pub use otherone_storage::*;
}
pub mod skills {
    pub use otherone_skills::*;
}
pub mod mcp {
    pub use otherone_mcp::*;
}
