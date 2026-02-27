import { CompactOptions } from './types';
import { EstimateTokens } from './estimateTokens';

/**
 * 作用：压缩上下文消息，保留最新的消息，压缩旧的消息
 * 关联：被combineContext调用，当token使用量超过阈值时触发
 * 预期结果：返回压缩后的messages数组
 */
export function CompactMessages(options: CompactOptions): any[] {
    // 参数有效性检查
    if (!options.messages || options.messages.length === 0) {
        return [];
    }

    if (!options.contextTokens) {
        throw new Error('contextTokens is required');
    }

    if (!options.contextWindow) {
        throw new Error('contextWindow is required');
    }

    // 设置默认保留比例为40%
    const keepRatio = options.compactRatio ?? 0.4;

    // 计算保留的token阈值（40%的上下文窗口）
    const keepTokenThreshold = options.contextWindow * keepRatio;

    // 从后往前查找切割点
    let accumulatedTokens = 0;
    let cutoffIndex = 0; // 切割点索引：从此索引开始保留消息

    // 从后往前遍历messages数组
    for (let i = options.messages.length - 1; i >= 0; i--) {
        const message = options.messages[i];
        
        // 计算当前消息的token数量
        const messageTokens = EstimateTokens({ messages: [message] });
        
        // 检查加上这条消息是否会超过40%阈值
        if (accumulatedTokens + messageTokens <= keepTokenThreshold) {
            // 未超过阈值，继续累加，更新切割点
            accumulatedTokens += messageTokens;
            cutoffIndex = i; // 从i开始保留
        } else {
            // 超过阈值，停止遍历
            break;
        }
    }

    // 如果所有消息都在阈值内，不需要压缩
    if (cutoffIndex === 0) {
        return options.messages;
    }

    // 根据切割点消息的role类型调整切割位置
    const cutoffMessage = options.messages[cutoffIndex];
    
    if (cutoffMessage.role === 'assistant') {
        // 从切割点往前查找第一条user消息
        for (let i = cutoffIndex - 1; i >= 0; i--) {
            if (options.messages[i].role === 'user') {
                // 找到user消息，将user及之前的压缩，user之后的保留
                // 所以新的切割点是user的下一个位置
                cutoffIndex = i + 1;
                break;
            }
        }
        // 如果没找到user，保持原切割点
    }

    // 分割消息数组
    const messagesToCompact = options.messages.slice(0, cutoffIndex); // 需要压缩的消息（0到cutoffIndex-1）
    const messagesToKeep = options.messages.slice(cutoffIndex); // 保留的消息（从cutoffIndex开始）

    // 如果没有需要压缩的消息，直接返回原数组
    if (messagesToCompact.length === 0) {
        return options.messages;
    }

    // TODO: 调用LLM进行压缩，提取关键信息
    
    // 暂时返回一个占位的压缩摘要消息
    const compactedMessage = {
        role: 'system',
        content: `[压缩的上下文：包含${messagesToCompact.length}条消息，约${options.contextTokens - accumulatedTokens} tokens]`
    };

    // 返回压缩后的消息数组：压缩摘要 + 保留的最新消息
    return [compactedMessage, ...messagesToKeep];
}
