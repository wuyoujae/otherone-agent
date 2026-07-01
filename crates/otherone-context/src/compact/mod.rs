// 作用：上下文压缩子模块
// 关联：被 otherone-context 的 combine_context 调用
// 预期结果：提供 token 估算、阈值检查、消息序列化、压缩 prompts、压缩执行

pub mod check_threshold;
pub mod estimate_tokens;
pub mod messages_to_sequence;
pub mod prompts;

use crate::compact::estimate_tokens::estimate_tokens;
use crate::compact::messages_to_sequence::messages_to_sequence;
use crate::compact::prompts::{
    SUMMARIZATION_SYSTEM_PROMPT, TURN_PREFIX_SUMMARIZATION_PROMPT, UPDATE_SUMMARIZATION_PROMPT,
};
use crate::error::ContextError;
use otherone_ai::types::{Message, MessageContent, ProviderType};
use otherone_storage::types::{RuntimeContext, StorageType, WriteCompactedEntryOptions};

/// 压缩上下文消息
/// 作用：保留最新的消息，压缩旧的消息为一段摘要
/// 关联：被 combine_context 调用，当 token 使用量超过阈值时触发
/// 预期结果：返回压缩后的 messages 数组：[compacted_message, ...messages_to_keep]
pub async fn compact_messages(
    messages: &[Message],
    _context_tokens: u32,
    context_window: u32,
    compact_ratio: Option<f32>,
    ai_config: Option<&serde_json::Value>,
    has_compacted_content: bool,
    session_id: Option<&str>,
    storage_type: Option<&StorageType>,
    database_config: Option<&otherone_storage::types::DatabaseConfig>,
    original_entries: Option<&[otherone_storage::types::Entry]>,
) -> Result<Vec<Message>, ContextError> {
    compact_messages_with_context(
        messages,
        _context_tokens,
        context_window,
        compact_ratio,
        ai_config,
        has_compacted_content,
        session_id,
        storage_type,
        database_config,
        original_entries,
        None,
    )
    .await
}

pub async fn compact_messages_with_context(
    messages: &[Message],
    _context_tokens: u32,
    context_window: u32,
    compact_ratio: Option<f32>,
    ai_config: Option<&serde_json::Value>,
    has_compacted_content: bool,
    session_id: Option<&str>,
    storage_type: Option<&StorageType>,
    database_config: Option<&otherone_storage::types::DatabaseConfig>,
    original_entries: Option<&[otherone_storage::types::Entry]>,
    runtime_context: Option<&RuntimeContext>,
) -> Result<Vec<Message>, ContextError> {
    if messages.is_empty() {
        return Ok(Vec::new());
    }

    // 提取压缩 LLM 配置
    let compact_llm = extract_compact_llm_config(ai_config).ok_or_else(|| {
        ContextError::ConfigError("AI configuration is required for compaction".to_string())
    })?;

    // 设置默认保留比例为 40%
    let keep_ratio = compact_ratio.unwrap_or(0.4);
    let keep_token_threshold = (context_window as f32 * keep_ratio) as u32;

    // 从后往前查找切割点
    let mut accumulated_tokens: u32 = 0;
    let mut cutoff_index: usize = 0;

    for i in (0..messages.len()).rev() {
        let message_tokens = estimate_tokens(&[messages[i].clone()]);
        if accumulated_tokens + message_tokens <= keep_token_threshold {
            accumulated_tokens += message_tokens;
            cutoff_index = i;
        } else {
            break;
        }
    }

    // 如果所有消息都在阈值内，不需要压缩
    if cutoff_index == 0 {
        return Ok(messages.to_vec());
    }

    // 根据切割点消息的 role 类型调整切割位置
    let cutoff_message = &messages[cutoff_index];
    let adjusted_cutoff = if cutoff_message.role == "assistant" {
        // 从切割点往前查找第一条 user 消息
        let mut found = cutoff_index;
        for i in (0..cutoff_index).rev() {
            if messages[i].role == "user" {
                found = i + 1;
                break;
            }
        }
        found
    } else {
        cutoff_index
    };

    // 分割消息数组
    let messages_to_compact = &messages[..adjusted_cutoff];
    let messages_to_keep = &messages[adjusted_cutoff..];

    if messages_to_compact.is_empty() {
        return Ok(messages.to_vec());
    }

    // 调用 LLM 进行压缩
    let compressed_summary =
        call_compact_llm(messages_to_compact, &compact_llm, has_compacted_content).await?;

    // 创建压缩摘要消息
    let compacted_message = Message {
        role: "user".to_string(),
        content: MessageContent::Text(compressed_summary.clone()),
        name: None,
        tool_calls: None,
        tool_call_id: None,
    };

    // 存储压缩记录
    if let (Some(sid), Some(st), Some(entries)) = (session_id, storage_type, original_entries) {
        let trigger_entry_id = if has_compacted_content && adjusted_cutoff > 0 {
            entries
                .get(adjusted_cutoff - 1)
                .map(|e| e.entry_id.clone())
                .unwrap_or_default()
        } else {
            entries
                .get(adjusted_cutoff - 1)
                .map(|e| e.entry_id.clone())
                .unwrap_or_default()
        };

        if !trigger_entry_id.is_empty() {
            let _ = otherone_storage::write_compacted_entry(&WriteCompactedEntryOptions {
                storage_type: st.clone(),
                session_id: sid.to_string(),
                summary: compressed_summary,
                trigger_entry_id,
                create_at: None,
                database_config: database_config.cloned(),
                runtime_context: runtime_context.cloned(),
                metadata: Default::default(),
            })
            .await;
        }
    }

    // 返回压缩后的消息数组
    let mut result = vec![compacted_message];
    result.extend_from_slice(messages_to_keep);
    Ok(result)
}

