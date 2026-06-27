// 作用：Agent 循环驱动 — 整个框架的核心，驱动 AI 对话流程
// 关联：调用 ai、context、tools、storage 等所有子模块
// 预期结果：根据 input 和 ai 配置，执行完整的 Agent 循环，返回最终响应

pub mod error;
pub mod response_parser;
pub mod types;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::warn;

use crate::error::AgentError;
use crate::response_parser::parse_ai_response;
use crate::types::{
    AiOptions, ContextLoadType as AgentContextLoadType, InputOptions,
    StorageType as AgentStorageType,
};
use otherone_storage::types::{StorageType, WriteEntryOptions};

const LONG_TERM_MEMORY_TOOL_NAME: &str = "otherone_add_long_term_memory";
const LONG_TERM_MEMORY_RECALL_TOOL_NAME: &str = "otherone_recall_long_term_memory";
const LONG_TERM_MEMORY_COMMIT_TOOL_NAME: &str = "otherone_commit_long_term_memory";
const DEFAULT_LONG_TERM_MEMORY_RECALL_MAX_TYPES: usize = 5;
const MAX_LONG_TERM_MEMORY_RECALL_MAX_TYPES: usize = 16;
const LONG_TERM_MEMORY_SYSTEM_PROMPT: &str = r#"Long-term memory is enabled.

Evaluate the current user message for reusable long-term information. Reusable information includes stable user preferences, identity facts, durable project rules, long-term goals, recurring habits, and constraints that may help future conversations.

Do not store one-off requests, temporary state, casual filler, reminders, calculations, translation requests, or information that is only useful for the current response.

Do not store secrets, credentials, API keys, passwords, verification codes, payment details, or other highly sensitive data. If the user gives sensitive data, respond normally but do not call the memory tool for that content.

Bias toward calling the memory tool when the user explicitly says information can be remembered long-term, or when the message contains durable identity, preferred name, preferred form of address, default language, stable preference, recurring habit, durable project rule, allergy, or long-term constraint.

When reusable information exists, call `otherone_add_long_term_memory` with:
- `storage`: a concise natural-language memory statement.
- `types`: the specific semantic category for this exact memory node. Prefer concrete categories over broad ones. Do not use a broad parent category when a more specific child category is available.
- `possible_parent_types`: 1-5 short keywords or semantic tags for likely existing parent memory categories. These must be compact category hints, not full sentences. Examples: `food`, `noodles`, `diet preference`, `project rule`, `coding style`.

If you tell the user that you recorded, remembered, saved, or noted durable information, you must have called `otherone_add_long_term_memory` for that information in the same turn. Do not claim durable memory was recorded unless you called the tool.

Examples:
- "My name is Lin Che" -> storage: "用户的称呼是林澈", types: "用户称呼", possible_parent_types: ["身份信息", "称呼", "用户资料"].
- "Call me Jae from now on" -> storage: "用户的称呼是 Jae", types: "用户称呼", possible_parent_types: ["身份信息", "称呼", "用户资料"].
- "Use Chinese by default" -> storage: "与用户交流默认使用中文", types: "交流语言偏好", possible_parent_types: ["沟通偏好", "语言偏好", "对话设置"].
- "I like zhajiangmian" -> storage: "用户喜欢吃炸酱面", types: "用户喜欢吃的面食", possible_parent_types: ["食物偏好", "饮食偏好", "面食偏好"].
- "I like sweet and sour ribs" -> storage: "用户喜欢吃糖醋排骨", types: "用户喜欢吃的菜", possible_parent_types: ["食物偏好", "饮食偏好", "菜肴偏好"].
- "I dislike cilantro" -> storage: "用户不喜欢吃香菜", types: "用户不喜欢的食材", possible_parent_types: ["食物偏好", "饮食偏好", "忌口偏好"].
- "I prefer concise Rust code" -> storage: "用户偏好简洁的 Rust 代码", types: "Rust 代码风格偏好", possible_parent_types: ["编码偏好", "Rust", "代码风格"].

When the current user request could benefit from known long-term information, call `otherone_recall_long_term_memory` before answering. Pass `memory_types` as compact type keywords or semantic tags, such as ["饮食偏好"], ["用户称呼", "语言偏好"], ["Rust 错误处理偏好"], or ["前端设计偏好"].

The recall tool returns only remembered facts, not internal node JSON. Use those facts as background context for the answer.

Call the recall tool at most once per user request unless the first recall clearly misses a separate required topic.

The add-memory tool only queues the memory for later processing and returns immediately. After tools return, continue the normal response. Do not mention memory storage unless the user asks."#;
const LONG_TERM_MEMORY_MODEL_SYSTEM_PROMPT: &str = r#"You are the long-term memory write planner for Otherone.

You receive:
- A candidate memory submitted by the main agent.
- A table of nearby existing memory nodes selected by system search.

The memory tree has a headless point with no semantic content. Its direct children are root points. Deeper nodes must have more specific `types` than their parents.

Your task is to choose the best write operation. Always call `otherone_commit_long_term_memory`. Do not answer in natural language.

Rules:
- If no existing node is semantically relevant, use `insert_root`.
- If an existing node is a good parent and its `types` is already broad enough, use `insert_child`.
- If an existing node should become a broader parent category before inserting the new memory, use `update_parent_and_insert_child`.
- If the candidate duplicates or corrects an existing node, use `update_existing`.
- If the candidate is not reusable long-term information, or contains secrets, credentials, API keys, passwords, verification codes, payment details, temporary state, reminders, one-off requests, calculations, or casual filler, use `ignore`.
- Do not ignore durable identity, preferred name, preferred form of address, default communication language, stable design/coding/work preference, project rule, diet constraint, allergy, recurring habit, or long-term user constraint.
- If the candidate is a sibling of an existing specific memory, do not simply attach it under that specific memory. Use `update_parent_and_insert_child` to broaden the existing node's `storage` and `types`, then insert the candidate as a more specific child.
- A parent node's `storage` must make sense as a parent summary for all of its children. If the parent storage only describes one specific item, broaden it before adding a sibling child.
- Keep `storage` as a concise reusable fact.
- Keep `types` specific for the inserted or updated node. Parent `types` may become broader only when needed to cover both the old and new child concepts.

Canonical example:
- Existing node: point_id="p1", storage="用户喜欢吃炸酱面", types="用户喜欢的面食".
- Candidate: storage="用户喜欢吃糖醋排骨", types="用户喜欢的菜肴".
- Correct action: `update_parent_and_insert_child`.
- Correct parent update: parent_id="p1", parent_storage="用户喜欢吃炸酱面和糖醋排骨等食物", parent_types="用户喜欢的食物".
- Correct inserted child: storage="用户喜欢吃糖醋排骨", types="用户喜欢的菜肴".

Canonical language example:
- Existing candidates may be unrelated.
- Candidate: storage="与用户交流默认使用中文", types="交流语言偏好".
- Correct action: `insert_root`, unless an existing communication preference node should be updated.

Canonical food restriction example:
- Existing node may describe liked foods.
- Candidate: storage="用户不喜欢吃香菜", types="用户不喜欢的食材".
- Correct action: store it. Use `update_parent_and_insert_child` if a food preference parent should broaden into food preferences and restrictions, otherwise use `insert_root`. Do not ignore stable dislikes or diet restrictions."#;

const LONG_TERM_MEMORY_FALLBACK_TYPES: &str = "待分类长期记忆";

