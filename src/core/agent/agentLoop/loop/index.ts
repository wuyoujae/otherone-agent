import { InputOptions, AIOptions } from './types';
import { CombineTools } from '../../tools';
import { CombineContext } from '../../context/combineContext';

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
        const messages = CombineContext({
            sessionId: input.sessionId,
            loadType: input.contextLoadType
        });
        
        // 将历史消息添加到ai配置中
        ai.messages = messages;
    }
}