/// 调用 LLM 进行上下文压缩
/// 作用：调用 AI 模块将旧消息压缩为一段摘要
/// 关联：被 compact_messages 调用
/// 预期结果：返回压缩后的摘要文本
async fn call_compact_llm(
    messages_to_compact: &[Message],
    compact_llm_config: &CompactLLMConfig,
    has_compacted_content: bool,
) -> Result<String, ContextError> {
    // 将消息转换为序列文本
    let message_sequence = messages_to_sequence(messages_to_compact);

    // 根据是否已有压缩内容选择不同的 prompt
    let user_prompt = if has_compacted_content {
        let previous_summary = extract_message_text(&messages_to_compact[0]);
        format!(
            "<previous-summary>\n{}\n</previous-summary>\n\n{}\n\n{}",
            previous_summary, UPDATE_SUMMARIZATION_PROMPT, message_sequence
        )
    } else {
        format!(
            "{}\n\n{}",
            TURN_PREFIX_SUMMARIZATION_PROMPT, message_sequence
        )
    };

    // 构建压缩请求的 messages
    let compact_messages = vec![
        Message {
            role: "system".to_string(),
            content: MessageContent::Text(SUMMARIZATION_SYSTEM_PROMPT.to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        },
        Message {
            role: "user".to_string(),
            content: MessageContent::Text(user_prompt),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    // 构建 AI 配置并调用
    let ai_config = serde_json::json!({
        "model": compact_llm_config.model,
        "messages": compact_messages,
    });

    let response = otherone_ai::invoke_model(
        compact_llm_config.provider.clone(),
        &compact_llm_config.api_key,
        &compact_llm_config.base_url,
        ai_config,
    )
    .await
    .map_err(|e| ContextError::CompactionError(e.to_string()))?;

    // 提取压缩内容
    let compressed = match compact_llm_config.provider {
        ProviderType::OpenAI
        | ProviderType::Fetch
        | ProviderType::OpenRouter
        | ProviderType::Local => response
            .choices
            .first()
            .and_then(|c| c.message.as_ref())
            .and_then(|m| m.content.as_deref())
            .unwrap_or("")
            .to_string(),
        ProviderType::Anthropic => response
            .choices
            .first()
            .and_then(|c| c.message.as_ref())
            .and_then(|m| m.content.as_deref())
            .unwrap_or("")
            .to_string(),
    };

    if compressed.is_empty() {
        return Err(ContextError::CompactionError(
            "Unable to extract content from compaction response".to_string(),
        ));
    }

    Ok(compressed)
}

/// 压缩 LLM 配置
struct CompactLLMConfig {
    provider: ProviderType,
    api_key: String,
    base_url: String,
    model: String,
}

/// 从 AI 配置中提取压缩 LLM 配置
fn extract_compact_llm_config(ai_config: Option<&serde_json::Value>) -> Option<CompactLLMConfig> {
    let config = ai_config?;
    let obj = config.as_object()?;

    let provider_str = obj
        .get("compact_llm_provider")
        .or_else(|| obj.get("provider"))
        .and_then(|v| v.as_str())
        .unwrap_or("openai");

    let provider = match provider_str {
        "anthropic" => ProviderType::Anthropic,
        "fetch" => ProviderType::Fetch,
        _ => ProviderType::OpenAI,
    };

    let api_key: String = obj
        .get("compact_llm_apiKey")
        .or_else(|| obj.get("apiKey"))
        .or_else(|| obj.get("api_key"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())?;

    let base_url: String = obj
        .get("compact_llm_baseUrl")
        .or_else(|| obj.get("baseUrl"))
        .or_else(|| obj.get("base_url"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())?;

    let model: String = obj
        .get("compact_llm_model")
        .or_else(|| obj.get("model"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())?;

    Some(CompactLLMConfig {
        provider,
        api_key,
        base_url,
        model,
    })
}

/// 提取 MessageContent 中的文本
fn extract_message_text(message: &Message) -> String {
    match &message.content {
        MessageContent::Text(t) => t.clone(),
        _ => "[非文本内容]".to_string(),
    }
}
