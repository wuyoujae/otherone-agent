# Otherone Multi-Agent Runtime Architecture Design

## Document Status

- Status: local Multi-Agent Runtime implemented; optional distributed extensions remain future work
- Scope: the next-generation Agent execution API and local Multi-Agent runtime
- Compatibility target: existing `Otherone::invoke_agent*` callers remain available unchanged during migration
- Core principle: Agent definitions are peers; root/caller/callee relationships exist only during a run

## 中文评审摘要

这份方案最终选择的是 **Multi-Agent Runtime**，不是固定的 Main Agent / Sub Agent 类型体系。

核心决策如下：

- 所有 `AgentDefinition` 在定义层平等，应用通过 `entry_agent` 决定本次任务从谁开始；
- Root、Caller、Callee、Sub Agent 都只是某一次运行中的动态关系；
- 主运行与子运行复用唯一的 `AgentRunner`，不实现第二套 Sub Agent Loop；
- Agent 之间通过结构化 `call_agent` 请求和结果通信，而不是共享一个可变对话历史；
- `AgentRuntime` 统一持有 Agent、Model、Tool、Skill、Router、Supervisor、Event 和 Storage；
- 每个根任务固定一份完整 `RuntimeSnapshot`，所有后代运行看到相同版本的配置；
- 子运行默认使用独立 session，只获得 task 和显式 context，完整父会话不会自动泄漏；
- Agent、Tool、Skill、Memory 权限采用 allowlist 和多层策略交集，模型不能扩大权限；
- 工具系统升级为异步结构化接口，普通工具、MCP、Memory 和 Agent Call 走同一执行链路；
- 结果通道与事件通道分离，UI 事件丢失或拥塞不能影响最终运行结果；
- 根任务共享深度、运行数、Token、模型调用、工具调用、超时和输出预算；
- 父运行等待子运行时不持有模型或工具并发许可，避免嵌套调用死锁；
- 取消从根运行向全部后代传播，子运行失败默认作为结构化 Tool Result 返回父运行；
- 新 API 使用 builder 和新类型，旧 `invoke_agent*` API 通过兼容适配器逐步迁移；
- 第一阶段只实现本地同步等待型 Agent Call，后台运行、handoff、远程和持久化恢复后续扩展。

文档后续章节给出了公共类型草案、调用流程、存储模型、事件模型、权限、并发、取消、
幂等、兼容策略、分阶段实现计划和测试验收矩阵。

## Implementation Status

The local runtime described by this document is implemented under
`crates/otherone-agent/src/multi_agent/`.

Implemented:

- peer `AgentDefinition`, `AgentRegistry`, `ModelRegistry`, and immutable `RuntimeSnapshot`;
- builder-based `AgentRuntime` API and facade re-exports;
- one streaming Runner for root and child runs;
- dynamic `otherone.call_agent` routing with structured results;
- async `ToolRegistry`, synchronous closure adapter, and MCP adapter;
- Anthropic streaming `tool_use` index/`partial_json` accumulation;
- persistent stdio MCP transport for post-initialization `tools/call` requests;
- child session isolation, lineage metadata, internal-session filtering, and localfile write locking;
- Agent/Tool/Skill/Memory permission policies;
- context transfer policies and inherited `RuntimeContext` isolation;
- typed parent/child events, opt-in thinking deltas, and an independent completion channel;
- cancellation, deadlines, finite limits, bounded streaming payloads, shared budgets, cycle
  protection, and session writer locks;
- Agent-call and root-run in-process idempotency whose producers survive cancelled waiters;
- provider-specific model configuration and accumulated streaming token usage;
- fatal/recoverable tool error enforcement and authoritative child result-contract validation;
- bounded model/tool concurrency without holding execution permits while waiting for children;
- in-memory Run Store and developer inspection APIs;
- scoped read-only/read-write/private memory tools with source provenance;
- compatibility preservation for the existing `Otherone::invoke_agent*` API surface.

Intentionally not implemented because they are listed as first-version non-goals or Phase 4
extensions:

- remote/distributed Agent transport;
- durable process-crash recovery and run resumption;
- background `spawn_agent`/`await_agent` and control-transfer `handoff_agent`;
- filesystem `AGENT.md` discovery;
- capability search, approval suspension, and external artifact storage.

The legacy `invoke_agent*` loops remain available unchanged during migration. New Multi-Agent code
uses the unified Runner; removing the legacy implementation would be a later breaking cleanup, not a
requirement for using the runtime.

## Executive Decision

Otherone will be designed as a **Multi-Agent Runtime**, not as a fixed main-agent/sub-agent
framework.

Every Agent is an independent, reusable definition. An application chooses one Agent as the entry
point of a root run. During that run, any Agent with sufficient permission may call another Agent.
The caller/callee relationship is dynamic and scoped to that call.

```text
Application starts Agent A

Root Run: A
├── Child Run: B
│   └── Child Run: C
└── Child Run: D
```

In another application or another request, B may be the root Agent and A may be one of its callees.
There is no permanent main Agent or permanent sub Agent.

The runtime must nevertheless retain an explicit run hierarchy. It is required for:

- routing results back to the correct caller;
- context and session isolation;
- permission checks;
- cancellation and deadline propagation;
- recursion and cycle control;
- shared budget accounting;
- tracing, storage, debugging, and UI event grouping.

The central model is therefore:

```text
peer Agent definitions + hierarchical Agent runs + controlled Agent calls
```

## Goals

- Support multiple reusable Agent definitions without assigning permanent hierarchy.
- Allow an application to select any registered Agent as the entry Agent.
- Allow authorized Agents to call other Agents through one stable routing abstraction.
- Reuse one Agent Runner for root runs and child runs.
- Keep model providers, tools, MCP, Skills, memory, context, and storage modular.
- Isolate child context by default and prevent accidental privilege escalation.
- Support streaming, non-streaming, cancellation, timeouts, concurrency, and nested calls.
- Preserve enough lineage and state for production observability and debugging.
- Keep the first implementation local and simple while preserving extension points for background,
  remote, durable, and distributed execution.
- Maintain compatibility with the current public API while the new runtime becomes the core.

## Non-Goals For The First Implementation

- Distributed scheduling across multiple processes or machines.
- Durable process-crash recovery and automatic run resumption.
- Unrestricted peer-to-peer messaging between active runs.
- Shared mutable message history between multiple Agents.
- Automatic retries of arbitrary Agent or tool side effects.
- A workflow DSL, graph compiler, or visual workflow editor.
- Consensus, voting, debate, or swarm algorithms built into the core.
- A mandatory filesystem format for Agent definitions.

These features may be implemented later on top of the same runtime boundaries.

## Current Foundation And Required Changes

The design builds on existing Otherone capabilities instead of replacing them:

- `otherone-ai` already provides provider-neutral model invocation and streaming;
- `otherone-agent` already has tool-calling loops, session context, streaming events, and runtime
  user-message queuing;
