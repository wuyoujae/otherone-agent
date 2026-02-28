# otherone-agent

一个轻量级、可扩展的 AI Agent 基础架构，使用 Node.js + TypeScript 构建。

## 特性

- 🤖 支持多种 AI 提供商（OpenAI、Anthropic、Fetch）
- 💾 自动上下文管理和存储
- 🔄 智能上下文压缩
- 🛠️ 工具调用支持
- 📦 模块化设计，易于扩展
- 💪 完整的 TypeScript 类型支持

## 安装

```bash
npm install otherone-agent
```

## 快速开始

### 1. 创建新的会话

在开始对话之前，首先需要创建一个新的会话：

```typescript
import { CreateNewSession } from 'otherone-agent';

// 创建新会话，返回 session_id
const sessionId = CreateNewSession();
console.log('新会话ID:', sessionId);
```

### 2. 调用 Agent

使用 `InvokeAgent` 函数来启动 AI 对话：

```typescript
import { InvokeAgent, InputOptions, AIOptions } from 'otherone-agent';

// 配置输入参数
const input: InputOptions = {
    // 会话ID（必填）- 使用 CreateNewSession() 创建的 session_id
    sessionId: 'your-session-id',
    
    // 上下文加载类型（必填）- 'localfile' 或 'database'
    // 目前只支持 'localfile'
    contextLoadType: 'localfile',
    
    // 存储类型（可选，默认 'localfile'）
    storageType: 'localfile',
    
    // 模型的上下文窗口大小（必填）
    // 例如：GPT-4 是 128000，GPT-3.5 是 16385
    contextWindow: 128000,
    
    // 触发压缩的阈值百分比（可选，默认 0.8，即 80%）
    // 当 token 使用量超过 contextWindow * thresholdPercentage 时触发压缩
    thresholdPercentage: 0.8,
    
    // 最大循环次数（可选，默认 999999）
    // 防止无限循环，建议设置为 50-100
    maxIterations: 50
};

// 配置 AI 参数
const ai: AIOptions = {
    // AI 提供商类型（必填）- 'openai' | 'anthropic' | 'fetch'
    // 目前只实现了 'openai'
    provider: 'openai',
    
    // API 密钥（必填）
    apiKey: 'your-openai-api-key',
    
    // API 基础 URL（必填）
    baseUrl: 'https://api.openai.com/v1',
    
    // 模型名称（必填）
    model: 'gpt-4',
    
    // 用户提示词（必填）- 本次对话的用户输入
    userPrompt: '你好，请介绍一下你自己',
    
    // 系统提示词（可选）
    systemPrompt: '你是一个友好的AI助手',
    
    // 采样温度（可选，默认 1.0）
    // 范围：0.0 - 2.0，值越高输出越随机
    temperature: 0.7,
    
    // 核采样参数（可选，默认 1.0）
    // 范围：0.0 - 1.0
    topP: 0.9,
    
    // 上下文长度限制（可选）- 即 max_tokens
    contextLength: 4096,
    
    // 是否启用流式响应（可选，默认 false）
    stream: false,
    
    // 工具定义数组（可选）
    tools: [
        {
            type: 'function',
            function: {
                name: 'get_weather',
                description: '获取指定城市的天气信息',
                parameters: {
                    type: 'object',
                    properties: {
                        city: {
                            type: 'string',
                            description: '城市名称'
                        }
                    },
                    required: ['city']
                }
            }
        }
    ],
    
    // 工具实现函数映射（可选）
    // 键名必须与 tools 中的 function.name 一致
    tools_realize: {
        get_weather: async (args: any) => {
            // 实现获取天气的逻辑
            const { city } = args;
            return {
                city: city,
                temperature: 25,
                condition: '晴天'
            };
        }
    },
    
    // 控制工具调用行为（可选）
    // 'none' - 不调用工具
    // 'auto' - 自动决定是否调用工具（默认）
    // 'required' - 必须调用工具
    // { type: 'function', function: { name: 'tool_name' } } - 强制调用指定工具
    toolChoice: 'auto',
    
    // 是否启用并行工具调用（可选，默认 true）
    parallelToolCalls: true,
    
    // 其他兼容参数（可选）
    other: {
        // 客户端构建参数（用于初始化 OpenAI 客户端）
        client: {
            timeout: 60000,
            maxRetries: 3
        },
        // 聊天请求参数（用于 completion 请求）
        chat: {
            presence_penalty: 0,
            frequency_penalty: 0
        }
    }
};

// 调用 Agent
async function main() {
    try {
        const response = await InvokeAgent(input, ai);
        
        console.log('AI 响应:', response);
        // 响应格式：
        // {
        //     content: 'AI 的回复内容',
        //     role: 'assistant',
        //     token_consumption: 1234,  // token 消耗量
        //     tools: null,              // 如果有工具调用，这里会包含工具信息
        //     thinking: null,           // 思考内容（如果支持）
        //     raw_response: {...}       // 原始响应对象
        // }
        
    } catch (error) {
        console.error('调用失败:', error);
    }
}

main();
```

