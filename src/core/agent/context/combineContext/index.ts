import { CombineContextOptions } from './types';
import { ReadSessionData } from '../storage/localfile/reader';
import { CheckThreshold } from '../compact/checkThreshold';
import { EstimateTokens } from '../compact/estimateTokens';
import { CompactMessages } from '../compact';

/**
 * 作用：组合context配置，根据session_id加载历史消息
 * 关联：被loop模块调用，在InvokeAgent循环中处理context
 * 预期结果：返回messages数组，包含历史对话记录
 */
export async function CombineContext(options: CombineContextOptions): Promise<any[]> {
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
                compacted_entries: []
            };
            break;
        case 'localfile':
            sessionData = ReadSessionData(options.sessionId);
            break;
        default:
            throw new Error(`Unsupported loadType: ${options.loadType}`);
    }

    // 处理压缩记录，找到最新的压缩内容和起始entry_id
    const { latestCompactedSummary, startEntryId } = ProcessCompactedEntries(sessionData);

    // 根据startEntryId过滤entries
    const filteredEntries = FilterEntriesFromStart(sessionData.entries, startEntryId);

    // 更新sessionData的entries为过滤后的结果
    sessionData.entries = filteredEntries;

    // 根据provider类型转换数据格式
    const messages = TransformToMessages(sessionData, options.provider, latestCompactedSummary);

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
        // 检查messages中是否已经有压缩内容（第一条消息的content包含压缩标记）
        const hasCompactedContent = messages.length > 0 && 
            messages[0].role === 'user' && 
            (messages[0].content?.includes('[压缩') || messages[0].content?.includes('[Compressed'));
        
        // 调用CompactMessages进行压缩
        const compactedMessages = await CompactMessages({
            messages: messages,
            contextTokens: contextTokens,
            contextWindow: options.contextWindow,
            ai: options.ai,
            hasCompactedContent: hasCompactedContent
        });
        
        return compactedMessages;
    }

    // 不需要压缩，直接返回messages
    return messages;
}

/**
 * 作用：处理压缩记录，找到最新的压缩内容和起始entry_id
 * 关联：被CombineContext调用
 * 预期结果：返回最新的压缩摘要和起始entry_id
 */
function ProcessCompactedEntries(sessionData: any): { latestCompactedSummary: string | null, startEntryId: string | null } {
    // 如果没有压缩记录，返回null
    if (!sessionData.compacted_entries || sessionData.compacted_entries.length === 0) {
        return { latestCompactedSummary: null, startEntryId: null };
    }

    // 按create_at降序排序，取最新的压缩记录
    const sortedCompacted = [...sessionData.compacted_entries].sort((a, b) => {
        return new Date(b.create_at).getTime() - new Date(a.create_at).getTime();
    });

    const latestCompacted = sortedCompacted[0];
    
    // 如果summary为空，返回null
    if (!latestCompacted.summary) {
        return { latestCompactedSummary: null, startEntryId: null };
    }

    // 递归查找最终的trigger_entry_id
    const finalEntryId = FindFinalTriggerEntryId(
        latestCompacted.trigger_entry_id,
        sessionData.compacted_entries,
        sessionData.entries
    );

    return {
        latestCompactedSummary: latestCompacted.summary,
        startEntryId: finalEntryId
    };
}

/**
 * 作用：递归查找最终的trigger_entry_id，直到找到在entries表中的entry_id
 * 关联：被ProcessCompactedEntries调用
 * 预期结果：返回在entries表中的entry_id
 */
function FindFinalTriggerEntryId(
    triggerId: string,
    compactedEntries: any[],
    entries: any[]
): string | null {
    // 防止循环引用，最多递归10次
    const maxDepth = 10;
    let currentId = triggerId;
    const visitedIds = new Set<string>();

    for (let i = 0; i < maxDepth; i++) {
        // 检查是否已经访问过这个ID（防止循环引用）
        if (visitedIds.has(currentId)) {
            console.warn(`检测到循环引用: ${currentId}`);
            return null;
        }
        visitedIds.add(currentId);

        // 先检查是否在entries中
        const entryExists = entries.some((e: any) => e.entry_id === currentId);
        if (entryExists) {
            return currentId;
        }

        // 如果不在entries中，检查是否在compacted_entries中
        const compactedEntry = compactedEntries.find((c: any) => c.entry_id === currentId);
        if (compactedEntry) {
            // 继续查找这个压缩记录的trigger_entry_id
            currentId = compactedEntry.trigger_entry_id;
        } else {
            // 既不在entries也不在compacted_entries中，数据不一致
            console.warn(`trigger_entry_id不存在: ${currentId}`);
            return null;
        }
    }

    // 超过最大递归深度
    console.warn(`超过最大递归深度，最后的ID: ${currentId}`);
    return null;
}

/**
 * 作用：从startEntryId开始过滤entries
 * 关联：被CombineContext调用
 * 预期结果：返回从startEntryId开始往后的所有entries
 */
function FilterEntriesFromStart(entries: any[], startEntryId: string | null): any[] {
    // 如果没有startEntryId，返回所有entries
    if (!startEntryId) {
        return entries;
    }

    // 找到startEntryId在entries中的索引
    const startIndex = entries.findIndex((e: any) => e.entry_id === startEntryId);

    // 如果没找到，返回所有entries
    if (startIndex === -1) {
        console.warn(`startEntryId不存在于entries中: ${startEntryId}`);
        return entries;
    }

    // 返回从startIndex开始往后的所有entries
    return entries.slice(startIndex);
}

/**
 * 作用：将session数据转换为AI请求的messages格式
 * 关联：被CombineContext调用
 * 预期结果：返回符合不同provider要求的messages数组
 */
function TransformToMessages(sessionData: any, provider: string, compactedSummary: string | null): any[] {
    // 根据provider类型进行不同的转换
    switch (provider) {
        case 'openai':
            return TransformToOpenAIFormat(sessionData, compactedSummary);
        case 'anthropic':
            return TransformToAnthropicFormat(sessionData, compactedSummary);
        case 'fetch':
            return TransformToFetchFormat(sessionData, compactedSummary);
        default:
            throw new Error(`Unsupported provider: ${provider}`);
    }
}

/**
 * 作用：转换为OpenAI格式的messages
 * 关联：被TransformToMessages调用
 * 预期结果：返回OpenAI格式的messages数组
 */
function TransformToOpenAIFormat(sessionData: any, compactedSummary: string | null): any[] {
    const messages: any[] = [];
    
    // 如果有压缩摘要，作为第一条user消息添加
    if (compactedSummary) {
        messages.push({
            role: 'user',
            content: compactedSummary
        });
    }
    
    // 检查是否有entries
    if (!sessionData.entries || sessionData.entries.length === 0) {
        return messages;
    }
    
    // entries已经是按时间顺序排列的（从早到晚），直接遍历即可
    for (const entry of sessionData.entries) {
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
function TransformToAnthropicFormat(sessionData: any, compactedSummary: string | null): any[] {
    // TODO: 实现Anthropic格式转换
    return [];
}

/**
 * 作用：转换为Fetch格式的messages
 * 关联：被TransformToMessages调用
 * 预期结果：返回Fetch格式的messages数组
 */
function TransformToFetchFormat(sessionData: any, compactedSummary: string | null): any[] {
    // TODO: 实现Fetch格式转换
    return [];
}
