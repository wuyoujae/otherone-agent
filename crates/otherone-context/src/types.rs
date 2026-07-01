use otherone_storage::types::{DatabaseConfig, RuntimeContext};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextLoadType {
    Database,
    LocalFile,
}

#[derive(Debug, Clone)]
pub struct CombineContextOptions {
    pub session_id: String,
    pub load_type: ContextLoadType,
    pub provider: otherone_ai::types::ProviderType,
    pub context_window: u32,
    pub threshold_percentage: Option<f32>,
    pub ai: Option<serde_json::Value>,
    pub system_prompt: Option<String>,
    pub tools: Option<Vec<otherone_ai::types::Tool>>,
    pub database_config: Option<DatabaseConfig>,
    pub runtime_context: Option<RuntimeContext>,
}