- `otherone-storage` already provides session/entry persistence, metadata, and `RuntimeContext`;
- `otherone-tools` already maps provider tool calls to local handlers;
- Skills, MCP, memory, and context compaction already have independent crate boundaries.

The required architectural changes are:

- streaming and non-streaming loops must converge on one internal Runner;
- `AiOptions` must stop being the long-term owner of credentials, Agent identity, prompts, messages,
  and executable tool closures at the same time;
- synchronous `Fn(Value) -> String` tool handlers must gain an asynchronous structured contract;
- string event types must gain typed run lineage;
- a long-lived runtime must own registries, policies, scheduling, and cancellation;
- run lifecycle persistence must be distinguished from conversation persistence.

These changes are prerequisites for clean Multi-Agent support. Implementing Agent calls directly in
the current synchronous tool branch would create a temporary feature that later blocks MCP,
cancellation, concurrency, and remote execution.

## Terminology

### Agent Definition

An immutable configuration describing an Agent's identity, instructions, model selection,
capabilities, permissions, and limits.

### Agent ID

A stable logical identifier such as `planner`, `researcher`, or `code_reviewer`. It identifies a
definition, not a running process.

Recommended validation:

```text
[a-z][a-z0-9_-]{0,63}
```

### Entry Agent

The Agent selected by the application to start a root run. "Main Agent" may be used as UI wording,
but it is not a separate framework type.

### Agent Run

One execution instance of an Agent definition. Calling the same Agent twice creates two different
runs with different `run_id` values.

### Root Run

The first run created by an application request. It has no caller.

### Caller Run / Callee Run

The caller initiates an Agent call. The callee executes the requested task. "Sub Agent" refers to a
callee run in this specific relationship, not to an Agent definition category.

### Session

Conversation state used by context loading and message persistence. A session is not a run. One
session may have multiple sequential runs, while a nested Agent call normally receives a separate
child session.

### Agent Call

A structured request-response operation from one run to a newly created run. It is closer to
asynchronous RPC than to two Agents sharing an open chat room.

### Active Run Message

A message sent to an already running `run_id`. This is a different capability from calling an
`agent_id` and must use a separate API.

## Design Principles

1. **One Agent type**: root and child execution use the same `AgentDefinition` and `AgentRunner`.
2. **Hierarchy belongs to runs**: `parent_run_id` is runtime state, never Agent configuration.
3. **Explicit capability boundaries**: tools, Skills, memory, and callable Agents use allowlists.
4. **Least privilege by default**: an Agent call cannot widen permissions or runtime scope.
5. **Context is copied, not shared**: child runs receive explicit immutable input.
6. **One session writer**: only one active run may write to a given session at a time.
7. **Structured calls and results**: Agent calls do not depend on parsing prose protocols.
8. **Typed events**: all events carry run lineage and stable event variants.
9. **Result delivery is independent from event delivery**: losing stream events must not lose the
   actual Agent result.
10. **Configuration is immutable during a root run**: each root run uses one complete runtime
    snapshot.
11. **Safe defaults, configurable limits**: recursion, concurrency, time, output, and token use are
    always bounded in the new runtime API.
12. **Compatibility through adapters**: old APIs wrap the runtime instead of creating a second
    permanent implementation.

## High-Level Architecture

```text
Application / Otherone facade
            |
            v
      AgentRuntime
      ├── ModelRegistry
      ├── AgentRegistry
      ├── Async ToolRegistry
      ├── SkillRegistry
      ├── AgentRouter
      ├── RunSupervisor
      ├── AgentRunner
      ├── EventBus
      ├── ConversationStore
      └── RunStore

Root AgentRun
      |
      | otherone.call_agent
      v
AgentRouter -> RunSupervisor -> Child AgentRun -> AgentRunner
      ^                                      |
      └──────────── AgentCallResult ─────────┘
```

## Component Responsibilities

### `AgentRuntime`

The long-lived runtime object assembled by the application. It owns shared registries and services.
It is cheap to clone through `Arc` and safe to use concurrently.

Responsibilities:

- validate runtime configuration during construction;
- create immutable `RuntimeSnapshot` values for root runs;
- expose root `start`, `run`, and `run_stream` APIs;
- provide internal handles to the Router, Supervisor, Runner, and Event Bus;
- keep credentials and trusted application policy outside model-visible data.

The runtime replaces the need to place all mutable behavior inside `AiOptions`.

`RuntimeSnapshot` contains the Agent, model, tool, Skill, and trusted policy snapshots used by one
root tree. Mutable connection pools and service clients may be referenced through `Arc`, but their
logical registrations and permissions remain fixed for the lifetime of the root run.

### `AgentRegistry`

Stores immutable `AgentDefinition` values by `AgentId`.

Rules:

- duplicate IDs are errors unless the application explicitly calls a replace API;
- all referenced model profiles, tools, Skills, and callable Agents are validated at build time;
- IDs, descriptions, prompts, metadata, and policy collections have explicit size limits;
- descriptions are non-empty, model-visible, and limited to a recommended maximum of 1024 bytes;
- definitions containing unknown IDs or invalid permission combinations fail before any run starts;
- active root runs access definitions through an `Arc<RuntimeSnapshot>`;
- registry changes affect new root runs only;
- definitions include a version/hash recorded with each run.

### `ModelRegistry`

Stores named model profiles containing provider connection details and generation defaults.

Agent definitions reference a profile ID instead of embedding API credentials. This allows different
Agents to use different providers or models without copying secrets into configuration files,
events, prompts, or stored run records.

### `ToolRegistry`

Stores tool definitions and asynchronous handlers. Local tools, MCP tools, memory tools, and the
built-in Agent-call tool use the same handler contract.

Tool IDs should eventually be namespaced, for example:

```text
local.read_file
mcp.github.search_code
memory.recall
otherone.call_agent
```

Legacy unqualified names remain supported through compatibility registration.

### `AgentRouter`

The only component allowed to create an Agent call.

Responsibilities:

- resolve the target Agent from the root `RuntimeSnapshot`;
- calculate effective permissions;
- run optional application authorization hooks;
- reject invalid, recursive, over-budget, or unauthorized calls;
- create the child request and child session;
- ask the Supervisor to start the child run;
- return a structured result to the caller;
- later support local or remote transports without changing Agent-call semantics.

### `RunSupervisor`

Owns runtime lifecycle and resource control.

Responsibilities:

- allocate `run_id`, `root_run_id`, `parent_run_id`, and `call_id` relationships;
- maintain run states;
- enforce depth, run-count, model-call, tool-call, token, output, and time budgets;
- enforce global and per-root concurrency;
- propagate cancellation and deadlines;
- manage session writer locks;
- deduplicate Agent calls by idempotency key;
- ensure parent runs do not deadlock while waiting for child runs.

