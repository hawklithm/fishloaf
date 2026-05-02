//! 微信协议客户端实现
//! 参考itchat4j的Java代码实现

mod api;
mod core;
mod login;

pub use core::WechatCore;

use anyhow::Result;
use async_trait::async_trait;
use parking_lot::RwLock;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::models::ContactUserInfo;

/// 消息回调接口
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// 处理文本消息
    async fn handle_text(&self, msg: &Value);
    /// 处理图片消息（暂未实现）
    async fn handle_pic(&self, _msg: &Value) {}
    /// 处理语音消息（暂未实现）
    async fn handle_voice(&self, _msg: &Value) {}
    /// 处理视频消息（暂未实现）
    async fn handle_video(&self, _msg: &Value) {}
    /// 处理名片消息（暂未实现）
    async fn handle_namecard(&self, _msg: &Value) {}
    /// 处理系统消息（暂未实现）
    async fn handle_sys(&self, _msg: &Value) {}
    /// 处理验证添加好友消息（暂未实现）
    async fn handle_verify(&self, _msg: &Value) {}
    /// 处理多媒体消息（暂未实现）
    async fn handle_media(&self, _msg: &Value) {}
}

/// 微信客户端
pub struct WechatClient {
    core: Arc<WechatCore>,
    tx_message: mpsc::Sender<String>,
}

impl WechatClient {
    /// 创建新的微信客户端实例
    pub fn new(tx_message: mpsc::Sender<String>) -> Self {
        let http_client = Client::builder()
            .cookie_store(true)
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .expect("Failed to create HTTP client");

        let core = Arc::new(WechatCore::new(http_client));

        WechatClient { core, tx_message }
    }

    /// 启动登录流程
    pub async fn login(&self, qr_path: &str) -> Result<()> {
        info!("开始微信登录流程...");

        // 1. 获取UUID
        let uuid = self.core.get_uuid().await?;
        info!("获取到UUID: {}", uuid);

        // 2. 生成二维码图片
        self.core.get_qr(qr_path, &uuid).await?;
        info!("二维码图片已保存到: {}", qr_path);

        // 3. 等待用户扫描并确认
        self.core.wait_for_login().await?;
        info!("登录成功！");

        // 4. 微信初始化
        self.core.web_wx_init().await?;
        info!("微信初始化完成");

        // 5. 状态通知
        self.core.wx_status_notify().await?;
        info!("状态通知已发送");

        // 6. 获取联系人
        self.core.web_wx_get_contact().await?;
        info!("联系人列表获取完成，共 {} 个联系人", self.core.get_contact_count());

        // 7. 获取群成员
        self.core.web_wx_batch_get_contact().await?;
        info!("群成员列表获取完成");

        // 8. 开启消息接收循环
        let core_clone = self.core.clone();
        let tx = self.tx_message.clone();
        tokio::spawn(async move {
            if let Err(e) = core_clone.start_receiving(tx).await {
                error!("消息接收出错: {}", e);
            }
        });

        Ok(())
    }

    /// 获取联系人列表
    pub async fn list_users_and_groups(&self) -> Result<Vec<ContactUserInfo>> {
        let mut contacts = Vec::new();

        // 获取联系人（好友）
        let contact_list = self.core.get_contact_list();
        for user in contact_list {
            let unique_id = user.get("UserName").and_then(|v| v.as_str()).unwrap_or("");
            let nick_name = user.get("NickName").and_then(|v| v.as_str()).unwrap_or("");
            let remark_name = user.get("RemarkName").and_then(|v| v.as_str()).unwrap_or("");

            contacts.push(ContactUserInfo {
                unique_id: unique_id.to_string(),
                display_name: if remark_name.is_empty() {
                    nick_name.to_string()
                } else {
                    remark_name.to_string()
                },
                is_group: false,
            });
        }

        // 获取群列表
        let group_list = self.core.get_group_list();
        for group in group_list {
            let unique_id = group.get("UserName").and_then(|v| v.as_str()).unwrap_or("");
            let nick_name = group.get("NickName").and_then(|v| v.as_str()).unwrap_or("");

            contacts.push(ContactUserInfo {
                unique_id: unique_id.to_string(),
                display_name: nick_name.to_string(),
                is_group: true,
            });
        }

        Ok(contacts)
    }

    /// 发送消息
    pub async fn send_message(&self, message: &str, target_id: &str) -> Result<()> {
        self.core.send_msg(message, target_id).await
    }

    /// 获取当前用户昵称
    pub fn get_nickname(&self) -> String {
        self.core.get_nickname()
    }
}