import { CreateDatabaseClient } from './client';
import { DatabaseConfig } from './types';

/**
 * 作用：初始化数据库表结构
 * 关联：被用户调用，用于首次创建veloca所需的数据库表
 * 预期结果：在PostgreSQL中创建veloca_session、veloca_entries、veloca_compacted_entries三张表
 */
export async function InitDatabase(config: DatabaseConfig): Promise<void> {
    // 创建数据库客户端
    const db = CreateDatabaseClient(config);

    try {
        // 创建会话表
        await db.query(`
            CREATE TABLE IF NOT EXISTS veloca_session (
                session_id VARCHAR(36) PRIMARY KEY,
                status SMALLINT NOT NULL DEFAULT 0,
                create_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
        `);

        // 创建会话表索引
        await db.query(`
            CREATE INDEX IF NOT EXISTS idx_session_status ON veloca_session(status)
        `);
        await db.query(`
            CREATE INDEX IF NOT EXISTS idx_session_create_at ON veloca_session(create_at)
        `);

        // 创建消息记录表
        await db.query(`
            CREATE TABLE IF NOT EXISTS veloca_entries (
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
        `);

        // 创建消息记录表索引
        await db.query(`
            CREATE INDEX IF NOT EXISTS idx_entries_session_id ON veloca_entries(session_id)
        `);
        await db.query(`
            CREATE INDEX IF NOT EXISTS idx_entries_status ON veloca_entries(status)
        `);
        await db.query(`
            CREATE INDEX IF NOT EXISTS idx_entries_create_at ON veloca_entries(create_at)
        `);
        await db.query(`
            CREATE INDEX IF NOT EXISTS idx_entries_is_compaction ON veloca_entries(is_compaction)
        `);

        // 创建压缩记录表
        await db.query(`
            CREATE TABLE IF NOT EXISTS veloca_compacted_entries (
                entry_id VARCHAR(36) PRIMARY KEY,
                session_id VARCHAR(36) NOT NULL,
                trigger_entry_id VARCHAR(36) NOT NULL,
                summary TEXT NOT NULL,
                create_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                status SMALLINT NOT NULL DEFAULT 0
            )
        `);

        // 创建压缩记录表索引
        await db.query(`
            CREATE INDEX IF NOT EXISTS idx_compacted_session_id ON veloca_compacted_entries(session_id)
        `);
        await db.query(`
            CREATE INDEX IF NOT EXISTS idx_compacted_trigger_entry_id ON veloca_compacted_entries(trigger_entry_id)
        `);
        await db.query(`
            CREATE INDEX IF NOT EXISTS idx_compacted_status ON veloca_compacted_entries(status)
        `);
        await db.query(`
            CREATE INDEX IF NOT EXISTS idx_compacted_create_at ON veloca_compacted_entries(create_at)
        `);

        console.log('Database initialized successfully');
    } catch (error) {
        console.error('Failed to initialize database:', error);
        throw error;
    } finally {
        // 关闭连接
        await db.end();
    }
}
