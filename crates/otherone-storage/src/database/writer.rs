// 作用：PostgreSQL 数据库写入操作
// 关联：被 storage/lib.rs 调用
// 预期结果：将 entry 和 compacted_entry 数据写入 PostgreSQL 数据库

use chrono::Utc;
use uuid::Uuid;

use crate::error::StorageError;
use crate::types::DatabaseConfig;

use super::client::create_database_client;

/// 在数据库中创建新会话
/// 作用：生成新的 session_id 并插入 otherone_session 表
/// 关联：被用户调用，用于开始新的对话会话
/// 预期结果：返回新生成的 session_id
pub async fn create_new_session_in_database(
    config: &DatabaseConfig,
) -> Result<String, StorageError> {
    let pool = create_database_client(config).await?;

    let session_id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO otherone_session (session_id, status, create_at) VALUES ($1, $2, CURRENT_TIMESTAMP)",
    )
    .bind(&session_id)
    .bind(0i16)
    .execute(&pool)
    .await?;

    pool.close().await;
    Ok(session_id)
}

/// 将 entry 写入数据库
/// 作用：将 entry 数据插入 otherone_entries 表
/// 关联：被 storage/lib.rs 的 write_entry 调用
/// 预期结果：entry 数据成功写入数据库
pub async fn write_entry_to_database(
    config: &DatabaseConfig,
    session_id: &str,
    role: &str,
    content: &str,
    tools: Option<&serde_json::Value>,
    token_consumption: Option<u32>,
    create_at: Option<&str>,
) -> Result<(), StorageError> {
    let pool = create_database_client(config).await?;

    let entry_id = Uuid::new_v4().to_string();
    let create_at_dt = match create_at {
        Some(s) => chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.fZ")
            .unwrap_or_else(|_| Utc::now().naive_utc()),
        None => Utc::now().naive_utc(),
    };
    let tools_json = tools.map(|t| serde_json::to_string(t).unwrap_or_default());

    // 确保 session 存在（幂等 upsert）
    sqlx::query(
        "INSERT INTO otherone_session (session_id, status, create_at) VALUES ($1, $2, CURRENT_TIMESTAMP) \
         ON CONFLICT (session_id) DO NOTHING",
    )
    .bind(session_id)
    .bind(0i16)
    .execute(&pool)
    .await?;

    // 写入 entry
    sqlx::query(
        "INSERT INTO otherone_entries (entry_id, session_id, content, role, token_consumption, status, tools, create_at, is_compaction) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(&entry_id)
    .bind(session_id)
    .bind(content)
    .bind(role)
    .bind(token_consumption.map(|t| t as i32).unwrap_or(0))
    .bind(0i16)
    .bind(&tools_json)
    .bind(create_at_dt)
    .bind(1i16)
    .execute(&pool)
    .await?;

    pool.close().await;
    Ok(())
}

/// 将压缩记录写入数据库
/// 作用：将压缩记录插入 otherone_compacted_entries 表
/// 关联：被 storage/lib.rs 的 write_compacted_entry 调用
/// 预期结果：压缩记录成功写入数据库
pub async fn write_compacted_entry_to_database(
    config: &DatabaseConfig,
    session_id: &str,
    summary: &str,
    trigger_entry_id: &str,
    create_at: Option<&str>,
) -> Result<(), StorageError> {
    let pool = create_database_client(config).await?;

    let entry_id = Uuid::new_v4().to_string();
    let create_at_dt = match create_at {
        Some(s) => chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.fZ")
            .unwrap_or_else(|_| Utc::now().naive_utc()),
        None => Utc::now().naive_utc(),
    };

    sqlx::query(
        "INSERT INTO otherone_compacted_entries (entry_id, session_id, trigger_entry_id, summary, create_at, status) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&entry_id)
    .bind(session_id)
    .bind(trigger_entry_id)
    .bind(summary)
    .bind(create_at_dt)
    .bind(0i16)
    .execute(&pool)
    .await?;

    pool.close().await;
    Ok(())
}