### `AgentRunner`

The single execution state machine for every Agent run.

It must replace the current duplicated streaming and non-streaming loops. Provider streaming changes
how model output is observed, not the logical Agent state machine.

Recommended internal states:

```text
Preparing
  -> LoadingContext
  -> CallingModel
  -> PersistingAssistantMessage
  -> ExecutingActions
  -> LoadingContext ...
  -> Completing
```

`ExecutingActions` may execute ordinary tools or the built-in Agent-call tool. The Runner must not
contain a hard-coded `if tool_name == call_agent` branch. Agent calls use the asynchronous tool
contract and receive an `AgentRouterHandle` through `ToolCallContext`.

### `EventBus`

Publishes typed events from the root run and all descendant runs. Event delivery is for observation,
not for returning the authoritative result.

### `ConversationStore`

Stores sessions, user messages, assistant messages, tool calls, tool results, and compacted context.
The existing `otherone-storage` behavior belongs here.

### `RunStore`

Stores execution lifecycle independently from conversation messages. It records run lineage,
status, timestamps, usage, failure information, and definition versions.

The first implementation may use an in-memory Run Store plus metadata in the existing storage
records, but the public boundary should be defined from the beginning.

## Crate And Module Boundaries

No new `otherone-subagent` crate should be created. A child run is an Agent runtime concept and is
tightly coupled to the Agent lifecycle.

Recommended modules:

```text
crates/otherone-agent/src/
├── definition.rs
├── registry.rs
├── model_registry.rs
├── runtime.rs
├── runner.rs
├── run_context.rs
├── router.rs
├── supervisor.rs
├── event.rs
├── command.rs
├── result.rs
└── error.rs
```

Crate responsibilities remain:

- `otherone-ai`: provider requests, responses, and streams;
- `otherone-tools`: asynchronous tool traits, definitions, registry, and adapters;
- `otherone-context`: context loading, token estimation, and compaction;
- `otherone-storage`: conversation and run persistence backends;
- `otherone-memory`: memory implementation and memory tools;
- `otherone-skills`: Skill discovery, validation, and prompt rendering;
- `otherone-mcp`: MCP clients adapted into `ToolRegistry` entries;
- `otherone-agent`: definitions, runs, routing, supervision, and execution;
- `otherone`: facade, builders, compatibility APIs, and optional subsystem assembly.

## Core Public Types

All new public configuration types should use constructors/builders and `#[non_exhaustive]` where
appropriate. Applications should not be forced to update every struct literal whenever a future
optional field is added.

### IDs

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentCallId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelProfileId(String);
```

Newtypes prevent accidental interchange of Agent, run, session, and tool-call identifiers.

### Runtime Snapshot

```rust
pub struct RuntimeSnapshot {
    pub version: String,
    pub agents: Arc<AgentRegistrySnapshot>,
    pub models: Arc<ModelRegistrySnapshot>,
    pub tools: Arc<ToolRegistrySnapshot>,
    pub skills: Arc<SkillRegistrySnapshot>,
    pub policy: Arc<RuntimePolicySnapshot>,
}
```

The snapshot is created when a root run starts and shared by every descendant. Registration APIs may
update the live runtime for future roots, but never mutate this snapshot in place.

### Agent Definition

```rust
#[non_exhaustive]
pub struct AgentDefinition {
    pub id: AgentId,
    pub description: String,
    pub system_prompt: String,
    pub model: ModelSelector,
    pub model_overrides: ModelOverrides,
    pub tools: ToolAccessPolicy,
    pub skills: SkillAccessPolicy,
    pub callable_agents: AgentAccessPolicy,
    pub memory: MemoryPolicy,
    pub context: ContextTransferPolicy,
    pub limits: AgentLimits,
    pub metadata: AttributeBag,
}
```

Rules:

- `description` is model-visible and explains when the Agent should be called;
- `system_prompt` is never supplied or overridden by another model;
- `model` references runtime-managed model configuration;
- permissions default to none unless explicitly granted;
- metadata is configuration metadata, not a place for secrets;
- definitions are immutable after registration.

### Model Selection

```rust
pub enum ModelSelector {
    RuntimeDefault,
    Named(ModelProfileId),
    InheritCaller,
}
```

Resolution rules:

- `RuntimeDefault` uses the runtime's default profile;
- `Named` uses the referenced profile;
- `InheritCaller` uses the caller's resolved profile for a child run and the runtime default for a
  root run;
- missing profiles fail runtime validation before execution;
- model-visible Agent-call arguments cannot select or override model profiles.

### Access Policies

```rust
pub enum AccessPolicy<T> {
    None,
    Allow(BTreeSet<T>),
    All,
}
```

`All` is convenient for trusted local applications but should never be the default.

Effective access is always an intersection:

```text
runtime policy
  ∩ trusted application policy
  ∩ Agent definition policy
  ∩ per-run restriction
```

A caller-provided or model-generated request may only tighten effective access. It may never widen
it.

### Root Run Request

```rust
#[non_exhaustive]
pub struct AgentRunRequest {
    pub entry_agent: AgentId,
    pub input: AgentInput,
    pub session: SessionTarget,
    pub runtime_context: Option<RuntimeContext>,
    pub limits: Option<RunLimitOverrides>,
    pub idempotency_key: Option<String>,
    pub metadata: AttributeBag,
}
```

`RunLimitOverrides` from an ordinary caller may only reduce limits. A separate trusted
administrative API may replace policy when required.

### Run Context

```rust
pub struct AgentRunContext {
    pub run_id: RunId,
    pub root_run_id: RunId,
    pub parent_run_id: Option<RunId>,
    pub agent_call_id: Option<AgentCallId>,
    pub agent_id: AgentId,
    pub session_id: String,
    pub parent_session_id: Option<String>,
    pub depth: usize,
    pub ancestry: Vec<AgentId>,
    pub runtime_context: Option<RuntimeContext>,
    pub deadline: Option<Instant>,
    pub cancellation: CancellationToken,
    pub budget: Arc<BudgetLedger>,
    pub snapshot: Arc<RuntimeSnapshot>,
}
```

`RuntimeContext` is inherited from the root application request and cannot be replaced by an Agent.
This preserves tenant, user, workspace, and project isolation through all nested calls.

### Agent Call Request

The internal programmatic API supports structured calls:

```rust
#[non_exhaustive]
pub struct AgentCallRequest {
    pub target: AgentSelector,
    pub task: String,
    pub context: Option<serde_json::Value>,
    pub result_contract: ResultContract,
    pub limits: Option<CallLimitOverrides>,
    pub metadata: AttributeBag,
}

pub enum AgentSelector {
    Id(AgentId),
    // Reserved for a future capability-based router.
    Capability(String),
}

