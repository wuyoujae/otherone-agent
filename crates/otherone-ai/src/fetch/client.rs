// 作用：通用 Fetch HTTP 客户端 — 实现 AiProvider trait
// 关联：被 otherone-ai/lib.rs 的 invoke_model 入口调用
// 预期结果：通过 reqwest 发送通用 HTTP 请求

use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client as HttpClient;
use std::time::Duration;

use crate::error::AiError;
use crate::traits::{AiProvider, ChatStream};
use crate::types::{ChatRequest, ChatResponse, ProviderType};

/// Fetch HTTP 客户端
pub struct FetchClient {
    http_client: HttpClient,
    api_key: String,
    base_url: String,
}

impl FetchClient {
    /// 创建新的 Fetch 客户端
    pub fn new(api_key: String, base_url: String) -> Self {
        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        let base_url = base_url.trim_end_matches('/').to_string();

        FetchClient {
            http_client,
            api_key,
            base_url,
        }
    }
}

#[async_trait]
impl AiProvider for FetchClient {
    /// 非流式聊天请求
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, AiError> {
        let url = format!("{}/chat/completions", self.base_url);

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AiError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let chat_response: ChatResponse = response.json().await?;
        Ok(chat_response)
    }

    /// 流式聊天请求
    async fn chat_stream(&self, request: ChatRequest) -> Result<ChatStream, AiError> {
        let url = format!("{}/chat/completions", self.base_url);
        let mut body = serde_json::to_value(&request)?;
        body["stream"] = serde_json::json!(true);

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AiError::ApiError {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let byte_stream = response.bytes_stream();
        let stream = byte_stream.flat_map(|result| {
            let items = match result {
                Ok(b) => {
                    let text = String::from_utf8_lossy(&b);
                    let mut items = Vec::new();
                    for line in text.lines() {
                        let line = line.trim();
                        if line.is_empty() || line == "data: [DONE]" {
                            continue;
                        }
                        if let Some(json_str) = line.strip_prefix("data: ") {
                            if let Ok(chunk) = serde_json::from_str::<ChatResponse>(json_str) {
                                items.push(Ok(chunk));
                            }
                        }
                    }
                    items
                }
                Err(e) => vec![Err(AiError::HttpError(e))],
            };
            futures::stream::iter(items)
        });

        Ok(Box::pin(stream))
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Fetch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_url_no_trailing_slash() {
        let client = FetchClient::new(
            "test-key".to_string(),
            "https://custom.api.com/v1/".to_string(),
        );
        assert_eq!(client.base_url, "https://custom.api.com/v1");
    }
}
