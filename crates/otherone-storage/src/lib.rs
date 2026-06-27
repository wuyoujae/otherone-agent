// 作用：存储模块统一入口
// 关联：被 otherone-agent、otherone-context、用户调用
// 预期结果：根据存储类型分发到不同后端实现

pub mod database;
pub mod error;
pub mod localfile;
pub mod redis;
pub mod types;

use error::StorageError;
use types::{StorageType, WriteCompactedEntryOptions, WriteEntryOptions};

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
            )?;
            Ok(())
        }
        StorageType::Database | StorageType::Mysql => {
            let db_config = options.database_config.as_ref().ok_or_else(|| {
                StorageError::ConfigError("database_config is required".to_string())
            })?;
            if matches!(options.storage_type, StorageType::Mysql) {
                database::mysql::write_entry_mysql(
                    db_config,
                    &options.session_id,
                    &options.role,
                    &options.content,
                    options.tools.as_ref(),
                    options.token_consumption,
                )
                .await?;
            } else {
                database::writer::write_entry_to_database(
                    db_config,
                    &options.session_id,
                    &options.role,
                    &options.content,
                    options.tools.as_ref(),
                    options.token_consumption,
                    options.create_at.as_deref(),
                )
                .await?;
            }
            Ok(())
        }
        StorageType::Mongodb => {
            let db_config = options.database_config.as_ref().ok_or_else(|| {
                StorageError::ConfigError("database_config is required".to_string())
            })?;
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
            )?;
            Ok(())
        }
        StorageType::Database | StorageType::Mysql | StorageType::Mongodb | StorageType::Redis => {
            let db_config = options.database_config.as_ref().ok_or_else(|| {
                StorageError::ConfigError("database_config is required".to_string())
            })?;
            if matches!(options.storage_type, StorageType::Database) {
                database::writer::write_compacted_entry_to_database(
                    db_config,
                    &options.session_id,
                    &options.summary,
                    &options.trigger_entry_id,
                    options.create_at.as_deref(),
                )
                .await?;
            }
            // MySQL/MongoDB/Redis compacted entries: basic support via the same interface
            Ok(())
        }
    }
}
