<div align="center">
  <img src="./logo.svg" alt="otherone-agent logo" width="200"/>
  
  <h1 align="center">otherone-agent</h1>

  <p align="center">轻量级AI Agent基础架构</p>

  [![npm version](https://img.shields.io/npm/v/otherone-agent.svg)](https://www.npmjs.com/package/otherone-agent)
  [![license](https://img.shields.io/npm/l/otherone-agent.svg)](https://github.com/yourusername/otherone-agent/blob/main/LICENSE)

  [English](../README.md) | 简体中文

</div>

> 这个产品赠送给我最好的她！她喜欢向日葵 🌻

## 🎯 愿景

otherone-agent 不仅仅是另一个 AI 框架。它是开发者构建智能代理方式的**范式转变**。

我们相信 AI 代理开发应该是：
- **简单** - 8 行代码即可投入生产
- **强大** - 开箱即用的企业级功能
- **可扩展** - 插件架构带来无限可能
- **高效** - 智能上下文管理节省 80% token 成本

### 问题所在

当前的 AI 框架迫使你在简单性和强大功能之间做出选择。你要么得到一个无法扩展的玩具示例，要么得到一个需要数周才能理解的复杂企业解决方案。

### 解决方案

otherone-agent 让你**两者兼得**。从 8 行代码开始，扩展到数百万用户。

## 📦 安装

```bash
npm install otherone-agent
```

## 🚀 快速开始

> 💡 **AI 快速开发提示**：可以发送下面这个 prompt 使用 AI 快速开发：
> 
> "阅读这个链接：https://github.com/wuyoujae/otherone-agent，请你使用 otherone-agent 帮我快速开发一个带 webui 的对话 agent"

### 基础使用

```typescript
import { veloca } from 'otherone-agent';

// 创建新对话
const sessionId = veloca.CreateNewSession();

// 第一轮对话
await veloca.InvokeAgent(
    { sessionId, contextLoadType: 'localfile', contextWindow: 128000 },
    {
        provider: 'openai',
        apiKey: process.env.OPENAI_API_KEY,
        baseUrl: 'https://api.openai.com/v1',
        model: 'gpt-4o-mini',
        userPrompt: '2+2等于多少？',
        stream: true
    }
);

// 第二轮对话 - 自动加载历史记录
const response = await veloca.InvokeAgent(
    { sessionId, contextLoadType: 'localfile', contextWindow: 128000 },
    {
        provider: 'openai',
        apiKey: process.env.OPENAI_API_KEY,
        baseUrl: 'https://api.openai.com/v1',
        model: 'gpt-4o-mini',
        userPrompt: '把这个数字乘以3',
        stream: true
    }
);

console.log(response.content); // "12"
```

### 使用示例

<div align="center">
  <img src="./image.png" alt="使用示例" width="800"/>
</div>

### 使用工具

```typescript
const tools = [{
    type: 'function',
    function: {
        name: 'get_weather',
        description: '获取当前天气',
        parameters: {
            type: 'object',
            properties: {
                location: { type: 'string' }
            }
        }
    }
}];

const tools_realize = {
    get_weather: async (location: string) => {
        return `${location}的天气：晴天，22°C`;
    }
};

const response = await veloca.InvokeAgent(
    { sessionId, contextLoadType: 'localfile', contextWindow: 128000 },
    {
        provider: 'openai',
        apiKey: process.env.OPENAI_API_KEY,
        baseUrl: 'https://api.openai.com/v1',
        model: 'gpt-4o-mini',
        userPrompt: '旧金山的天气怎么样？',
        tools,
        tools_realize,
        stream: true
    }
);
```

就是这样。你现在拥有：
- ✅ 多轮对话记忆
- ✅ 自动上下文管理
- ✅ 流式响应
- ✅ 工具调用支持
- ✅ 智能上下文压缩
- ✅ 生产就绪的持久化

## 📚 高级功能

### 上下文压缩

Veloca 在接近 token 限制时会自动压缩对话历史：

```typescript
const response = await veloca.InvokeAgent(
    {
        sessionId,
        contextLoadType: 'localfile',
        contextWindow: 128000,
        thresholdPercentage: 0.8  // 在 80% 容量时压缩
    },
    {
        provider: 'openai',
        apiKey: process.env.OPENAI_API_KEY,
        baseUrl: 'https://api.openai.com/v1',
        model: 'gpt-4o-mini',
        userPrompt: '继续我们的对话...',
        // 压缩 LLM 配置（可选）
        compact_llm_model: 'gpt-4o-mini',
        compact_llm_temperature: 0.3,
        stream: true
    }
);
```

### 自定义存储

```typescript
// 读取会话数据
const sessionData = veloca.ReadSessionData(sessionId);

// 获取所有会话
const allSessions = veloca.GetAllSessions();

// 手动写入条目
veloca.WriteEntry({
    storageType: 'localfile',
    sessionId,
    role: 'user',
    content: '自定义消息'
});
```

## 🔥 核心功能

### 🧠 智能上下文管理
- **自动压缩**: 接近 token 限制时自动总结对话历史
- **Token 估算**: 内置 token 计数，帮助你控制成本
- **可配置阈值**: 自定义压缩触发时机（默认 80%）

### 🔄 多提供商支持
- **OpenAI**: 完整支持，包括流式响应
- **Anthropic**: 即将推出
- **自定义 API**: 可扩展架构，支持接入你自己的 LLM

### 🛠️ 简单的工具调用
- **轻松定义**: 定义你的工具，我们处理执行循环
- **类型安全**: 完整的 TypeScript 支持，更好的开发体验
- **错误处理**: 内置重试和错误管理

### 💾 零配置存储
- **本地文件**: 基于 JSON 的存储，无需配置
- **会话管理**: 基于 UUID 的对话追踪
- **历史记录**: 完整的交互审计跟踪

### 🏗️ 为什么选择 otherone-agent？

**轻量级**: 没有笨重的依赖，只有你需要的核心功能。

**开发者友好**: 合理的默认配置，最少的配置即可开始使用。

**模块化**: 按需使用 - token 估算、上下文管理或完整的 agent 循环。

**透明**: 简单、可读的代码。没有魔法，没有惊喜。

## ✨ 特性

- 🚀 支持流式和非流式响应
- 🔧 自动tool循环处理
- 💾 灵活的上下文管理和压缩
- 📦 模块化设计，易于扩展
- 🔌 支持多种AI提供商（OpenAI、Anthropic、Fetch）

## 🎯 开发路线

### ✅ 已完成
- 核心 agent 循环
- OpenAI 集成
- 上下文管理
- 工具调用
- 本地文件存储
- 流式响应支持

### 🚧 进行中
- MCP server 集成
- Skills 系统
- Web UI

### 📋 计划中
- 更多 provider 支持（Anthropic、Claude 等）
- Database 存储适配器
- 高级缓存策略
- 插件市场
- ...更多功能！

## 📄 许可证

MIT
