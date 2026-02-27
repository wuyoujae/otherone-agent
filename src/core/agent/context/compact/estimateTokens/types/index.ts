/**
 * 作用：定义token估算模块的类型
 * 关联：被estimateTokens/index.ts使用
 * 预期结果：提供清晰的类型定义
 */

// Token估算参数类型
export interface EstimateTokensOptions {
    // 转换后的messages数组
    messages: any[];
    // 最后一条找到token_consumption的assistant消息在原始entries中的索引位置
    lastTokenIndex?: number;
}
