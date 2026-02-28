/**
 * otherone-agent - 轻量级AI Agent基础架构
 * 作用：包的主入口文件，导出所有公开API
 */

// 导出核心Agent循环
export { InvokeAgent } from './core/agent/agentLoop/loop';

// 导出AI模块
export { InvokeModel } from './core/agent/agentLoop/ai';

// 导出工具模块
export { ProcessTools, CombineTools } from './core/agent/tools';

// 导出上下文管理模块
export { CombineContext } from './core/agent/context/combineContext';
export { CompactMessages } from './core/agent/context/compact';
export { CheckThreshold } from './core/agent/context/compact/checkThreshold';
export { EstimateTokens } from './core/agent/context/compact/estimateTokens';

// 导出存储模块
export { WriteEntry, WriteCompactedEntry } from './core/agent/context/storage';
export { ReadSessionData, ReadStorageFile, GetAllSessions } from './core/agent/context/storage/localfile/reader';
export { CreateNewSession } from './core/agent/context/storage/localfile/writer';

// 导出类型定义
export type {
    InputOptions,
    AIOptions,
    ProviderType as LoopProviderType,
    ContextLoadType,
    StorageType
} from './core/agent/agentLoop/loop/types';

export type {
    ConfigOptions as AIConfigOptions,
    ProviderType as AIProviderType
} from './core/agent/agentLoop/ai/types';

export type {
    ConfigOptions as OpenAIConfigOptions,
    OtherParams as OpenAIOtherParams
} from './core/agent/agentLoop/ai/openai/types';

export type {
    CombineContextOptions,
    ProviderType as ContextProviderType
} from './core/agent/context/combineContext/types';

export type {
    CompactOptions
} from './core/agent/context/compact/types';

export type {
    CheckThresholdOptions
} from './core/agent/context/compact/checkThreshold/types';

export type {
    EstimateTokensOptions
} from './core/agent/context/compact/estimateTokens/types';

export type {
    WriteEntryOptions,
    WriteCompactedEntryOptions,
    StorageType as StorageTypeEnum
} from './core/agent/context/storage/types';

export type {
    ToolsOptions
} from './core/agent/tools/types';
