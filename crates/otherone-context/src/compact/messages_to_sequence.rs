// 作用：将消息数组转换为文本序列格式
// 关联：被 compact 压缩模块调用，用于将消息转换为可发送给 LLM 的文本格式
// 预期结果：返回格式化的文本序列字符串

use otherone_ai::types::Message;

/// 将消息数组转换为文本序列
/// 作用：将 messages 数组格式化为 "[Role]: content" 的文本序列
/// 关联：被 compact_messages 调用，作为压缩 LLM 的输入
/// 预期结果：返回以双换行符分隔的文本序列
pub fn messages_to_sequence(messages: &[Message]) -> String {
    let mut sequences: Vec<String> = Vec::new();

    for message in messages {
        let content = match &message.content {
            otherone_ai::types::MessageContent::Text(text) => text.clone(),
            _ => "[非文本内容]".to_string(),
        };

        match message.role.as_str() {
            "user" => {
                sequences.push(format!("[User]: {}", content));
            }
            "assistant" => {
                let mut entry = format!("[Assistant]: {}", content);

                // 如果有工具调用，添加工具调用信息
                if let Some(ref tool_calls) = message.tool_calls {
                    if !tool_calls.is_empty() {
                        let tool_calls_info: Vec<String> = tool_calls
                            .iter()
                            .map(|tc| {
                                format!("  - {}({})", tc.function.name, tc.function.arguments)
                            })
                            .collect();
                        entry.push_str(&format!("\n[Tool Calls]:\n{}", tool_calls_info.join("\n")));
                    }
                }

                sequences.push(entry);
            }
            "tool" => {
                let tool_name = message.name.as_deref().unwrap_or("unknown_tool");
                sequences.push(format!("[Tool Result - {}]: {}", tool_name, content));
            }
            "system" => {
                sequences.push(format!("[System]: {}", content));
            }
            "developer" => {
                sequences.push(format!("[Developer]: {}", content));
            }
            other => {
                sequences.push(format!("[{}]: {}", other, content));
            }
        }
    }

    sequences.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use otherone_ai::types::{FunctionCall, MessageContent, ToolCall};

    #[test]
    fn test_messages_to_sequence_empty() {
        let result = messages_to_sequence(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_messages_to_sequence_user() {
        let messages = vec![Message {
            role: "user".to_string(),
            content: MessageContent::Text("Hello".to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }];
        let result = messages_to_sequence(&messages);
        assert_eq!(result, "[User]: Hello");
    }

    #[test]
    fn test_messages_to_sequence_assistant() {
        let messages = vec![Message {
            role: "assistant".to_string(),
            content: MessageContent::Text("Hi there!".to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }];
        let result = messages_to_sequence(&messages);
        assert_eq!(result, "[Assistant]: Hi there!");
    }

    #[test]
    fn test_messages_to_sequence_with_tool_calls() {
        let messages = vec![Message {
            role: "assistant".to_string(),
            content: MessageContent::Text("Let me check.".to_string()),
            name: None,
            tool_calls: Some(vec![ToolCall {
                index: None,
                id: "call_1".to_string(),
                call_type: "function".to_string(),
                function: FunctionCall {
                    name: "get_weather".to_string(),
                    arguments: r#"{"city":"Beijing"}"#.to_string(),
                },
            }]),
            tool_call_id: None,
        }];
        let result = messages_to_sequence(&messages);
        assert!(result.contains("[Assistant]:"));
        assert!(result.contains("[Tool Calls]:"));
        assert!(result.contains("get_weather"));
    }

    #[test]
    fn test_messages_to_sequence_multiple() {
        let messages = vec![
            Message {
                role: "user".to_string(),
                content: MessageContent::Text("Hello".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: "assistant".to_string(),
                content: MessageContent::Text("Hi!".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
        ];
        let result = messages_to_sequence(&messages);
        assert!(result.contains("\n\n"));
    }
}
