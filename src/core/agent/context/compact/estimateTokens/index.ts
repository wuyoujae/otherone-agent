import { EstimateTokensOptions } from './types';

/**
 * 作用：估算当前会话的token使用量
 * 关联：被combineContext调用，当无法从assistant消息中获取token_consumption时使用
 * 预期结果：返回估算的token数量
 */
export function EstimateTokens(options: EstimateTokensOptions): number {
    // 参数有效性检查
    if (!options.messages || options.messages.length === 0) {
        return 0;
    }

    let totalTokens = 0;

    // 遍历messages数组，根据role类型进行不同处理
    for (const message of options.messages) {
        switch (message.role) {
            case 'user':
                totalTokens += EstimateUserMessageTokens(message);
                break;
            case 'assistant':
                totalTokens += EstimateAssistantMessageTokens(message);
                break;
            case 'system':
                totalTokens += EstimateSystemMessageTokens(message);
                break;
            case 'tool':
                totalTokens += EstimateToolMessageTokens(message);
                break;
            case 'developer':
                totalTokens += EstimateDeveloperMessageTokens(message);
                break;
            default:
                // 未知角色，按照普通文本处理
                if (message.content) {
                    totalTokens += EstimateContentTokens(message.content);
                }
                break;
        }
    }

    return totalTokens;
}

/**
 * 作用：估算user消息的token数量
 * 关联：被EstimateTokens调用
 * 预期结果：返回估算的token数量
 */
function EstimateUserMessageTokens(message: any): number {
    let tokens = 0;

    // user消息的content可能是string或array
    if (typeof message.content === 'string') {
        // 纯文本消息
        tokens += EstimateContentTokens(message.content);
    } else if (Array.isArray(message.content)) {
        // 多模态消息（文本+图片+视频+音频）
        for (const item of message.content) {
            if (item.type === 'text') {
                // 文本内容
                tokens += EstimateContentTokens(item.text);
            } else if (item.type === 'image_url') {
                // 图片内容，按照5000个字符计算
                tokens += Math.ceil(5000 / 4);
            } else if (item.type === 'video_url') {
                // 视频内容，按照5分钟计算，每秒263token
                // 5分钟 = 300秒，300 * 263 = 78900 tokens
                tokens += 300 * 263;
            } else if (item.type === 'input_audio') {
                // 音频内容，按照8分钟计算，每秒32token
                // 8分钟 = 480秒，480 * 32 = 15360 tokens
                tokens += 480 * 32;
            }
        }
    }

    return tokens;
}

/**
 * 作用：估算assistant消息的token数量
 * 关联：被EstimateTokens调用
 * 预期结果：返回估算的token数量
 */
function EstimateAssistantMessageTokens(message: any): number {
    let tokens = 0;

    // assistant消息的content可能是string、array或null
    if (typeof message.content === 'string') {
        // 纯文本消息
        tokens += EstimateContentTokens(message.content);
    } else if (Array.isArray(message.content)) {
        // 多模态消息（文本+图片+音频）
        for (const item of message.content) {
            if (item.type === 'text') {
                // 文本内容
                tokens += EstimateContentTokens(item.text);
            } else if (item.type === 'image_url') {
                // 图片内容，按照5000个字符计算
                tokens += Math.ceil(5000 / 4);
            } else if (item.type === 'audio') {
                // 音频内容，按照8分钟计算，每秒32token
                // 8分钟 = 480秒，480 * 32 = 15360 tokens
                tokens += 480 * 32;
                // 如果有transcript，也计算transcript的token
                if (item.audio && item.audio.transcript) {
                    tokens += EstimateContentTokens(item.audio.transcript);
                }
            }
        }
    }

    // 如果有thinking字段，也需要计算
    if (message.thinking) {
        tokens += EstimateContentTokens(message.thinking);
    }

    // 如果有tool_calls，计算tool_calls的token
    if (message.tool_calls && Array.isArray(message.tool_calls)) {
        for (const toolCall of message.tool_calls) {
            // 计算function name的token
            if (toolCall.function && toolCall.function.name) {
                tokens += EstimateContentTokens(toolCall.function.name);
            }
            // 计算arguments的token
            if (toolCall.function && toolCall.function.arguments) {
                tokens += EstimateContentTokens(toolCall.function.arguments);
            }
        }
    }

    return tokens;
}

/**
 * 作用：估算system消息的token数量
 * 关联：被EstimateTokens调用
 * 预期结果：返回估算的token数量
 */
function EstimateSystemMessageTokens(message: any): number {
    let tokens = 0;

    if (message.content) {
        tokens += EstimateContentTokens(message.content);
    }

    return tokens;
}

/**
 * 作用：估算tool消息的token数量
 * 关联：被EstimateTokens调用
 * 预期结果：返回估算的token数量
 */
function EstimateToolMessageTokens(message: any): number {
    let tokens = 0;

    // tool消息的content可能是string或array
    if (typeof message.content === 'string') {
        // 纯文本消息
        tokens += EstimateContentTokens(message.content);
    } else if (Array.isArray(message.content)) {
        // 多模态消息（文本+图片+视频+音频）
        for (const item of message.content) {
            if (item.type === 'text') {
                // 文本内容
                tokens += EstimateContentTokens(item.text);
            } else if (item.type === 'image') {
                // 图片内容，按照5000个字符计算
                tokens += Math.ceil(5000 / 4);
            } else if (item.type === 'video') {
                // 视频内容，按照5分钟计算，每秒263token
                // 5分钟 = 300秒，300 * 263 = 78900 tokens
                tokens += 300 * 263;
            } else if (item.type === 'audio') {
                // 音频内容，按照8分钟计算，每秒32token
                // 8分钟 = 480秒，480 * 32 = 15360 tokens
                tokens += 480 * 32;
            }
        }
    }

    // 计算name字段的token
    if (message.name) {
        tokens += EstimateContentTokens(message.name);
    }

    return tokens;
}

/**
 * 作用：估算developer消息的token数量
 * 关联：被EstimateTokens调用
 * 预期结果：返回估算的token数量
 */
function EstimateDeveloperMessageTokens(message: any): number {
    let tokens = 0;

    if (message.content) {
        tokens += EstimateContentTokens(message.content);
    }

    return tokens;
}

/**
 * 作用：估算单个content的token数量
 * 关联：被各个估算函数调用
 * 预期结果：返回估算的token数量
 */
function EstimateContentTokens(content: string): number {
    if (!content || typeof content !== 'string') {
        return 0;
    }

    // 简单估算规则：
    // 1. 统计中文字符数（Unicode范围：\u4e00-\u9fa5）
    // 2. 统计英文字符数
    // 3. 中文约1.5字符/token，英文约4字符/token

    const chineseChars = (content.match(/[\u4e00-\u9fa5]/g) || []).length;
    const totalChars = content.length;
    const englishChars = totalChars - chineseChars;

    // 中文token估算
    const chineseTokens = Math.ceil(chineseChars / 1.5);
    // 英文token估算
    const englishTokens = Math.ceil(englishChars / 4);

    return chineseTokens + englishTokens;
}
