//! 管理员会话建立、状态查询与短期令牌的跨端契约。
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
#[serde(deny_unknown_fields)]
pub struct AdminSessionRequest {
    pub token: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct AdminSessionStatus {
    pub enabled: bool,
    pub authenticated: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, TS)]
pub struct AdminSessionToken {
    pub token: String,
    pub expires_in_seconds: u32,
}
