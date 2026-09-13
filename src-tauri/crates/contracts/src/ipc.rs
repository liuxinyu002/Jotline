//! stdio JSON-RPC 2.0 域（主进程 ↔ sidecar；ADR-2 零监听、stdio 驱动）。
//!
//! 信封结构冻结；方法注册表见 contracts/PROTOCOL.md（Phase-1 仅定稿 health.ping，
//! capture.structured / memory.extract 等随 Phase-5/10 增补）。
//! 约束：`id` 统一为 string（JSON-RPC 2.0 允许形态之一，两端自约束）。

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// JSON-RPC 2.0 协议版本字面量。
pub const JSONRPC_VERSION: &str = "2.0";

/// health.ping 方法名。
pub const METHOD_HEALTH_PING: &str = "health.ping";

/// JSON-RPC 请求（主进程 → sidecar）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct IpcRequest {
    /// 固定 `"2.0"`
    pub jsonrpc: String,
    pub id: String,
    /// 方法名（注册表登记）
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Object)]
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC 响应（sidecar → 主进程；`result` 与 `error` 恰居其一）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct IpcResponse {
    pub jsonrpc: String,
    /// 与请求同 id（ping-pong 往返锚点）
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Object)]
    pub result: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<IpcError>,
}

/// JSON-RPC 通知（无 id，无需响应）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct IpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Object)]
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC 错误对象。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct IpcError {
    pub code: i32,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Object)]
    pub data: Option<serde_json::Value>,
}

/// 健康检查请求（method 固定 `health.ping`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct IpcHealthRequest {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
}

impl IpcHealthRequest {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.into(),
            id: id.into(),
            method: METHOD_HEALTH_PING.into(),
        }
    }
}

/// 健康检查结果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct IpcHealthResult {
    /// `"ok"`（异常形态随 Phase-8 生命周期设计增补）
    pub status: String,
}

/// 健康检查响应（同一 id 的成功响应 = JSON-RPC 2.0 信封）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct IpcHealthResponse {
    pub jsonrpc: String,
    pub id: String,
    pub result: IpcHealthResult,
}

impl IpcHealthResponse {
    pub fn pong(request: &IpcHealthRequest) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.into(),
            id: request.id.clone(),
            result: IpcHealthResult { status: "ok".into() },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_ping_pong_same_id() {
        let req = IpcHealthRequest::new("ping-1");
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "health.ping");
        let resp = IpcHealthResponse::pong(&req);
        assert_eq!(resp.id, "ping-1");
        assert_eq!(resp.result.status, "ok");
        // wire 形态
        let s = serde_json::to_string(&resp).unwrap();
        assert_eq!(
            s,
            "{\"jsonrpc\":\"2.0\",\"id\":\"ping-1\",\"result\":{\"status\":\"ok\"}}"
        );
        let back: IpcHealthResponse = serde_json::from_str(&s).unwrap();
        assert_eq!(back, resp);
    }

    #[test]
    fn request_params_omitted_when_none() {
        let r = IpcRequest {
            jsonrpc: "2.0".into(),
            id: "1".into(),
            method: "health.ping".into(),
            params: None,
        };
        let s = serde_json::to_string(&r).unwrap();
        assert!(!s.contains("params"), "{s}");
    }
}
