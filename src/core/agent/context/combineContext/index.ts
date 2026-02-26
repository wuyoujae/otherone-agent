import { CombineContextOptions } from './types';
import { ReadSessionData } from '../storage/localfile/reader';

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
    return TransformToMessages(sessionData, options.provider);
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
