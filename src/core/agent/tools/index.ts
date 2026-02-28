import { ToolsOptions } from './types';

/**
 * 作用：处理AI返回的tool调用
 * 关联：被loop模块调用，处理AI请求的tool_calls
 * 预期结果：执行tool调用并返回结果数组
 */
export async function ProcessTools(tool_calls: any[], tools_realize: Record<string, Function>): Promise<any[]> {
    // 参数有效性检查
    if (!tool_calls || !Array.isArray(tool_calls)) {
        throw new Error('tool_calls must be an array');
    }

    if (tool_calls.length === 0) {
        throw new Error('tool_calls array is empty');
    }

    if (!tools_realize || typeof tools_realize !== 'object') {
        throw new Error('tools_realize must be an object');
    }

    // 存储所有tool调用的结果
    const results: any[] = [];

    // 遍历每个tool call
    for (const toolCall of tool_calls) {
        const toolCallId = toolCall.id;
        const functionName = toolCall.function?.name;
        const argumentsStr = toolCall.function?.arguments;

        // 验证tool call格式
        if (!toolCallId) {
            throw new Error('tool_call missing id');
        }

        if (!functionName) {
            throw new Error(`tool_call ${toolCallId} missing function name`);
        }

        // 查找对应的函数实现
        const functionImpl = tools_realize[functionName];

        if (!functionImpl) {
            throw new Error(`Function '${functionName}' not found in tools_realize`);
        }

        if (typeof functionImpl !== 'function') {
            throw new Error(`tools_realize['${functionName}'] is not a function`);
        }

        // 解析arguments（JSON字符串）
        let args: any = {};
        if (argumentsStr) {
            try {
                args = JSON.parse(argumentsStr);
            } catch (error: any) {
                throw new Error(`Failed to parse arguments for '${functionName}': ${error.message}`);
            }
        }

        // 调用函数
        try {
            console.log(`调用Tool: ${functionName}, 参数:`, args);
            
            // 调用函数（支持同步和异步函数）
            // 将args对象的值按照函数参数顺序传递
            const result = await functionImpl(...Object.values(args));
            
            console.log(`Tool ${functionName} 执行成功，结果:`, result);

            // 将结果添加到结果数组
            results.push({
                tool_call_id: toolCallId,
                function_name: functionName,
                result: result
            });

        } catch (error: any) {
            console.error(`Tool ${functionName} 执行失败:`, error.message);
            
            // 即使失败也要记录结果
            results.push({
                tool_call_id: toolCallId,
                function_name: functionName,
                error: error.message
            });
        }
    }

    return results;
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
