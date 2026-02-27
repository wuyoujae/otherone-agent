import { InputOptions, AIOptions } from './types';
import { CombineTools } from '../../tools';
import { CombineContext } from '../../context/combineContext';
import { InvokeModel } from '../ai';
import { WriteEntry } from '../../context/storage';

/**
 * 作用：调用Agent，驱动整个AI对话流程
 * 关联：调用ai模块、contextManager、toolsCalling等其他模块，是整个Agent的核心驱动
 * 预期结果：根据input和ai配置，执行完整的Agent循环，返回最终响应
 */
export async function InvokeAgent(input: InputOptions, ai: AIOptions): Promise<any> {
    // TODO: 实现Agent调用逻辑
    while(true){
        // 组合tools配置
        CombineTools(ai);
        
        // 组合context配置，加载历史消息
        const messages = await CombineContext({
            sessionId: input.sessionId,
            loadType: input.contextLoadType,
            provider: ai.provider,
            contextWindow: input.contextWindow,
            thresholdPercentage: input.thresholdPercentage,
            ai: ai,
            systemPrompt: ai.systemPrompt,
            tools: ai.tools
        });
        
        // 将历史消息添加到ai配置中
        ai.messages = messages;
        
        // 调用AI模型
        const response = await InvokeModel(ai);
        
        // 检查是否是流式响应
        if (response && typeof response[Symbol.asyncIterator] === 'function') {
            // 处理流式响应
            const parsedResponse = await ParseStreamResponse(response, ai.provider);
            
            // 存储AI响应到storage
            WriteEntry({
                storageType: input.storageType || 'localfile',
                sessionId: input.sessionId,
                role: parsedResponse.role,
                content: parsedResponse.content,
                tools: parsedResponse.tools,
                tokenConsumption: parsedResponse.token_consumption
            });
            
            return parsedResponse;
        } else {
            // 处理非流式响应
            const parsedResponse = ParseAIResponse(response, ai.provider);
            
            // 存储AI响应到storage
            WriteEntry({
                storageType: input.storageType || 'localfile',
                sessionId: input.sessionId,
                role: parsedResponse.role,
                content: parsedResponse.content,
                tools: parsedResponse.tools,
                tokenConsumption: parsedResponse.token_consumption
            });
            
            return parsedResponse;
        }
    }
}

/**
 * 作用：处理流式响应
 * 关联：被InvokeAgent调用，处理Stream类型的响应
 * 预期结果：返回完整的解析结果对象
 */
async function ParseStreamResponse(stream: any, provider: string): Promise<any> {
    // 根据provider类型进行不同处理
    switch (provider) {
        case 'openai':
            return await ParseOpenAIStreamResponse(stream);
        case 'anthropic':
            return await ParseAnthropicStreamResponse(stream);
        case 'fetch':
            return await ParseFetchStreamResponse(stream);
        default:
            throw new Error(`Unsupported provider type: ${provider}`);
    }
}

/**
 * 作用：处理OpenAI的流式响应
 * 关联：被ParseStreamResponse调用
 * 预期结果：返回完整的解析结果对象
 */
async function ParseOpenAIStreamResponse(stream: any): Promise<any> {
    let fullContent = '';
    let role = 'assistant';
    let toolCalls: any[] = [];
    let thinking = null;
    let tokenConsumption = 0;
    
    try {
        for await (const chunk of stream) {
            const delta = chunk.choices?.[0]?.delta;
            
            if (!delta) {
                continue;
            }
            
            // 提取role（通常只在第一个chunk出现）
            if (delta.role) {
                role = delta.role;
            }
            
            // 累积content
            if (delta.content) {
                fullContent += delta.content;
            }
            
            // 处理tool_calls（流式tool_calls需要累积）
            if (delta.tool_calls) {
                for (const toolCall of delta.tool_calls) {
                    const index = toolCall.index;
                    if (!toolCalls[index]) {
                        toolCalls[index] = {
                            id: toolCall.id || '',
                            type: toolCall.type || 'function',
                            function: {
                                name: '',
                                arguments: ''
                            }
                        };
                    }
                    
                    if (toolCall.id) {
                        toolCalls[index].id = toolCall.id;
                    }
                    
                    if (toolCall.function?.name) {
                        toolCalls[index].function.name += toolCall.function.name;
                    }
                    
                    if (toolCall.function?.arguments) {
                        toolCalls[index].function.arguments += toolCall.function.arguments;
                    }
                }
            }
            
            // 提取usage（通常在最后一个chunk）
            if (chunk.usage) {
                tokenConsumption = chunk.usage.total_tokens || 0;
            }
        }
        
        // 构建最终结果
        const result: any = {
            content: fullContent,
            role,
            token_consumption: tokenConsumption,
            tools: toolCalls.length > 0 ? { tool_calls: toolCalls } : null,
            thinking
        };
        
        return result;
        
    } catch (error: any) {
        console.error('处理流式响应时出错:', error.message);
        throw error;
    }
}

