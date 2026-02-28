# otherone-agent

轻量级AI Agent基础架构，支持多种AI提供商、流式响应、工具调用和上下文管理。

## 特性

- 🚀 支持流式和非流式响应
- 🔧 自动tool循环处理
- 💾 灵活的上下文管理和压缩
- 📦 模块化设计，易于扩展
- 🔌 支持多种AI提供商（OpenAI、Anthropic、Fetch）

## 安装

```bash
npm install otherone-agent
```

## 快速开始

### 基础使用

```typescript
import { InvokeAgent } from 'otherone-agent';

const input = {
    sessionId: 'my-session',
    contextLoadType: 'localfile',
    contextWindow: 4000
};

const ai = {
    provider: 'openai',
    apiKey: 'your-api-key',
    baseUrl: 'https://api.openai.com/v1',
    model: 'gpt-3.5-turbo',
    stream: false  // 非流式模式
};

const response = await InvokeAgent(input, ai);
console.log(response.content);
```

### 流式响应

```typescript
import { InvokeAgent } from 'otherone-agent';

const ai = {
    provider: 'openai',
    apiKey: 'your-api-key',
    baseUrl: 'https://api.openai.com/v1',
    model: 'gpt-3.5-turbo',
    stream: true  // 启用流式
};

const stream = await InvokeAgent(input, ai);

for await (const chunk of stream) {
    // 处理特殊消息
    if (chunk.type === 'thinking') {
        console.log(chunk.content);  // [thinking:...]
    } else if (chunk.type === 'tool_calls') {
        console.log(chunk.content);  // [tool_calls:...]
    } else if (chunk.type === 'error') {
        console.error(chunk.content);  // [error:...]
    }
    // 处理普通内容
    else if (chunk.choices?.[0]?.delta?.content) {
        process.stdout.write(chunk.choices[0].delta.content);
    }
}
```

### 工具调用

```typescript
import { InvokeAgent } from 'otherone-agent';

// 定义工具
function get_weather(city: string): any {
    return {
        city,
        temperature: 22,
        condition: '晴天'
    };
}

const ai = {
    provider: 'openai',
    apiKey: 'your-api-key',
    baseUrl: 'https://api.openai.com/v1',
    model: 'gpt-3.5-turbo',
    stream: true,
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
    tools_realize: { get_weather },
    toolChoice: 'auto'
};

const stream = await InvokeAgent(input, ai);

for await (const chunk of stream) {
    if (chunk.type === 'tool_calls') {
        console.log('AI正在调用工具:', chunk.content);
    } else if (chunk.choices?.[0]?.delta?.content) {
        process.stdout.write(chunk.choices[0].delta.content);
    }
}
```

## API文档

### InvokeAgent

核心Agent调用方法，支持流式和非流式响应。

```typescript
function InvokeAgent(
    input: InputOptions,
    ai: AIOptions
): Promise<any | AsyncGenerator<any, any, unknown>>
```

#### InputOptions

```typescript
interface InputOptions {
    sessionId: string;              // 会话ID
    contextLoadType: 'database' | 'localfile';  // 上下文加载类型
    storageType?: 'localfile' | 'database';     // 存储类型
    contextWindow: number;          // 上下文窗口大小
    thresholdPercentage?: number;   // 压缩阈值（默认0.8）
    maxIterations?: number;         // 最大循环次数（默认999999）
}
```

#### AIOptions

```typescript
interface AIOptions {
    provider: 'openai' | 'anthropic' | 'fetch';  // AI提供商
    apiKey: string;                 // API密钥
    baseUrl: string;                // 基础URL
    model: string;                  // 模型名称
    userPrompt?: string;            // 用户提示词
    systemPrompt?: string;          // 系统提示词
    messages?: any[];               // 消息列表
    contextLength?: number;         // 上下文长度限制
    temperature?: number;           // 采样温度
    topP?: number;                  // 核采样参数
    tools?: any[];                  // 工具定义数组
    tools_realize?: Record<string, Function>;  // 工具实现函数映射
    toolChoice?: 'none' | 'auto' | 'required';  // 工具调用行为
    parallelToolCalls?: boolean;    // 是否启用并行工具调用
    stream?: boolean;               // 启用流式响应
    other?: any;                    // 其他兼容参数
}
```

### 特殊消息类型

流式响应中会包含以下特殊消息：

#### thinking消息
```typescript
{
    type: 'thinking',
    content: '[thinking:AI的思考过程]'
}
```

#### tool_calls消息
```typescript
{
    type: 'tool_calls',
    content: '[tool_calls:get_weather({"city":"北京"})]'
}
```

#### error消息
```typescript
{
    type: 'error',
    content: '[error:错误信息]',
    error: '错误信息'
}
```

## 其他功能

### 上下文管理

```typescript
import { CombineContext, CompactMessages } from 'otherone-agent';

// 组合上下文
const messages = await CombineContext({
    sessionId: 'my-session',
    loadType: 'localfile',
    provider: 'openai',
    contextWindow: 4000,
    ai: aiOptions
});

// 压缩消息
const compactedMessages = await CompactMessages({
    messages: messages,
    contextTokens: 3000,
    contextWindow: 4000,
    ai: aiOptions
});
```

### 存储管理

```typescript
import { 
    WriteEntry, 
    ReadSessionData, 
    CreateNewSession 
} from 'otherone-agent';

// 创建新会话
CreateNewSession('my-session', 'localfile');

// 写入entry
WriteEntry({
    storageType: 'localfile',
    sessionId: 'my-session',
    role: 'user',
    content: '你好'
});

// 读取会话数据
const sessionData = ReadSessionData('my-session');
```

## 开发

```bash
# 安装依赖
npm install

# 编译
npx tsc

# 运行测试
npx ts-node test-script/test-stream-response.ts
```

## 许可证

MIT
