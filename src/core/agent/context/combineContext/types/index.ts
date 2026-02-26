/**
 * 作用：定义combineContext模块的类型
 * 关联：被combineContext/index.ts使用
 * 预期结果：提供清晰的类型定义
 */

// Context加载类型
export type ContextLoadType = 'database' | 'localfile';

// CombineContext参数类型定义
export interface CombineContextOptions {
    // 会话ID
    sessionId: string;
    // Context加载类型
    loadType: ContextLoadType;
}
