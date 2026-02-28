/**
 * 作用：定义loop模块的类型
 * 关联：被loop/index.ts使用，定义InvokeAgent的参数类型
 * 预期结果：提供清晰的类型定义，便于后续扩展
 */

// Context加载类型
export type ContextLoadType = 'database' | 'localfile';

// 存储类型
export type StorageType = 'localfile' | 'database';

// AI提供商类型
export type ProviderType = 'openai' | 'anthropic' | 'fetch';

// Input参数类型定义
export interface InputOptions {
    // 会话ID
    sessionId: string;
    // Context加载类型
    contextLoadType: ContextLoadType;
    // 存储类型（可选，默认localfile）
    storageType?: StorageType;
    // 模型的上下文窗口大小
    contextWindow: number;
    // 触发压缩的阈值百分比（默认0.8，即80%）
    thresholdPercentage?: number;
    // 最大循环次数（默认999999）
    maxIterations?: number;
}

// AI配置参数类型定义
export interface AIOptions {
    // AI提供商类型
    provider: ProviderType;
    // API密钥
    apiKey: string;
    // 基础URL
    baseUrl: string;
    // 模型名称
    model: string;
    // 系统提示词
    systemPrompt?: string;
    // 消息列表
    messages?: any[];
    // 上下文长度限制
    contextLength?: number;
    // 采样温度
    temperature?: number;
    // 核采样参数
    topP?: number;
    // 工具定义数组
    tools?: any[];
    // 工具实现函数映射
    tools_realize?: Record<string, Function>;
    // 控制工具调用行为
    toolChoice?: 'none' | 'auto' | 'required' | { type: 'function'; function: { name: string } };
    // 是否启用并行工具调用
    parallelToolCalls?: boolean;
    // 启用流式响应
    stream?: boolean;
    // 其他兼容参数
    other?: any;
}
