# 运行中用户消息队列设计

## 背景

桌面端需要支持用户在 agent 正在运行时继续发送补充消息。这个场景类似微信对话，用户可能在 assistant 思考、流式输出正文、等待工具返回时连续发送多条消息。

当前框架公开的是 `invoke_agent_stream(input, ai, auxiliary_ai)` 单向事件流。一旦模型流式请求已经发出，当前支持的 provider 都不能修改这次 HTTP 请求里的 `messages`。因此运行中追加的用户消息必须先进入队列，只能在 agent loop 的安全边界写入会话历史。

## 目标

- 允许调用方在 stream run 活跃期间 enqueue 用户 prompt。
- 保留现有 `invoke_agent_stream` API 和行为，兼容当前调用方。
- 同一个 session 仍然只保留一个活跃 agent run，不为同一会话启动第二个模型流。
- 保持 OpenAI-compatible provider 和 Anthropic 的 tool-call 消息顺序合法。
- 通过现有 `otherone_storage::write_entry` 支持 localfile 和 database。
- 修改范围收敛在 `otherone-agent` 公开 API 和 stream loop 内部。

## 非目标

- 不向正在进行的模型流中强行插入消息。
- 不打断或取消当前 assistant 回复。
- 不在安全排序点之前持久化 queued prompt。
- v1 不引入分布式队列或 durable pending-message 表。
- 除非测试暴露直接兼容问题，否则本功能不顺带改 provider 请求格式。

## 安全插入规则

provider 的消息顺序要求如下：

- assistant 正在流式输出正文时：只进入队列。
- assistant 回复结束且没有 tool calls：drain queued prompts。如果写入了任何 prompt，继续 agent loop，不发送 `complete`。
- assistant 回复结束且有 tool calls：先执行工具并写入所有 `role="tool"` entries，再 drain queued prompts，然后继续 agent loop。
- 绝不把用户 prompt 写在带 `tool_calls` 的 assistant entry 和对应 tool result entries 中间。

最终安全顺序是：

```text
user
assistant(tool_calls)
tool
tool
user(queued prompt)
assistant(next model response)
```

## 推荐 API

新增一个 interactive stream 入口和命令 channel，同时保留现有 API。

```rust
pub enum AgentStreamCommand {
    EnqueueUserPrompts(Vec<String>),
}

pub struct AgentStreamHandle {
    pub events: mpsc::Receiver<StreamAgentEvent>,
    pub commands: mpsc::Sender<AgentStreamCommand>,
}

pub async fn invoke_agent_stream_interactive(
    input: InputOptions,
    ai: AiOptions,
    auxiliary_ai: Option<AiOptions>,
) -> Result<AgentStreamHandle, AgentError>;
```

兼容包装：

```rust
pub async fn invoke_agent_stream(
    input: InputOptions,
    ai: AiOptions,
    auxiliary_ai: Option<AiOptions>,
) -> Result<mpsc::Receiver<StreamAgentEvent>, AgentError> {
    let handle = invoke_agent_stream_interactive(input, ai, auxiliary_ai).await?;
    Ok(handle.events)
}
```

`commands` sender 可以 clone，桌面端可以在 stream 活跃期间按 `session_id` 保存它。

## 实现路径

### `crates/otherone-agent/src/types.rs`

- 新增 `AgentStreamCommand`。
- 新增 `AgentStreamHandle`。
- 保持 `InputOptions` 和 `AiOptions` 不变。

### `crates/otherone-agent/src/lib.rs`

- 新增 `invoke_agent_stream_interactive`。
- 修改旧 `invoke_agent_stream`，让它委托到 interactive API。
- 给 `run_stream_loop` 传入 command receiver。
- 在每个 active run 内部维护一个轻量的 `VecDeque<String>` queued prompts。
- 新增 helper：
  - `drain_agent_commands(command_rx, queued_prompts)`
  - `write_user_prompt_entry(...)`
  - `write_queued_user_prompts(...) -> Result<usize, AgentError>`