type LongTermMemoryQueue = Arc<Mutex<Vec<PendingLongTermMemory>>>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PendingLongTermMemory {
    storage: String,
    types: String,
    #[serde(default)]
    possible_parent_types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct LongTermMemoryRecallArgs {
    #[serde(default)]
    memory_types: Vec<String>,
}

#[derive(Debug, Clone)]
struct MemoryModelConfig {
    provider: otherone_ai::types::ProviderType,
    api_key: String,
    base_url: String,
    model: String,
    context_length: Option<u32>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    other: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct LongTermMemoryCommitArgs {
    action: String,
    storage: Option<String>,
    types: Option<String>,
    parent_id: Option<String>,
    target_id: Option<String>,
    parent_storage: Option<String>,
    parent_types: Option<String>,
}

/// 流式 Agent 事件类型
/// 作用：定义流式 Agent 循环中 yield 给调用者的事件
/// 关联：被 invoke_agent_stream 使用
/// 预期结果：调用者可以根据 event_type 区分不同的事件
#[derive(Debug, Clone)]
pub struct StreamAgentEvent {
    /// 事件类型: "chunk" | "thinking" | "tool_calls" | "complete" | "error"
    pub event_type: String,
    /// 事件内容
    pub content: String,
    /// 原始 chunk 数据（当 event_type 为 "chunk" 时）
    pub raw_chunk: Option<serde_json::Value>,
    /// 错误信息（当 event_type 为 "error" 时）
    pub error: Option<String>,
}

/// 流式 Agent 调用
/// 作用：启动流式 Agent 循环，通过 mpsc channel 实时发送事件
/// 关联：被 otherone 主 crate 的 Otherone::invoke_agent_stream 调用
/// 预期结果：返回 mpsc Receiver，调用者可以异步迭代接收事件
pub async fn invoke_agent_stream(
    input: InputOptions,
    mut ai: AiOptions,
    auxiliary_ai: Option<AiOptions>,
) -> Result<mpsc::Receiver<StreamAgentEvent>, AgentError> {
    let (tx, rx) = mpsc::channel::<StreamAgentEvent>(256);

    tokio::spawn(async move {
        let mut auxiliary_ai = auxiliary_ai;
        if let Err(e) = run_stream_loop(input, &mut ai, auxiliary_ai.as_mut(), &tx).await {
            let _ = tx
                .send(StreamAgentEvent {
                    event_type: "error".to_string(),
                    content: format!("[error:{}]", e),
                    raw_chunk: None,
                    error: Some(e.to_string()),
                })
                .await;
        }
    });

    Ok(rx)
}

/// 流式循环实际逻辑
/// 作用：在后台 task 中执行完整的流式 Agent 循环
/// 关联：被 invoke_agent_stream 调用
/// 预期结果：通过 tx channel 发送所有事件，循环结束后返回
async fn run_stream_loop(
    input: InputOptions,
    ai: &mut AiOptions,
    auxiliary_ai: Option<&mut AiOptions>,
    tx: &mpsc::Sender<StreamAgentEvent>,
) -> Result<(), AgentError> {
    let long_term_memory_enabled = input.enable_long_term_memory.unwrap_or(false);
    let memory_queue = if long_term_memory_enabled {
        Some(Arc::new(Mutex::new(Vec::new())))
    } else {
        None
    };
    let memory_model_config = if long_term_memory_enabled {
        let model_source = auxiliary_ai.as_ref().map(|aux| &**aux).unwrap_or(ai);
        Some(MemoryModelConfig::from_ai(model_source))
    } else {
        None
    };
    let long_term_memory_recall_max_types =
        normalize_recall_max_types(input.long_term_memory_recall_max_types);

    if let Some(queue) = &memory_queue {
        configure_long_term_memory(ai, Arc::clone(queue), long_term_memory_recall_max_types);
    }

    let storage_type: StorageType = match &input.storage_type {
        Some(AgentStorageType::LocalFile) => StorageType::LocalFile,
        Some(AgentStorageType::Database) => StorageType::Database,
        None => StorageType::LocalFile,
    };

    // 存储用户消息
    if let Some(ref user_prompt) = ai.user_prompt {
        otherone_storage::write_entry(&WriteEntryOptions {
            storage_type: storage_type.clone(),
            session_id: input.session_id.clone(),
            role: "user".to_string(),
            content: user_prompt.clone(),
            tools: None,
            token_consumption: None,
            create_at: None,
            database_config: input.database_config.clone(),
        })
        .await
        .map_err(|e| AgentError::ContextError(e.to_string()))?;
    }

    let max_iterations = input.max_iterations.unwrap_or(999999);
    let mut iteration: u32 = 0;
    let mut long_term_memory_activity = false;

    while iteration < max_iterations {
        iteration += 1;

        // 组合 tools 配置
        let tools = otherone_tools::combine_tools(ai.tools.clone());
        ai.tools = tools;

        // 加载上下文
        let context_load_type = match &input.context_load_type {
            AgentContextLoadType::LocalFile => otherone_context::types::ContextLoadType::LocalFile,
            AgentContextLoadType::Database => otherone_context::types::ContextLoadType::Database,
        };

        let messages =
            otherone_context::combine_context(&otherone_context::types::CombineContextOptions {
                session_id: input.session_id.clone(),
                load_type: context_load_type,
                provider: ai.provider.clone(),
                context_window: input.context_window,
                threshold_percentage: input.threshold_percentage,
                ai: ai.other.clone(),
                system_prompt: ai.system_prompt.clone(),
                tools: ai.tools.clone(),
                database_config: input.database_config.clone(),
            })
            .await
            .map_err(|e| AgentError::ContextError(e.to_string()))?;

        // 构建 AI 配置，强制流式
        let mut ai_config = build_ai_config(ai, &messages);
        ai_config["stream"] = serde_json::json!(true);

        // 调用流式 AI 模型
        let mut stream = otherone_ai::invoke_model_stream(
            ai.provider.clone(),
            &ai.api_key,
            &ai.base_url,
            ai_config,
        )
        .await?;

        // 累积变量
        let mut full_content = String::new();
        let mut role = "assistant".to_string();
        let mut stream_tool_calls: Vec<otherone_ai::types::ToolCall> = Vec::new();
        let mut token_consumption: u32 = 0;

        // 遍历流，实时发送给调用者
        use futures::StreamExt;
        while let Some(chunk_result) = stream.next().await {
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx
                        .send(StreamAgentEvent {
                            event_type: "error".to_string(),
                            content: format!("[error:{}]", e),
                            raw_chunk: None,
                            error: Some(e.to_string()),
                        })
                        .await;
                    return Err(AgentError::AiError(e));
                }
            };

            // 发送原始 chunk
            let raw_json = serde_json::to_value(&chunk).unwrap_or_default();
            let _ = tx
                .send(StreamAgentEvent {
                    event_type: "chunk".to_string(),
                    content: String::new(),
                    raw_chunk: Some(raw_json),
                    error: None,
                })
                .await;

            // 提取 delta 信息
            if let Some(choice) = chunk.choices.first() {
                if let Some(ref delta) = choice.delta {
                    if let Some(ref r) = delta.role {
                        role = r.clone();
                    }
                    if let Some(ref c) = delta.content {
                        full_content.push_str(c);
                    }
                    if let Some(thinking) = delta_thinking_content(delta) {
                        let _ = tx
                            .send(StreamAgentEvent {
                                event_type: "thinking".to_string(),
                                content: thinking.to_string(),
                                raw_chunk: None,
                                error: None,
                            })
                            .await;
                    }
                }

                if let Some(ref delta) = choice.delta {
                    // 累积 tool_calls
                    if let Some(ref tc) = delta.tool_calls {
                        for tool_call in tc {
                            // 按 index 或 id 匹配合并
                            merge_tool_call_delta(&mut stream_tool_calls, tool_call);
                        }
                    }
                }

                if let Some(ref usage) = chunk.usage {
                    token_consumption = usage.total_tokens.unwrap_or(0);
                }
            }
        }

        // 流结束，构建完整响应
        let tools_wrapper = if stream_tool_calls.is_empty() {
            None
        } else {
            Some(otherone_ai::types::ToolCallsWrapper {
                tool_calls: stream_tool_calls.clone(),
            })
        };

        // 存储 AI 响应
        otherone_storage::write_entry(&WriteEntryOptions {
            storage_type: storage_type.clone(),
            session_id: input.session_id.clone(),
            role: role.clone(),
            content: full_content.clone(),
            tools: tools_wrapper
                .as_ref()
                .map(|tw| serde_json::json!({ "tool_calls": tw.tool_calls })),
            token_consumption: Some(token_consumption),
            create_at: None,
            database_config: input.database_config.clone(),
        })
        .await
        .map_err(|e| AgentError::ContextError(e.to_string()))?;

        // 检查是否有 tool 调用
        if let Some(ref tw) = tools_wrapper {
            if !tw.tool_calls.is_empty() {
                if tw
                    .tool_calls
                    .iter()
                    .any(|tool_call| tool_call.function.name == LONG_TERM_MEMORY_TOOL_NAME)
                {
                    long_term_memory_activity = true;
                }

                let tool_calls_info: Vec<String> = tw
                    .tool_calls
                    .iter()
                    .map(|tc| format!("{}({})", tc.function.name, tc.function.arguments))
                    .collect();

                let _ = tx
                    .send(StreamAgentEvent {
                        event_type: "tool_calls".to_string(),
                        content: format!("[tool_calls:{}]", tool_calls_info.join(", ")),
                        raw_chunk: None,
                        error: None,
                    })
                    .await;

                // 处理 tool 调用
                let default_tools_realize: HashMap<
                    String,
                    Box<dyn Fn(serde_json::Value) -> String + Send + Sync>,
                > = HashMap::new();
                let tools_realize = ai.tools_realize.as_ref().unwrap_or(&default_tools_realize);

                let tool_results = otherone_tools::process_tools(&tw.tool_calls, tools_realize)
                    .map_err(|e| AgentError::ToolError(e))?;

                for tool_result in &tool_results {
                    let tool_content = serde_json::to_string(
                        tool_result
                            .result
                            .as_ref()
                            .unwrap_or(&serde_json::Value::Null),
                    )
                    .unwrap_or_default();

                    let tools_value = serde_json::json!({
                        "tool_call_id": tool_result.tool_call_id,
                        "function_name": tool_result.function_name,
                        "result": tool_result.result,
                        "error": tool_result.error,
                    });

                    otherone_storage::write_entry(&WriteEntryOptions {
                        storage_type: storage_type.clone(),
                        session_id: input.session_id.clone(),
                        role: "tool".to_string(),
                        content: tool_content,
                        tools: Some(tools_value),
                        token_consumption: None,
                        create_at: None,
                        database_config: input.database_config.clone(),
                    })
                    .await
                    .map_err(|e| AgentError::ContextError(e.to_string()))?;
                }

                spawn_drained_long_term_memory_processing(&memory_queue, &memory_model_config);

                tokio::time::sleep(Duration::from_millis(1500)).await;
                continue;
            }
        }

        // 发送完成事件
        spawn_long_term_memory_heuristic_fallback(
            &memory_model_config,
            ai.user_prompt.as_deref(),
            long_term_memory_activity,
        );

        let _ = tx
            .send(StreamAgentEvent {
                event_type: "complete".to_string(),
                content: full_content,
                raw_chunk: None,
                error: None,
            })
            .await;

        return Ok(());
    }

    let err_msg = format!(
        "Agent循环次数超过限制({}次)，可能陷入无限循环",
        max_iterations
    );
    let _ = tx
        .send(StreamAgentEvent {
            event_type: "error".to_string(),
            content: format!("[error:{}]", err_msg),
            raw_chunk: None,
            error: Some(err_msg.clone()),
        })
        .await;
    Err(AgentError::MaxIterationsExceeded(max_iterations))
}

