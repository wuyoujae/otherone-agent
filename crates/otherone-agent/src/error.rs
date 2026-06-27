// 作用：Agent 循环模块的错误类型
// 关联：被 otherone-agent 内部使用

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("AI error: {0}")]
    AiError(#[from] otherone_ai::error::AiError),

    #[error("Context error: {0}")]
    ContextError(String),

    #[error("Tool error: {0}")]
    ToolError(String),

    #[error("Max iterations exceeded: {0}")]
    MaxIterationsExceeded(u32),
}
