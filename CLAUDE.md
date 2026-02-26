# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**veloca** is a newly initialized Node.js project intended to become an extensible, lightweight AI Agent基础架构 (AI Agent basic framework).

这是一个自研的可拓展性的轻量级的AI Agent基础架构，以Node.js模块形式构建。核心设计原则是：可拓展性、模块化。每个方法应尽可能打包成独立单元并模块化相互调用。

开发语言：Node.js + TypeScript

### Reference Documentation

The `src/docs/` directory contains extensive reference documentation on Claude API features that this framework may implement:

- **Tool Infrastructure**: Tool search, Programmatic tool calling, Fine-grained tool streaming
- **Context Management**: Context windows, Compaction, Context editing, Prompt caching, Token counting

These docs should be read before implementing corresponding features.

## Current State

- No build tools configured (TypeScript, bundler, etc.)
- No test framework configured
- No linting or formatting tools configured
- No CI/CD configuration
- `package.json` entry point is `index.js` but source uses `src/index.ts`
- No implementation yet - project is at skeleton phase

## Available Commands

```bash
npm test    # Currently returns an error - no test framework configured
```

## Architecture Principles

项目目录架构必须按照模块化思维构建。每个模块/目录应包含自己的类型定义文件。

### 目录层级设计原则

当一个模块可能包含多个不同实现时，应该进一步拆分为子目录：

```
// 错误的结构：所有代码混在一起
src/core/agent/ai/
├── index.ts
├── openai.ts
├── anthropic.ts
└── types.ts

// 正确的结构：按实现拆分
src/core/agent/ai/
├── index.ts          # 统一入口，负责provider分发
├── types/            # 共享类型定义（所有provider通用）
│   └── index.ts
├── openai/           # OpenAI专用目录
│   ├── index.ts      # 模块导出
│   └── client.ts     # 具体实现
├── anthropic/        # Anthropic专用目录
│   └── ...
└── fetch/            # Fetch专用目录
    └── ...
```

**何时创建子目录：**

1. **多种实现方式**：当模块支持多种不同实现（如多个API提供商、多种存储方式）
2. **独立依赖**：不同实现可能需要不同的外部SDK或依赖
3. **复杂逻辑**：单个实现的代码量较大，需要拆分为多个文件

**示例场景：**
- AI模块支持多个提供商（OpenAI、Anthropic等）→ 每个提供商一个子目录
- 数据库模块支持多种存储（MySQL、MongoDB等）→ 每种存储一个子目录
- 工具模块有多种类别（文件操作、网络请求等）→ 每种类别一个子目录

## Development Workflow

### Required Actions (必须要做的事情)

1. **Completion Message**: After every task, end with: "任务已经完成了杰哥，请你检查结果是否成功，如果有任何需要再叫我"

2. **Decision Authority**: For any subjective decisions, present options with detailed explanations and effects, then let the user choose.

3. **Documentation Maintenance**: After completing tasks, update CLAUDE.md with learned experience. Read relevant docs in `src/docs/` before development and update documentation accordingly.

4. **Incremental Development**: Do not complete multiple tasks at once. Complete only what is requested for better testing.

5. **Test Scripts**: Place any test scripts in `test-script/` directory.

## Code Conventions

### Comments (注释)

Use VS Code style comments with key information. No emojis.

Required information:
- 有什么作用？ (What does it do?)
- 和什么关联？ (What is it related to?)
- 预期结果是什么？ (What is the expected result?)

Use natural language with appropriate examples. Comments should be in Chinese.

### Naming Conventions (命名规范)

- **Functions**: PascalCase with meaningful names using professional terminology
- **Parameters**: kebab-case with meaningful names, following grammar standards (tense, plurals, etc.)

Example:
```typescript
// 正确示例
function ProcessUserData(user-inputs: string[], is-validating: boolean): void

// 错误示例
function process_user_data(userInputs: string[], isValidating: boolean): void
```

# 只需要完成我要求的代码！

例如：
要求：写一个模型配置的入口，这个入口的作用就是暴露一个方法，带一个参数是可以传入API类型，先完成openai，anthropic，fecth（网络请求）三种类型，可以使用像switch case那样，对于不同类型单独进行处理。然后其他的参数，必填的是apikey,baseurl，

正确的：
你完成一个方法，然后在对应目录中的types.ts中完成对上面这些参数的定义【没有定义任何其他的我没有提到过的参数】，，做好参数处理和判断，然后写一个switch case方法来匹配对应的类型，但是里面没有任何实现代码。

# 完成任务之后把你没有用的注释和测试脚本要删除掉

# 对应的代码要写到对应的目录之中

例如你在完成ai模块的代码，那么所有的例如types.ts和其他子模块都应该要写在ai目录里面！

注意，核心代码都要写在core/agent里面的子目录，那里是子模块
