// 作用：PostgreSQL 数据库读取操作
// 关联：被 storage/lib.rs 和 combine_context 调用
// 预期结果：从 PostgreSQL 中查询 session 和 entries 数据

use crate::error::StorageError;
use crate::types::{CompactedEntry, DatabaseConfig, Entry, Session, SessionData};

use super::client::create_database_client;

/// 查询所有会话信息
/// 作用：获取所有活跃 session 的基本信息
/// 关联：被用户调用，用于获取所有会话列表
/// 预期结果：返回所有 session 的基本信息数组（不包含 entries 和 compacted_entries）
pub async fn get_all_sessions_from_database(
    config: &DatabaseConfig,
) -> Result<Vec<Session>, StorageError> {
    let pool = create_database_client(config).await?;

    let rows = sqlx::query_as::<_, (String, i16, chrono::NaiveDateTime)>(
        "SELECT session_id, status, create_at FROM otherone_session WHERE status = 0 ORDER BY create_at DESC",
    )
    .fetch_all(&pool)
    .await?;

    let sessions = rows
        .into_iter()
        .map(|(session_id, status, create_at)| Session {
            session_id,
            status,
            create_at: create_at.to_string(),
        })
        .collect();

    pool.close().await;
    Ok(sessions)
}

/// 根据 session_id 从数据库读取该会话的所有数据
/// 作用：读取指定会话的 session、entries 和 compacted_entries
/// 关联：被 combine_context 调用
/// 预期结果：返回包含该会话所有相关数据的对象，session 不存在则返回空数据结构
pub async fn read_session_data_from_database(
    session_id: &str,
    config: &DatabaseConfig,
) -> Result<SessionData, StorageError> {
    if session_id.is_empty() {
        return Err(StorageError::ConfigError(
            "session_id is required".to_string(),
        ));
    }

    let pool = create_database_client(config).await?;

    // 查询 session
    let session_result = sqlx::query_as::<_, (String, i16, chrono::NaiveDateTime)>(
        "SELECT session_id, status, create_at FROM otherone_session WHERE session_id = $1 AND status = 0",
    )
    .bind(session_id)
    .fetch_optional(&pool)
    .await?;

    let session = match session_result {
        None => {
            pool.close().await;
            return Ok(SessionData {
                session: None,
                entries: Vec::new(),
                compacted_entries: Vec::new(),
            });
        }
        Some((sid, status, create_at)) => Session {
            session_id: sid,
            status,
            create_at: create_at.to_string(),
        },
    };

    // 查询 entries
    let entries_rows = sqlx::query_as::<_, (String, String, String, String, Option<i32>, i16, Option<String>, chrono::NaiveDateTime, i16)>(
        "SELECT entry_id, session_id, content, role, token_consumption, status, tools, create_at, is_compaction \
         FROM otherone_entries WHERE session_id = $1 AND status = 0 ORDER BY create_at ASC",
    )
    .bind(session_id)
    .fetch_all(&pool)
    .await?;

    let entries: Vec<Entry> = entries_rows
        .into_iter()
        .map(
            |(
                entry_id,
                sid,
                content,
                role,
                token_consumption,
                status,
                tools,
                create_at,
                is_compaction,
            )| Entry {
                entry_id,
                session_id: sid,
                content,
                role,
                token_consumption: token_consumption.map(|t| t as u32),
                status,
                tools: tools.and_then(|t| serde_json::from_str(&t).ok()),
                create_at: create_at.to_string(),
                is_compaction,
            },
        )
        .collect();

    // 查询 compacted_entries
    let compacted_rows = sqlx::query_as::<_, (String, String, String, String, chrono::NaiveDateTime, i16)>(
        "SELECT entry_id, session_id, trigger_entry_id, summary, create_at, status \
         FROM otherone_compacted_entries WHERE session_id = $1 AND status = 0 ORDER BY create_at ASC",
    )
    .bind(session_id)
    .fetch_all(&pool)
    .await?;

    let compacted_entries: Vec<CompactedEntry> = compacted_rows
        .into_iter()
        .map(
            |(entry_id, sid, trigger_entry_id, summary, create_at, status)| CompactedEntry {
                entry_id,
                session_id: sid,
                trigger_entry_id,
                summary,
                create_at: create_at.to_string(),
                status,
            },
        )
        .collect();

    pool.close().await;

    Ok(SessionData {
        session: Some(session),
        entries,
        compacted_entries,
    })
}