## 完整示例

### 基础对话示例

```typescript
import { 
    CreateNewSession, 
    InvokeAgent, 
    InputOptions, 
    AIOptions 
} from 'otherone-agent';

async function basicChat() {
    // 1. 创建新会话
    const sessionId = CreateNewSession();
    console.log('会话ID:', sessionId);
    
    // 2. 配置参数
    const input: InputOptions = {
        sessionId: sessionId,
        contextLoadType: 'localfile',
        contextWindow: 128000,
        thresholdPercentage: 0.8
    };
    
    const ai: AIOptions = {
        provider: 'openai',
        apiKey: process.env.OPENAI_API_KEY || '',
        baseUrl: 'https://api.openai.com/v1',
        model: 'gpt-4',
        userPrompt: '你好，请介绍一下你自己',
        systemPrompt: '你是一个友好的AI助手',
        temperature: 0.7
    };
    
    // 3. 调用 Agent
    const response = await InvokeAgent(input, ai);
    console.log('AI:', response.content);
}

basicChat();
```

### 多轮对话示例

```typescript
import { 
    CreateNewSession, 
    InvokeAgent, 
    InputOptions, 
    AIOptions 
} from 'otherone-agent';

async function multiTurnChat() {
    // 创建会话
    const sessionId = CreateNewSession();
    
    // 配置基础参数（多轮对话中保持不变）
    const input: InputOptions = {
        sessionId: sessionId,
        contextLoadType: 'localfile',
        contextWindow: 128000
    };
    
    const baseAI: AIOptions = {
        provider: 'openai',
        apiKey: process.env.OPENAI_API_KEY || '',
        baseUrl: 'https://api.openai.com/v1',
        model: 'gpt-4',
        systemPrompt: '你是一个友好的AI助手',
        temperature: 0.7
    };
    
    // 第一轮对话
    console.log('用户: 你好，我叫小明');
    let response = await InvokeAgent(input, {
        ...baseAI,
        userPrompt: '你好，我叫小明'
    });
    console.log('AI:', response.content);
    
    // 第二轮对话（会自动加载历史上下文）
    console.log('\n用户: 我刚才告诉你我叫什么名字？');
    response = await InvokeAgent(input, {
        ...baseAI,
        userPrompt: '我刚才告诉你我叫什么名字？'
    });
    console.log('AI:', response.content);
    
    // 第三轮对话
    console.log('\n用户: 帮我写一首关于春天的诗');
    response = await InvokeAgent(input, {
        ...baseAI,
        userPrompt: '帮我写一首关于春天的诗'
    });
    console.log('AI:', response.content);
}

multiTurnChat();
```

### 工具调用示例

