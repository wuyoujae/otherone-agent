# OpenAI Tool Call Format

本文档详细说明OpenAI API中tool调用的响应格式。

## 非流式响应格式

### 单个Tool调用

```json
{
  "id": "chatcmpl-abc123",
  "object": "chat.completion",
  "created": 1699896916,
  "model": "gpt-4",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": null,
        "tool_calls": [
          {
            "id": "call_abc123",
            "type": "function",
            "function": {
              "name": "get_weather",
              "arguments": "{\"city\":\"北京\",\"unit\":\"celsius\"}"
            }
          }
        ]
      },
      "finish_reason": "tool_calls"
    }
  ],
  "usage": {
    "prompt_tokens": 82,
    "completion_tokens": 17,
    "total_tokens": 99
  }
}
```

### 多个Tool调用（并行调用）

当`parallel_tool_calls`为true时，AI可以同时调用多个tools：

```json
{
  "choices": [
    {
      "message": {
        "role": "assistant",
        "content": null,
        "tool_calls": [
          {
            "id": "call_abc123",
            "type": "function",
            "function": {
              "name": "get_weather",
              "arguments": "{\"city\":\"北京\"}"
            }
          },
          {
            "id": "call_def456",
            "type": "function",
            "function": {
              "name": "get_time",
              "arguments": "{\"timezone\":\"Asia/Shanghai\"}"
            }
          },
          {
            "id": "call_ghi789",
            "type": "function",
            "function": {
              "name": "search_news",
              "arguments": "{\"query\":\"今日新闻\",\"limit\":5}"
            }
          }
        ]
      },
      "finish_reason": "tool_calls"
    }
  ]
}
```

## 流式响应格式

流式响应中，tool_calls会分多个chunk传输，需要累积：

### Chunk 1: 开始tool call
```json
{
  "id": "chatcmpl-abc123",
  "object": "chat.completion.chunk",
  "created": 1699896916,
  "model": "gpt-4",
  "choices": [
    {
      "index": 0,
      "delta": {
        "role": "assistant",
        "content": null,
        "tool_calls": [
          {
            "index": 0,
            "id": "call_abc123",
            "type": "function",
            "function": {
              "name": "get_weather",
              "arguments": ""
            }
          }
        ]
      },
      "finish_reason": null
    }
  ]
}
```

### Chunk 2-N: 累积arguments
```json
{
  "choices": [
    {
      "index": 0,
      "delta": {
        "tool_calls": [
          {
            "index": 0,
            "function": {
              "arguments": "{\"city\""
            }
          }
        ]
      }
    }
  ]
}

{
  "choices": [
    {
      "index": 0,
      "delta": {
        "tool_calls": [
          {
            "index": 0,
            "function": {
              "arguments": ":\"北京\""
            }
          }
        ]
      }
    }
  ]
}

{
  "choices": [
    {
      "index": 0,
      "delta": {
        "tool_calls": [
          {
            "index": 0,
            "function": {
              "arguments": "}"
            }
          }
        ]
      }
    }
  ]
}
```

### 最后一个Chunk: finish_reason
```json
{
  "choices": [
    {
      "index": 0,
      "delta": {},
      "finish_reason": "tool_calls"
    }
  ]
}
```

## Tool Call字段说明

### tool_calls数组
- 每个tool call是一个对象
- 可以包含多个tool call（并行调用）
- 按照index顺序排列

### 单个tool call对象
```typescript
{
  id: string;           // 唯一标识符，格式：call_xxxxx
  type: string;         // 类型，通常是 "function"
  function: {
    name: string;       // 函数名称
    arguments: string;  // JSON字符串格式的参数
  }
}
```

### 流式响应中的index
- `index`: tool call在数组中的位置（从0开始）
- 用于在流式响应中正确累积多个tool calls
- 同一个tool call的所有chunks都有相同的index

## 重要注意事项

1. **content字段**：当有tool_calls时，content通常为null或空字符串
2. **finish_reason**：tool调用时为"tool_calls"
3. **arguments格式**：始终是JSON字符串，需要JSON.parse()解析
4. **并行调用**：tool_calls数组可以包含多个元素
5. **流式累积**：流式响应需要按index累积id、name和arguments

## 处理流程

1. 检查`message.tool_calls`或`delta.tool_calls`是否存在
2. 遍历tool_calls数组
3. 对于每个tool call：
   - 提取id、type、function.name
   - 解析function.arguments（JSON字符串）
   - 调用对应的函数
   - 收集结果
4. 将结果作为tool消息发送回AI
