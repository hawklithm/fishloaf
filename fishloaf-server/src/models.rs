//! 数据结构定义

use serde::{Deserialize, Serialize};

/// 基础请求结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseRequest {
    pub method: String,
    pub trace_id: String,
}

/// 发送消息请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSendRequest {
    #[serde(flatten)]
    pub base: BaseRequest,
    pub message: String,
    pub target_id: String,
}

/// 操作结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult<T> {
    pub data: Option<T>,
    pub success: bool,
    pub message: Option<String>,
    #[serde(rename = "traceId")]
    pub trace_id: String,
    pub method: String,
}

impl<T> ActionResult<T> {
    pub fn create_success(data: T, trace_id: impl Into<String>, method: impl Into<String>) -> Self {
        ActionResult {
            data: Some(data),
            success: true,
            message: None,
            trace_id: trace_id.into(),
            method: method.into(),
        }
    }

    pub fn create_error(message: impl Into<String>, trace_id: impl Into<String>, method: impl Into<String>) -> Self {
        ActionResult {
            data: None,
            success: false,
            message: Some(message.into()),
            trace_id: trace_id.into(),
            method: method.into(),
        }
    }
}

/// 联系人信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactUserInfo {
    #[serde(rename = "uniqueId")]
    pub unique_id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(default)]
    pub is_group: bool,
}

/// 推送消息 - 个人消息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct PushMessagePersonal {
    pub text: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "isGroupMsg", default)]
    pub is_group_msg: bool,
}

/// 推送消息 - 群消息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum PushMessage {
    Personal(PushMessagePersonal),
    Group(PushMessageGroup),
    Echo(PushMessageEcho),
}

/// 推送消息 - 群消息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct PushMessageGroup {
    pub text: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "groupName")]
    pub group_name: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "isGroupMsg", default)]
    pub is_group_msg: bool,
}

/// 推送消息 - 自己发送的回显
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct PushMessageEcho {
    pub text: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(default)]
    pub echo: bool,
}