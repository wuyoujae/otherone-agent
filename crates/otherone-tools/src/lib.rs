// 作用：工具调用模块 — 处理 AI 返回的 tool_calls 并执行对应的函数实现
// 关联：被 otherone-agent 的 invoke_agent 循环调用
// 预期结果：执行 tool 调用并返回结果数组

pub mod types;

use otherone_ai::types::ToolCall;
use std::collections::HashMap;
use types::ToolResult;

/// 处理 AI 返回的 tool 调用
/// 作用：解析 tool_calls 数组，从 tools_realize 映射中查找并执行对应的函数
/// 关联：被 agent loop 模块调用
/// 预期结果：执行 tool 调用并返回结果数组
pub fn process_tools(
    tool_calls: &[ToolCall],
    tools_realize: &HashMap<String, Box<dyn Fn(serde_json::Value) -> String + Send + Sync>>,
) -> Result<Vec<ToolResult>, String> {
    if tool_calls.is_empty() {
        return Err("tool_calls array is empty".to_string());
    }

    let mut results = Vec::new();

    for tool_call in tool_calls {
        let tool_call_id = &tool_call.id;
        let function_name = &tool_call.function.name;
        let arguments_str = &tool_call.function.arguments;

        // 查找对应的函数实现
        let function_impl = tools_realize
            .get(function_name)
            .ok_or_else(|| format!("Function '{}' not found in tools_realize", function_name))?;

        // 解析 arguments（JSON 字符串）→ 完整传给实现函数
        let args: serde_json::Value = if !arguments_str.is_empty() {
            serde_json::from_str(arguments_str)
                .map_err(|e| format!("Failed to parse arguments for '{}': {}", function_name, e))?
        } else {
            serde_json::Value::Object(serde_json::Map::new())
        };

        // 调用函数 — 传入完整的 JSON Value，由实现函数自行反序列化
        let result = function_impl(args);

        results.push(ToolResult {
            tool_call_id: tool_call_id.clone(),
            function_name: function_name.clone(),
            result: Some(serde_json::Value::String(result)),
            error: None,
        });
    }

    Ok(results)
}

/// 组合 tools 配置
/// 作用：合并并去重 tools 数组，为 process_tools 做准备
/// 关联：被 agent loop 模块在循环中调用
/// 预期结果：返回去重后的 tools 数组
pub fn combine_tools(
    tools: Option<Vec<otherone_ai::types::Tool>>,
) -> Option<Vec<otherone_ai::types::Tool>> {
    let tools = tools?;
    if tools.is_empty() {
        return Some(tools);
    }

    // 按 function.name 去重，保留第一个出现的
    let mut seen = std::collections::HashSet::new();
    let mut deduped = Vec::new();

    for tool in tools {
        if seen.insert(tool.function.name.clone()) {
            deduped.push(tool);
        }
    }

    Some(deduped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use otherone_ai::types::{FunctionCall, ToolCall};

    #[test]
    fn test_process_tools_empty_array() {
        let result = process_tools(&[], &HashMap::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_process_tools_function_not_found() {
        let tool_calls = vec![ToolCall {
            index: None,
            id: "call_1".to_string(),
            call_type: "function".to_string(),
            function: FunctionCall {
                name: "unknown_fn".to_string(),
                arguments: "{}".to_string(),
            },
        }];

        let result = process_tools(&tool_calls, &HashMap::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_process_tools_success() {
        let tool_calls = vec![ToolCall {
            index: None,
            id: "call_1".to_string(),
            call_type: "function".to_string(),
            function: FunctionCall {
                name: "get_weather".to_string(),
                arguments: r#"{"city":"Beijing"}"#.to_string(),
            },
        }];

        let mut tools_realize: HashMap<
            String,
            Box<dyn Fn(serde_json::Value) -> String + Send + Sync>,
        > = HashMap::new();
        tools_realize.insert(
            "get_weather".to_string(),
            Box::new(|args: serde_json::Value| {
                let city = args
                    .get("city")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                format!("Weather in {}: sunny", city)
            }),
        );

        let result = process_tools(&tool_calls, &tools_realize).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].tool_call_id, "call_1");
        assert_eq!(result[0].function_name, "get_weather");
        assert!(result[0].error.is_none());
    }

    #[test]
    fn test_combine_tools() {
        let tools = Some(vec![otherone_ai::types::Tool {
            tool_type: "function".to_string(),
            function: otherone_ai::types::FunctionDefinition {
                name: "test_fn".to_string(),
                description: "A test function".to_string(),
                parameters: None,
            },
        }]);
        let result = combine_tools(tools);
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 1);
    }
}
