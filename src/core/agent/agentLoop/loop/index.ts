import { InputOptions, AIOptions } from './types';
import { CombineTools, ProcessTools } from '../../tools';
import { CombineContext } from '../../context/combineContext';
import { InvokeModel } from '../ai';
import { WriteEntry } from '../../context/storage';

/**
 * 作用：调用Agent，驱动整个AI对话流程
 * 关联：调用ai模块、contextManager、toolsCalling等其他模块，是整个Agent的核心驱动
 * 预期结果：根据input和ai配置，执行完整的Agent循环，返回最终响应或流式生成器
 */
export async function InvokeAgent(input: InputOptions, ai: AIOptions): Promise<any> {
    // 如果启用流式响应，调用流式版本
    if (ai.stream) {
        return InvokeAgentStream(input, ai);
    }
    
    // 非流式版本（原有逻辑）
    return InvokeAgentNonStream(input, ai);
}

/**
 * 作用：非流式版本的Agent调用
 * 关联：被InvokeAgent调用，处理非流式响应
 * 预期结果：返回最终的解析结果对象
 */
async function InvokeAgentNonStream(input: InputOptions, ai: AIOptions): Promise<any> {
    // 如果有userPrompt，先存储用户消息
    if (ai.userPrompt) {
        WriteEntry({
            storageType: input.storageType || 'localfile',
            sessionId: input.sessionId,
            role: 'user',
            content: ai.userPrompt
        });
    }
    
    // 从input参数读取循环次数限制，默认999999
    const maxIterations = input.maxIterations || 999999;
    let iteration = 0;
    
    while(iteration < maxIterations){
        iteration++;
        
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
        
        // 处理响应
        const parsedResponse = ParseAIResponse(response, ai.provider);
        
        // 如果有thinking内容，添加到content前面
        if (parsedResponse.thinking) {
            parsedResponse.content = `[thinking:${parsedResponse.thinking}]\n\n${parsedResponse.content}`;
        }
        
        // 存储AI响应到storage
        WriteEntry({
            storageType: input.storageType || 'localfile',
            sessionId: input.sessionId,
            role: parsedResponse.role,
            content: parsedResponse.content,
            tools: parsedResponse.tools,
            tokenConsumption: parsedResponse.token_consumption
        });
        
        // 检查是否有tool调用
        if (parsedResponse.tools && parsedResponse.tools.tool_calls && parsedResponse.tools.tool_calls.length > 0) {
            // 添加tool_calls信息到返回内容
            const toolCallsInfo = parsedResponse.tools.tool_calls.map((tc: any) => 
                `${tc.function.name}(${tc.function.arguments})`
            ).join(', ');
            parsedResponse.content = `[tool_calls:${toolCallsInfo}]\n\n${parsedResponse.content}`;
            
            // 处理tool调用
            const toolResults = await ProcessTools(parsedResponse.tools.tool_calls, ai.tools_realize || {});
            
            // 遍历每个tool结果，分别存储
            for (const toolResult of toolResults) {
                // 将单个tool结果转换为content字符串
                const toolResultContent = JSON.stringify(toolResult.result || toolResult.error);
                
                // 存储单个tool结果（role为'tool'）
                WriteEntry({
                    storageType: input.storageType || 'localfile',
                    sessionId: input.sessionId,
                    role: 'tool',
                    content: toolResultContent,
                    tools: {
                        tool_call_id: toolResult.tool_call_id,
                        function_name: toolResult.function_name,
                        result: toolResult.result,
                        error: toolResult.error
                    }
                });
            }
            
            // 添加1.5秒延迟，避免调用过快
            await Sleep(1500);
            
            // 继续while循环，不要return
            continue;
        }
        
        return parsedResponse;
    }
    
    // 如果循环次数超限，抛出错误
    throw new Error(`Agent循环次数超过限制(${maxIterations}次)，可能陷入无限循环`);
}

/**
 * 作用：流式版本的Agent调用
 * 关联：被InvokeAgent调用，处理流式响应
 * 预期结果：返回异步生成器，实时yield chunk，同时在后台处理tool循环
 */
