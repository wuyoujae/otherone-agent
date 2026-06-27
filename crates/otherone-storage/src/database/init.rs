// 作用：初始化数据库表结构
// 关联：被用户调用，用于首次创建 otherone 所需的数据库表
// 预期结果：在 PostgreSQL 中创建 otherone_session、otherone_entries、otherone_compacted_entries 三张表及索引

use tracing::info;

use crate::error::StorageError;
use crate::types::DatabaseConfig;

use super::client::create_database_client;

/// 初始化数据库表结构
/// 作用：创建 otherone 所需的三张表及索引（幂等操作，使用 IF NOT EXISTS）
/// 关联：被用户调用，在首次使用数据库存储前执行
/// 预期结果：数据库中创建好所有必要的表和索引
pub async fn init_database(config: &DatabaseConfig) -> Result<(), StorageError> {
    let pool = create_database_client(config).await?;

    // 创建会话表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS otherone_session (
            session_id VARCHAR(36) PRIMARY KEY,
            status SMALLINT NOT NULL DEFAULT 0,
            create_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // 创建会话表索引
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_session_status ON otherone_session(status)")
        .execute(&pool)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_session_create_at ON otherone_session(create_at)")
        .execute(&pool)
        .await?;

    // 创建消息记录表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS otherone_entries (
            entry_id VARCHAR(36) PRIMARY KEY,
            session_id VARCHAR(36) NOT NULL,
            content TEXT NOT NULL,
            role VARCHAR(50) NOT NULL,
            token_consumption INT DEFAULT 0,
            status SMALLINT NOT NULL DEFAULT 0,
            tools TEXT,
            create_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            is_compaction SMALLINT NOT NULL DEFAULT 1
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // 创建消息记录表索引
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_entries_session_id ON otherone_entries(session_id)",
    )
    .execute(&pool)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_entries_status ON otherone_entries(status)")
        .execute(&pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_entries_create_at ON otherone_entries(create_at)")
        .execute(&pool)
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_entries_is_compaction ON otherone_entries(is_compaction)",
    )
    .execute(&pool)
    .await?;

    // 创建压缩记录表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS otherone_compacted_entries (
            entry_id VARCHAR(36) PRIMARY KEY,
            session_id VARCHAR(36) NOT NULL,
            trigger_entry_id VARCHAR(36) NOT NULL,
            summary TEXT NOT NULL,
            create_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            status SMALLINT NOT NULL DEFAULT 0
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // 创建压缩记录表索引
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_compacted_session_id ON otherone_compacted_entries(session_id)",
    )
    .execute(&pool)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_compacted_trigger_entry_id ON otherone_compacted_entries(trigger_entry_id)",
    )
    .execute(&pool)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_compacted_status ON otherone_compacted_entries(status)",
    )
    .execute(&pool)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_compacted_create_at ON otherone_compacted_entries(create_at)",
    )
    .execute(&pool)
    .await?;

    pool.close().await;
    info!("Database initialized successfully");

    Ok(())
}
