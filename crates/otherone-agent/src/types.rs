use otherone_storage::types::{DatabaseConfig, RuntimeContext};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContextLoadType {
    Database,
    LocalFile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageType {
    LocalFile,
    Database,
}

#[derive(Debug, Clone)]
pub struct InputOptions {
    pub session_id: String,
    pub context_load_type: ContextLoadType,
    pub storage_type: Option<StorageType>,
    pub database_config: Option<DatabaseConfig>,
    pub runtime_context: Option<RuntimeContext>,
    pub context_window: u32,
    pub threshold_percentage: Option<f32>,
    pub max_iterations: Option<u32>,
    pub enable_long_term_memory: Option<bool>,
    pub long_term_memory_recall_max_types: Option<usize>,
}

pub struct AiOptions {
    pub provider: otherone_ai::types::ProviderType,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub user_prompt: Option<String>,
    pub system_prompt: Option<String>,
    pub messages: Option<Vec<otherone_ai::types::Message>>,
    pub context_length: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub tools: Option<Vec<otherone_ai::types::Tool>>,
    pub tools_realize: Option<
        std::collections::HashMap<String, Box<dyn Fn(serde_json::Value) -> String + Send + Sync>>,
    >,
    pub tool_choice: Option<otherone_ai::types::ToolChoice>,
    pub parallel_tool_calls: Option<bool>,
    pub stream: Option<bool>,
    pub other: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct StreamAgentEvent {
    pub event_type: String,
    pub content: String,
    pub raw_chunk: Option<serde_json::Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum AgentStreamCommand {
    EnqueueUserPrompts(Vec<String>),
}

pub struct AgentStreamHandle {
    pub events: mpsc::Receiver<StreamAgentEvent>,
    pub commands: mpsc::Sender<AgentStreamCommand>,
}
