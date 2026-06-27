// 作用：MySQL 数据库存储模块
// 关联：被 storage/lib.rs 调用，与 PostgreSQL 共享 sqlx 接口
// 预期结果：提供 MySQL 数据库的完整存储支持

use chrono::Utc;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::error::StorageError;
use crate::types::{DatabaseConfig, Entry, Session, SessionData};

/// 创建 MySQL 连接池
pub async fn create_mysql_client(config: &DatabaseConfig) -> Result<MySqlPool, StorageError> {
    if config.host.is_empty() {
        return Err(StorageError::ConfigError("host is required".to_string()));
    }
    if config.port == 0 {
        return Err(StorageError::ConfigError("port is required".to_string()));
    }
    if config.database.is_empty() {
        return Err(StorageError::ConfigError(
            "database is required".to_string(),
        ));
    }
    if config.user.is_empty() {
        return Err(StorageError::ConfigError("user is required".to_string()));
    }
    if config.password.is_empty() {
        return Err(StorageError::ConfigError(
            "password is required".to_string(),
        ));
    }

    let database_url = format!(
        "mysql://{}:{}@{}:{}/{}",
        config.user, config.password, config.host, config.port, config.database
    );

    let pool = MySqlPoolOptions::new()
        .max_connections(config.max.unwrap_or(10))
        .connect(&database_url)
        .await?;

    Ok(pool)
}

/// 初始化 MySQL 数据库表
pub async fn init_mysql_database(config: &DatabaseConfig) -> Result<(), StorageError> {
    let pool = create_mysql_client(config).await?;

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS otherone_session (
            session_id VARCHAR(36) PRIMARY KEY,
            status SMALLINT NOT NULL DEFAULT 0,
            create_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4"#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS otherone_entries (
            entry_id VARCHAR(36) PRIMARY KEY,
            session_id VARCHAR(36) NOT NULL,
            content TEXT NOT NULL,
            role VARCHAR(50) NOT NULL,
            token_consumption INT DEFAULT 0,
            status SMALLINT NOT NULL DEFAULT 0,
            tools TEXT,
            create_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            is_compaction SMALLINT NOT NULL DEFAULT 1,
            INDEX idx_entries_session (session_id),
            INDEX idx_entries_status (status)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4"#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS otherone_compacted_entries (
            entry_id VARCHAR(36) PRIMARY KEY,
            session_id VARCHAR(36) NOT NULL,
            trigger_entry_id VARCHAR(36) NOT NULL,
            summary TEXT NOT NULL,
            create_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            status SMALLINT NOT NULL DEFAULT 0,
            INDEX idx_compacted_session (session_id)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4"#,
    )
    .execute(&pool)
    .await?;

    pool.close().await;
    Ok(())
}

/// MySQL 创建新 session
pub async fn create_session_mysql(config: &DatabaseConfig) -> Result<String, StorageError> {
    let pool = create_mysql_client(config).await?;
    let session_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO otherone_session (session_id, status) VALUES (?, ?)")
        .bind(&session_id)
        .bind(0i16)
        .execute(&pool)
        .await?;
    pool.close().await;
    Ok(session_id)
}

/// MySQL 写入 entry
pub async fn write_entry_mysql(
    config: &DatabaseConfig,
    session_id: &str,
    role: &str,
    content: &str,
    tools: Option<&serde_json::Value>,
    token_consumption: Option<u32>,
) -> Result<(), StorageError> {
    let pool = create_mysql_client(config).await?;
    let entry_id = Uuid::new_v4().to_string();
    let tools_json = tools.map(|t| serde_json::to_string(t).unwrap_or_default());

    sqlx::query("INSERT IGNORE INTO otherone_session (session_id, status) VALUES (?, ?)")
        .bind(session_id)
        .bind(0i16)
        .execute(&pool)
        .await?;

    sqlx::query(
        "INSERT INTO otherone_entries (entry_id, session_id, content, role, token_consumption, tools, create_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    ).bind(&entry_id).bind(session_id).bind(content).bind(role)
     .bind(token_consumption.map(|t| t as i32).unwrap_or(0))
     .bind(&tools_json).bind(Utc::now()).execute(&pool).await?;

    pool.close().await;
    Ok(())
}

/// MySQL 查询所有 session
pub async fn get_all_sessions_mysql(config: &DatabaseConfig) -> Result<Vec<Session>, StorageError> {
    let pool = create_mysql_client(config).await?;
    let rows = sqlx::query_as::<_, (String, i16, chrono::NaiveDateTime)>(
        "SELECT session_id, status, create_at FROM otherone_session WHERE status = 0 ORDER BY create_at DESC"
    ).fetch_all(&pool).await?;

    let sessions = rows
        .into_iter()
        .map(|(sid, status, ca)| Session {
            session_id: sid,
            status,
            create_at: ca.to_string(),
        })
        .collect();
    pool.close().await;
    Ok(sessions)
}

/// MySQL 读取 session 数据
pub async fn read_session_mysql(
    session_id: &str,
    config: &DatabaseConfig,
) -> Result<SessionData, StorageError> {
    if session_id.is_empty() {
        return Err(StorageError::ConfigError(
            "session_id is required".to_string(),
        ));
    }
    let pool = create_mysql_client(config).await?;

    let session_opt = sqlx::query_as::<_, (String, i16, chrono::NaiveDateTime)>(
        "SELECT session_id, status, create_at FROM otherone_session WHERE session_id = ? AND status = 0"
    ).bind(session_id).fetch_optional(&pool).await?;

    let session = match session_opt {
        None => {
            pool.close().await;
            return Ok(SessionData {
                session: None,
                entries: vec![],
                compacted_entries: vec![],
            });
        }
        Some((sid, status, ca)) => Session {
            session_id: sid,
            status,
            create_at: ca.to_string(),
        },
    };

    let entry_rows = sqlx::query_as::<_, (String, String, String, String, Option<i32>, i16, Option<String>, chrono::NaiveDateTime, i16)>(
        "SELECT entry_id, session_id, content, role, token_consumption, status, tools, create_at, is_compaction \
         FROM otherone_entries WHERE session_id = ? AND status = 0 ORDER BY create_at ASC"
    ).bind(session_id).fetch_all(&pool).await?;

    let entries: Vec<Entry> = entry_rows
        .into_iter()
        .map(|(eid, sid, c, r, tc, s, t, ca, ic)| Entry {
            entry_id: eid,
            session_id: sid,
            content: c,
            role: r,
            token_consumption: tc.map(|v| v as u32),
            status: s,
            tools: t.and_then(|v| serde_json::from_str(&v).ok()),
            create_at: ca.to_string(),
            is_compaction: ic,
        })
        .collect();

    pool.close().await;
    Ok(SessionData {
        session: Some(session),
        entries,
        compacted_entries: vec![],
    })
}
