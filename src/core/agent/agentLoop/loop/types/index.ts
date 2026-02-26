/**
 * 作用：定义loop模块的类型
 * 关联：被loop/index.ts使用，定义InvokeAgent的参数类型
 * 预期结果：提供清晰的类型定义，便于后续扩展
 */

// Context加载类型
export type ContextLoadType = 'database' | 'localfile';

// Input参数类型定义
export interface InputOptions {
    // 会话ID
    sessionId: string;
    // Context加载类型
    contextLoadType: ContextLoadType;
}

// AI配置参数类型定义
export interface AIOptions {
    // 消息列表
    messages?: any[];
}
