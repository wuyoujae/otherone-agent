import { ConfigOptions as BaseConfigOptions } from './types';
import { ConfigOptions as OpenAIConfigOptions } from './openai/types';
import { CreateOpenAIClient } from './openai/client';

/**
 * 作用：调用AI模型并返回对应提供商的响应
 * 关联：被messageLoop模块调用，用于发送AI对话请求
 * 预期结果：根据provider类型调用对应的API并返回响应对象，参数无效时抛出错误
 */
export async function InvokeModel(options: BaseConfigOptions | OpenAIConfigOptions): Promise<any> {
    // 通用参数有效性检查
    if (!options.provider) {
        throw new Error('provider is required');
    }

    if (!options.apiKey) {
        throw new Error('apiKey is required');
    }

    if (!options.baseUrl) {
        throw new Error('baseUrl is required');
    }

    // 根据provider类型进行不同处理
    switch (options.provider) {
        case 'openai':
            return HandleOpenAIConfig(options as OpenAIConfigOptions);
        case 'anthropic':
            return HandleAnthropicConfig(options);
        case 'fetch':
            return HandleFetchConfig(options);
        default:
            throw new Error(`Unsupported provider type: ${options.provider}`);
    }
}

/**
 * 作用：处理OpenAI类型的配置
 * 关联：被InvokeModel调用，调用CreateOpenAIClient发送请求
 * 预期结果：返回OpenAI API的响应对象
 */
async function HandleOpenAIConfig(options: OpenAIConfigOptions): Promise<any> {
    return await CreateOpenAIClient(options);
}

/**
 * 作用：处理Anthropic类型的配置
 * 关联：被InvokeModel调用
 * 预期结果：返回Anthropic特定的响应对象
 */
async function HandleAnthropicConfig(options: BaseConfigOptions): Promise<any> {
    // TODO: 实现Anthropic配置处理
    return {};
}

/**
 * 作用：处理Fetch（通用网络请求）类型的配置
 * 关联：被InvokeModel调用
 * 预期结果：返回Fetch特定的响应对象
 */
async function HandleFetchConfig(options: BaseConfigOptions): Promise<any> {
    // TODO: 实现Fetch配置处理
    return {};
}