pub enum ResultContract {
    Text,
    Json { schema: Option<serde_json::Value> },
    Auto,
}
```

The model-facing v1 tool exposes only `agent`, `task`, and optional `context`. Model-generated calls
cannot set metadata, deadlines, model profiles, tools, Skills, memory access, or permissions.

### Run Result

```rust
#[non_exhaustive]
pub struct AgentRunResult {
    pub run_id: RunId,
    pub root_run_id: RunId,
    pub agent_id: AgentId,
    pub session_id: String,
    pub outcome: RunOutcome,
    pub output: Option<AgentOutput>,
    pub usage: RunUsage,
    pub artifacts: Vec<ArtifactRef>,
    pub metadata: AttributeBag,
}

pub enum AgentOutput {
    Text(String),
    Json(serde_json::Value),
}

pub enum RunOutcome {
    Completed,
    Failed(AgentFailure),
    Cancelled,
    TimedOut,
    BudgetExceeded,
    HandedOff { successor_run_id: RunId },
}

pub enum RunOutcomeKind {
    Completed,
    Failed,
    Cancelled,
    TimedOut,
    BudgetExceeded,
    HandedOff,
}

pub struct RunUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub model_calls: u32,
    pub tool_calls: u32,
    pub agent_calls: u32,
    pub duration_millis: u64,
}

pub struct ArtifactRef {
    pub artifact_id: String,
    pub media_type: Option<String>,
    pub name: Option<String>,
    pub size_bytes: Option<u64>,
}

pub struct AgentCallResult {
    pub schema_version: u16,
    pub call_id: AgentCallId,
    pub target: AgentId,
    pub run_id: Option<RunId>,
    pub outcome: RunOutcome,
    pub output: Option<AgentOutput>,
    pub usage: RunUsage,
    pub artifacts: Vec<ArtifactRef>,
}
```

Full child transcripts are not returned to the parent by default. They remain available through the
child session and Run Store. Large outputs should eventually be stored as artifacts and returned by
reference instead of being copied into the parent's context.

## Runtime API

The runtime has one execution primitive and convenience wrappers:

```rust
let runtime = AgentRuntime::builder()
    .default_model("default")
    .register_model(default_model)
    .register_agent(planner)
    .register_agent(researcher)
    .register_agent(reviewer)
    .register_tool(read_file_tool)
    .runtime_limits(RuntimeLimits::safe_defaults())
    .build()?;

let result = runtime
    .run(AgentRunRequest::new("planner", "Design this feature"))
    .await?;
```

Streaming uses the same Runner:

```rust
let mut handle = runtime.start(request).await?;

while let Some(event) = handle.events.recv().await {
    // Render or record events.
}

let result = handle.result().await?;
```

Recommended API responsibilities:

- `start(request) -> AgentRunHandle`: start a root run and expose events, commands, and completion;
- `run(request) -> AgentRunResult`: start and await completion without requiring event consumption;
- `run_stream(request)`: optional compatibility/convenience wrapper around `start`;
- `call_agent(caller_context, request)`: internal and trusted programmatic Agent call;
- `send_command(run_id, command)`: send a message, cancel, or control an active run.

Completion must use a dedicated result channel. A full or lagging event channel must never block the
authoritative result indefinitely.

### Application Configuration Example

The exact builder names may change during implementation, but the intended user experience is:

```rust
let researcher = AgentDefinition::builder("researcher")
    .description("Research facts and return concise evidence.")
    .system_prompt("You are a focused research Agent.")
    .model(ModelSelector::Named("research".try_into()?))
    .allow_tools(["mcp.web.search", "local.read_file"])
    .allow_skills(["web-research"])
    .memory(MemoryPolicy::ReadOnlyShared)
    .limits(AgentLimits::default().max_iterations(8))
    .build()?;

let reviewer = AgentDefinition::builder("reviewer")
    .description("Review a proposed answer and identify concrete problems.")
    .system_prompt("You are a strict independent reviewer.")
    .allow_tools(["local.read_file"])
    .build()?;

let planner = AgentDefinition::builder("planner")
    .description("Plan work and coordinate specialists.")
    .system_prompt("You coordinate work and integrate specialist results.")
    .allow_agents(["researcher", "reviewer"])
    .build()?;

let runtime = AgentRuntime::builder()
    .register_model(default_model)
    .register_model(research_model)
    .register_agent(planner)
    .register_agent(researcher)
    .register_agent(reviewer)
    .build()?;

let result = runtime
    .run(AgentRunRequest::new("planner", "Compare the available approaches"))
    .await?;
```

The same registry may start a different application flow without redefining hierarchy:

```rust
let result = runtime
    .run(AgentRunRequest::new("reviewer", "Review this patch directly"))
    .await?;
```

## Asynchronous Tool Contract

The current synchronous closure contract cannot await an Agent call or a normal asynchronous MCP
operation. It must evolve into:

```rust
#[async_trait]
pub trait ToolHandler: Send + Sync {
    async fn call(
        &self,
        arguments: serde_json::Value,
        context: ToolCallContext,
    ) -> Result<serde_json::Value, ToolError>;
}

