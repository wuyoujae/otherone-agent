// 作用：定义 combine_context 模块的类型
// 关联：被 lib.rs 使用
// 预期结果：提供清晰的类型定义

use otherone_storage::types::DatabaseConfig;

/// Context 加载类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextLoadType {
    Database,
    LocalFile,
}

/// CombineContext 参数类型
#[derive(Debug, Clone)]
pub struct CombineContextOptions {
    /// 会话 ID
    pub session_id: String,
    /// Context 加载类型
    pub load_type: ContextLoadType,
    /// AI 提供商类型
    pub provider: otherone_ai::types::ProviderType,
    /// 模型的上下文窗口大小
    pub context_window: u32,
    /// 触发压缩的阈值百分比（默认 0.8，即 80%）
    pub threshold_percentage: Option<f32>,
    /// AI 配置参数（用于压缩 LLM 调用）
    pub ai: Option<serde_json::Value>,
    /// 系统提示词
    pub system_prompt: Option<String>,
    /// 工具定义数组
    pub tools: Option<Vec<otherone_ai::types::Tool>>,
    /// 数据库存储配置（当 load_type 为 database 时必填）
    pub database_config: Option<DatabaseConfig>,
}
