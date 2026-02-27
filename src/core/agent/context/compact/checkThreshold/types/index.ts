/**
 * 作用：定义token阈值检查模块的类型
 * 关联：被checkThreshold/index.ts使用
 * 预期结果：提供清晰的类型定义
 */

// Token阈值检查参数类型
export interface CheckThresholdOptions {
    // 当前已使用的token数量
    contextTokens: number;
    // 模型的上下文窗口大小
    contextWindow: number;
    // 触发压缩的阈值百分比（默认0.8，即80%）
    thresholdPercentage?: number;
}
