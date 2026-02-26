import OpenAI from 'openai';
import { ConfigOptions } from './types';

/**
 * 作用：创建OpenAI客户端并发送聊天请求
 * 关联：被InvokeModel通过openai/index.ts调用，使用OpenAI SDK
 * 预期结果：返回OpenAI API的响应内容
 */
export async function CreateOpenAIClient(options: ConfigOptions): Promise<string> {
    // OpenAI专用参数检查
    if (!options.model) {
        throw new Error('model is required for OpenAI');
    }

    if (!options.messages || options.messages.length === 0) {
        throw new Error('messages is required for OpenAI');
    }

    // 构建客户端配置参数
    const clientConfig: Record<string, unknown> = {
        baseURL: options.baseUrl,
        apiKey: options.apiKey
    };

    // 添加other.client中的嵌套参数
    if (options.other?.client) {
        Object.assign(clientConfig, options.other.client);
    }

    // 初始化OpenAI客户端
    const inferenceClient = new OpenAI(clientConfig);

    // 构建请求参数
    const requestParams: Record<string, unknown> = {
        model: options.model,
        messages: options.messages
    };

    // contextLength处理（即max_tokens）
    if (options.contextLength !== undefined && options.contextLength !== null) {
        requestParams.max_tokens = options.contextLength;
    }

    // 采样参数
    if (options.temperature !== undefined) {
        requestParams.temperature = options.temperature;
    }

    if (options.topP !== undefined) {
        requestParams.top_p = options.topP;
    }

    // 工具调用参数
    if (options.tools !== undefined) {
        requestParams.tools = options.tools;
    }

    if (options.toolChoice !== undefined) {
        requestParams.tool_choice = options.toolChoice;
    }

    if (options.parallelToolCalls !== undefined) {
        requestParams.parallel_tool_calls = options.parallelToolCalls;
    }

    // 流式参数
    if (options.stream !== undefined) {
        requestParams.stream = options.stream;
    }

    // 添加other.chat中的扩展参数
    if (options.other?.chat) {
        Object.assign(requestParams, options.other.chat);
    }

    // 发送请求
    const completion = await inferenceClient.chat.completions.create(requestParams as any);

    // 处理流式响应
    if (options.stream) {
        // 流式响应需要特殊处理，这里暂时返回空字符串
        // TODO: 实现流式响应处理
        return '';
    }

    // 返回响应内容
    return completion.choices[0].message.content || '';
}