fn delta_thinking_content(delta: &otherone_ai::types::ResponseDelta) -> Option<&str> {
    delta
        .reasoning_content
        .as_deref()
        .or(delta.reasoning.as_deref())
        .or(delta.thinking.as_deref())
        .or(delta.thought.as_deref())
        .filter(|content| !content.is_empty())
}

fn merge_tool_call_delta(
    tool_calls: &mut Vec<otherone_ai::types::ToolCall>,
    delta: &otherone_ai::types::ToolCall,
) {
    let target = match delta.index {
        Some(index) => {
            let target_index = index as usize;
            while tool_calls.len() <= target_index {
                tool_calls.push(empty_tool_call(Some(tool_calls.len() as u32)));
            }
            &mut tool_calls[target_index]
        }
        None => match tool_calls
            .iter()
            .position(|existing| !delta.id.is_empty() && existing.id == delta.id)
        {
            Some(position) => &mut tool_calls[position],
            None => {
                tool_calls.push(empty_tool_call(None));
                tool_calls.last_mut().expect("new tool call exists")
            }
        },
    };

    if delta.index.is_some() {
        target.index = delta.index;
    }
    if !delta.id.is_empty() {
        target.id = delta.id.clone();
    }
    if !delta.call_type.is_empty() {
        target.call_type = delta.call_type.clone();
    }
    if !delta.function.name.is_empty() {
        target.function.name.push_str(&delta.function.name);
    }
    if !delta.function.arguments.is_empty() {
        target
            .function
            .arguments
            .push_str(&delta.function.arguments);
    }
}

fn empty_tool_call(index: Option<u32>) -> otherone_ai::types::ToolCall {
    otherone_ai::types::ToolCall {
        index,
        id: String::new(),
        call_type: "function".to_string(),
        function: otherone_ai::types::FunctionCall::default(),
    }
}

/// 调用 Agent — 核心驱动方法
/// 作用：驱动完整的 AI 对话流程，支持非流式模式
/// 关联：调用所有子模块
/// 预期结果：返回最终的 ParsedResponse
pub async fn invoke_agent(
    input: &InputOptions,
    ai: &mut AiOptions,
    auxiliary_ai: Option<&mut AiOptions>,
) -> Result<otherone_ai::types::ParsedResponse, AgentError> {
    let long_term_memory_enabled = input.enable_long_term_memory.unwrap_or(false);
    let memory_queue = if long_term_memory_enabled {
        Some(Arc::new(Mutex::new(Vec::new())))
    } else {
        None
    };
    let memory_model_config = if long_term_memory_enabled {
        let model_source = auxiliary_ai.as_ref().map(|aux| &**aux).unwrap_or(ai);
        Some(MemoryModelConfig::from_ai(model_source))
    } else {
        None
    };
    let long_term_memory_recall_max_types =
        normalize_recall_max_types(input.long_term_memory_recall_max_types);

    if let Some(queue) = &memory_queue {
        configure_long_term_memory(ai, Arc::clone(queue), long_term_memory_recall_max_types);
    }

    let storage_type: StorageType = match &input.storage_type {
        Some(AgentStorageType::LocalFile) => StorageType::LocalFile,
        Some(AgentStorageType::Database) => StorageType::Database,
        None => StorageType::LocalFile,
    };

    // 如果有 user_prompt，先存储用户消息
    if let Some(ref user_prompt) = ai.user_prompt {
        otherone_storage::write_entry(&WriteEntryOptions {
            storage_type: storage_type.clone(),
            session_id: input.session_id.clone(),
            role: "user".to_string(),
            content: user_prompt.clone(),
            tools: None,
            token_consumption: None,
            create_at: None,
            database_config: input.database_config.clone(),
        })
        .await
        .map_err(|e| AgentError::ContextError(e.to_string()))?;
    }

    // 循环次数限制
    let max_iterations = input.max_iterations.unwrap_or(999999);
    let mut iteration: u32 = 0;
    let mut long_term_memory_activity = false;

    while iteration < max_iterations {
        iteration += 1;

        // 组合 tools 配置
        let tools = otherone_tools::combine_tools(ai.tools.clone());
        ai.tools = tools;

        // 组合 context 配置，加载历史消息
        let context_load_type = match &input.context_load_type {
            AgentContextLoadType::LocalFile => otherone_context::types::ContextLoadType::LocalFile,
            AgentContextLoadType::Database => otherone_context::types::ContextLoadType::Database,
        };

        let messages =
            otherone_context::combine_context(&otherone_context::types::CombineContextOptions {
                session_id: input.session_id.clone(),
                load_type: context_load_type,
                provider: ai.provider.clone(),
                context_window: input.context_window,
                threshold_percentage: input.threshold_percentage,
                ai: ai.other.clone(),
                system_prompt: ai.system_prompt.clone(),
                tools: ai.tools.clone(),
                database_config: input.database_config.clone(),
            })
            .await
            .map_err(|e| AgentError::ContextError(e.to_string()))?;

        // 构建 AI 配置 JSON
        let ai_config = build_ai_config(ai, &messages);

        // 调用 AI 模型
        let response =
            otherone_ai::invoke_model(ai.provider.clone(), &ai.api_key, &ai.base_url, ai_config)
                .await?;

        // 解析响应
        let mut parsed =
            parse_ai_response(&response, &ai.provider).map_err(|e| AgentError::ContextError(e))?;

        // 存储 AI 响应到 storage
        otherone_storage::write_entry(&WriteEntryOptions {
            storage_type: storage_type.clone(),
            session_id: input.session_id.clone(),
            role: parsed.role.clone(),
            content: parsed.content.clone(),
            tools: parsed
                .tools
                .as_ref()
                .map(|tw| serde_json::json!({ "tool_calls": tw.tool_calls })),
            token_consumption: Some(parsed.token_consumption),
            create_at: None,
            database_config: input.database_config.clone(),
        })
        .await
        .map_err(|e| AgentError::ContextError(e.to_string()))?;

        // 检查是否有 tool 调用
        if let Some(ref tools_wrapper) = parsed.tools {
            if !tools_wrapper.tool_calls.is_empty() {
                if tools_wrapper
                    .tool_calls
                    .iter()
                    .any(|tool_call| tool_call.function.name == LONG_TERM_MEMORY_TOOL_NAME)
                {
                    long_term_memory_activity = true;
                }

                let tool_calls_info: Vec<String> = tools_wrapper
                    .tool_calls
                    .iter()
                    .map(|tc| format!("{}({})", tc.function.name, tc.function.arguments))
                    .collect();
                parsed.content = format!(
                    "[tool_calls:{}]\n\n{}",
                    tool_calls_info.join(", "),
                    parsed.content
                );

                // 使用用户传入的 tools_realize，若未提供则使用空 HashMap
                let default_tools_realize: HashMap<
                    String,
                    Box<dyn Fn(serde_json::Value) -> String + Send + Sync>,
                > = HashMap::new();
                let tools_realize = ai.tools_realize.as_ref().unwrap_or(&default_tools_realize);

                let tool_results =
                    otherone_tools::process_tools(&tools_wrapper.tool_calls, tools_realize)
                        .map_err(|e| AgentError::ToolError(e))?;

                for tool_result in &tool_results {
                    let tool_content = serde_json::to_string(
                        tool_result
                            .result
                            .as_ref()
                            .unwrap_or(&serde_json::Value::Null),
                    )
                    .unwrap_or_default();

                    let tools_value = serde_json::json!({
                        "tool_call_id": tool_result.tool_call_id,
                        "function_name": tool_result.function_name,
                        "result": tool_result.result,
                        "error": tool_result.error,
                    });

                    otherone_storage::write_entry(&WriteEntryOptions {
                        storage_type: storage_type.clone(),
                        session_id: input.session_id.clone(),
                        role: "tool".to_string(),
                        content: tool_content,
                        tools: Some(tools_value),
                        token_consumption: None,
                        create_at: None,
                        database_config: input.database_config.clone(),
                    })
                    .await
                    .map_err(|e| AgentError::ContextError(e.to_string()))?;
                }

                spawn_drained_long_term_memory_processing(&memory_queue, &memory_model_config);

                tokio::time::sleep(Duration::from_millis(1500)).await;
                continue;
            }
        }

        spawn_long_term_memory_heuristic_fallback(
            &memory_model_config,
            ai.user_prompt.as_deref(),
            long_term_memory_activity,
        );

        return Ok(parsed);
    }

    Err(AgentError::MaxIterationsExceeded(max_iterations))
}

