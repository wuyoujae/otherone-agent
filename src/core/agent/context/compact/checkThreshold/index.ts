import { CheckThresholdOptions } from './types';

/**
 * 作用：检查当前token使用量是否超过阈值，判断是否需要触发压缩
 * 关联：被compact模块调用，用于判断是否需要进行context压缩
 * 预期结果：返回boolean值，true表示需要压缩，false表示不需要压缩
 */
export function CheckThreshold(options: CheckThresholdOptions): boolean {
    // 参数有效性检查
    if (options.contextTokens === undefined || options.contextTokens === null) {
        throw new Error('contextTokens is required');
    }

    if (!options.contextWindow) {
        throw new Error('contextWindow is required');
    }

    // 设置默认阈值百分比为80%
    const thresholdPercentage = options.thresholdPercentage ?? 0.8;

    // 计算触发压缩的阈值
    // 公式：contextTokens > contextWindow * thresholdPercentage
    const threshold = options.contextWindow * thresholdPercentage;

    // 判断是否超过阈值
    const shouldCompress = options.contextTokens >= threshold;

    return shouldCompress;
}