pub struct ToolCallContext {
    pub run: AgentRunContextView,
    pub runtime_context: Option<RuntimeContext>,
    pub cancellation: CancellationToken,
    pub deadline: Option<Instant>,
    pub event_sink: EventSinkHandle,
    pub agent_router: AgentRouterHandle,
}
```

Handlers are stored as `Arc<dyn ToolHandler>`.

Compatibility:

- provide `SyncToolHandler<F>` for existing `Fn(Value) -> String` closures;
- convert legacy strings into `serde_json::Value::String`;
- deprecate `AiOptions.tools_realize` after the runtime API is stable;
- do not remove the legacy field in the first compatibility release.

Tool failures are classified:

```rust
pub enum ToolErrorKind {
    Recoverable,
    PermissionDenied,
    InvalidArguments,
    TimedOut,
    Cancelled,
    Fatal,
}
```

Recoverable tool errors are written as tool results so the model can adjust. Fatal framework or
storage failures terminate the current run.

## Built-In Agent Call Tool

The model-facing tool name is:

```text
otherone.call_agent
```

Schema:

```json
{
  "type": "object",
  "properties": {
    "agent": {
      "type": "string",
      "enum": ["researcher", "reviewer"]
    },
    "task": {
      "type": "string"
    },
    "context": {
      "type": "object"
    }
  },
  "required": ["agent", "task"],
  "additionalProperties": false
}
```

Rules:

- only effectively callable Agents appear in the enum;
- descriptions of reachable Agents are included in the tool description;
- the handler registration is static, while the model-visible tool definition is synthesized for
  each run from that run's `RuntimeSnapshot` and effective access policy;
- the tool is not injected when no Agent is callable;
- task and context size are validated before a child run is created;
- model input cannot choose a model, enable tools, change runtime scope, or raise limits;
- the handler calls `AgentRouter`, not `AgentRunner` directly;
- the result is serialized as structured JSON and stored as the matching parent tool result.

`ResultContract::Auto` is the v1 default. If a trusted application requests JSON output, the Runner
validates the final value against the configured contract; invalid structured output becomes a
typed, recoverable child failure rather than silently falling back to unvalidated prose.

A single generic tool is preferred over one tool per Agent because it keeps the tool namespace
stable and avoids excessive tool definitions. For very large registries, a future
`otherone.search_agents` tool may expose capability search without putting every Agent into one
schema.

## Agent Call Execution Flow

```text
1. Caller model emits otherone.call_agent(agent, task, context).
2. Parent Runner persists the assistant tool-call message.
3. ToolRegistry invokes AgentCallTool asynchronously.
4. AgentRouter resolves the target from the root `RuntimeSnapshot`.
5. Router calculates effective permissions and runs authorization hooks.
6. Supervisor reserves run count and budget, validates depth and ancestry.
7. Supervisor allocates child run ID and isolated child session.
8. Child Runner executes the target Agent with task and explicit context.
9. Child lifecycle events are published with child lineage.
10. Child completion is persisted to Run Store.
11. Router converts the child result into AgentCallResult.
12. Parent Runner persists that result as the matching tool message.
13. Parent Runner reloads context and continues its own loop.
```

The parent waits for the child in v1. Parent waiting is suspension, not active model execution.

## Call, Spawn, And Message Semantics

These operations must remain distinct:

### `call_agent`

- target: an `AgentId`;
- creates a new run;
- request-response;
- caller normally waits;
- implemented in v1.

### `spawn_agent`

- target: an `AgentId`;
- creates a new background run;
- immediately returns a `run_id`/task handle;
- result is later retrieved with `await_agent` or `get_run`;
- planned after synchronous call semantics are stable.

### `send_message`

- target: an active `RunId`, not an `AgentId`;
- appends input at a safe Runner boundary;
- does not create a new Agent run;
- current interactive prompt queue is the initial form of this capability.

### `cancel_run`

- target: a `RunId`;
- cancels that run and its descendants;
- cancelling a child does not cancel its parent unless configured as fail-fast.

### `handoff_agent`

- target: an `AgentId`;
- transfers responsibility instead of returning a normal child result;
- the current run completes with a handed-off outcome and the root handle follows the successor run;
- useful for routing between specialists or changing the user-facing active Agent;
- reserved for a later phase because session ownership and UI identity must be explicit.

The first release must not allow a child run to send a reentrant message into an ancestor run. That
can cause ordering ambiguity and deadlocks. A child returns a result through its Agent call instead.

## Context Isolation And Transfer

Child runs do not inherit the full parent conversation by default.

Default child input contains:

1. framework protocol instructions;
2. child Agent system prompt;
3. child Agent's allowed Skill catalog/instructions;
4. the caller's task;
5. explicitly supplied context;
6. runtime-owned lineage metadata that is not model-writable.

Recommended policy:

```rust
pub enum ContextTransferPolicy {
    ExplicitOnly,
    ParentSummary { max_tokens: u32 },
    ParentWindow { max_tokens: u32 },
}
```

Default: `ExplicitOnly`.

Rules:

- parent tool-call/tool-result sequences are never copied partially;
- explicit context is treated as untrusted data, not as system instructions;
- transferred context is bounded by bytes and estimated tokens;
- the parent may summarize context before calling a child;
- a child may retrieve shared data through its own allowed tools;
- root `RuntimeContext` is inherited unchanged and is not included in model-writable arguments;
- secrets, credentials, cancellation handles, and internal policy are never transferred as context.

## Sessions And Persistence

### Root Session

The root run uses the application-provided session or creates a new user-visible session.

### Child Session

Default v1 policy: `IsolatedPersistent`.

- every Agent call creates a distinct child session;
- the child session is marked `session_kind = agent_internal`;
- ordinary session listing hides internal child sessions by default;
- developer/debug APIs may include them;
- only the final Agent-call result is copied into the parent session;
- child messages never become direct parent history entries.

Future policies may include:

```rust
pub enum ChildSessionPolicy {
    IsolatedPersistent,
    Ephemeral,
    ReuseNamed(String),
}
```

`ReuseNamed` requires the same one-active-writer session lock as root sessions and is not part of v1.

### Run Records

Recommended record:

```rust
pub struct RunRecord {
    pub run_id: RunId,
    pub root_run_id: RunId,
    pub parent_run_id: Option<RunId>,
    pub agent_call_id: Option<AgentCallId>,
    pub agent_id: AgentId,
    pub agent_definition_hash: String,
    pub session_id: String,
    pub status: RunStatus,
    pub depth: usize,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub usage: RunUsage,
    pub failure: Option<AgentFailure>,
    pub metadata: AttributeBag,
}
```

Run statuses:

```text
created -> queued -> running -> waiting_for_child -> running -> completed
                             └-> failed
                             └-> cancelled
                             └-> timed_out
                             └-> budget_exceeded
```

Conversation writes required for context correctness remain fail-fast. Optional telemetry sinks may
be best-effort, but authoritative Run Store status transitions should surface failures according to
the configured durability policy.

### Multi-Tenant Scope

Every child run inherits the root `RuntimeContext` and therefore the same `partition_key` security
boundary. The framework adds lineage metadata but cannot change the application-owned partition.

Suggested metadata on child sessions and entries:

```text
root_run_id
parent_run_id
agent_call_id
agent_id
depth
session_kind = agent_internal
```

## Events And Streaming

Replace stringly typed events with a typed envelope:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEventEnvelope {
    pub schema_version: u16,
    pub event_id: String,
    pub sequence: u64,
    pub timestamp: String,
    pub root_run_id: RunId,
    pub run_id: RunId,
    pub parent_run_id: Option<RunId>,
    pub agent_id: AgentId,
    pub depth: usize,
    pub event: AgentEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum AgentEvent {
    RunStarted,
    ModelStarted,
    ModelDelta { content: String },
    ThinkingDelta { content: String },
    ModelCompleted { usage: Option<RunUsage> },
    ToolStarted { tool_call_id: String, tool_name: String },
    ToolCompleted { tool_call_id: String, tool_name: String },
    AgentCallStarted { call_id: AgentCallId, target: AgentId },
    AgentCallCompleted { call_id: AgentCallId, target: AgentId, outcome: RunOutcomeKind },
    UserMessageQueued { count: usize },
    RunCompleted,
    RunFailed { failure: AgentFailure },
    RunCancelled,
}
```

Event rules:

- `sequence` is allocated from one root-run sequence so concurrent child events can be ordered;
- root subscribers receive descendant events by default;
- consumers may filter by `run_id`, `agent_id`, depth, or event type;
- event visibility policy may expose all child deltas, lifecycle only, or no child events;
- raw provider chunks are opt-in because they may contain provider-specific or sensitive data;
- thinking/reasoning events are opt-in and should be considered sensitive;
- completion result delivery does not depend on the Event Bus;
- lifecycle state remains queryable from Run Store even if transient deltas are dropped.

