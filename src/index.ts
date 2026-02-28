/**
 * otherone-agent - 轻量级AI Agent基础架构
 * 作用：包的主入口文件，导出veloca对象和类型定义
 */

// 导入所有模块
import { InvokeAgent } from './core/agent/agentLoop/loop';
import { InvokeModel } from './core/agent/agentLoop/ai';
import { ProcessTools, CombineTools } from './core/agent/tools';
import { CombineContext } from './core/agent/context/combineContext';
import { CompactMessages } from './core/agent/context/compact';
import { CheckThreshold } from './core/agent/context/compact/checkThreshold';
import { EstimateTokens } from './core/agent/context/compact/estimateTokens';
import { WriteEntry, WriteCompactedEntry } from './core/agent/context/storage';
import { ReadSessionData, ReadStorageFile, GetAllSessions } from './core/agent/context/storage/localfile/reader';
import { CreateNewSession } from './core/agent/context/storage/localfile/writer';

/**
 * 作用：veloca核心对象，封装所有公开API
 * 关联：用户通过导入veloca对象来使用所有功能
 * 预期结果：提供统一的API入口，简化用户使用
 */
export const veloca = {
    // Agent核心方法
    InvokeAgent,
    
    // AI模块方法
    InvokeModel,
    
    // 工具模块方法
    ProcessTools,
    CombineTools,
    
    // 上下文管理方法
    CombineContext,
    CompactMessages,
    CheckThreshold,
    EstimateTokens,
    
    // 存储模块方法
    WriteEntry,
    WriteCompactedEntry,
    ReadSessionData,
    ReadStorageFile,
    GetAllSessions,
    CreateNewSession
};

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