### Stream Loop 调整

每一轮 loop 顶部，在 `combine_context` 前：

1. 非阻塞 drain pending commands。
2. 按 FIFO 顺序把 queued prompts 写入 storage。
3. 继续现有 context loading。

assistant stream 结束且没有 tool calls 时：

1. 再 drain 一次 pending commands。
2. 如果写入了 queued prompts，就 `continue`。
3. 否则发送 `complete` 并 return。

assistant stream 结束且有 tool calls 时：

1. 按当前逻辑写入带 `tool_calls` 的 assistant entry。
2. 按当前逻辑发送 `tool_calls` 事件。
3. 按当前逻辑执行 tools。
4. 按当前逻辑写入每个 `tool` result entry。
5. drain 并写入 queued prompts。
6. 保持现有 memory drain 行为。
7. 继续 agent loop。

这样可以保持 provider message history 合法，同时不改变 tool 执行语义。

### `crates/otherone/src/lib.rs`

- re-export `AgentStreamCommand` 和 `AgentStreamHandle`。
- 新增 `Otherone::invoke_agent_stream_interactive`。
- 保持 `Otherone::invoke_agent_stream` 对现有桌面端和第三方调用方兼容。

## 桌面端集成说明

框架改完后，桌面端应从 `Otherone::invoke_agent_stream` 切到 `Otherone::invoke_agent_stream_interactive`。

桌面端预期改动：

- 用 `session_id` 作为 key，把 `commands` 保存到 active stream map。
- 新增一个 Tauri command，例如 `enqueue_chat_message`。
- 如果 session 正在 streaming，发送 `AgentStreamCommand::EnqueueUserPrompts(prompts)`，不要启动第二个 stream。
- 前端继续保持 optimistic UI 展示。
- 桌面端不要 prewrite queued prompts，安全持久化时机由 framework 负责。
- stream `complete`、`error` 或 `cancelled` 后，从 active stream map 移除 command sender。

## Provider 兼容性

- OpenAI、OpenRouter、Fetch、Local 都是 OpenAI-compatible 路径，要求 assistant tool calls 后面先跟匹配的 tool messages，再出现后续 user messages。
- Anthropic 要求 `tool_result` content 跟在 `tool_use` 后，再出现普通用户文本。相同的 storage 顺序仍然正确。
- 当前 Anthropic message conversion 需要单独测试，因为 `transform_to_anthropic_format` 目前委托给 OpenAI-style message conversion。如果 Anthropic tool loop 失败，应该作为独立兼容任务修 adapter。

## 风险

- queued prompts 在下一个安全插入点前只存在内存中。如果进程在此之前退出，v1 可能丢失这些 queued prompts。
- framework 无法全局强制一个 session 只能有一个 active stream。调用方仍要保留桌面端 active-session guard。
- 如果 `complete` 已经发出后用户才发送 prompt，调用方应启动普通新 run，不应复用旧 command sender。
- 长时间运行的 tools 仍然不会被本设计打断。queued prompt 会在 tool result 写入后生效。

## 回滚

- 现有 `invoke_agent_stream` 保持兼容。
- 如果 interactive API 有问题，桌面端可以切回旧 stream 入口。
- 队列功能只对主动使用 `invoke_agent_stream_interactive` 的调用方生效。

## 验证计划

- 单测 command draining：过滤空 prompt、保持 FIFO、支持 batched prompts。
- 单测 localfile storage 下 queued prompt 写入。
- stream-loop 无 tool calls 测试：最终 complete 前如果有 queued prompt，应继续下一轮，而不是立即 `complete`。
- stream-loop 有 tool calls 测试：storage 顺序必须是 `assistant(tool_calls) -> tool -> user`。
- 桌面端手动验证：assistant 正文 streaming 时发送、tool running 时发送、正常 complete 后发送。
