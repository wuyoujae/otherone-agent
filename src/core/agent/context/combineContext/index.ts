import { CombineContextOptions } from './types';
import { ReadSessionData } from '../storage/localfile/reader';
import { CheckThreshold } from '../compact/checkThreshold';
import { EstimateTokens } from '../compact/estimateTokens';

/**
 * 作用：组合context配置，根据session_id加载历史消息
 * 关联：被loop模块调用，在InvokeAgent循环中处理context
 * 预期结果：返回messages数组，包含历史对话记录
 */
export function CombineContext(options: CombineContextOptions): any[] {
    // 参数有效性检查
    if (!options.sessionId) {
        throw new Error('sessionId is required');
    }

    if (!options.loadType) {
        throw new Error('loadType is required');
    }

    if (!options.provider) {
        throw new Error('provider is required');
    }

    if (!options.contextWindow) {
        throw new Error('contextWindow is required');
    }

    // 根据loadType类型加载数据
    let sessionData: any;
    switch (options.loadType) {
        case 'database':
            // TODO: 实现从数据库加载逻辑
            sessionData = {
                session: {},
                entries: [],
                compacted_dialogues: []
            };
            break;
        case 'localfile':
            sessionData = ReadSessionData(options.sessionId);
            break;
        default:
            throw new Error(`Unsupported loadType: ${options.loadType}`);
    }

    // 根据provider类型转换数据格式
    const messages = TransformToMessages(sessionData, options.provider);

    // 获取最后一条assistant消息的token_consumption
    // 如果没有找到，继续往前找，最多找3次
    let contextTokens = 0;
    let foundTokenIndex = -1;
    const assistantEntries = [...sessionData.entries]
        .reverse()
        .filter((entry: any) => entry.role === 'assistant');

    // 最多检查3条assistant消息
    const maxAttempts = Math.min(3, assistantEntries.length);
    for (let i = 0; i < maxAttempts; i++) {
        const entry = assistantEntries[i];
        if (entry.token_consumption !== undefined && entry.token_consumption !== null) {
            contextTokens = entry.token_consumption;
            foundTokenIndex = sessionData.entries.indexOf(entry);
            break;
        }
    }

    // 如果3次都没找到token_consumption，或者找到了但最后一条消息不是assistant
    // 需要调用EstimateTokens进行估算
    if (contextTokens === 0 || foundTokenIndex < sessionData.entries.length - 1) {
        let messagesToEstimate: any[];
        
        if (contextTokens === 0) {
            // 没找到token_consumption，估算所有messages
            messagesToEstimate = messages;
        } else {
            // 找到了token_consumption，只估算foundTokenIndex之后的messages
            // 因为entries是反转的，所以需要计算正确的起始位置
            const startIndex = foundTokenIndex + 1;
            const messagesToEstimateCount = sessionData.entries.length - startIndex;
            // messages数组是反转后的，所以从前面取messagesToEstimateCount个
            messagesToEstimate = messages.slice(0, messagesToEstimateCount);
        }
        
        const estimatedTokens = EstimateTokens({
            messages: messagesToEstimate,
            lastTokenIndex: foundTokenIndex >= 0 ? foundTokenIndex : undefined
        });
        
        // 如果找到了token_consumption，则在其基础上加上估算值
        // 如果没找到，则直接使用估算值
        contextTokens = contextTokens + estimatedTokens;
    }

    // 检查是否需要压缩
    const shouldCompress = CheckThreshold({
        contextTokens,
        contextWindow: options.contextWindow,
        thresholdPercentage: options.thresholdPercentage
    });

    // 如果需要压缩
    if (shouldCompress) {
        // TODO: 实现压缩逻辑
        // 暂时直接返回messages
        return messages;
    }

    // 不需要压缩，直接返回messages
    return messages;
}

/**
 * 作用：将session数据转换为AI请求的messages格式
 * 关联：被CombineContext调用
 * 预期结果：返回符合不同provider要求的messages数组
 */
function TransformToMessages(sessionData: any, provider: string): any[] {
    // 根据provider类型进行不同的转换
    switch (provider) {
        case 'openai':
            return TransformToOpenAIFormat(sessionData);
        case 'anthropic':
            return TransformToAnthropicFormat(sessionData);
        case 'fetch':
            return TransformToFetchFormat(sessionData);
        default:
            throw new Error(`Unsupported provider: ${provider}`);
    }
}

/**
 * 作用：转换为OpenAI格式的messages
 * 关联：被TransformToMessages调用
 * 预期结果：返回OpenAI格式的messages数组
 */
function TransformToOpenAIFormat(sessionData: any): any[] {
    const messages: any[] = [];
    
    // 检查是否有entries
    if (!sessionData.entries || sessionData.entries.length === 0) {
        return messages;
    }
    
    // 按照栈的方式处理：越早的消息越在数组末尾
    // 先将entries反转，使得最早的消息在最前面
    const sortedEntries = [...sessionData.entries].reverse();
    
    // 遍历所有entries，转换为OpenAI格式
    for (const entry of sortedEntries) {
        const message: any = {
            role: entry.role,
            content: entry.content
        };
        
        // 如果是assistant且有tool_calls，添加tool_calls字段
        if (entry.role === 'assistant' && entry.tools && entry.tools.tool_calls) {
            message.tool_calls = entry.tools.tool_calls;
            // 如果有tool_calls，content可以为null
            if (!entry.content) {
                message.content = null;
            }
        }
        
        // 如果是tool角色，需要特殊处理
        if (entry.role === 'tool') {
            // tool消息需要tool_call_id
            if (entry.tools && entry.tools.tool_call_id) {
                message.tool_call_id = entry.tools.tool_call_id;
            }
        }
        
        messages.push(message);
    }
    
    return messages;
}

/**
 * 作用：转换为Anthropic格式的messages
 * 关联：被TransformToMessages调用
 * 预期结果：返回Anthropic格式的messages数组
 */
function TransformToAnthropicFormat(sessionData: any): any[] {
    // TODO: 实现Anthropic格式转换
    return [];
}

/**
 * 作用：转换为Fetch格式的messages
 * 关联：被TransformToMessages调用
 * 预期结果：返回Fetch格式的messages数组
 */
function TransformToFetchFormat(sessionData: any): any[] {
    // TODO: 实现Fetch格式转换
    return [];
}