/// 构建 AI 调用的配置 JSON
fn build_ai_config(ai: &AiOptions, messages: &[otherone_ai::types::Message]) -> serde_json::Value {
    let mut config = serde_json::json!({
        "model": ai.model,
        "messages": messages,
    });

    if let Some(context_length) = ai.context_length {
        config["contextLength"] = serde_json::json!(context_length);
    }
    if let Some(temperature) = ai.temperature {
        config["temperature"] = serde_json::json!(temperature);
    }
    if let Some(top_p) = ai.top_p {
        config["topP"] = serde_json::json!(top_p);
    }
    if let Some(ref tools) = ai.tools {
        config["tools"] = serde_json::to_value(tools).unwrap();
    }
    if let Some(ref tool_choice) = ai.tool_choice {
        config["toolChoice"] = serde_json::to_value(tool_choice).unwrap();
    }
    if let Some(parallel_tool_calls) = ai.parallel_tool_calls {
        config["parallelToolCalls"] = serde_json::json!(parallel_tool_calls);
    }
    if let Some(stream) = ai.stream {
        config["stream"] = serde_json::json!(stream);
    }
    if let Some(ref other) = ai.other {
        if let serde_json::Value::Object(ref obj) = other {
            for (key, value) in obj {
                config[key] = value.clone();
            }
        }
    }

    config
}

impl MemoryModelConfig {
    fn from_ai(ai: &AiOptions) -> Self {
        Self {
            provider: ai.provider.clone(),
            api_key: ai.api_key.clone(),
            base_url: ai.base_url.clone(),
            model: ai.model.clone(),
            context_length: ai.context_length,
            temperature: ai.temperature,
            top_p: ai.top_p,
            other: ai.other.clone(),
        }
    }
}

fn configure_long_term_memory(
    ai: &mut AiOptions,
    queue: LongTermMemoryQueue,
    recall_max_types: usize,
) {
    inject_long_term_memory_system_prompt(ai);
    inject_long_term_memory_tools(ai, recall_max_types);
    inject_long_term_memory_tool_handlers(ai, queue, recall_max_types);
}

fn normalize_recall_max_types(max_types: Option<usize>) -> usize {
    max_types
        .unwrap_or(DEFAULT_LONG_TERM_MEMORY_RECALL_MAX_TYPES)
        .clamp(1, MAX_LONG_TERM_MEMORY_RECALL_MAX_TYPES)
}

fn inject_long_term_memory_system_prompt(ai: &mut AiOptions) {
    match ai.system_prompt.as_mut() {
        Some(system_prompt) => {
            if !system_prompt.contains(LONG_TERM_MEMORY_SYSTEM_PROMPT) {
                system_prompt.push_str("\n\n");
                system_prompt.push_str(LONG_TERM_MEMORY_SYSTEM_PROMPT);
            }
        }
        None => {
            ai.system_prompt = Some(LONG_TERM_MEMORY_SYSTEM_PROMPT.to_string());
        }
    }
}

fn inject_long_term_memory_tools(ai: &mut AiOptions, recall_max_types: usize) {
    let tools = ai.tools.get_or_insert_with(Vec::new);

    if !tools
        .iter()
        .any(|tool| tool.function.name == LONG_TERM_MEMORY_RECALL_TOOL_NAME)
    {
        tools.insert(0, long_term_memory_recall_tool_definition(recall_max_types));
    }

    if !tools
        .iter()
        .any(|tool| tool.function.name == LONG_TERM_MEMORY_TOOL_NAME)
    {
        tools.insert(0, long_term_memory_tool_definition());
    }
}

fn inject_long_term_memory_tool_handlers(
    ai: &mut AiOptions,
    queue: LongTermMemoryQueue,
    recall_max_types: usize,
) {
    let tools_realize = ai.tools_realize.get_or_insert_with(HashMap::new);
    tools_realize.insert(
        LONG_TERM_MEMORY_TOOL_NAME.to_string(),
        Box::new(move |args| enqueue_long_term_memory(args, Arc::clone(&queue))),
    );
    tools_realize.insert(
        LONG_TERM_MEMORY_RECALL_TOOL_NAME.to_string(),
        Box::new(move |args| recall_long_term_memory(args, recall_max_types)),
    );
}

fn long_term_memory_tool_definition() -> otherone_ai::types::Tool {
    otherone_ai::types::Tool {
        tool_type: "function".to_string(),
        function: otherone_ai::types::FunctionDefinition {
            name: LONG_TERM_MEMORY_TOOL_NAME.to_string(),
            description: "Queue reusable user information for long-term memory processing."
                .to_string(),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "storage": {
                        "type": "string",
                        "description": "Concise reusable memory statement."
                    },
                    "types": {
                        "type": "string",
                        "description": "Specific semantic category for the memory node."
                    },
                    "possible_parent_types": {
                        "type": "array",
                        "description": "Short keywords or semantic tags that may match existing parent memory categories.",
                        "items": {
                            "type": "string"
                        },
                        "minItems": 1,
                        "maxItems": 5
                    }
                },
                "required": ["storage", "types", "possible_parent_types"],
                "additionalProperties": false
            })),
        },
    }
}

