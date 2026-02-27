import { CompactOptions } from './types';
import { EstimateTokens } from './estimateTokens';
import { MessagesToSequence } from './messagesToSequence';
import { WriteCompactedEntry } from '../storage';

/**
 * 作用：压缩上下文消息，保留最新的消息，压缩旧的消息
 * 关联：被combineContext调用，当token使用量超过阈值时触发
 * 预期结果：返回压缩后的messages数组
 */
export async function CompactMessages(options: CompactOptions): Promise<any[]> {
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

    // 提取压缩LLM配置
    const compactLLMConfig = ExtractCompactLLMConfig(options.ai);

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

    // 调用LLM进行压缩
    const compressedSummary = await CallCompactLLM(
        messagesToCompact,
        compactLLMConfig,
        options.hasCompactedContent || false
    );
    
    // 创建压缩摘要消息（作为user消息）
    const compactedMessage = {
        role: 'user',
        content: compressedSummary
    };

    // 存储压缩记录
    if (options.sessionId && options.storageType && options.originalEntries) {
        // 获取trigger_entry_id（切割点前一条消息对应的entry_id）
        // cutoffIndex-1是需要压缩的最后一条消息的索引
        // 如果有压缩内容，originalEntries[0]是压缩摘要（没有entry_id），所以要从originalEntries[cutoffIndex]获取
        let triggerEntryId: string;
        
        if (options.hasCompactedContent && cutoffIndex > 0) {
            // 有压缩内容时，originalEntries[0]是压缩摘要，实际的entry从[1]开始
            // cutoffIndex-1是messagesToCompact的最后一条，对应originalEntries[cutoffIndex-1]
            const triggerEntry = options.originalEntries[cutoffIndex - 1];
            triggerEntryId = triggerEntry?.entry_id || '';
        } else {
            // 没有压缩内容时，直接从originalEntries获取
            const triggerEntry = options.originalEntries[cutoffIndex - 1];
            triggerEntryId = triggerEntry?.entry_id || '';
        }
        
        // 如果没有找到trigger_entry_id，抛出错误
        if (!triggerEntryId) {
            throw new Error('Unable to find trigger_entry_id for compaction');
        }
        
        WriteCompactedEntry({
            storageType: options.storageType,
            sessionId: options.sessionId,
            summary: compressedSummary,
            triggerEntryId: triggerEntryId
        });
    }

    // 返回压缩后的消息数组：压缩摘要 + 保留的最新消息
    return [compactedMessage, ...messagesToKeep];
}


/**
 * 作用：调用LLM进行上下文压缩
 * 关联：被CompactMessages调用
 * 预期结果：返回压缩后的摘要文本
 */
async function CallCompactLLM(
    messagesToCompact: any[],
    compactLLMConfig: any,
    hasCompactedContent: boolean
): Promise<string> {
    // 导入prompt
    const { SUMMARIZATION_SYSTEM_PROMPT } = await import('./conpactPrompt/systemPrompt');
    const { TURN_PREFIX_SUMMARIZATION_PROMPT } = await import('./conpactPrompt/conpactPrompt');
    const { UPDATE_SUMMARIZATION_PROMPT } = await import('./conpactPrompt/updateCompactPrompt');
    
    // 将消息转换为序列文本
    const messageSequence = MessagesToSequence(messagesToCompact);
    
    // 根据是否已有压缩内容选择不同的prompt
    let userPrompt: string;
    if (hasCompactedContent) {
        // 如果已有压缩内容，使用更新prompt
        // 第一条消息应该是之前的压缩摘要
        const previousSummary = messagesToCompact[0]?.content || '';
        userPrompt = `<previous-summary>\n${previousSummary}\n</previous-summary>\n\n${UPDATE_SUMMARIZATION_PROMPT}\n\n${messageSequence}`;
    } else {
        // 首次压缩，使用基础prompt
        userPrompt = `${TURN_PREFIX_SUMMARIZATION_PROMPT}\n\n${messageSequence}`;
    }
    
    // 构建压缩请求的messages
    const compactMessages = [
        {
            role: 'system',
            content: SUMMARIZATION_SYSTEM_PROMPT
        },
        {
            role: 'user',
            content: userPrompt
        }
    ];
    
    // 调用AI模块
    const { InvokeModel } = await import('../../agentLoop/ai');
    
    const compactAIOptions = {
        ...compactLLMConfig,
        messages: compactMessages,
        stream: false
    };
    
    const response = await InvokeModel(compactAIOptions);
    
    // 提取压缩结果
    // 根据不同的响应格式提取内容
    let compressedContent = '';
    if (typeof response === 'string') {
        compressedContent = response;
    } else if (response.choices && response.choices[0]) {
        compressedContent = response.choices[0].message?.content || response.choices[0].text || '';
    } else if (response.content) {
        compressedContent = response.content;
    } else {
        throw new Error('Unable to extract compressed content from LLM response');
    }
    
    return compressedContent;
}

/**
 * 作用：从ai配置中提取压缩LLM的配置信息
 * 关联：被CompactMessages调用
 * 预期结果：返回压缩LLM的配置对象
 */
function ExtractCompactLLMConfig(ai: any): any {
    if (!ai) {
        throw new Error('AI configuration is required for compaction');
    }

    // 构建压缩LLM配置
    const compactConfig: any = {
        provider: ai.compact_llm_provider || ai.provider,
        apiKey: ai.compact_llm_apiKey || ai.apiKey,
        baseUrl: ai.compact_llm_baseUrl || ai.baseUrl,
        model: ai.compact_llm_model || ai.model,
        temperature: ai.compact_llm_temperature || ai.temperature || 0.3,
        topP: ai.compact_llm_topP || ai.topP,
        contextLength: ai.compact_llm_contextLength || ai.contextLength,
        stream: false, // 压缩不需要流式响应
        other: ai.compact_llm_other || ai.other
    };

    return compactConfig;
}
