import { ToolsOptions } from './types';

/**
 * 作用：处理工具调用配置
 * 关联：被agentLoop模块调用，将用户传入的tools数组传递给AI模块
 * 预期结果：返回处理后的tools配置，供AI模块使用
 */
export function ProcessTools(tools: ToolsOptions): any[] {
    // 参数有效性检查
    if (!tools.tools || !Array.isArray(tools.tools)) {
        throw new Error('tools must be an array');
    }

    // 直接返回tools数组
    return tools.tools;
}

/**
 * 作用：组合tools配置，修改ai参数中的tools
 * 关联：被loop模块调用，在InvokeAgent循环中处理tools
 * 预期结果：修改传入的ai对象，将tools组合后更新到ai.tools中
 */
export function CombineTools(ai: any): any {
    // TODO: 实现tools组合逻辑
    return ai;
}
