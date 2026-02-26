/**
 * 作用：定义storage模块的类型
 * 关联：被storage/index.ts使用，定义存储相关的参数类型
 * 预期结果：提供清晰的类型定义
 */

// 存储类型定义
export type StorageType = 'localfile' | 'database';

// Storage配置参数类型定义
export interface StorageOptions {
    // 存储类型
    type: StorageType;
}