```typescript
import { 
    CreateNewSession, 
    InvokeAgent, 
    InputOptions, 
    AIOptions 
} from 'otherone-agent';

async function toolCallingExample() {
    const sessionId = CreateNewSession();
    
    const input: InputOptions = {
        sessionId: sessionId,
        contextLoadType: 'localfile',
        contextWindow: 128000
    };
    
    const ai: AIOptions = {
        provider: 'openai',
        apiKey: process.env.OPENAI_API_KEY || '',
        baseUrl: 'https://api.openai.com/v1',
        model: 'gpt-4',
        userPrompt: '北京今天天气怎么样？',
        systemPrompt: '你是一个天气助手',
        
        // 定义工具
        tools: [
            {
                type: 'function',
                function: {
                    name: 'get_weather',
                    description: '获取指定城市的天气信息',
                    parameters: {
                        type: 'object',
                        properties: {
                            city: {
                                type: 'string',
                                description: '城市名称，例如：北京、上海'
                            },
                            unit: {
                                type: 'string',
                                enum: ['celsius', 'fahrenheit'],
                                description: '温度单位'
                            }
                        },
                        required: ['city']
                    }
                }
            }
        ],
        
        // 实现工具函数
        tools_realize: {
            get_weather: async (args: any) => {
                const { city, unit = 'celsius' } = args;
                
                // 这里应该调用真实的天气API
                // 示例返回模拟数据
                return {
                    city: city,
                    temperature: unit === 'celsius' ? 25 : 77,
                    unit: unit,
                    condition: '晴天',
                    humidity: 60,
                    wind_speed: 15
                };
            }
        },
        
        toolChoice: 'auto'
    };
    
    const response = await InvokeAgent(input, ai);
    console.log('AI:', response.content);
}

toolCallingExample();
```

### 流式响应示例

```typescript
import { 
    CreateNewSession, 
    InvokeAgent, 
    InputOptions, 
    AIOptions 
} from 'otherone-agent';

async function streamingExample() {
    const sessionId = CreateNewSession();
    
    const input: InputOptions = {
        sessionId: sessionId,
        contextLoadType: 'localfile',
        contextWindow: 128000
    };
    
    const ai: AIOptions = {
        provider: 'openai',
        apiKey: process.env.OPENAI_API_KEY || '',
        baseUrl: 'https://api.openai.com/v1',
        model: 'gpt-4',
        userPrompt: '请写一篇关于人工智能的短文',
        systemPrompt: '你是一个专业的写作助手',
        stream: true  // 启用流式响应
    };
    
    const response = await InvokeAgent(input, ai);
    
    // 流式响应会在内部处理，最终返回完整内容
    console.log('完整响应:', response.content);
    console.log('Token消耗:', response.token_consumption);
}

streamingExample();
```

## API 文档

### 核心函数

#### `InvokeAgent(input, ai)`

调用 AI Agent 进行对话。

**参数：**
- `input: InputOptions` - 输入配置
- `ai: AIOptions` - AI 配置

**返回：**
```typescript
Promise<{
    content: string;           // AI 回复内容
    role: string;              // 角色（通常是 'assistant'）
    token_consumption: number; // token 消耗量
    tools: any | null;         // 工具调用信息
    thinking: any | null;      // 思考内容
    raw_response: any;         // 原始响应
}>
```

### 会话管理

#### `CreateNewSession()`

创建新的会话。

**返回：** `string` - 新创建的 session_id

**示例：**
```typescript
const sessionId = CreateNewSession();
```

#### `GetAllSessions()`

获取所有会话的基本信息。

**返回：** `Array<{ session_id: string; status: number; create_at: string }>`

**示例：**
```typescript
const sessions = GetAllSessions();
console.log('所有会话:', sessions);
// [
//   { session_id: 'xxx', status: 0, create_at: '2024-01-01T00:00:00.000Z' },
//   ...
// ]
```

#### `ReadSessionData(sessionId)`

读取指定会话的完整数据。

**参数：**
- `sessionId: string` - 会话ID

**返回：**
```typescript
{
    session: {
        session_id: string;
        status: number;
        create_at: string;
    } | null;
    entries: Array<any>;           // 对话记录
    compacted_entries: Array<any>; // 压缩记录
}
```

**示例：**
```typescript
import { ReadSessionData } from 'otherone-agent';

const sessionData = ReadSessionData('your-session-id');
console.log('会话数据:', sessionData);
```

## 类型定义

### InputOptions

