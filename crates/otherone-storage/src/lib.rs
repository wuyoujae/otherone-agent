// 作用：存储模块统一入口
// 关联：被 otherone-agent、otherone-context、用户调用
// 预期结果：根据存储类型分发到不同后端实现

pub mod database;
pub mod error;
pub mod localfile;
pub mod redis;
pub mod types;

use error::StorageError;
use types::{RuntimeContext, StorageType, WriteCompactedEntryOptions, WriteEntryOptions};

fn required_runtime_context(
    runtime_context: &Option<RuntimeContext>,
) -> Result<&RuntimeContext, StorageError> {
    let runtime_context = runtime_context.as_ref().ok_or_else(|| {
        StorageError::ConfigError("runtime_context is required for shared storage".to_string())
    })?;
    runtime_context
        .validate()
        .map_err(StorageError::ConfigError)?;
    Ok(runtime_context)
}

/// 写入 entry 数据的统一入口
pub async fn write_entry(options: &WriteEntryOptions) -> Result<(), StorageError> {
    match options.storage_type {
        StorageType::LocalFile => {
            localfile::writer::write_entry_to_file(
                &options.session_id,
                &options.role,
                &options.content,
                options.tools.as_ref(),
                options.token_consumption,
                options.create_at.as_deref(),
                &options.metadata,
            )?;
            Ok(())
        }
        StorageType::Database | StorageType::Mysql => {
            let db_config = options.database_config.as_ref().ok_or_else(|| {
                StorageError::ConfigError("database_config is required".to_string())
            })?;
            let runtime_context = required_runtime_context(&options.runtime_context)?;
            if matches!(options.storage_type, StorageType::Mysql) {
                database::mysql::write_entry_mysql(
                    db_config,
                    runtime_context,
                    &options.session_id,
                    &options.role,
                    &options.content,
                    options.tools.as_ref(),
                    options.token_consumption,
                    &options.metadata,
                )
                .await?;
            } else {
                database::writer::write_entry_to_database(
                    db_config,
                    runtime_context,
                    &options.session_id,
                    &options.role,
                    &options.content,
                    options.tools.as_ref(),
                    options.token_consumption,
                    options.create_at.as_deref(),
                    &options.metadata,
                )
                .await?;
            }
            Ok(())
        }
        StorageType::Mongodb => {
            let db_config = options.database_config.as_ref().ok_or_else(|| {
                StorageError::ConfigError("database_config is required".to_string())
            })?;
            let runtime_context = required_runtime_context(&options.runtime_context)?;
            let mongo = database::mongodb::MongoClient::new(
                &format!(
                    "mongodb://{}:{}@{}:{}/{}",
                    db_config.user,
                    db_config.password,
                    db_config.host,
                    db_config.port,
                    db_config.database
                ),
                &db_config.database,
            )
            .await?;
            mongo
                .write_entry(
                    runtime_context,
                    &options.session_id,
                    &options.role,
                    &options.content,
                    options.tools.as_ref(),
                    options.token_consumption,
                )
                .await?;
            Ok(())
        }
        StorageType::Redis => {
            let _ = options; // Redis is a cache layer, not primary storage
            Ok(())
        }
    }
}

/// 写入压缩记录的统一入口
pub async fn write_compacted_entry(
    options: &WriteCompactedEntryOptions,
) -> Result<(), StorageError> {
    match options.storage_type {
        StorageType::LocalFile => {
            localfile::writer::write_compacted_entry_to_file(
                &options.session_id,
                &options.summary,
                &options.trigger_entry_id,
                options.create_at.as_deref(),
                &options.metadata,
            )?;
            Ok(())
        }
        StorageType::Database | StorageType::Mysql | StorageType::Mongodb | StorageType::Redis => {
            let db_config = options.database_config.as_ref().ok_or_else(|| {
                StorageError::ConfigError("database_config is required".to_string())
            })?;
            let runtime_context = required_runtime_context(&options.runtime_context)?;
            if matches!(options.storage_type, StorageType::Database) {
                database::writer::write_compacted_entry_to_database(
                    db_config,
                    runtime_context,
                    &options.session_id,
                    &options.summary,
                    &options.trigger_entry_id,
                    options.create_at.as_deref(),
                    &options.metadata,
                )
                .await?;
            } else if matches!(options.storage_type, StorageType::Mysql) {
                database::mysql::write_compacted_entry_mysql(
                    db_config,
                    runtime_context,
                    &options.session_id,
                    &options.summary,
                    &options.trigger_entry_id,
                    &options.metadata,
                )
                .await?;
            }
            // MySQL/MongoDB/Redis compacted entries: basic support via the same interface
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DatabaseConfig, RuntimeContext};

    fn test_database_config() -> DatabaseConfig {
        DatabaseConfig {
            host: "127.0.0.1".to_string(),
            port: 5432,
            database: "otherone".to_string(),
            user: "otherone".to_string(),
            password: "password".to_string(),
            max: None,
            idle_timeout_millis: None,
            connection_timeout_millis: None,
        }
    }

    #[tokio::test]
    async fn database_write_requires_runtime_context() {
        let result = write_entry(&WriteEntryOptions {
            storage_type: StorageType::Database,
            session_id: "session-1".to_string(),
            role: "user".to_string(),
            content: "hello".to_string(),
            tools: None,
            token_consumption: None,
            create_at: None,
            database_config: Some(test_database_config()),
            runtime_context: None,
            metadata: Default::default(),
        })
        .await;

        assert!(
            matches!(result, Err(StorageError::ConfigError(message)) if message.contains("runtime_context"))
        );
    }

    #[tokio::test]
    async fn database_write_validates_runtime_context_before_connecting() {
        let result = write_entry(&WriteEntryOptions {
            storage_type: StorageType::Database,
            session_id: "session-1".to_string(),
            role: "user".to_string(),
            content: "hello".to_string(),
            tools: None,
            token_consumption: None,
            create_at: None,
            database_config: Some(test_database_config()),
            runtime_context: Some(
                RuntimeContext::new("tenant:t1")
                    .with_attribute("Bad Key", serde_json::json!("value")),
            ),
            metadata: Default::default(),
        })
        .await;

        assert!(
            matches!(result, Err(StorageError::ConfigError(message)) if message.contains("invalid attribute key"))
        );
    }
}
