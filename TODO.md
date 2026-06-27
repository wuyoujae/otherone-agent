# Rust Agent Streaming Stabilization TODO

## Goal

Make the Rust `otherone-agent` framework usable for desktop chat streaming without requiring the UI to switch sessions before replies appear.

## Checklist

- [x] Fix OpenAI-compatible SSE parsing so JSON events split across TCP chunks are preserved.
- [x] Add tests for split SSE frames, multiple frames per chunk, and `[DONE]` handling.
- [x] Preserve OpenAI-compatible extra request parameters such as `reasoning_effort`.
- [x] Fix streamed tool-call accumulation to merge by `index` like the TS framework.
- [x] Store assistant `tool_calls` in localfile/database entries.
- [x] Expose configurable localfile storage root without requiring global `current_dir`.
- [x] Keep old current-dir behavior as the default for compatibility.
- [x] Add a live ignored stream test using environment variables.
- [x] Run workspace checks and targeted tests.
- [x] Publish all `0.1.2` crates.
- [x] Switch desktop dependency from local path back to published `otherone = "0.1.2"`.

## Current Publish State

- Published or confirmed already uploaded on crates.io: `otherone-ai 0.1.2`, `otherone-storage 0.1.2`, `otherone-context 0.1.2`, `otherone-tools 0.1.2`, `otherone-mcp 0.1.2`, `otherone-agent 0.1.2`, `otherone 0.1.2`.

## Rollback

- Desktop can temporarily use the local path dependency to validate the fixed framework.
- Once crates.io is reachable, publish remaining crates and set desktop back to `otherone = "0.1.2"`.

## 运行中用户消息队列 TODO

目标：允许调用方在 `invoke_agent_stream` 活跃期间 enqueue 用户 prompts，并且只在 agent-loop 安全边界提交这些 prompts。

方案文档：`docs/mid-run-user-message-queue.md`

清单：

- [x] 在 `crates/otherone-agent/src/types.rs` 新增 `AgentStreamCommand` 和 `AgentStreamHandle`。
- [x] 新增 `invoke_agent_stream_interactive`，并保持 `invoke_agent_stream` 向后兼容。
- [x] 给 `run_stream_loop` 传入 command receiver。
- [x] 在 `crates/otherone-agent/src/lib.rs` 新增 queue drain helpers 和 user-prompt write helper。
- [x] 在安全 loop 边界、`combine_context` 前 drain queued prompts。
- [x] 无 tool 的 assistant 完成后，如果写入了 queued prompts，则继续 loop；只有队列为空才 emit `complete`。
- [x] tool 执行后，先写 tool entries，再写 queued user prompts，然后继续 loop。
- [x] 从 `crates/otherone/src/lib.rs` re-export interactive API。
- [ ] 增加 FIFO queue draining 和 tool result 周围 storage ordering 的聚焦测试。
- [x] 验证桌面端可以保持单个 active run，并通过新 command sender enqueue。

关键决策：

- 不修改 in-flight model stream。
- 不把 user prompt 写在 `assistant(tool_calls)` 和 `tool` result entries 中间。
- v1 只把 queued prompts 放在内存里；durable pending-message storage 作为后续增强。

回滚：

- 现有调用方可以继续使用 `invoke_agent_stream`。
- 如果 interactive API 需要关闭，桌面端可以切回旧 stream 入口。