/**
 * 作用：处理Anthropic的流式响应
 * 关联：被ParseStreamResponse调用
 * 预期结果：返回完整的解析结果对象
 */
async function ParseAnthropicStreamResponse(stream: any): Promise<any> {
    // TODO: 实现Anthropic流式响应解析
    throw new Error('Anthropic stream response parsing not implemented yet');
}

/**
 * 作用：处理Fetch的流式响应
 * 关联：被ParseStreamResponse调用
 * 预期结果：返回完整的解析结果对象
 */
async function ParseFetchStreamResponse(stream: any): Promise<any> {
    // TODO: 实现Fetch流式响应解析
    throw new Error('Fetch stream response parsing not implemented yet');
}

/**
 * 作用：解析AI模型的响应，提取关键信息
 * 关联：被InvokeAgent调用，根据不同provider解析响应格式
 * 预期结果：返回统一格式的解析结果对象
 */
function ParseAIResponse(response: any, provider: string): any {
    // 根据provider类型进行不同处理
    switch (provider) {
        case 'openai':
            return ParseOpenAIResponse(response);
        case 'anthropic':
            return ParseAnthropicResponse(response);
        case 'fetch':
            return ParseFetchResponse(response);
        default:
            throw new Error(`Unsupported provider type: ${provider}`);
    }
}

/**
 * 作用：解析OpenAI的响应格式
 * 关联：被ParseAIResponse调用
 * 预期结果：返回包含content、role、token_consumption、tools、thinking等字段的对象
 */
function ParseOpenAIResponse(response: any): any {
    // 参数有效性检查
    if (!response.choices || !response.choices[0]) {
        throw new Error('Invalid OpenAI response format: missing choices');
    }

    const message = response.choices[0].message;
    
    // 提取content（消息内容）
    const content = message?.content || '';
    
    // 提取role（角色，通常是assistant）
    const role = message?.role || 'assistant';
    
    // 提取token_consumption（token消耗量）
    let tokenConsumption = 0;
    if (response.usage) {
        // OpenAI的usage包含：prompt_tokens, completion_tokens, total_tokens
        tokenConsumption = response.usage.total_tokens || 0;
    }
    
    // 提取tools（工具调用信息）
    let tools = null;
    if (message?.tool_calls && message.tool_calls.length > 0) {
        tools = {
            tool_calls: message.tool_calls
        };
    }
    
    // 提取thinking（思考内容）
    // OpenAI目前没有thinking字段，预留为null
    const thinking = null;
    
    // 返回解析后的结果
    return {
        content,
        role,
        token_consumption: tokenConsumption,
        tools,
        thinking,
        // 保留原始响应，供后续处理使用
        raw_response: response
    };
}

/**
 * 作用：解析Anthropic的响应格式
 * 关联：被ParseAIResponse调用
 * 预期结果：返回包含content、role、token_consumption、tools、thinking等字段的对象
 */
function ParseAnthropicResponse(response: any): any {
    // TODO: 实现Anthropic响应解析
    throw new Error('Anthropic response parsing not implemented yet');
}

/**
 * 作用：解析Fetch的响应格式
 * 关联：被ParseAIResponse调用
 * 预期结果：返回包含content、role、token_consumption、tools、thinking等字段的对象
 */
function ParseFetchResponse(response: any): any {
    // TODO: 实现Fetch响应解析
    throw new Error('Fetch response parsing not implemented yet');
}
