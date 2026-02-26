/**
 * 作用：定义所有AI提供商通用的类型
 * 关联：被各个provider子模块（openai、anthropic、fetch）导入使用
 * 预期结果：提供统一的基础类型定义，各provider可以基于此扩展
 */

// 定义AI提供商类型
export type ProviderType = 'openai' | 'anthropic' | 'fetch';

// 通用配置参数类型定义
export interface ConfigOptions {
    // API提供商类型
    provider: ProviderType;
    // API密钥
    apiKey: string;
    // 基础URL
    baseUrl: string;
}