fn long_term_memory_recall_tool_definition(recall_max_types: usize) -> otherone_ai::types::Tool {
    otherone_ai::types::Tool {
        tool_type: "function".to_string(),
        function: otherone_ai::types::FunctionDefinition {
            name: LONG_TERM_MEMORY_RECALL_TOOL_NAME.to_string(),
            description: "Recall relevant long-term memory facts by semantic type keywords."
                .to_string(),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "memory_types": {
                        "type": "array",
                        "description": "Compact semantic type keywords or tags for memories to retrieve.",
                        "items": {
                            "type": "string"
                        },
                        "minItems": 1,
                        "maxItems": recall_max_types
                    }
                },
                "required": ["memory_types"],
                "additionalProperties": false
            })),
        },
    }
}

fn enqueue_long_term_memory(args: serde_json::Value, queue: LongTermMemoryQueue) -> String {
    match parse_pending_long_term_memory(args) {
        Ok(pending_memory) => match queue.lock() {
            Ok(mut pending_queue) => pending_queue.push(pending_memory),
            Err(error) => warn!("failed to lock long-term memory queue: {error}"),
        },
        Err(error) => warn!("failed to parse long-term memory tool arguments: {error}"),
    }

    "done".to_string()
}

fn parse_pending_long_term_memory(
    args: serde_json::Value,
) -> Result<PendingLongTermMemory, String> {
    let mut pending: PendingLongTermMemory =
        serde_json::from_value(args).map_err(|error| error.to_string())?;

    pending.storage = pending.storage.trim().to_string();
    pending.types = pending.types.trim().to_string();
    let mut seen_parent_types = HashSet::new();
    pending.possible_parent_types = pending
        .possible_parent_types
        .into_iter()
        .map(|parent_type| parent_type.trim().to_string())
        .filter(|parent_type| !parent_type.is_empty())
        .filter(|parent_type| seen_parent_types.insert(parent_type.clone()))
        .take(5)
        .collect();

    if pending.storage.is_empty() {
        return Err("storage cannot be empty".to_string());
    }
    if pending.types.is_empty() {
        return Err("types cannot be empty".to_string());
    }
    if pending.possible_parent_types.is_empty() {
        pending.possible_parent_types.push(pending.types.clone());
    }

    Ok(pending)
}

fn recall_long_term_memory(args: serde_json::Value, recall_max_types: usize) -> String {
    match recall_long_term_memory_inner(args, recall_max_types) {
        Ok(recalled_memory) => recalled_memory,
        Err(error) => {
            warn!("failed to recall long-term memory: {error}");
            format!("Long-term memory recall failed: {error}")
        }
    }
}

fn recall_long_term_memory_inner(
    args: serde_json::Value,
    recall_max_types: usize,
) -> Result<String, String> {
    let memory_types = parse_recall_memory_types(args, recall_max_types)?;
    let tree = otherone_memory::read_memory_tree().map_err(|error| error.to_string())?;
    let branches = tree
        .recall_subtree_memories(
            &memory_types,
            &otherone_memory::MemoryRecallOptions {
                max_types: recall_max_types,
                max_matches_per_type: 3,
                max_returned_memories: 64,
            },
        )
        .map_err(|error| error.to_string())?;

    Ok(format_recalled_memories_for_agent(&branches))
}

fn parse_recall_memory_types(
    args: serde_json::Value,
    recall_max_types: usize,
) -> Result<Vec<String>, String> {
    let recall_args: LongTermMemoryRecallArgs =
        serde_json::from_value(args).map_err(|error| error.to_string())?;
    let mut seen = HashSet::new();
    let memory_types: Vec<String> = recall_args
        .memory_types
        .into_iter()
        .map(|memory_type| memory_type.trim().to_string())
        .filter(|memory_type| !memory_type.is_empty())
        .filter(|memory_type| seen.insert(memory_type.clone()))
        .take(recall_max_types)
        .collect();

    if memory_types.is_empty() {
        return Err("memory_types cannot be empty".to_string());
    }

    Ok(memory_types)
}

fn format_recalled_memories_for_agent(branches: &[otherone_memory::MemoryRecallBranch]) -> String {
    let mut seen = HashSet::new();
    let memories: Vec<String> = branches
        .iter()
        .flat_map(|branch| branch.memories.iter())
        .map(|memory| memory.trim())
        .filter(|memory| !memory.is_empty())
        .filter(|memory| seen.insert((*memory).to_string()))
        .map(|memory| memory.to_string())
        .collect();

    if memories.is_empty() {
        return "<long-term-memory>\nNo relevant long-term memory found.\n</long-term-memory>"
            .to_string();
    }

    let mut lines = vec!["<long-term-memory>".to_string()];
    for memory in memories {
        lines.push(format!("- {memory}"));
    }
    lines.push("</long-term-memory>".to_string());
    lines.join("\n")
}

fn spawn_drained_long_term_memory_processing(
    queue: &Option<LongTermMemoryQueue>,
    model_config: &Option<MemoryModelConfig>,
) {
    let pending_items = drain_long_term_memory_queue(queue);
    if pending_items.is_empty() {
        return;
    }

    let Some(model_config) = model_config.clone() else {
        return;
    };

    tokio::spawn(async move {
        let _guard = long_term_memory_processing_lock().lock().await;

        for pending_memory in pending_items {
            if let Err(error) =
                process_long_term_memory_candidate(&model_config, pending_memory).await
            {
                warn!("failed to process long-term memory candidate: {error}");
            }
        }
    });
}

fn spawn_long_term_memory_heuristic_fallback(
    model_config: &Option<MemoryModelConfig>,
    user_prompt: Option<&str>,
    already_queued_memory: bool,
) {
    if already_queued_memory {
        return;
    }

    let Some(user_prompt) = user_prompt else {
        return;
    };
    if !should_queue_long_term_memory_fallback(user_prompt) {
        return;
    }

    let Some(model_config) = model_config.clone() else {
        return;
    };
    let pending_memory = PendingLongTermMemory {
        storage: user_prompt.trim().to_string(),
        types: LONG_TERM_MEMORY_FALLBACK_TYPES.to_string(),
        possible_parent_types: fallback_possible_parent_types(user_prompt),
    };

    tokio::spawn(async move {
        let _guard = long_term_memory_processing_lock().lock().await;
        if let Err(error) = process_long_term_memory_candidate(&model_config, pending_memory).await
        {
            warn!("failed to process long-term memory fallback candidate: {error}");
        }
    });
}

fn should_queue_long_term_memory_fallback(user_prompt: &str) -> bool {
    let normalized = normalize_memory_fallback_text(user_prompt);
    if normalized.is_empty() {
        return false;
    }

    let negative_terms = [
        "临时",
        "这次",
        "当前",
        "今天",
        "刚刚",
        "一次性",
        "十分钟",
        "提醒我",
        "明天提醒",
        "验证码",
        "密码",
        "apikey",
        "api密钥",
        "不要长期",
        "不要记",
        "不用新增",
        "不用重复",
        "已经记住",
        "翻译",
        "算一下",
        "cargotest",
        "ignored",
    ];
    if negative_terms.iter().any(|term| normalized.contains(term)) {
        return false;
    }

    let positive_terms = [
        "可以长期",
        "长期",
        "以后",
        "默认",
        "偏好",
        "喜欢",
        "不喜欢",
        "过敏",
        "习惯",
        "项目里",
        "规则",
        "约定",
        "叫我",
        "我叫",
        "称呼",
        "不吃",
        "必须",
        "采用",
    ];
    positive_terms.iter().any(|term| normalized.contains(term))
}

