import { CreateDatabaseClient } from './client';
import { DatabaseConfig } from './types';

/**
 * 作用：从数据库查询所有会话信息
 * 关联：被用户调用，用于获取所有会话列表
 * 预期结果：返回所有session的基本信息数组（不包含entries和compacted_entries）
 */
export async function GetAllSessionsFromDatabase(config: DatabaseConfig): Promise<any[]> {
    if (!config) {
        throw new Error('databaseConfig is required');
    }
    const db = CreateDatabaseClient(config);
    try {
        const result = await db.query(
            `SELECT session_id, status, create_at FROM veloca_session WHERE status = 0 ORDER BY create_at DESC`
        );
        return result.rows.map((row: any) => ({
            session_id: row.session_id,
            status: row.status,
            create_at: row.create_at
        }));
    } finally {
        await db.end();
    }
}

/**
 * 作用：根据session_id从数据库读取该会话的所有数据
 * 关联：被combineContext调用，读取指定会话的session、entries和compacted_entries
 * 预期结果：返回包含该会话所有相关数据的对象，如果session不存在则返回空数据结构
 */
export async function ReadSessionDataFromDatabase(
    sessionId: string,
    config: DatabaseConfig
): Promise<any> {
    if (!sessionId) {
        throw new Error('sessionId is required');
    }
    if (!config) {
        throw new Error('databaseConfig is required');
    }
    const db = CreateDatabaseClient(config);
    try {
        const sessionResult = await db.query(
            `SELECT session_id, status, create_at FROM veloca_session WHERE session_id = $1 AND status = 0`,
            [sessionId]
        );
        if (sessionResult.rows.length === 0) {
            return {
                session: null,
                entries: [],
                compacted_entries: []
            };
        }
        const session = {
            session_id: sessionResult.rows[0].session_id,
            status: sessionResult.rows[0].status,
            create_at: sessionResult.rows[0].create_at
        };
        const entriesResult = await db.query(
            `SELECT entry_id, session_id, content, role, token_consumption, status, tools, create_at, is_compaction
             FROM veloca_entries WHERE session_id = $1 AND status = 0 ORDER BY create_at ASC`,
            [sessionId]
        );
        const entries = entriesResult.rows.map((row: any) => ({
            entry_id: row.entry_id,
            session_id: row.session_id,
            content: row.content,
            role: row.role,
            token_consumption: row.token_consumption,
            status: row.status,
            tools: row.tools ? JSON.parse(row.tools) : null,
            create_at: row.create_at,
            is_compaction: row.is_compaction
        }));
        const compactedResult = await db.query(
            `SELECT entry_id, session_id, trigger_entry_id, summary, create_at, status
             FROM veloca_compacted_entries WHERE session_id = $1 AND status = 0 ORDER BY create_at ASC`,
            [sessionId]
        );
        const compacted_entries = compactedResult.rows.map((row: any) => ({
            entry_id: row.entry_id,
            session_id: row.session_id,
            trigger_entry_id: row.trigger_entry_id,
            summary: row.summary,
            create_at: row.create_at,
            status: row.status
        }));
        return {
            session,
            entries,
            compacted_entries
        };
    } finally {
        await db.end();
    }
}
