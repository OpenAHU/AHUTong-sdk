use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct InitReq {
    /// Android 启动后把本地保存的 cookies 发过来
    #[serde(default)]
    pub cookies_json: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginReq {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct ServerInfoResp {
    pub port: u16,
    pub token: String,
}

/// 通用错误返回（也可以不用包一层，直接用 axum 的 status code）
#[derive(Debug, Serialize)]
pub struct ErrorResp {
    pub error: String,
}

/// 有些接口你现在返回 serde_json::Value，这里可以直接透传
pub type JsonValue = Value;
