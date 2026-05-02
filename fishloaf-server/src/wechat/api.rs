//! API模块 - 消息发送和接收

use anyhow::{anyhow, Result};
use serde_json::Value;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::wechat::WechatCore;

/// 同步检查结果
#[derive(Debug)]
pub struct SyncCheckResult {
    pub retcode: String,
    pub selector: String,
}

impl WechatCore {
    /// 发送文本消息
    pub async fn send_msg(&self, text: &str, to_user_name: &str) -> Result<()> {
        let url = self.get_login_info_value("url")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .ok_or_else(|| anyhow!("未获取url"))?;

        let send_url = crate::wechat::login::url_enum::send_msg_url(&url);

        let client_msg_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let param = serde_json::json!({
            "BaseRequest": self.get_base_request(),
            "Msg": {
                "Type": 1,
                "Content": text,
                "FromUserName": self.get_user_name().unwrap_or_default(),
                "ToUserName": to_user_name,
                "LocalID": client_msg_id * 10,
                "ClientMsgId": client_msg_id * 10
            },
            "Scene": 0
        });

        let response = self.http_client()
            .post(&send_url)
            .json(&param)
            .send()
            .await?;

        let text = response.text().await?;
        let json: Value = serde_json::from_str(&text)
            .map_err(|e| anyhow!("解析发送消息响应失败: {}", e))?;

        // 检查BaseResponse.Ret
        if let Some(ret) = json.get("BaseResponse")
            .and_then(|br| br.get("Ret"))
            .and_then(|r| r.as_i64())
        {
            if ret != 0 {
                return Err(anyhow!("发送消息失败 ret={}", ret));
            }
        }

        info!("消息发送成功: {} -> {}", self.get_nickname(), to_user_name);
        Ok(())
    }