Delivery rules:

- terminal results use a dedicated completion channel and are never reconstructed from events;
- lifecycle state is persisted before its corresponding best-effort notification is published;
- high-volume deltas may be dropped or coalesced for lagging observers according to policy;
- terminal lifecycle events should not be silently dropped, but Run Store remains authoritative;
- the Runner must not wait on a bounded UI event channel while holding model/tool permits or
  session locks;
- `run()` installs an internal no-op/telemetry sink and therefore does not require a consumer to
  drain stream events.

Compatibility adapters convert typed events into the existing `StreamAgentEvent` shape during the
migration period.

## Commands And Interactive Input

Generalize the current stream command channel:

```rust
pub enum AgentRunCommand {
    EnqueueUserMessages(Vec<AgentInputMessage>),
    Cancel,
}
```

Rules:

- commands target a `run_id`;
- messages are persisted only at safe Agent-loop boundaries;
- user input is never inserted between an assistant tool call and its required tool results;
- root messages received while waiting for a child are queued until the child returns;
- child interactive input is disabled by default but may be exposed by trusted applications;
- a closed or completed run rejects new commands with a typed error.

If a child needs human input in v1, it should return a structured result asking the parent to obtain
that input. Durable suspended human-in-the-loop runs belong to a later phase.

## Permissions And Security

### Agent-To-Agent Authorization

Both sides participate in authorization:

- the caller must allow the target Agent;
- runtime/application policy must allow the edge;
- the target must exist in the root `RuntimeSnapshot`;
- optional `AgentCallAuthorizer` may inspect trusted runtime context;
- call arguments may only reduce limits;
- the model cannot register Agents or modify policies.

Suggested hook:

```rust
#[async_trait]
pub trait AgentCallAuthorizer: Send + Sync {
    async fn authorize(
        &self,
        caller: &AgentRunContextView,
        target: &AgentDefinition,
        request: &AgentCallRequestView,
    ) -> Result<AuthorizationDecision, AgentError>;
}
```

### Tool And Skill Permissions

Each child uses its own Agent definition. It does not inherit all caller tools.

Effective resources are intersections of trusted policies. A caller can deliberately provide less
access for a specific call, but cannot grant a tool or Skill absent from the target definition.

### Prompt-Injection Boundaries

- Agent descriptions and system prompts are trusted configuration;
- task and context are untrusted model data;
- child output is returned to the parent as tool data, not system instructions;
- model-generated tool arguments cannot contain credentials or trusted runtime context;
- authorization decisions never rely only on model text;
- event and error serialization redact API keys and secrets.

### Registry Sources

Programmatic registration is the v1 source of truth. A later filesystem source may use
`.otherone/agents/<id>/AGENT.md`, but project-local files must not automatically gain broader
permissions than the host application grants.

Duplicate Agent IDs from multiple sources are errors unless precedence is explicitly configured.
Silent shadowing is unsafe for Agent definitions.

## Recursion And Cycle Control

Default behavior:

- root depth is `0`;
- direct self-call is denied;
- calling any Agent already present in `ancestry` is denied;
- repeated sibling calls to the same Agent are allowed;
- recursion can be enabled explicitly per Agent/runtime policy;
- depth and total-run limits still apply when recursion is enabled.

Examples:

```text
A -> B -> A       denied by ancestry policy
A -> A            denied by self-call policy
A -> B, A -> B    allowed as two independent sibling runs
```

Cycle prevention is a runtime guarantee, not merely a prompt instruction.

## Limits And Budgeting

The new runtime must never use an effectively infinite default such as `999999` iterations.

Recommended safe default profile:

```rust
pub struct RuntimeLimits {
    pub max_depth: usize,                  // 3
    pub max_total_runs: usize,             // 32
    pub max_children_per_run: usize,       // 8
    pub max_concurrent_model_calls: usize, // 4
    pub max_concurrent_tool_calls: usize,  // 8
    pub max_iterations_per_run: u32,       // 12
    pub max_model_calls: u32,              // 64 per root tree
    pub max_tool_calls: u32,               // 128 per root tree
    pub max_agent_calls: u32,              // 32 per root tree
    pub max_context_transfer_bytes: usize, // 64 KiB
    pub max_result_bytes: usize,           // 256 KiB
    pub run_timeout: Duration,             // 5 minutes
    pub root_timeout: Duration,            // 15 minutes
    pub token_budget: Option<u64>,
}
```

Exact defaults remain configurable, but all new-runtime limits are finite.

Budget rules:

- one `BudgetLedger` is shared by every run in a root tree;
- reservations are atomic before parallel work starts;
- actual usage is committed after provider/tool completion;
- token limits are best-effort when a provider does not report usage;
- estimated input tokens are checked before a model request;
- a child cannot receive a deadline later than its parent/root deadline;
- Agent and per-call limits may tighten runtime limits but cannot loosen them.

### Concurrency Deadlock Rule

Do not hold one full-run semaphore permit while a parent waits for a child. With nested calls this
can deadlock when all permits are held by waiting ancestors.

Instead:

- total runs are controlled by a non-blocking budget counter;
- model concurrency permits are acquired only around model requests;
- tool concurrency permits are acquired only around tool execution;
- a parent waiting for a child holds no model/tool execution permit;
- child-call fan-out is separately bounded by per-run and root budgets.

## Parallel Tool And Agent Calls

When a provider returns multiple tool calls and parallel execution is enabled:

- validate and reserve all calls before spawning work;
- execute independent asynchronous handlers through bounded concurrency;
- associate every result with its original `tool_call_id`;
- persist tool results in provider-valid tool-call order;
- event order may reflect real completion order;
- one failure does not cancel siblings unless fail-fast policy is enabled;
- parent context continues only after all required tool results have been persisted.

This supports parallel child Agents without changing model message semantics.

## Cancellation And Deadlines

Use `tokio_util::sync::CancellationToken` or an equivalent hierarchical token.

Rules:

- cancelling a root run cancels every descendant;
- cancelling a child cancels its descendants but not its parent by default;
- parent timeout cancels the active child call;
- providers and tool handlers receive cancellation/deadline context;
- handlers that cannot be interrupted are marked accordingly and their late result is ignored;
- cancelled runs transition once and cannot later become completed;
- cancellation is represented separately from failure.

The existing interactive handle should gain a `Cancel` command through the compatibility path.

## Idempotency And Retries

Agent and tool execution may have side effects. The framework must not blindly retry them.

Rules:

- each Agent tool call has an `AgentCallId`, normally derived from the provider `tool_call_id`;
- `(root_run_id, caller_run_id, agent_call_id)` is an idempotency key;
- duplicate delivery returns the existing in-flight or completed result;
- root application requests may provide their own idempotency key;
- automatic retries are off by default;
- retry policy may only retry failures marked retryable;
- non-idempotent tools must declare that property;
- provider stream reconnection must not execute an already committed tool call again.

