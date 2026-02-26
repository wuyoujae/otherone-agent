# Veloca本地文件存储格式说明

## 概述

本地文件存储使用JSON格式，将数据库的三张表映射为JSON对象的三个数组。

## 数据结构

### 根对象
```json
{
  "sessions": [],           // 会话列表
  "entries": [],            // 消息记录列表
  "compacted_dialogues": [] // 压缩对话列表
}
```

### sessions - 会话列表

存储所有会话的基本信息。

**字段说明：**
- `session_id` (string): 会话ID，使用UUID
- `name` (string): 会话名称
- `status` (number): 会话状态，0-正常使用，1-删除
- `create_at` (string): 创建时间，ISO 8601格式

**示例：**
```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "我的第一个会话",
  "status": 0,
  "create_at": "2024-01-01T00:00:00.000Z"
}
```

### entries - 消息记录列表

存储所有对话消息。

**字段说明：**
- `entry_id` (string): 消息ID，使用UUID
- `session_id` (string): 关联的会话ID
- `message` (string): 消息内容
- `role` (string): 角色（user/assistant/system）
- `token_consumption` (number): 这次对话消耗的token数
- `status` (number): 消息状态，0-正常使用，1-不使用
- `tools` (object|null): assistant调用tools的记录信息，没有调用时为null
- `create_at` (string): 创建时间，ISO 8601格式
- `is_compaction` (number): 压缩状态，0-被压缩，1-没有被压缩

**tools字段格式（当assistant调用工具时）：**
```json
{
  "tool_calls": [
    {
      "id": "call_123",
      "type": "function",
      "function": {
        "name": "function_name",
        "arguments": "{\"param\":\"value\"}"
      }
    }
  ]
}
```

**示例：**
```json
{
  "entry_id": "660e8400-e29b-41d4-a716-446655440001",
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "北京今天天气怎么样？",
  "role": "user",
  "token_consumption": 12,
  "status": 0,
  "tools": null,
  "create_at": "2024-01-01T00:00:00.000Z",
  "is_compaction": 1
}
```

### compacted_dialogues - 压缩对话列表

存储压缩后的对话摘要。

**字段说明：**
- `entry_id` (string): 压缩记录ID，使用UUID
- `trigger_entry_id` (string): 触发压缩的消息ID，此消息之前的消息不会加入下一个会话
- `summary` (string): 压缩的内容
- `create_at` (string): 创建时间，ISO 8601格式
- `status` (number): 状态，0-正常，1-不使用

**示例：**
```json
{
  "entry_id": "770e8400-e29b-41d4-a716-446655440002",
  "trigger_entry_id": "660e8400-e29b-41d4-a716-446655440010",
  "summary": "用户询问了天气情况，助手提供了北京的天气信息。",
  "create_at": "2024-01-01T00:10:00.000Z",
  "status": 0
}
```

## 操作说明

### 创建（Create）
向对应的数组中push新对象。

### 读取（Read）
使用数组的filter/find方法查询数据。

### 更新（Update）
找到对应的对象，修改其属性值。

### 删除（Delete）
通常不真正删除，而是将status设置为1（软删除）。

## 文件位置

实际运行时，存储文件位于项目根目录：
```
project-root/
  └── veloca-storage.json
```

## 注意事项

1. 所有时间使用ISO 8601格式（UTC时区）
2. UUID使用标准的36字符格式（包含连字符）
3. status字段统一使用数字：0表示正常，1表示删除/不使用
4. 文件操作需要考虑并发安全性
5. 定期备份存储文件
