/**
 * 作用：将消息数组转换为文本序列格式
 * 关联：被CompactMessages调用，用于将消息转换为可发送给LLM的文本格式
 * 预期结果：返回格式化的文本序列字符串
 */
export function MessagesToSequence(messages: any[]): string {
    const sequences: string[] = [];

    for (const message of messages) {
        const role = message.role;
        const content = message.content;

        switch (role) {
            case 'user':
                // 用户消息格式：[User]: 消息内容
                sequences.push(`[User]: ${content}`);
                break;

            case 'assistant':
                // 助手消息格式：[Assistant]: 回复内容
                sequences.push(`[Assistant]: ${content || '(无文本回复)'}`);
                
                // 如果有工具调用，添加工具调用信息
                if (message.tool_calls && message.tool_calls.length > 0) {
                    const toolCallsInfo = message.tool_calls.map((tc: any) => {
                        const functionName = tc.function?.name || 'unknown';
                        const functionArgs = tc.function?.arguments || '{}';
                        return `  - ${functionName}(${functionArgs})`;
                    }).join('\n');
                    sequences.push(`[Tool Calls]:\n${toolCallsInfo}`);
                }
                break;

            case 'tool':
                // 工具结果格式：[Tool Result]: 结果内容
                const toolName = message.name || 'unknown_tool';
                sequences.push(`[Tool Result - ${toolName}]: ${content}`);
                break;

            case 'system':
                // 系统消息格式：[System]: 系统指令
                sequences.push(`[System]: ${content}`);
                break;

            case 'developer':
                // 开发者消息格式：[Developer]: 开发者指令
                sequences.push(`[Developer]: ${content}`);
                break;

            default:
                // 未知类型，使用通用格式
                sequences.push(`[${role}]: ${content}`);
                break;
        }
    }

    // 使用双换行符分隔不同的消息，使序列更清晰
    return sequences.join('\n\n');
}