## Error Model

Extend `AgentError` with stable categories:

```rust
pub enum AgentError {
    InvalidConfiguration(String),
    AgentNotFound(AgentId),
    AgentCallDenied { caller: AgentId, target: AgentId },
    AgentCallCycle { ancestry: Vec<AgentId>, target: AgentId },
    MaxDepthExceeded { max: usize },
    BudgetExceeded(BudgetKind),
    SessionBusy(String),
    TimedOut,
    Cancelled,
    Model(otherone_ai::error::AiError),
    Tool(ToolError),
    Context(String),
    Storage(String),
    Internal(String),
}
```

The model-facing Agent-call result uses a serializable, redacted failure:

```rust
pub struct AgentFailure {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}
```

Child failures normally become structured tool results so the parent can recover, choose another
Agent, or explain the failure. Corruption, root cancellation, or required storage failure may still
terminate the parent according to policy.

## Model Profiles And Credentials

The current `AiOptions` combines credentials, model generation settings, prompts, messages, and tool
handlers. Multi-Agent execution requires these concerns to be separated.

Recommended model profile:

```rust
pub struct ModelProfile {
    pub id: ModelProfileId,
    pub provider: ProviderType,
    pub api_key: SecretString,
    pub base_url: String,
    pub model: String,
    pub context_length: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub provider_options: Option<serde_json::Value>,
}
```

Rules:

- credentials are not `Serialize` and use redacted `Debug`;
- Agent definitions store only profile references and safe overrides;
- profile resolution occurs before a run starts;
- each child may use a different profile;
- the existing `auxiliary_ai` becomes a named profile/use case, such as `memory`, instead of a
  special positional Agent parameter.

## Skills Integration

Skills remain instruction resources, not Agents and not executable code.

Each Agent definition declares a Skill access policy. At run preparation:

- the runtime resolves allowed Skills from the root `RuntimeSnapshot`;
- only allowed Skill names/descriptions are exposed;
- full `SKILL.md` content is loaded on demand through allowed read tooling;
- project Skill changes do not alter an active root run's `RuntimeSnapshot`;
- a child does not automatically inherit the caller's Skill access.

An Agent may use the same Skill as another Agent without sharing session state.

## MCP Integration

MCP tools are adapted into asynchronous `ToolRegistry` entries.

Rules:

- Agent Runner does not know MCP transport details;
- server/tool names are qualified to avoid collisions;
- each Agent sees only its allowed MCP tools;
- MCP calls receive cancellation and deadlines where transport permits;
- Agent-call routing is not implemented as an MCP server internally, though a future remote
  transport may expose compatible semantics.

## Memory Integration

Memory access is explicit per Agent:

```rust
pub enum MemoryPolicy {
    Disabled,
    ReadOnlyShared,
    ReadWriteShared,
    PrivateAgent,
}
```

Default for child Agents: `Disabled` unless the definition explicitly grants memory.

Rules:

- memory is scoped by inherited `RuntimeContext`;
- child Agents do not automatically write global long-term memory;
- read-only access is preferred for research/review children;
- private memory additionally scopes data by `AgentId`;
- memory writes include source `run_id`, `session_id`, and `agent_id`;
- memory model selection uses a named model profile instead of `auxiliary_ai` positional wiring.

## Agent Definition Discovery

Programmatic definitions are implemented first. A future filesystem adapter may mirror the existing
Skills discovery style without conflating Agent and Skill concepts.

Suggested optional format:

```text
.otherone/agents/researcher/AGENT.md
```

```markdown
---
schema_version: 1
id: researcher
description: Research facts and return cited findings.
model: research
tools:
  - mcp.web.search
  - local.read_file
skills:
  - web-research
callable_agents: []
memory: read_only_shared
max_iterations: 8
---

You are a focused research Agent...
```

Security rules:

- filesystem definitions are declarative and never contain credentials;
- host runtime policy intersects all requested permissions;
- unknown tools, Skills, Agents, or model profiles fail validation;
- duplicate IDs fail unless explicit source precedence is configured;
- schema versions are required for forward-compatible parsing.

## Extensibility Boundaries

The design intentionally leaves stable extension points for:

- `AgentDefinitionSource`: programmatic, filesystem, database, or remote catalog;
- `AgentCallAuthorizer`: tenant policy, billing, approvals, and business permissions;
- `AgentTransport`: local execution first, remote/distributed execution later;
- `EventSink`: in-process channel, tracing, database, OpenTelemetry, or message bus;
- `ConversationStore`: local file and database backends;
- `RunStore`: in-memory, SQL, document database, or durable scheduler;
- `ArtifactStore`: large files and structured outputs;
- `AgentMiddleware`: tracing, metrics, policy, audit, and approval hooks;
- `AgentSelector`: explicit ID first, capability routing later;
- `Scheduler`: immediate local execution first, queued/durable execution later.

Extension traits should be introduced when their first real implementation is needed. The core types
and ownership model must not prevent them, but v1 should avoid empty abstractions with no behavior.

## Compatibility Strategy

Existing APIs remain available:

```rust
Otherone::invoke_agent(...)
Otherone::invoke_agent_stream(...)
Otherone::invoke_agent_stream_interactive(...)
```

Migration behavior:

- legacy `InputOptions` and `AiOptions` are converted into one temporary Agent definition and model
  profile;
- legacy APIs create/use a runtime with no callable Agents unless explicitly configured through the
  new API;
- `AiOptions.tools_realize` is adapted through `SyncToolHandler`;
- `StreamAgentEvent` is produced from typed events;
- current session and storage behavior remains unchanged for legacy calls;
- current `auxiliary_ai` maps to a memory model profile in the adapter;
- old APIs are deprecated only after equivalent runtime examples and migration docs exist.

Do not add Multi-Agent fields directly to the existing public structs. Current callers commonly use
struct literals, so adding fields is source-breaking. Introduce new builder-based types instead.

## Implementation Plan

### Phase 0: Structural Preparation

1. Split the current large `otherone-agent/src/lib.rs` into internal modules without changing
   behavior.
2. Define typed errors, IDs, event envelopes, results, and builders.
3. Add compatibility conversions for existing public types.
4. Add finite safe defaults for new runtime APIs while preserving legacy defaults temporarily.

Acceptance criteria:

- current tests and examples continue to pass;
- no Agent-call behavior exists yet;
- streaming and non-streaming external behavior remains compatible.

### Phase 1: Async Tools And Unified Runner

1. Introduce `ToolHandler`, `ToolRegistry`, `ToolCallContext`, and sync adapters.
2. Adapt local tools, memory tools, and MCP integration.
3. Extract one internal Agent Runner state machine.
4. Make non-streaming and streaming APIs different observers of the same Runner.
5. Preserve provider-valid assistant/tool message ordering.

