/**
 * 作用：定义OpenAI专用的类型
 * 关联：被openai/client.ts使用，扩展通用ConfigOptions
 * 预期结果：提供OpenAI特有的参数类型定义
 */

import { ConfigOptions as BaseConfigOptions } from '../types';

// other参数结构定义
export interface OtherParams {
    // 客户端构建参数（用于初始化OpenAI客户端对象）
    client?: Record<string, unknown>;
    // 聊天请求参数（用于completion请求）
    chat?: Record<string, unknown>;
}

// OpenAI专用配置参数类型定义（扩展通用配置）
export interface ConfigOptions extends BaseConfigOptions {
    // 模型名称
    model: string;
    // 消息列表
    messages: any[];
    // 上下文长度限制（即maxTokens）
    contextLength?: number;
    // 采样温度
    temperature?: number;
    // 核采样参数
    topP?: number;
    // 工具定义数组
    tools?: any[];
    // 控制工具调用行为
    toolChoice?: 'none' | 'auto' | 'required' | { type: 'function'; function: { name: string } };
    // 是否启用并行工具调用
    parallelToolCalls?: boolean;
    // 启用流式响应
    stream?: boolean;
    // 其他兼容参数（嵌套结构）
    other?: OtherParams;
}
