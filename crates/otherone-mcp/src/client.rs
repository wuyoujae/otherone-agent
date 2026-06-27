// MCP 客户端 — 管理与单个 MCP server 的通信
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

use crate::error::McpError;
use crate::types::*;

pub struct McpClient {
    config: McpServerConfig,
    tools: Vec<McpTool>,
    request_id: AtomicU64,
    child_process: Option<Child>,
    http_client: Option<reqwest::Client>,
}

impl McpClient {
    pub fn new(config: McpServerConfig) -> Self {
        McpClient {
            config,
            tools: vec![],
            request_id: AtomicU64::new(0),
            child_process: None,
            http_client: None,
        }
    }

    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    fn build_request(&self, method: &str, params: Option<serde_json::Value>) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_id(),
            method: method.to_string(),
            params,
        }
    }

    pub async fn initialize(&mut self) -> Result<(), McpError> {
        match self.config.transport.as_str() {
            "stdio" => self.init_stdio().await,
            "sse" | "streamable_http" => self.init_http().await,
            o => Err(McpError::UnsupportedTransport(o.to_string())),
        }
    }

    async fn init_stdio(&mut self) -> Result<(), McpError> {
        let cmd_str = self
            .config
            .command
            .as_ref()
            .ok_or(McpError::ConfigError("command required".into()))?;
        let mut cmd = Command::new(cmd_str);
        if let Some(ref args) = self.config.args {
            cmd.args(args);
        }
        if let Some(ref env) = self.config.env {
            for (k, v) in env {
                cmd.env(k, v);
            }
        }
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        let mut child = cmd
            .spawn()
            .map_err(|e| McpError::ConnectionError(format!("spawn: {}", e)))?;
        let stdin = child
            .stdin
            .take()
            .ok_or(McpError::ConnectionError("stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or(McpError::ConnectionError("stdout".into()))?;
        let mut w = tokio::io::BufWriter::new(stdin);
        let mut r = BufReader::new(stdout);

        let init = serde_json::to_string(&self.build_request("initialize", Some(serde_json::json!({
            "protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"otherone-mcp","version":"0.1.0"}
        }))))? + "\n";
        w.write_all(init.as_bytes()).await?;
        w.flush().await?;
        let mut line = String::new();
        r.read_line(&mut line)
            .await
            .map_err(|e| McpError::ProtocolError(format!("read: {}", e)))?;
        let _: JsonRpcResponse = serde_json::from_str(&line)
            .map_err(|e| McpError::ProtocolError(format!("parse: {}", e)))?;

        let notif = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#.to_string() + "\n";
        w.write_all(notif.as_bytes()).await?;
        w.flush().await?;

        let tools_req = serde_json::to_string(&self.build_request("tools/list", None))? + "\n";
        w.write_all(tools_req.as_bytes()).await?;
        w.flush().await?;
        let mut line2 = String::new();
        r.read_line(&mut line2)
            .await
            .map_err(|e| McpError::ProtocolError(format!("read: {}", e)))?;
        let resp: JsonRpcResponse = serde_json::from_str(&line2)
            .map_err(|e| McpError::ProtocolError(format!("parse: {}", e)))?;
        if let Some(result) = resp.result {
            let tl: ToolsListResult = serde_json::from_value(result)
                .map_err(|e| McpError::ProtocolError(format!("tools: {}", e)))?;
            self.tools = tl.tools;
        }
        self.child_process = Some(child);
        Ok(())
    }

    async fn init_http(&mut self) -> Result<(), McpError> {
        let url = self
            .config
            .url
            .clone()
            .ok_or(McpError::ConfigError("url required".into()))?;
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        if let Some(ref key) = self.config.api_key {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", key))
                    .map_err(|e| McpError::ConfigError(format!("key: {}", e)))?,
            );
        }
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| McpError::ConnectionError(format!("client: {}", e)))?;

        let init = self.build_request("initialize", Some(serde_json::json!({
            "protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"otherone-mcp","version":"0.1.0"}
        })));
        let _: JsonRpcResponse = client
            .post(&url)
            .json(&init)
            .send()
            .await
            .map_err(|e| McpError::ConnectionError(format!("init: {}", e)))?
            .json()
            .await
            .map_err(|e| McpError::ProtocolError(format!("parse: {}", e)))?;

        let tools_req = self.build_request("tools/list", None);
        let resp: JsonRpcResponse = client
            .post(&url)
            .json(&tools_req)
            .send()
            .await
            .map_err(|e| McpError::ConnectionError(format!("tools: {}", e)))?
            .json()
            .await
            .map_err(|e| McpError::ProtocolError(format!("parse: {}", e)))?;
        if let Some(result) = resp.result {
            let tl: ToolsListResult = serde_json::from_value(result)
                .map_err(|e| McpError::ProtocolError(format!("tools: {}", e)))?;
            self.tools = tl.tools;
        }
        self.http_client = Some(client);
        Ok(())
    }

    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.iter().any(|t| t.name == name)
    }

    pub async fn call_tool(
        &self,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        let params = ToolCallParams {
            name: name.to_string(),
            arguments: arguments.clone(),
        };
        let req = self.build_request("tools/call", Some(serde_json::to_value(&params)?));
        if let Some(ref client) = self.http_client {
            let url = self
                .config
                .url
                .as_ref()
                .ok_or(McpError::ConfigError("url required".into()))?;
            let resp: JsonRpcResponse = client
                .post(url)
                .json(&req)
                .send()
                .await
                .map_err(|e| McpError::ConnectionError(format!("call: {}", e)))?
                .json()
                .await
                .map_err(|e| McpError::ProtocolError(format!("parse: {}", e)))?;
            if let Some(err) = resp.error {
                return Err(McpError::ToolCallError(format!(
                    "[{}]: {}",
                    err.code, err.message
                )));
            }
            Ok(resp.result.unwrap_or(serde_json::Value::Null))
        } else {
            Err(McpError::ToolCallError(
                "stdio not supported for subsequent calls".into(),
            ))
        }
    }

    pub fn get_tools(&self) -> Vec<otherone_ai::types::Tool> {
        self.tools
            .iter()
            .map(|t| otherone_ai::types::Tool {
                tool_type: "function".to_string(),
                function: otherone_ai::types::FunctionDefinition {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    parameters: Some(t.input_schema.clone()),
                },
            })
            .collect()
    }

    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }

    pub async fn shutdown(&mut self) -> Result<(), McpError> {
        if let Some(ref mut c) = self.child_process {
            let _ = c.kill().await;
        }
        self.child_process = None;
        self.http_client = None;
        Ok(())
    }
}