fn fallback_possible_parent_types(user_prompt: &str) -> Vec<String> {
    let normalized = normalize_memory_fallback_text(user_prompt);

    if contains_any(&normalized, &["我叫", "叫我", "称呼"]) {
        return string_vec(&["身份信息", "称呼", "用户资料"]);
    }
    if contains_any(&normalized, &["中文", "英文", "语言", "交流", "沟通"]) {
        return string_vec(&["沟通偏好", "语言偏好", "对话设置"]);
    }
    if contains_any(
        &normalized,
        &["吃", "食物", "饮食", "菜", "辣", "香菜", "过敏"],
    ) {
        return string_vec(&["食物偏好", "饮食偏好", "忌口偏好"]);
    }
    if contains_any(
        &normalized,
        &[
            "rust",
            "代码",
            "项目",
            "规则",
            "架构",
            "memory",
            "向量数据库",
        ],
    ) {
        return string_vec(&["项目规则", "编码偏好", "技术规范"]);
    }
    if contains_any(
        &normalized,
        &["前端", "设计", "界面", "主题", "ui", "紫色", "渐变"],
    ) {
        return string_vec(&["设计偏好", "前端设计", "UI 主题"]);
    }
    if contains_any(&normalized, &["会议", "开会", "提纲", "资料"]) {
        return string_vec(&["工作习惯", "会议习惯", "沟通偏好"]);
    }
    if contains_any(&normalized, &["旅行", "景点", "路线"]) {
        return string_vec(&["旅行偏好", "生活方式偏好", "休闲偏好"]);
    }
    if contains_any(&normalized, &["跑步", "训练", "运动"]) {
        return string_vec(&["运动习惯", "健康偏好", "生活习惯"]);
    }

    string_vec(&["用户偏好", "长期约束", "用户资料"])
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn string_vec(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

fn normalize_memory_fallback_text(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn drain_long_term_memory_queue(queue: &Option<LongTermMemoryQueue>) -> Vec<PendingLongTermMemory> {
    let Some(queue) = queue else {
        return Vec::new();
    };

    match queue.lock() {
        Ok(mut pending_queue) => pending_queue.drain(..).collect(),
        Err(error) => {
            warn!("failed to drain long-term memory queue: {error}");
            Vec::new()
        }
    }
}

fn long_term_memory_processing_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

async fn process_long_term_memory_candidate(
    model_config: &MemoryModelConfig,
    pending_memory: PendingLongTermMemory,
) -> Result<(), String> {
    let mut tree = otherone_memory::read_memory_tree().map_err(|error| error.to_string())?;
    let search_options = otherone_memory::MemoryTypeSearchOptions::default();
    let neighborhoods = tree
        .find_type_neighborhoods(&pending_memory.possible_parent_types, &search_options)
        .map_err(|error| error.to_string())?;
    let existing_memory_table =
        otherone_memory::MemoryTree::format_type_neighborhoods_for_model(&neighborhoods);

    let ai_config = build_long_term_memory_model_config(
        model_config,
        build_long_term_memory_messages(&pending_memory, &existing_memory_table),
    );
    let response = otherone_ai::invoke_model(
        model_config.provider.clone(),
        &model_config.api_key,
        &model_config.base_url,
        ai_config,
    )
    .await
    .map_err(|error| error.to_string())?;

    let commit_args = extract_long_term_memory_commit_args(&response)?;
    apply_long_term_memory_commit(&mut tree, &commit_args)?;
    tree.validate().map_err(|error| error.to_string())?;
    otherone_memory::write_memory_tree(&tree).map_err(|error| error.to_string())
}

fn build_long_term_memory_messages(
    pending_memory: &PendingLongTermMemory,
    existing_memory_table: &str,
) -> Vec<otherone_ai::types::Message> {
    vec![
        otherone_ai::types::Message {
            role: "system".to_string(),
            content: otherone_ai::types::MessageContent::Text(
                LONG_TERM_MEMORY_MODEL_SYSTEM_PROMPT.to_string(),
            ),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        },
        otherone_ai::types::Message {
            role: "user".to_string(),
            content: otherone_ai::types::MessageContent::Text(format!(
                "<candidate-memory>\n{}\n</candidate-memory>\n\n{}",
                serde_json::to_string_pretty(pending_memory).unwrap_or_default(),
                existing_memory_table
            )),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        },
    ]
}

fn build_long_term_memory_model_config(
    model_config: &MemoryModelConfig,
    messages: Vec<otherone_ai::types::Message>,
) -> serde_json::Value {
    let mut config = serde_json::json!({});

    if let Some(ref other) = model_config.other {
        if let serde_json::Value::Object(ref obj) = other {
            for (key, value) in obj {
                config[key] = value.clone();
            }
        }
    }

    config["model"] = serde_json::json!(model_config.model);
    config["messages"] = serde_json::json!(messages);
    config["tools"] = serde_json::json!([long_term_memory_commit_tool_definition()]);
    config["parallelToolCalls"] = serde_json::json!(false);
    config["stream"] = serde_json::json!(false);

    if let Some(context_length) = model_config.context_length {
        config["contextLength"] = serde_json::json!(context_length);
    }
    if let Some(temperature) = model_config.temperature {
        config["temperature"] = serde_json::json!(temperature);
    }
    if let Some(top_p) = model_config.top_p {
        config["topP"] = serde_json::json!(top_p);
    }

    config
}

fn long_term_memory_commit_tool_definition() -> otherone_ai::types::Tool {
    otherone_ai::types::Tool {
        tool_type: "function".to_string(),
        function: otherone_ai::types::FunctionDefinition {
            name: LONG_TERM_MEMORY_COMMIT_TOOL_NAME.to_string(),
            description:
                "Commit one validated long-term memory write operation to the memory tree."
                    .to_string(),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": [
                            "insert_root",
                            "insert_child",
                            "update_parent_and_insert_child",
                            "update_existing",
                            "ignore"
                        ],
                        "description": "The write operation to perform."
                    },
                    "storage": {
                        "type": "string",
                        "description": "Memory statement for a new or updated node."
                    },
                    "types": {
                        "type": "string",
                        "description": "Specific semantic category for a new or updated node."
                    },
                    "parent_id": {
                        "type": "string",
                        "description": "Parent point id for insert_child or update_parent_and_insert_child."
                    },
                    "target_id": {
                        "type": "string",
                        "description": "Existing point id for update_existing."
                    },
                    "parent_storage": {
                        "type": "string",
                        "description": "Optional replacement storage for the parent before inserting."
                    },
                    "parent_types": {
                        "type": "string",
                        "description": "Optional broader replacement types for the parent before inserting."
                    }
                },
                "required": ["action"],
                "additionalProperties": false
            })),
        },
    }
}

fn extract_long_term_memory_commit_args(
    response: &otherone_ai::types::ChatResponse,
) -> Result<LongTermMemoryCommitArgs, String> {
    let tool_call = response
        .choices
        .iter()
        .filter_map(|choice| choice.message.as_ref())
        .filter_map(|message| message.tool_calls.as_ref())
        .flat_map(|tool_calls| tool_calls.iter())
        .find(|tool_call| tool_call.function.name == LONG_TERM_MEMORY_COMMIT_TOOL_NAME)
        .ok_or_else(|| {
            "long-term memory model did not call otherone_commit_long_term_memory".to_string()
        })?;

    serde_json::from_str(&tool_call.function.arguments)
        .map_err(|error| format!("invalid memory commit arguments: {error}"))
}

