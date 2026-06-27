// 作用：定义 tools 模块的类型
// 关联：被 tools/lib.rs 使用
// 预期结果：提供工具调用相关的类型定义

use serde::{Deserialize, Serialize};

/// Tools 配置参数类型
#[derive(Debug, Clone)]
pub struct ToolsOptions {
    /// 工具数组
    pub tools: Vec<otherone_ai::types::Tool>,
}

/// 工具调用结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// tool_call 的 ID
    pub tool_call_id: String,
    /// 函数名称
    pub function_name: String,
    /// 调用结果
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
