import { Pool, QueryResult } from 'pg';
import { DatabaseConfig } from './types';

/**
 * 作用：PostgreSQL数据库连接客户端
 * 关联：被reader.ts和writer.ts调用，提供数据库查询能力
 * 预期结果：返回包含query方法和连接池的db对象
 */
export function CreateDatabaseClient(config: DatabaseConfig) {
    // 参数校验
    if (!config.host) {
        throw new Error('host is required');
    }
    if (!config.port) {
        throw new Error('port is required');
    }
    if (!config.database) {
        throw new Error('database is required');
    }
    if (!config.user) {
        throw new Error('user is required');
    }
    if (!config.password) {
        throw new Error('password is required');
    }

    // 创建连接池
    const pool = new Pool({
        host: config.host,
        port: config.port,
        database: config.database,
        user: config.user,
        password: config.password,
        max: config.max || 10,
        idleTimeoutMillis: config.idleTimeoutMillis || 30000,
        connectionTimeoutMillis: config.connectionTimeoutMillis || 2000,
    });

    // 返回db对象
    const db = {
        // 执行查询
        query: async (text: string, params?: any[]): Promise<QueryResult> => {
            return await pool.query(text, params);
        },
        
        // 关闭连接池
        end: async (): Promise<void> => {
            await pool.end();
        },
        
        // 获取连接池实例
        pool: pool
    };

    return db;
}
