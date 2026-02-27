/**
 * 作用：定义compact模块的类型
 * 关联：被compact/index.ts使用
 * 预期结果：提供清晰的类型定义
 */

// Compact参数类型
export interface CompactOptions {
    // 需要压缩的messages数组
    messages: any[];
    // 当前已使用的token数量
    contextTokens: number;
    // 模型的上下文窗口大小
    contextWindow: number;
    // 压缩比例（默认0.6，即压缩前60%的内容）
    compactRatio?: number;
    // AI配置参数（用于调用压缩LLM）
    ai?: any;
    // 是否已经有压缩内容（用于判断使用哪个prompt）
    hasCompactedContent?: boolean;
    // 会话ID（用于存储压缩记录）
    sessionId?: string;
    // 存储类型（用于存储压缩记录）
    storageType?: 'localfile' | 'database';
    // 原始entries数据（用于获取entry_id）
    originalEntries?: any[];
}