async function* InvokeAgentStream(input: InputOptions, ai: AIOptions): AsyncGenerator<any, any, unknown> {
    // 如果有userPrompt，先存储用户消息
    if (ai.userPrompt) {
        WriteEntry({
            storageType: input.storageType || 'localfile',
            sessionId: input.sessionId,
            role: 'user',
            content: ai.userPrompt
        });
    }
    
    // 从input参数读取循环次数限制，默认999999
    const maxIterations = input.maxIterations || 999999;
    let iteration = 0;
    
    while(iteration < maxIterations){
        iteration++;
        
        try {
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
                // 累积变量
                let fullContent = '';
                let role = 'assistant';
                let toolCalls: any[] = [];
                let thinking: string | null = null;
                let tokenConsumption = 0;
                
                // 遍历stream，实时yield给调用者
                for await (const chunk of response) {
                    // 实时yield原始chunk
                    yield chunk;
                    
                    // 同时累积数据用于后续处理
                    const delta = chunk.choices?.[0]?.delta;
                    
                    if (!delta) {
                        continue;
                    }
                    
                    // 提取role
                    if (delta.role) {
                        role = delta.role;
                    }
                    
                    // 累积content
                    if (delta.content) {
                        fullContent += delta.content;
                    }
                    
                    // 处理tool_calls（流式累积）
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
                    
                    // 提取usage
                    if (chunk.usage) {
                        tokenConsumption = chunk.usage.total_tokens || 0;
                    }
                }
                
                // stream结束，构建完整响应
                const parsedResponse: any = {
                    content: fullContent,
                    role,
                    token_consumption: tokenConsumption,
                    tools: toolCalls.length > 0 ? { tool_calls: toolCalls } : null,
                    thinking
                };
                
                // 如果有thinking，yield thinking信息
                if (parsedResponse.thinking) {
                    yield {
                        type: 'thinking',
                        content: `[thinking:${parsedResponse.thinking}]`
                    };
                }
                
                // 存储AI响应到storage
                WriteEntry({
                    storageType: input.storageType || 'localfile',
                    sessionId: input.sessionId,
                    role: parsedResponse.role,
                    content: parsedResponse.content,
                    tools: parsedResponse.tools,
                    tokenConsumption: parsedResponse.token_consumption
                });
                
                // 检查是否有tool调用
                if (parsedResponse.tools && parsedResponse.tools.tool_calls && parsedResponse.tools.tool_calls.length > 0) {
                    // yield tool_calls信息
                    const toolCallsInfo = parsedResponse.tools.tool_calls.map((tc: any) => 
                        `${tc.function.name}(${tc.function.arguments})`
                    ).join(', ');
                    
                    yield {
                        type: 'tool_calls',
                        content: `[tool_calls:${toolCallsInfo}]`
                    };
                    
                    // 处理tool调用
                    const toolResults = await ProcessTools(parsedResponse.tools.tool_calls, ai.tools_realize || {});
                    
                    // 遍历每个tool结果，分别存储
                    for (const toolResult of toolResults) {
                        // 将单个tool结果转换为content字符串
                        const toolResultContent = JSON.stringify(toolResult.result || toolResult.error);
                        
                        // 存储单个tool结果（role为'tool'）
                        WriteEntry({
                            storageType: input.storageType || 'localfile',
                            sessionId: input.sessionId,
                            role: 'tool',
                            content: toolResultContent,
                            tools: {
                                tool_call_id: toolResult.tool_call_id,
                                function_name: toolResult.function_name,
                                result: toolResult.result,
                                error: toolResult.error
                            }
                        });
                    }
                    
                    // 添加1.5秒延迟，避免调用过快
                    await Sleep(1500);
                    
                    // 继续while循环
                    continue;
                }
                
                // 没有tool调用，返回最终结果
                return parsedResponse;
                
            } else {
                // 非流式响应（不应该发生，因为ai.stream=true）
                const parsedResponse = ParseAIResponse(response, ai.provider);
                
                // yield完整响应
                yield {
                    type: 'complete',
                    content: parsedResponse.content,
                    ...parsedResponse
                };
                
                return parsedResponse;
            }
            
        } catch (error: any) {
            // 错误处理：yield错误信息
            yield {
                type: 'error',
                content: `[error:${error.message}]`,
                error: error.message
            };
            
            throw error;
        }
    }
    
    // 如果循环次数超限，抛出错误
    const errorMsg = `Agent循环次数超过限制(${maxIterations}次)，可能陷入无限循环`;
    yield {
        type: 'error',
        content: `[error:${errorMsg}]`,
        error: errorMsg
    };
    throw new Error(errorMsg);
}

/**
 * 作用：延迟函数，用于在循环中添加等待时间
 * 关联：被InvokeAgent调用，避免API调用过快
 * 预期结果：等待指定的毫秒数后继续执行
 */
function Sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
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
