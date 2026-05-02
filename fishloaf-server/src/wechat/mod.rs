//! 微信模块

pub mod core;
pub mod login;
pub mod api;

pub use core::WechatCore;

use anyhow::Result;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::models::ContactUserInfo;

/// 微信客户端
pub struct WechatClient {
    core: Arc<WechatCore>,
    tx_message: mpsc::Sender<String>,
}

impl WechatClient {
    /// 创建新的微信客户端实例
    pub fn new(tx_message: mpsc::Sender<String>) -> Self {
        // 创建共享的cookie jar（两个client共享cookie，与Java同一个HttpClient行为一致）
        let cookie_jar = Arc::new(reqwest::cookie::Jar::default());
        let user_agent = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_11_6) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/54.0.2840.71 Safari/537.36";

        // 允许重定向的client（用于登录轮询，对应Java doGet(redirect=true)）
        let http_client = Client::builder()
            .cookie_provider(Arc::clone(&cookie_jar))
            .user_agent(user_agent)
            .build()
            .expect("Failed to create HTTP client");

        // 禁用重定向的client（用于processLoginInfo，对应Java doGet(redirect=false)）
        let no_redirect_client = Client::builder()
            .cookie_provider(cookie_jar)
            .user_agent(user_agent)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Failed to create no-redirect HTTP client");

        let core = Arc::new(WechatCore::new(http_client, no_redirect_client));

        WechatClient { core, tx_message }
    }

    /// 启动登录流程
    pub async fn login(&self, qr_path: &str) -> Result<()> {
        info!("开始微信登录流程...");

        // 1. 获取UUID
        let uuid = self.core.fetch_uuid().await?;
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
    #[allow(dead_code)]
    pub fn get_nickname(&self) -> String {
        self.core.get_nickname()
    }
}