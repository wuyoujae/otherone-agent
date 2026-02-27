/**
 * 作用：定义combineContext模块的类型
 * 关联：被combineContext/index.ts使用
 * 预期结果：提供清晰的类型定义
 */

// Context加载类型
export type ContextLoadType = 'database' | 'localfile';

// AI提供商类型
export type ProviderType = 'openai' | 'anthropic' | 'fetch';

// CombineContext参数类型定义
export interface CombineContextOptions {
    // 会话ID
    sessionId: string;
    // Context加载类型
    loadType: ContextLoadType;
    // AI提供商类型
    provider: ProviderType;
    // 模型的上下文窗口大小
    contextWindow: number;
    // 触发压缩的阈值百分比（默认0.8，即80%）
    thresholdPercentage?: number;
    // AI配置参数（用于压缩LLM调用）
    ai?: any;
    // 系统提示词
    systemPrompt?: string;
    // 工具定义数组
    tools?: any[];
}
