/**
 * 作用：定义数据库相关的类型
 * 关联：被database模块使用
 * 预期结果：提供数据库连接配置和查询相关的类型定义
 */

// 数据库连接配置
export interface DatabaseConfig {
    host: string;
    port: number;
    database: string;
    user: string;
    password: string;
    // 连接池配置（可选）
    max?: number;
    idleTimeoutMillis?: number;
    connectionTimeoutMillis?: number;
}
