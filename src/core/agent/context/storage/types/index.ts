/**
 * 作用：定义storage模块的类型
 * 关联：被storage/index.ts使用，定义存储相关的参数类型
 * 预期结果：提供清晰的类型定义
 */

// 存储类型定义
export type StorageType = 'localfile' | 'database';

// 写入entry的参数类型
export interface WriteEntryOptions {
    // 存储类型
    storageType: StorageType;
    // 会话ID
    sessionId: string;
    // 角色类型
    role: string;
    // 消息内容
    content: string;
    // 工具相关信息（可选）
    tools?: any;
    // token消耗量（可选）
    tokenConsumption?: number;
    // 创建时间（可选，默认当前时间）
    createAt?: string;
}

// 写入压缩记录的参数类型
export interface WriteCompactedEntryOptions {
    // 存储类型
    storageType: StorageType;
    // 会话ID
    sessionId: string;
    // 压缩摘要内容
    summary: string;
    // 触发压缩的entry_id
    triggerEntryId: string;
    // 创建时间（可选，默认当前时间）
    createAt?: string;
}