fn apply_long_term_memory_commit(
    tree: &mut otherone_memory::MemoryTree,
    args: &LongTermMemoryCommitArgs,
) -> Result<(), String> {
    match args.action.trim() {
        "insert_root" => {
            let storage = required_commit_field(args.storage.as_ref(), "storage")?;
            let types = required_commit_field(args.types.as_ref(), "types")?;
            tree.insert_root(storage, types)
                .map(|_| ())
                .map_err(|error| error.to_string())
        }
        "insert_child" => {
            let parent_id = required_commit_field(args.parent_id.as_ref(), "parent_id")?;
            let storage = required_commit_field(args.storage.as_ref(), "storage")?;
            let types = required_commit_field(args.types.as_ref(), "types")?;
            tree.insert_child(&parent_id, storage, types)
                .map(|_| ())
                .map_err(|error| error.to_string())
        }
        "update_parent_and_insert_child" => {
            let parent_id = required_commit_field(args.parent_id.as_ref(), "parent_id")?;
            let storage = required_commit_field(args.storage.as_ref(), "storage")?;
            let types = required_commit_field(args.types.as_ref(), "types")?;
            let old_parent = tree
                .get(&parent_id)
                .cloned()
                .ok_or_else(|| format!("parent point not found: {parent_id}"))?;
            let parent_had_children = !tree
                .child_ids(&parent_id)
                .map_err(|error| error.to_string())?
                .is_empty();
            let old_parent_storage = old_parent
                .storage
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string);
            let old_parent_types = old_parent
                .types
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string);
            let new_parent_storage = optional_commit_field(args.parent_storage.as_ref());
            let should_preserve_old_parent_as_child = !parent_had_children
                && old_parent_storage
                    .as_ref()
                    .map(|old_storage| {
                        old_storage != &storage
                            && new_parent_storage
                                .as_ref()
                                .map(|parent_storage| !parent_storage.contains(old_storage))
                                .unwrap_or(false)
                    })
                    .unwrap_or(false);

            if let Some(parent_storage) = new_parent_storage {
                tree.update_storage(&parent_id, parent_storage)
                    .map_err(|error| error.to_string())?;
            }
            if let Some(parent_types) = optional_commit_field(args.parent_types.as_ref()) {
                tree.update_types(&parent_id, parent_types)
                    .map_err(|error| error.to_string())?;
            }

            if should_preserve_old_parent_as_child {
                let old_storage = old_parent_storage.expect("checked old parent storage exists");
                let old_types =
                    old_parent_types.unwrap_or_else(|| LONG_TERM_MEMORY_FALLBACK_TYPES.to_string());
                tree.insert_child(&parent_id, old_storage, old_types)
                    .map_err(|error| error.to_string())?;
            }

            tree.insert_child(&parent_id, storage, types)
                .map(|_| ())
                .map_err(|error| error.to_string())
        }
        "update_existing" => {
            let target_id = required_commit_field(args.target_id.as_ref(), "target_id")?;
            let mut updated = false;

            if let Some(storage) = optional_commit_field(args.storage.as_ref()) {
                tree.update_storage(&target_id, storage)
                    .map_err(|error| error.to_string())?;
                updated = true;
            }
            if let Some(types) = optional_commit_field(args.types.as_ref()) {
                tree.update_types(&target_id, types)
                    .map_err(|error| error.to_string())?;
                updated = true;
            }

            if updated {
                Ok(())
            } else {
                Err("update_existing requires storage or types".to_string())
            }
        }
        "ignore" => Ok(()),
        action => Err(format!("unsupported memory commit action: {action}")),
    }
}

fn required_commit_field(value: Option<&String>, field: &str) -> Result<String, String> {
    optional_commit_field(value).ok_or_else(|| format!("{field} is required"))
}