```typescript
interface InputOptions {
    sessionId: string;                    // 会话ID（必填）
    contextLoadType: 'database' | 'localfile'; // 上下文加载类型（必填）
    storageType?: 'localfile' | 'database';    // 存储类型（可选）
    contextWindow: number;                     // 上下文窗口大小（必填）
    thresholdPercentage?: number;              // 压缩阈值（可选，默认0.8）
    maxIterations?: number;                    // 最大循环次数（可选）
}
```

### AIOptions

```typescript
interface AIOptions {
    provider: 'openai' | 'anthropic' | 'fetch'; // AI提供商（必填）
    apiKey: string;                             // API密钥（必填）
    baseUrl: string;                            // 基础URL（必填）
    model: string;                              // 模型名称（必填）
    userPrompt?: string;                        // 用户提示词（可选）
    systemPrompt?: string;                      // 系统提示词（可选）
    messages?: any[];                           // 消息列表（可选）
    contextLength?: number;                     // 上下文长度限制（可选）
    temperature?: number;                       // 采样温度（可选）
    topP?: number;                              // 核采样参数（可选）
    tools?: any[];                              // 工具定义（可选）
    tools_realize?: Record<string, Function>;   // 工具实现（可选）
    toolChoice?: 'none' | 'auto' | 'required' | object; // 工具调用控制（可选）
    parallelToolCalls?: boolean;                // 并行工具调用（可选）
    stream?: boolean;                           // 流式响应（可选）
    other?: any;                                // 其他参数（可选）
}
```

## 高级功能

### 上下文压缩

当对话历史过长时，系统会自动触发上下文压缩：

1. 当 token 使用量超过 `contextWindow * thresholdPercentage` 时触发
2. 保留最近的对话（默认保留 40%）
3. 将旧的对话压缩成摘要
4. 压缩记录会自动存储

```typescript
const input: InputOptions = {
    sessionId: sessionId,
    contextLoadType: 'localfile',
    contextWindow: 128000,
    thresholdPercentage: 0.8  // 80% 时触发压缩
};
```

### 自定义压缩 LLM

可以为压缩功能指定不同的模型：

```typescript
const ai: AIOptions = {
    provider: 'openai',
    apiKey: 'your-api-key',
    baseUrl: 'https://api.openai.com/v1',
    model: 'gpt-4',  // 主对话模型
    
    // 其他参数...
    
    other: {
        // 压缩专用配置（可选）
        compact_llm_provider: 'openai',
        compact_llm_model: 'gpt-3.5-turbo',  // 使用更便宜的模型进行压缩
        compact_llm_apiKey: 'your-api-key',
        compact_llm_baseUrl: 'https://api.openai.com/v1',
        compact_llm_temperature: 0.3
    }
};
```

## 注意事项

1. **Session 管理**：每次新对话都应该创建新的 session，不要重复使用
2. **API Key 安全**：不要在代码中硬编码 API Key，使用环境变量
3. **Context Window**：确保设置正确的 contextWindow 值，不同模型有不同的限制
4. **工具函数**：tools_realize 中的函数名必须与 tools 定义中的 function.name 完全一致
5. **错误处理**：建议使用 try-catch 包裹 InvokeAgent 调用
6. **存储位置**：会话数据默认存储在 `.veloca/storage/veloca-storage.json`

## 常见问题

### Q: 如何继续之前的对话？

A: 使用相同的 sessionId 调用 InvokeAgent 即可，系统会自动加载历史上下文。

### Q: 如何清空对话历史？

A: 创建新的 session 或手动删除 `.veloca/storage/veloca-storage.json` 文件。

### Q: 支持哪些 AI 提供商？

A: 目前完整实现了 OpenAI，Anthropic 和 Fetch 正在开发中。

### Q: 工具调用失败怎么办？

A: 检查 tools_realize 中的函数名是否与 tools 定义匹配，确保函数返回值格式正确。

### Q: 如何调试？

A: 可以查看 `.veloca/storage/veloca-storage.json` 文件查看完整的对话历史和压缩记录。

## 许可证

MIT

## 贡献

欢迎提交 Issue 和 Pull Request！