    /// 同步检查是否有新消息
    pub async fn sync_check(&self) -> Result<SyncCheckResult> {
        let sync_url = self.get_login_info_value("syncUrl")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .ok_or_else(|| anyhow!("未获取syncUrl"))?;

        let url = format!("{}{}", sync_url, crate::wechat::login::url_enum::SYNC_CHECK_URL);

        let base_request = self.get_base_request();
        let mut params = vec![
            ("r".to_string(), std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
                .to_string()),
            ("synckey".to_string(), self.get_login_info_value("synckey")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default()
                .to_string()),
            ("_".to_string(), std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
                .to_string()),
        ];

        for (key, value) in &base_request {
            params.push((key.to_lowercase(), value.as_str().unwrap_or_default().to_string()));
        }

        let response = self.http_client()
            .get(&url)
            .query(&params)
            .send()
            .await?;

        let text = response.text().await?;

        // 解析 window.synccheck={retcode:"0",selector:"2"}
        let re = regex::Regex::new(r#"window\.synccheck=\{retcode:"(\d+)",selector:"(\d+)"\}"#).unwrap();

        if let Some(caps) = re.captures(&text) {
            let retcode = caps.get(1).map(|m| m.as_str()).unwrap_or_default().to_string();
            let selector = caps.get(2).map(|m| m.as_str()).unwrap_or_default().to_string();
            return Ok(SyncCheckResult { retcode, selector });
        }

        Ok(SyncCheckResult {
            retcode: "9999".to_string(),
            selector: "9999".to_string(),
        })
    }

    /// 获取新消息
    pub async fn web_wx_sync(&self) -> Result<Value> {
        let url = self.get_login_info_value("url")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .ok_or_else(|| anyhow!("未获取url"))?;

        let skey = self.get_login_info_value("skey")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        let wxsid = self.get_login_info_value("wxsid")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        let pass_ticket = self.get_login_info_value("pass_ticket")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        let sync_url = format!(
            "{}?sid={}&skey={}&pass_ticket={}",
            crate::wechat::login::url_enum::sync_url(&url),
            wxsid,
            skey,
            pass_ticket
        );

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let param = serde_json::json!({
            "BaseRequest": self.get_base_request(),
            "SyncKey": self.get_login_info_value("SyncKey").unwrap_or(Value::Null),
            "rr": -now
        });

        let response = self.http_client()
            .post(&sync_url)
            .json(&param)
            .send()
            .await?;

        let text = response.text().await?;
        let json: Value = serde_json::from_str(&text)
            .map_err(|e| anyhow!("解析同步消息响应失败: {}", e))?;

        // 检查BaseResponse.Ret
        if let Some(ret) = json.get("BaseResponse")
            .and_then(|br| br.get("Ret"))
            .and_then(|r| r.as_i64())
        {
            if ret != 0 {
                return Ok(Value::Null);
            }
        }

        // 更新SyncKey
        if let Some(sync_key) = json.get("SyncKey") {
            self.set_login_info("SyncKey".to_string(), sync_key.clone());

            if let Some(list) = sync_key.get("List").and_then(|l| l.as_array()) {
                let synckey: String = list.iter()
                    .filter_map(|item| {
                        let key = item.get("Key")?.as_i64()?;
                        let val = item.get("Val")?.as_i64()?;
                        Some(format!("{}_{}", key, val))
                    })
                    .collect::<Vec<_>>()
                    .join("|");
                self.set_login_info("synckey".to_string(), Value::String(synckey));
            }
        }

        Ok(json)
    }

    /// 开始接收消息循环
    pub async fn start_receiving(&self, tx: mpsc::Sender<String>) -> Result<()> {
        self.set_alive(true);

        while self.is_alive() {
            // 等待7秒
            tokio::time::sleep(Duration::from_secs(7)).await;

            let result = self.sync_check().await;

            match result {
                Ok(check) => {
                    info!("sync_check: retcode={}, selector={}", check.retcode, check.selector);

                    match check.retcode.as_str() {
                        "0" => {
                            // 正常响应
                            match check.selector.as_str() {
                                "0" => {
                                    // 无新消息
                                }
                                "2" | "6" => {
                                    // 有新消息
                                    match self.web_wx_sync().await {
                                        Ok(messages) => {
                                            self.process_new_messages(&messages, &tx).await;
                                        }
                                        Err(e) => {
                                            error!("web_wx_sync failed: {}", e);
                                        }
                                    }
                                }
                                "4" | "3" => {
                                    // 联系人或群信息有变化
                                    match self.web_wx_sync().await {
                                        Ok(messages) => {
                                            self.process_new_messages(&messages, &tx).await;
                                        }
                                        Err(e) => {
                                            error!("web_wx_sync failed: {}", e);
                                        }
                                    }
                                }
                                "7" => {
                                    // 突然进入/离开聊天界面
                                    let _ = self.web_wx_sync().await;
                                }
                                _ => {}
                            }
                        }
                        "1100" | "1102" => {
                            // 退出登录
                            info!("账号已在其他设备登录");
                            self.set_alive(false);
                            break;
                        }
                        "1101" => {
                            info!("账号在其他设备登录");
                            self.set_alive(false);
                            break;
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    error!("sync_check error: {}", e);
                }
            }
        }

        Ok(())
    }

    /// 处理新消息
    async fn process_new_messages(&self, messages: &Value, tx: &mpsc::Sender<String>) {
        if let Some(add_msg_list) = messages.get("AddMsgList").and_then(|v| v.as_array()) {
            for msg in add_msg_list {
                if let Some(text) = msg.get("Text").and_then(|v| v.as_str()) {
                    let from_user = msg.get("FromUserName").and_then(|v| v.as_str()).unwrap_or_default();
                    let is_group = from_user.contains("@@");
                    let user_self = self.get_user_name().unwrap_or_default();

                    // 检查是否为自己的消息
                    if from_user == user_self {
                        // 自己发送的消息，回显
                        let to_user = msg.get("ToUserName").and_then(|v| v.as_str()).unwrap_or_default();
                        let echo_msg = serde_json::json!({
                            "text": text,
                            "userId": to_user,
                            "displayName": self.get_nickname(),
                            "echo": true
                        });
                        let _ = tx.send(echo_msg.to_string()).await;
                        continue;
                    }

                    if !is_group {
                        // 个人消息
                        let user_info = self.get_user_info_map();
                        let display_name = user_info.get(from_user)
                            .and_then(|info| info.get("NickName").or(info.get("RemarkName")))
                            .and_then(|v| v.as_str().map(|s| s.to_string()))
                            .unwrap_or_else(|| from_user.to_string());

                        let push_msg = serde_json::json!({
                            "text": text,
                            "userId": from_user,
                            "displayName": display_name,
                            "isGroupMsg": false
                        });
                        let _ = tx.send(push_msg.to_string()).await;
                    } else {
                        // 群消息
                        let group_name: String = self.get_group_list()
                            .iter()
                            .find(|g| g.get("UserName").and_then(|v| v.as_str()) == Some(&from_user))
                            .and_then(|g| g.get("NickName").and_then(|v| v.as_str()))
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| from_user.to_string());

                        let sender = msg.get("RecommendInfo")
                            .and_then(|r| r.get("UserName"))
                            .and_then(|v| v.as_str())
                            .unwrap_or_default();

                        let group_members = self.get_group_member_map();
                        let display_name = group_members.get(from_user)
                            .and_then(|members| members.as_array())
                            .and_then(|arr| arr.iter().find(|m| m.get("UserName").and_then(|v| v.as_str()) == Some(sender)))
                            .and_then(|m| m.get("DisplayName").or(m.get("NickName")))
                            .and_then(|v| v.as_str().map(|s| s.to_string()))
                            .unwrap_or_else(|| sender.to_string());

                        let push_msg = serde_json::json!({
                            "text": text,
                            "userId": from_user,
                            "groupName": group_name,
                            "displayName": display_name,
                            "isGroupMsg": true
                        });
                        let _ = tx.send(push_msg.to_string()).await;
                    }
                }
            }
        }
    }
}