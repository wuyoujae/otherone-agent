import { WriteEntryOptions, WriteCompactedEntryOptions } from './types';
import { WriteEntryToFile, WriteCompactedEntryToFile } from './localfile/writer';

/**
 * 作用：写入entry数据的统一入口
 * 关联：被loop模块、compact模块调用，用于存储用户输入、AI响应、tool结果等
 * 预期结果：根据存储类型调用对应的writer实现
 */
export function WriteEntry(options: WriteEntryOptions): void {
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
            WriteEntryToDatabase(options);
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
export function WriteCompactedEntry(options: WriteCompactedEntryOptions): void {
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
            WriteCompactedEntryToDatabase(options);
            break;
        default:
            throw new Error(`Unsupported storageType: ${options.storageType}`);
    }
}

/**
 * 作用：将entry写入数据库
 * 关联：被WriteEntry调用
 * 预期结果：将entry数据插入数据库
 */
function WriteEntryToDatabase(options: WriteEntryOptions): void {
    // TODO: 实现数据库写入逻辑
}

/**
 * 作用：将压缩记录写入数据库
 * 关联：被WriteCompactedEntry调用
 * 预期结果：将压缩记录插入数据库
 */
function WriteCompactedEntryToDatabase(options: WriteCompactedEntryOptions): void {
    // TODO: 实现数据库写入逻辑
}
