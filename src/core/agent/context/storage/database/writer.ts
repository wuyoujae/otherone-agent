import { v4 as uuidv4 } from 'uuid';
import { CreateDatabaseClient } from './client';
import { DatabaseConfig } from './types';

/**
 * 作用：在数据库中创建新会话
 * 关联：被用户调用，用于开始新的对话会话
 * 预期结果：生成新的session_id并插入veloca_session表，返回session_id
 */
export async function CreateNewSessionInDatabase(config: DatabaseConfig): Promise<string> {
    if (!config) {
        throw new Error('databaseConfig is required');
    }
    const db = CreateDatabaseClient(config);
    const sessionId = uuidv4();
    try {
        await db.query(
            `INSERT INTO veloca_session (session_id, status, create_at) VALUES ($1, $2, CURRENT_TIMESTAMP)`,
            [sessionId, 0]
        );
        return sessionId;
    } finally {
        await db.end();
    }
}

/**
 * 作用：将entry写入数据库
 * 关联：被storage/index.ts的WriteEntryToDatabase调用
 * 预期结果：将entry数据插入veloca_entries表
 */
export async function WriteEntryToDatabase(options: {
    databaseConfig: DatabaseConfig;
    sessionId: string;
    role: string;
    content: string;
    tools?: any;
    tokenConsumption?: number;
    createAt?: string;
}): Promise<void> {
    if (!options.databaseConfig) {
        throw new Error('databaseConfig is required');
    }
    if (!options.sessionId) {
        throw new Error('sessionId is required');
    }
    if (!options.role) {
        throw new Error('role is required');
    }
    const db = CreateDatabaseClient(options.databaseConfig);
    const entryId = uuidv4();
    const createAt = options.createAt ? new Date(options.createAt) : new Date();
    const toolsJson = options.tools ? JSON.stringify(options.tools) : null;
    try {
        await db.query(
            `INSERT INTO veloca_session (session_id, status, create_at) VALUES ($1, $2, CURRENT_TIMESTAMP)
             ON CONFLICT (session_id) DO NOTHING`,
            [options.sessionId, 0]
        );
        await db.query(
            `INSERT INTO veloca_entries (entry_id, session_id, content, role, token_consumption, status, tools, create_at, is_compaction)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)`,
            [
                entryId,
                options.sessionId,
                options.content,
                options.role,
                options.tokenConsumption ?? 0,
                0,
                toolsJson,
                createAt,
                1
            ]
        );
    } finally {
        await db.end();
    }
}

/**
 * 作用：将压缩记录写入数据库
 * 关联：被storage/index.ts的WriteCompactedEntryToDatabase调用
 * 预期结果：将压缩记录插入veloca_compacted_entries表
 */
export async function WriteCompactedEntryToDatabase(options: {
    databaseConfig: DatabaseConfig;
    sessionId: string;
    summary: string;
    triggerEntryId: string;
    createAt?: string;
}): Promise<void> {
    if (!options.databaseConfig) {
        throw new Error('databaseConfig is required');
    }
    if (!options.sessionId) {
        throw new Error('sessionId is required');
    }
    if (!options.summary) {
        throw new Error('summary is required');
    }
    if (!options.triggerEntryId) {
        throw new Error('triggerEntryId is required');
    }
    const db = CreateDatabaseClient(options.databaseConfig);
    const entryId = uuidv4();
    const createAt = options.createAt ? new Date(options.createAt) : new Date();
    try {
        await db.query(
            `INSERT INTO veloca_compacted_entries (entry_id, session_id, trigger_entry_id, summary, create_at, status)
             VALUES ($1, $2, $3, $4, $5, $6)`,
            [entryId, options.sessionId, options.triggerEntryId, options.summary, createAt, 0]
        );
    } finally {
        await db.end();
    }
}