fn optional_commit_field(value: Option<&String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_ai_config_basic() {
        let ai = AiOptions {
            provider: otherone_ai::types::ProviderType::OpenAI,
            api_key: "test-key".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4".to_string(),
            user_prompt: None,
            system_prompt: None,
            messages: None,
            context_length: Some(4096),
            temperature: Some(0.7),
            top_p: None,
            tools: None,
            tools_realize: None,
            tool_choice: None,
            parallel_tool_calls: None,
            stream: None,
            other: None,
        };

        let messages = vec![];
        let config = build_ai_config(&ai, &messages);

        assert_eq!(config["model"], "gpt-4");
        assert_eq!(config["contextLength"], 4096);
        assert!((config["temperature"].as_f64().unwrap() - 0.7).abs() < 0.001);
    }

    #[test]
    fn configure_long_term_memory_adds_prompt_tool_and_handler() {
        let mut ai = AiOptions {
            provider: otherone_ai::types::ProviderType::OpenAI,
            api_key: "test-key".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4".to_string(),
            user_prompt: None,
            system_prompt: Some("You are helpful.".to_string()),
            messages: None,
            context_length: None,
            temperature: None,
            top_p: None,
            tools: None,
            tools_realize: None,
            tool_choice: None,
            parallel_tool_calls: None,
            stream: None,
            other: None,
        };

        let queue: LongTermMemoryQueue = Arc::new(Mutex::new(Vec::new()));

        configure_long_term_memory(&mut ai, Arc::clone(&queue), 4);
        configure_long_term_memory(&mut ai, Arc::clone(&queue), 4);

        let system_prompt = ai.system_prompt.as_deref().unwrap();
        assert!(system_prompt.contains("You are helpful."));
        assert_eq!(
            system_prompt
                .matches(LONG_TERM_MEMORY_SYSTEM_PROMPT)
                .count(),
            1
        );

        let tools = ai.tools.as_ref().unwrap();
        assert_eq!(
            tools
                .iter()
                .filter(|tool| tool.function.name == LONG_TERM_MEMORY_TOOL_NAME)
                .count(),
            1
        );
        assert_eq!(
            tools
                .iter()
                .filter(|tool| tool.function.name == LONG_TERM_MEMORY_RECALL_TOOL_NAME)
                .count(),
            1
        );
        let add_tool = tools
            .iter()
            .find(|tool| tool.function.name == LONG_TERM_MEMORY_TOOL_NAME)
            .unwrap();
        assert!(add_tool
            .function
            .parameters
            .as_ref()
            .unwrap()
            .to_string()
            .contains("possible_parent_types"));
        let recall_tool = tools
            .iter()
            .find(|tool| tool.function.name == LONG_TERM_MEMORY_RECALL_TOOL_NAME)
            .unwrap();
        assert_eq!(
            recall_tool.function.parameters.as_ref().unwrap()["properties"]["memory_types"]
                ["maxItems"],
            serde_json::json!(4)
        );

        let tool_calls = vec![otherone_ai::types::ToolCall {
            index: None,
            id: "call_memory".to_string(),
            call_type: "function".to_string(),
            function: otherone_ai::types::FunctionCall {
                name: LONG_TERM_MEMORY_TOOL_NAME.to_string(),
                arguments:
                    r#"{"storage":"用户喜欢炸酱面","types":"用户喜欢吃的面食","possible_parent_types":["饮食","面食"]}"#
                        .to_string(),
            },
        }];

        let results =
            otherone_tools::process_tools(&tool_calls, ai.tools_realize.as_ref().unwrap()).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].result, Some(serde_json::json!("done")));

        let pending_queue = queue.lock().unwrap();
        assert_eq!(pending_queue.len(), 1);
        assert_eq!(pending_queue[0].storage, "用户喜欢炸酱面");
        assert_eq!(pending_queue[0].possible_parent_types, vec!["饮食", "面食"]);
    }

    #[test]
    fn parse_pending_long_term_memory_sanitizes_fields() {
        let pending = parse_pending_long_term_memory(serde_json::json!({
            "storage": " 用户喜欢炸酱面 ",
            "types": " 用户喜欢吃的面食 ",
            "possible_parent_types": [" 饮食 ", "", "面食", "面食"]
        }))
        .unwrap();

        assert_eq!(pending.storage, "用户喜欢炸酱面");
        assert_eq!(pending.types, "用户喜欢吃的面食");
        assert_eq!(pending.possible_parent_types, vec!["饮食", "面食"]);

        let fallback = parse_pending_long_term_memory(serde_json::json!({
            "storage": "用户喜欢游泳",
            "types": "运动偏好",
            "possible_parent_types": []
        }))
        .unwrap();
        assert_eq!(fallback.possible_parent_types, vec!["运动偏好"]);
    }

    #[test]
    fn recalls_long_term_memory_storage_only() {
        let root = std::env::temp_dir().join(format!(
            "otherone-agent-recall-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        otherone_memory::set_memory_storage_root(root);

        let mut tree = otherone_memory::MemoryTree::new();
        let food = tree
            .insert_root("用户喜欢吃炸酱面和糖醋排骨等食物", "用户喜欢的食物")
            .unwrap();
        tree.insert_child(&food, "用户喜欢吃糖醋排骨", "用户喜欢吃的菜")
            .unwrap();
        tree.insert_root("用户周末喜欢跑步", "运动习惯").unwrap();
        otherone_memory::write_memory_tree(&tree).unwrap();

        let result = recall_long_term_memory_inner(
            serde_json::json!({
                "memory_types": ["食物偏好", "食物偏好", "运动习惯"]
            }),
            1,
        )
        .unwrap();

        assert!(result.contains("<long-term-memory>"));
        assert!(result.contains("- 用户喜欢吃炸酱面和糖醋排骨等食物"));
        assert!(result.contains("- 用户喜欢吃糖醋排骨"));
        assert!(!result.contains("point_id"));
        assert!(!result.contains("用户周末喜欢跑步"));

        otherone_memory::clear_memory_storage_root();
    }

    #[test]
    fn recall_max_types_is_clamped() {
        assert_eq!(
            normalize_recall_max_types(None),
            DEFAULT_LONG_TERM_MEMORY_RECALL_MAX_TYPES
        );
        assert_eq!(normalize_recall_max_types(Some(0)), 1);
        assert_eq!(
            normalize_recall_max_types(Some(usize::MAX)),
            MAX_LONG_TERM_MEMORY_RECALL_MAX_TYPES
        );
    }

    #[test]
    fn applies_insert_root_memory_commit() {
        let mut tree = otherone_memory::MemoryTree::new();
        let args = LongTermMemoryCommitArgs {
            action: "insert_root".to_string(),
            storage: Some("用户喜欢炸酱面".to_string()),
            types: Some("用户喜欢吃的面食".to_string()),
            parent_id: None,
            target_id: None,
            parent_storage: None,
            parent_types: None,
        };

        apply_long_term_memory_commit(&mut tree, &args).unwrap();

        assert_eq!(tree.memory_len(), 1);
        let root = tree.get(&tree.root_ids()[0]).unwrap();
        assert_eq!(root.storage.as_deref(), Some("用户喜欢炸酱面"));
        assert_eq!(root.types.as_deref(), Some("用户喜欢吃的面食"));
    }

    #[test]
    fn applies_update_parent_and_insert_child_memory_commit() {
        let mut tree = otherone_memory::MemoryTree::new();
        let parent_id = tree
            .insert_root("用户喜欢炸酱面", "用户喜欢吃的面食")
            .unwrap();
        let args = LongTermMemoryCommitArgs {
            action: "update_parent_and_insert_child".to_string(),
            storage: Some("用户喜欢糖醋排骨".to_string()),
            types: Some("用户喜欢吃的菜".to_string()),
            parent_id: Some(parent_id.clone()),
            target_id: None,
            parent_storage: Some("用户喜欢炸酱面和糖醋排骨".to_string()),
            parent_types: Some("用户喜欢吃的食物".to_string()),
        };

        apply_long_term_memory_commit(&mut tree, &args).unwrap();

        let parent = tree.get(&parent_id).unwrap();
        assert_eq!(parent.storage.as_deref(), Some("用户喜欢炸酱面和糖醋排骨"));
        assert_eq!(parent.types.as_deref(), Some("用户喜欢吃的食物"));

        let child_ids = tree.child_ids(&parent_id).unwrap();
        assert_eq!(child_ids.len(), 1);
        let child = tree.get(&child_ids[0]).unwrap();
        assert_eq!(child.storage.as_deref(), Some("用户喜欢糖醋排骨"));
        assert_eq!(child.types.as_deref(), Some("用户喜欢吃的菜"));
    }

    #[test]
    fn update_parent_and_insert_child_preserves_old_specific_parent_storage() {
        let mut tree = otherone_memory::MemoryTree::new();
        let parent_id = tree
            .insert_root("用户喜欢吃炸酱面", "用户喜欢吃的面食")
            .unwrap();
        let args = LongTermMemoryCommitArgs {
            action: "update_parent_and_insert_child".to_string(),
            storage: Some("用户不喜欢吃香菜".to_string()),
            types: Some("用户不喜欢的食材".to_string()),
            parent_id: Some(parent_id.clone()),
            target_id: None,
            parent_storage: Some("用户有关于食物的喜好和禁忌信息".to_string()),
            parent_types: Some("用户的食物偏好与限制".to_string()),
        };

        apply_long_term_memory_commit(&mut tree, &args).unwrap();

        let parent = tree.get(&parent_id).unwrap();
        assert_eq!(
            parent.storage.as_deref(),
            Some("用户有关于食物的喜好和禁忌信息")
        );

        let child_storages = tree
            .children(&parent_id)
            .unwrap()
            .into_iter()
            .filter_map(|point| point.storage.as_deref())
            .collect::<Vec<_>>();
        assert!(child_storages.contains(&"用户喜欢吃炸酱面"));
        assert!(child_storages.contains(&"用户不喜欢吃香菜"));
    }

    #[test]
    fn applies_update_existing_memory_commit() {
        let mut tree = otherone_memory::MemoryTree::new();
        let point_id = tree.insert_root("用户喜欢面食", "饮食偏好").unwrap();
        let args = LongTermMemoryCommitArgs {
            action: "update_existing".to_string(),
            storage: Some("用户喜欢炸酱面等面食".to_string()),
            types: Some("用户喜欢吃的面食".to_string()),
            parent_id: None,
            target_id: Some(point_id.clone()),
            parent_storage: None,
            parent_types: None,
        };

        apply_long_term_memory_commit(&mut tree, &args).unwrap();

        let point = tree.get(&point_id).unwrap();
        assert_eq!(point.storage.as_deref(), Some("用户喜欢炸酱面等面食"));
        assert_eq!(point.types.as_deref(), Some("用户喜欢吃的面食"));
    }

    #[test]
    fn applies_ignore_memory_commit_without_changing_tree() {
        let mut tree = otherone_memory::MemoryTree::new();
        let before = tree.to_points();
        let args = LongTermMemoryCommitArgs {
            action: "ignore".to_string(),
            storage: Some("验证码是 123456".to_string()),
            types: Some("验证码".to_string()),
            parent_id: None,
            target_id: None,
            parent_storage: None,
            parent_types: None,
        };

        apply_long_term_memory_commit(&mut tree, &args).unwrap();

        assert_eq!(tree.memory_len(), 0);
        assert_eq!(tree.to_points().len(), before.len());
    }

    #[test]
    fn heuristic_fallback_detects_durable_memory_cues() {
        assert!(should_queue_long_term_memory_fallback(
            "我叫林澈，这是可以长期记住的称呼。"
        ));
        assert!(should_queue_long_term_memory_fallback(
            "我长期不吃辣，点餐时可以参考。"
        ));
        assert!(should_queue_long_term_memory_fallback(
            "以后和我交流默认用中文。"
        ));
    }

    #[test]
    fn heuristic_fallback_rejects_temporary_and_sensitive_cues() {
        assert!(!should_queue_long_term_memory_fallback(
            "这次当前页面的按钮先临时改成红色。"
        ));
        assert!(!should_queue_long_term_memory_fallback(
            "我的银行卡验证码是 123456，这个不要长期记住。"
        ));
        assert!(!should_queue_long_term_memory_fallback(
            "把“今天项目进展顺利”翻译成英文。"
        ));
        assert!(!should_queue_long_term_memory_fallback(
            "再次提醒，我喜欢吃炸酱面。如果已经记住就不用新增重复节点。"
        ));
    }

    #[test]
    fn merge_tool_call_delta_uses_index_for_streamed_arguments() {
        let mut calls = Vec::new();

        merge_tool_call_delta(
            &mut calls,
            &otherone_ai::types::ToolCall {
                index: Some(0),
                id: "call_1".to_string(),
                call_type: "function".to_string(),
                function: otherone_ai::types::FunctionCall {
                    name: "read_file".to_string(),
                    arguments: String::new(),
                },
            },
        );
        merge_tool_call_delta(
            &mut calls,
            &otherone_ai::types::ToolCall {
                index: Some(0),
                id: String::new(),
                call_type: "function".to_string(),
                function: otherone_ai::types::FunctionCall {
                    name: String::new(),
                    arguments: "{\"path\":\"Cargo".to_string(),
                },
            },
        );
        merge_tool_call_delta(
            &mut calls,
            &otherone_ai::types::ToolCall {
                index: Some(0),
                id: String::new(),
                call_type: "function".to_string(),
                function: otherone_ai::types::FunctionCall {
                    name: String::new(),
                    arguments: ".toml\"}".to_string(),
                },
            },
        );

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_1");
        assert_eq!(calls[0].function.name, "read_file");
        assert_eq!(calls[0].function.arguments, "{\"path\":\"Cargo.toml\"}");
    }
}
