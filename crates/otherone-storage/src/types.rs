// 作用：定义 storage 模块的类型
// 关联：被 storage/lib.rs、localfile、database 模块使用
// 预期结果：提供存储相关的类型定义

use serde::{Deserialize, Serialize};

/// 存储类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageType {
    LocalFile,
    Database,
    Redis,
    Mysql,
    Mongodb,
}

/// 数据库类型（PostgreSQL 或 MySQL）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatabaseType {
    Postgres,
    Mysql,
}

/// 数据库连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
    /// 连接池最大连接数（可选）
    pub max: Option<u32>,
    /// 空闲连接超时毫秒（可选）
    pub idle_timeout_millis: Option<u64>,
    /// 连接超时毫秒（可选）
    pub connection_timeout_millis: Option<u64>,
}

/// 写入 entry 的参数类型
#[derive(Debug, Clone)]
pub struct WriteEntryOptions {
    /// 存储类型
    pub storage_type: StorageType,
    /// 会话 ID
    pub session_id: String,
    /// 角色类型
    pub role: String,
    /// 消息内容
    pub content: String,
    /// 工具相关信息（可选）
    pub tools: Option<serde_json::Value>,
    /// token 消耗量（可选）
    pub token_consumption: Option<u32>,
    /// 创建时间（可选，默认当前时间）
    pub create_at: Option<String>,
    /// 数据库存储配置（当 storage_type 为 database 时必填）
    pub database_config: Option<DatabaseConfig>,
}

/// 写入压缩记录的参数类型
#[derive(Debug, Clone)]
pub struct WriteCompactedEntryOptions {
    /// 存储类型
    pub storage_type: StorageType,
    /// 会话 ID
    pub session_id: String,
    /// 压缩摘要内容
    pub summary: String,
    /// 触发压缩的 entry_id
    pub trigger_entry_id: String,
    /// 创建时间（可选，默认当前时间）
    pub create_at: Option<String>,
    /// 数据库存储配置（当 storage_type 为 database 时必填）
    pub database_config: Option<DatabaseConfig>,
}

/// Entry 数据模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub entry_id: String,
    pub session_id: String,
    pub content: String,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_consumption: Option<u32>,
    pub status: i16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<serde_json::Value>,
    pub create_at: String,
    pub is_compaction: i16,
}

/// Session 数据模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub status: i16,
    pub create_at: String,
}

/// Session 完整数据（包含 entries 和 compacted_entries）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub session: Option<Session>,
    pub entries: Vec<Entry>,
    pub compacted_entries: Vec<CompactedEntry>,
}

/// 压缩记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactedEntry {
    pub entry_id: String,
    pub session_id: String,
    pub trigger_entry_id: String,
    pub summary: String,
    pub create_at: String,
    pub status: i16,
}

/// 本地存储文件结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageFile {
    pub sessions: Vec<StorageSession>,
}

/// 本地存储中的 session（嵌入 entries 和 compacted_entries）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSession {
    pub session_id: String,
    pub status: i16,
    pub create_at: String,
    #[serde(default)]
    pub entries: Vec<Entry>,
    #[serde(default)]
    pub compacted_entries: Vec<CompactedEntry>,
}
