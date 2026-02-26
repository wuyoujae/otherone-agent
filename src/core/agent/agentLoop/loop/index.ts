import { InputOptions, AIOptions } from './types';

/**
 * 作用：调用Agent，驱动整个AI对话流程
 * 关联：调用ai模块、contextManager、toolsCalling等其他模块，是整个Agent的核心驱动
 * 预期结果：根据input和ai配置，执行完整的Agent循环，返回最终响应
 */
export async function InvokeAgent(input: InputOptions, ai: AIOptions): Promise<any> {
    // TODO: 实现Agent调用逻辑
    while(true){
        
    }
}