Acceptance criteria:

- asynchronous tools can be awaited naturally;
- streaming and non-streaming use one logical loop;
- existing synchronous tools work through adapters;
- multiple parallel tool results preserve tool-call IDs and storage order.

### Phase 2: Local Multi-Agent Core

1. Implement `ModelRegistry`, `AgentDefinition`, and `AgentRegistry` snapshots.
2. Implement `AgentRuntime`, `AgentRouter`, and `RunSupervisor`.
3. Implement isolated child sessions and Run Store records.
4. Register `otherone.call_agent` through `ToolRegistry`.
5. Implement nested synchronous Agent calls.
6. Implement permission, ancestry, depth, deadline, and budget checks.
7. Forward child events through typed event envelopes.

Acceptance criteria:

- any registered Agent can be selected as entry Agent;
- authorized A -> B and A -> B -> C calls complete;
- unauthorized and cyclic calls are rejected deterministically;
- child context/session isolation is verified;
- parent receives a structured tool result and continues its loop.

### Phase 3: Lifecycle Hardening

1. Implement hierarchical cancellation.
2. Implement idempotency and duplicate-call protection.
3. Implement bounded parallel child calls without semaphore deadlock.
4. Add targeted run commands and root/child event filters.
5. Add run/session developer inspection APIs.
6. Add metrics and tracing hooks.

Acceptance criteria:

- cancellation reaches all descendants;
- duplicate tool delivery does not repeat an Agent call;
- parallel nested calls remain within limits;
- lifecycle events and stored states agree.

### Phase 4: Optional Extensions

- `spawn_agent`, `await_agent`, and background task handles;
- filesystem `AGENT.md` discovery;
- capability-based Agent selection;
- approval and human-in-the-loop suspension;
- artifact storage for large outputs;
- remote Agent transport and durable scheduling;
- process-crash recovery and resume.

These extensions must use the same `AgentRunRequest`, lineage, result, event, authorization, and
budget semantics.

## Verification Plan

### Registry And Configuration

- valid Agent and model profiles build successfully;
- duplicate Agent IDs fail;
- missing model/tool/Skill/Agent references fail before execution;
- registry replacement requires an explicit API;
- active runs continue using their original snapshot after registry changes;
- secrets never appear in `Debug`, events, or persisted definitions.

### Basic Execution

- any Agent can be an entry Agent;
- root-only run behaves like the current single-Agent loop;
- A -> B returns a structured tool result;
- A -> B -> C preserves lineage and returns results in order;
- calling B twice creates two independent sibling runs.

### Permissions And Context

- a caller cannot call an Agent outside its allowlist;
- a call cannot add tools, Skills, memory, or model access;
- child receives only task plus allowed explicit context;
- child does not receive the complete parent transcript;
- inherited `RuntimeContext.partition_key` cannot be overridden;
- untrusted task/context cannot become system instructions.

### Recursion And Budgets

- self-call is denied by default;
- A -> B -> A is denied by default;
- explicit recursive policy still respects max depth;
- max total runs is atomic under parallel calls;
- model/tool/token/output budgets terminate with typed outcomes;
- new-runtime iteration defaults are finite.

### Concurrency

- parallel child calls respect model and tool semaphores;
- waiting parents do not hold execution permits;
- concurrency limit `1` does not deadlock a parent waiting for one child;
- tool results are persisted in provider-valid order;
- sibling failure does not cancel other siblings unless fail-fast is enabled.

### Cancellation And Timeouts

- root cancellation cancels every descendant;
- child cancellation does not cancel the parent by default;
- parent deadline bounds child deadline;
- cancelled runs never transition to completed;
- late provider/tool responses are ignored safely.

### Storage

- root and child sessions are distinct;
- child sessions are hidden from normal user-session listing;
- parent session stores only call request and final result;
- Run Store lineage matches conversation metadata;
- same `(partition_key, session_id)` cannot have two active writers;
- two runtime partitions cannot observe each other's child runs;
- storage failure follows configured durability behavior.

### Events

- every event contains root/run/parent/Agent identity;
- root-wide sequence numbers remain unique under parallel children;
- child lifecycle filtering works;
- lagging event consumers do not prevent final result delivery;
- compatibility conversion produces existing `StreamAgentEvent` types;
- raw chunks and thinking are disabled or redacted according to policy.

### Idempotency And Failure Recovery

- duplicate Agent-call IDs return the same in-flight/completed result;
- unknown Agent, denied call, timeout, cancellation, and budget exhaustion have stable error codes;
- recoverable child failure is returned to the parent as tool data;
- no automatic retry repeats a non-idempotent tool or Agent call.

### Compatibility

- current `invoke_agent` tests pass through adapters;
- current streaming event consumers continue to work;
- current synchronous tool closures continue to work;
- current localfile/database session behavior remains unchanged for legacy APIs;
- memory behavior remains equivalent when no Multi-Agent features are configured.

## Rejected Alternatives

### Permanent Main Agent And Sub Agent Types

Rejected because hierarchy changes by application and by run. It would duplicate configuration and
execution logic and prevent a specialist Agent from being used as an entry Agent.

### Separate Sub-Agent Runner

Rejected because it would create a second Agent loop with divergent context, tool, provider, memory,
and storage behavior.

### Shared Parent/Child Conversation

Rejected because concurrent writes, provider tool-message ordering, context growth, permission
leakage, and cancellation semantics become ambiguous.

### Unrestricted Mutual Calls

Rejected because peer definitions do not imply unrestricted authority. Runtime calls require
allowlists, depth controls, budgets, and optional application authorization.

### Implement Agent Calls As A Special Synchronous Tool Branch

Rejected because an Agent call is asynchronous and should use the same extensible tool contract as
MCP and other asynchronous operations.

### Fire-And-Forget As The Only Call Mode

Rejected because the parent usually needs a deterministic result before continuing. Background
execution is a later additive mode.

### Copying `AiOptions` For Every Child

Rejected because it mixes credentials, prompts, messages, and non-cloneable tool handlers. Model
profiles, Agent definitions, run input, and runtime services must be separate.

### String Event Types Without Run Lineage

Rejected because nested concurrent runs require stable typing, filtering, ordering, and parent-child
identity.

## Final Architectural Contract

Otherone is a Multi-Agent Runtime made of peer Agent definitions. An application chooses an entry
Agent for each root run. Agent-to-Agent calls dynamically create child runs under a controlled
Router and Supervisor. Every run uses the same Agent Runner, while permissions, context, sessions,
budgets, events, and results remain explicitly isolated and traceable.

The word "sub Agent" describes a run's temporary relationship to its caller. It does not describe a
different kind of Agent.

This contract is the foundation for the first local implementation and for later background,
remote, durable, and distributed extensions.
