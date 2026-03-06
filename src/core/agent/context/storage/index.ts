import { WriteEntryOptions, WriteCompactedEntryOptions } from './types';
import { WriteEntryToFile, WriteCompactedEntryToFile } from './localfile/writer';
import { WriteEntryToDatabase, WriteCompactedEntryToDatabase } from './database/writer';

/**
 * 作用：写入entry数据的统一入口
 * 关联：被loop模块、compact模块调用，用于存储用户输入、AI响应、tool结果等
 * 预期结果：根据存储类型调用对应的writer实现
 */
export async function WriteEntry(options: WriteEntryOptions): Promise<void> {
    // 参数有效性检查
    if (!options.storageType) {
        throw new Error('storageType is required');
    }

    if (!options.sessionId) {
        throw new Error('sessionId is required');
    }

    if (!options.role) {
        throw new Error('role is required');
    }

    // 根据存储类型分发
    switch (options.storageType) {
        case 'localfile':
            WriteEntryToFile(options);
            break;
        case 'database':
            if (!options.databaseConfig) {
                throw new Error('databaseConfig is required when storageType is database');
            }
            await WriteEntryToDatabase({
                databaseConfig: options.databaseConfig,
                sessionId: options.sessionId,
                role: options.role,
                content: options.content,
                tools: options.tools,
                tokenConsumption: options.tokenConsumption,
                createAt: options.createAt
            });
            break;
        default:
            throw new Error(`Unsupported storageType: ${options.storageType}`);
    }
}

/**
 * 作用：写入压缩记录的统一入口
 * 关联：被compact模块调用，用于存储压缩后的摘要
 * 预期结果：根据存储类型调用对应的writer实现
 */
export async function WriteCompactedEntry(options: WriteCompactedEntryOptions): Promise<void> {
    // 参数有效性检查
    if (!options.storageType) {
        throw new Error('storageType is required');
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

    // 根据存储类型分发
    switch (options.storageType) {
        case 'localfile':
            WriteCompactedEntryToFile(options);
            break;
        case 'database':
            if (!options.databaseConfig) {
                throw new Error('databaseConfig is required when storageType is database');
            }
            await WriteCompactedEntryToDatabase({
                databaseConfig: options.databaseConfig,
                sessionId: options.sessionId,
                summary: options.summary,
                triggerEntryId: options.triggerEntryId,
                createAt: options.createAt
            });
            break;
        default:
            throw new Error(`Unsupported storageType: ${options.storageType}`);
    }
}
