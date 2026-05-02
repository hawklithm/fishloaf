//! 登录模块 - 实现微信网页版登录流程
//! 参考itchat4j的LoginController和LoginServiceImpl

use anyhow::{anyhow, Result};
use serde_json::Value;
use std::time::Duration;
use tracing::{info, warn};

use crate::wechat::WechatCore;

/// 微信API端点枚举
pub(crate) mod url_enum {
    pub const UUID_URL: &str = "https://login.weixin.qq.com/jslogin";
    pub const QRCODE_URL: &str = "https://login.weixin.qq.com/qrcode/";
    pub const LOGIN_URL: &str = "https://login.weixin.qq.com/cgi-bin/mmwebwx-bin/login";

    pub fn init_url(url: &str, r: i64, pass_ticket: &str) -> String {
        format!("{}/webwxinit?r={}&pass_ticket={}", url, r, pass_ticket)
    }
    pub fn status_notify_url(url: &str, pass_ticket: &str) -> String {
        format!("{}/webwxstatusnotify?lang=zh_CN&pass_ticket={}", url, pass_ticket)
    }
    pub fn get_contact_url(url: &str) -> String {
        format!("{}/webwxgetcontact", url)
    }
    pub fn batch_get_contact_url(url: &str) -> String {
        format!("{}/webwxbatchgetcontact?type=ex&lang=zh_CN", url)
    }

    pub const SYNC_CHECK_URL: &str = "/synccheck";
    pub fn sync_url(url: &str) -> String {
        format!("{}/webwxsync", url)
    }
    pub fn send_msg_url(url: &str) -> String {
        format!("{}/webwxsendmsg", url)
    }
    #[allow(dead_code)]
    pub fn sync_check_url(url: &str) -> String {
        format!("{}/synccheck", url)
    }
}

impl WechatCore {
    /// 获取UUID
    pub async fn fetch_uuid(&self) -> Result<String> {
        let url = format!(
            "{}?appid=wx782c26e4c19acffb&fun=new&lang=zh_CN&_{}",
            url_enum::UUID_URL,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        info!("Fetching UUID from: {}", url);

        let response = self.http_client().get(&url).send().await?;
        let text = response.text().await?;

        info!("UUID response: {}", text);

        // 解析 window.QRLogin.code = xxx; window.QRLogin.uuid = "xxx";
        let re = regex::Regex::new(r#"window\.QRLogin\.code = (\d+); window\.QRLogin\.uuid = "([^"]+)""#).unwrap();

        if let Some(caps) = re.captures(&text) {
            let code = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
            let uuid = caps.get(2).map(|m| m.as_str()).unwrap_or_default();

            if code == "200" {
                self.set_uuid(uuid.to_string());
                return Ok(uuid.to_string());
            }
        }

        Err(anyhow!("获取UUID失败: {}", text))
    }

    /// 获取并保存二维码图片
    pub async fn get_qr(&self, qr_path: &str, uuid: &str) -> Result<()> {
        let url = format!("{}{}", url_enum::QRCODE_URL, uuid);

        let response = self.http_client().get(&url).send().await?;
        let bytes = response.bytes().await?;

        // 保存二维码图片
        let qr_file = std::path::Path::new(qr_path).join("QR.jpg");
        std::fs::create_dir_all(qr_path)?;
        std::fs::write(&qr_file, &bytes).map_err(|e| anyhow!("保存二维码失败: {}", e))?;

        println!("\n二维码已生成，请扫描 login/QR.jpg 图片登录");
        println!("提示: 图片路径为 {}\n", qr_file.display());

        Ok(())
    }

    /// 等待登录确认
    pub async fn wait_for_login(&self) -> Result<()> {
        let uuid = self.get_uuid().ok_or_else(|| anyhow!("未获取UUID"))?;
        info!("开始等待登录，UUID: {}", uuid);

        // 使用WechatCore的http_client（它会在整个登录过程中积累cookies）
        // reqwest默认允许重定向，与Java的HttpClients.createDefault()行为一致
        let client = self.http_client();

        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;

            let url = format!(
                "{}?loginicon=true&uuid={}&tip=1&r={}&_{}",
                url_enum::LOGIN_URL,
                uuid,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() / 1579,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
            );

            let response = client.get(&url).send().await?;
            let text = response.text().await?;

            // 检查是否已经是XML响应（自动重定向后）
            if text.starts_with("<?xml") || text.starts_with("<error>") || text.starts_with("<xml>") || text.contains("<ret>") {
                info!("Got XML response after redirect");
                if let Some(msg) = self.parse_login_message(&text) {
                    if !msg.is_empty() {
                        return Err(anyhow!("登录被禁止: {}", msg));
                    }
                }
                self.parse_auth_response(&text).await?;
                self.set_alive(true);
                return Ok(());
            }

            // 否则解析JavaScript响应
            let re = regex::Regex::new(r"window\.code=(\d+)").unwrap();

            if let Some(caps) = re.captures(&text) {
                let code = caps.get(1).map(|m| m.as_str()).unwrap_or_default();

                match code {
                    "200" => {
                        info!("Login code 200, response: {}", text);
                        self.process_login_redirect(&text).await?;
                        self.set_alive(true);
                        info!("wait_for_login 正常退出");
                        return Ok(());
                    }
                    "201" => {
                        info!("请在手机上确认登录");
                    }
                    "408" => {
                        warn!("登录超时，请重新扫描二维码");
                    }
                    _ => {
                        info!("登录状态: {}", code);
                    }
                }
            }
        }
    }

    /// 处理登录跳转
    async fn process_login_redirect(&self, text: &str) -> Result<()> {
        let re = regex::Regex::new(r#"window\.redirect_uri="([^"]+)""#).unwrap();

        if let Some(caps) = re.captures(text) {
            let redirect_uri = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
            let base_url = redirect_uri[..redirect_uri.rfind('/').unwrap_or(redirect_uri.len())].to_string();

            // 保存基础URL
            self.set_login_info("url".to_string(), Value::String(base_url.clone()));

            // 可能的服务器映射
            let possible_urls: Vec<(&str, (&str, &str))> = vec![
                ("wx.qq.com", ("file.wx.qq.com", "webpush.wx.qq.com")),
                ("wx2.qq.com", ("file.wx2.qq.com", "webpush.wx2.qq.com")),
                ("wx8.qq.com", ("file.wx8.qq.com", "webpush.wx8.qq.com")),
                ("web2.wechat.com", ("file.web2.wechat.com", "webpush.web2.wechat.com")),
                ("wechat.com", ("file.web.wechat.com", "webpush.web.wechat.com")),
            ];

            for (prefix, (file_url, sync_url)) in possible_urls {
                if base_url.contains(prefix) {
                    self.set_login_info("fileUrl".to_string(), Value::String(format!("https://{}/cgi-bin/mmwebwx-bin", file_url)));
                    self.set_login_info("syncUrl".to_string(), Value::String(format!("https://{}/cgi-bin/mmwebwx-bin", sync_url)));
                    self.set_login_info("indexUrl".to_string(), Value::String(prefix.to_string()));
                    break;
                }
            }

            // 默认使用基础URL
            if self.get_login_info_value("fileUrl").is_none() {
                self.set_login_info("fileUrl".to_string(), Value::String(base_url.clone()));
                self.set_login_info("syncUrl".to_string(), Value::String(base_url));
            }

            // 生成设备ID
            let device_id = format!("e{}", rand::random::<u64>().to_string()[..15].to_string());
            self.set_login_info("deviceid".to_string(), Value::String(device_id));

            // 访问完整跳转URL获取认证信息 - 完全按照Java格式
            let full_url = format!("{}&fun=new&version=v2&mod=desktop&lang=zh_CN", redirect_uri);

            info!("Visiting redirect URL: {}", full_url);

            // Java第493行用 redirect=false + headers
            // 使用禁用重定向的client
            let client = self.no_redirect_client();

            // 构建请求（先不发送）
            let request_builder = client
                .get(&full_url)
                .header("client-version", "2.0.0")
                .header("extspam", "Go8FCIkFEokFCggwMDAwMDAwMRAGGvAESySibk50w5Wb3uTl2c2h64jVVrV7gNs06GFlWplHQbY/5FfiO++1yH4ykCyNPWKXmco+wfQzK5R98D3so7rJ5LmGFvBLjGceleySrc3SOf2Pc1gVehzJgODeS0lDL3/I/0S2SSE98YgKleq6Uqx6ndTy9yaL9qFxJL7eiA/R3SEfTaW1SBoSITIu+EEkXff+Pv8NHOk7N57rcGk1w0ZzRrQDkXTOXFN2iHYIzAAZPIOY45Lsh+A4slpgnDiaOvRtlQYCt97nmPLuTipOJ8Qc5pM7ZsOsAPPrCQL7nK0I7aPrFDF0q4ziUUKettzW8MrAaiVfmbD1/VkmLNVqqZVvBCtRblXb5FHmtS8FxnqCzYP4WFvz3T0TcrOqwLX1M/DQvcHaGGw0B0y4bZMs7lVScGBFxMj3vbFi2SRKbKhaitxHfYHAOAa0X7/MSS0RNAjdwoyGHeOepXOKY+h3iHeqCvgOH6LOifdHf/1aaZNwSkGotYnYScW8Yx63LnSwba7+hESrtPa/huRmB9KWvMCKbDThL/nne14hnL277EDCSocPu3rOSYjuB9gKSOdVmWsj9Dxb/iZIe+S6AiG29Esm+/eUacSba0k8wn5HhHg9d4tIcixrxveflc8vi2/wNQGVFNsGO6tB5WF0xf/plngOvQ1/ivGV/C1Qpdhzznh0ExAVJ6dwzNg7qIEBaw+BzTJTUuRcPk92Sn6QDn2Pu3mpONaEumacjW4w6ipPnPw+g2TfywJjeEcpSZaP4Q3YV5HG8D6UjWA4GSkBKculWpdCMadx0usMomsSS/74QgpYqcPkmamB4nVv1JxczYITIqItIKjD35IGKAUwAA==");

            // 调试：打印请求头和URL
            let request = request_builder.try_clone().unwrap().build().unwrap();
            info!("Request URL: {}", request.url());
            info!("Request headers: {:?}", request.headers());

            let response = request_builder.send().await?;

            let status = response.status();
            info!("Redirect response status: {}", status);

            let xml_text = response.text().await?;
            info!("Redirect response body: {}", xml_text);

            // 检查登录是否被禁止（message标签内容不为空）
            if let Some(msg) = self.parse_login_message(&xml_text) {
                if !msg.is_empty() {
                    return Err(anyhow!("登录被禁止: {}", msg));
                }
            }

            // 解析XML响应获取skey等
            self.parse_auth_response(&xml_text).await?;

            return Ok(());
        }

        Err(anyhow!("未找到redirect_uri"))
    }

    /// 解析登录返回的消息，如果成功登录则message为空
    fn parse_login_message(&self, xml: &str) -> Option<String> {
        let re = regex::Regex::new(r"<message>([^<]*)</message>").ok()?;
        re.captures(xml).map(|c| c.get(1).map(|m| m.as_str().to_string()).unwrap_or_default())
    }

    /// 解析认证响应XML
    async fn parse_auth_response(&self, xml: &str) -> Result<()> {
        // 简单的XML解析
        fn extract_value(xml: &str, tag: &str) -> Option<String> {
            let pattern = format!("<{}[^>]*>([^<]+)</{}>", tag, tag);
            let re = regex::Regex::new(&pattern).ok()?;
            re.captures(xml).map(|c| c.get(1).unwrap().as_str().to_string())
        }

        if let Some(skey) = extract_value(xml, "skey") {
            self.set_login_info("skey".to_string(), Value::String(skey));
        }
        if let Some(wxsid) = extract_value(xml, "wxsid") {
            self.set_login_info("wxsid".to_string(), Value::String(wxsid));
        }
        if let Some(wxuin) = extract_value(xml, "wxuin") {
            self.set_login_info("wxuin".to_string(), Value::String(wxuin));
        }
        if let Some(pass_ticket) = extract_value(xml, "pass_ticket") {
            self.set_login_info("pass_ticket".to_string(), Value::String(pass_ticket));
        }

        Ok(())
    }

    /// 微信初始化
    pub async fn web_wx_init(&self) -> Result<()> {
        let url = match self.get_login_info_value("url") {
            Some(v) => v.as_str().map(|s| s.to_string()).ok_or_else(|| anyhow!("未获取url"))?,
            None => return Err(anyhow!("未获取url")),
        };

        let pass_ticket = self.get_login_info_value("pass_ticket")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        let r = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64 / 3158;

        let init_url = url_enum::init_url(&url, r, &pass_ticket);

        let param = self.get_base_request_param();

        let response = self.http_client()
            .post(&init_url)
            .json(&param)
            .send()
            .await?;

        let text = response.text().await?;
        let json: Value = serde_json::from_str(&text)
            .map_err(|e| anyhow!("解析初始化响应失败: {} - {}", e, text))?;

        // 检查BaseResponse
        if let Some(ret) = json.get("BaseResponse")
            .and_then(|br| br.get("Ret"))
            .and_then(|r| r.as_i64())
        {
            if ret != 0 {
                return Err(anyhow!("初始化失败 ret={}", ret));
            }
        }

        // 保存用户信息
        if let Some(user) = json.get("User") {
            let user_name = user.get("UserName")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default();
            let nick_name = user.get("NickName")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default();
            self.set_user_name(user_name);
            self.set_nickname(nick_name);
            self.set_user_self(user.clone());
        }

        // 保存SyncKey
        if let Some(sync_key) = json.get("SyncKey") {
            self.set_login_info("SyncKey".to_string(), sync_key.clone());

            // 转换为字符串格式
            let sync_list = sync_key.get("List").and_then(|l| l.as_array());
            if let Some(list) = sync_list {
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

        // 获取聊天列表中的群ID
        if let Some(chat_set) = json.get("ChatSet").and_then(|v| v.as_str().map(|s| s.to_string())) {
            for group_id in chat_set.split(',') {
                if group_id.contains("@@") {
                    self.add_group_id(group_id.to_string());
                }
            }
        }

        Ok(())
    }

    /// 状态通知
    pub async fn wx_status_notify(&self) -> Result<()> {
        let url = self.get_login_info_value("url")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .ok_or_else(|| anyhow!("未获取url"))?;

        let pass_ticket = self.get_login_info_value("pass_ticket")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        let notify_url = url_enum::status_notify_url(&url, &pass_ticket);

        let mut param = self.get_base_request_param();
        param.insert("Code".to_string(), Value::Number(1.into()));
        param.insert("FromUserName".to_string(), Value::String(self.get_user_name().unwrap_or_default()));
        param.insert("ToUserName".to_string(), Value::String(self.get_user_name().unwrap_or_default()));
        param.insert("ClientMsgId".to_string(), Value::Number(
            serde_json::Number::from(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64)
        ));

        let _response = self.http_client()
            .post(&notify_url)
            .json(&param)
            .send()
            .await?;

        Ok(())
    }

    /// 获取联系人列表
    pub async fn web_wx_get_contact(&self) -> Result<()> {
        let url = self.get_login_info_value("url")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("未获取url"))?;

        let contact_url = url_enum::get_contact_url(&url);

        let param = self.get_base_request_param();

        let response = self.http_client()
            .post(&contact_url)
            .json(&param)
            .send()
            .await?;

        let text = response.text().await?;
        let json: Value = serde_json::from_str(&text)
            .map_err(|e| anyhow!("解析联系人响应失败: {}", e))?;

        let member_count = json.get("MemberCount").and_then(|v| v.as_i64()).unwrap_or(0);
        let mut members = json.get("MemberList").and_then(|v| v.as_array())
            .map(|arr| arr.clone())
            .unwrap_or_default();

        // 处理分页获取
        let mut seq = json.get("Seq").and_then(|v| v.as_i64()).unwrap_or(0);
        while seq > 0 {
            let page_url = format!("{}?r={}&seq={}", contact_url,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis(),
                seq
            );

            let response = self.http_client()
                .get(&page_url)
                .send()
                .await?;

            let text = response.text().await?;
            let page_json: Value = serde_json::from_str(&text)
                .map_err(|e| anyhow!("解析分页响应失败: {}", e))?;

            seq = page_json.get("Seq").and_then(|v| v.as_i64()).unwrap_or(0);

            if let Some(new_members) = page_json.get("MemberList").and_then(|v| v.as_array()) {
                members.extend(new_members.clone());
            }
        }

        // 分类处理联系人
        for member in members {
            let user_name = member.get("UserName").and_then(|v| v.as_str().map(|s| s.to_string())).unwrap_or_default();
            let verify_flag = member.get("VerifyFlag").and_then(|v| v.as_i64()).unwrap_or(0);

            if user_name.contains("@@") {
                // 群聊
                if !self.get_group_id_list().contains(&user_name.to_string()) {
                    self.add_group_id(user_name.to_string());
                    self.add_group(member.clone());
                }
            } else if verify_flag & 8 != 0 {
                // 公众号/服务号 - 暂时跳过
            } else if user_name != self.get_user_name().unwrap_or_default() {
                // 普通联系人
                self.add_contact(member.clone());
            }
        }

        info!("联系人总数: {}", member_count);
        Ok(())
    }

    /// 获取群成员列表
    pub async fn web_wx_batch_get_contact(&self) -> Result<()> {
        let url = self.get_login_info_value("url")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .ok_or_else(|| anyhow!("未获取url"))?;

        let pass_ticket = self.get_login_info_value("pass_ticket")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        let batch_url = format!(
            "{}?r={}&pass_ticket={}",
            url_enum::batch_get_contact_url(&url),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            pass_ticket
        );

        let group_ids = self.get_group_id_list();
        let mut param = serde_json::json!({
            "Count": group_ids.len(),
            "List": group_ids.iter().map(|id| {
                serde_json::json!({
                    "UserName": id,
                    "EncryChatRoomId": ""
                })
            }).collect::<Vec<_>>()
        });

        // 添加BaseRequest
        let base_request = self.get_base_request();
        param["BaseRequest"] = Value::Object(base_request);

        let response = self.http_client()
            .post(&batch_url)
            .json(&param)
            .send()
            .await?;

        let text = response.text().await?;
        let json: Value = serde_json::from_str(&text)
            .map_err(|e| anyhow!("解析群成员响应失败: {}", e))?;

        if let Some(contacts) = json.get("ContactList").and_then(|v| v.as_array()) {
            for contact in contacts {
                let user_name = contact.get("UserName").and_then(|v| v.as_str().map(|s| s.to_string())).unwrap_or_default();
                if user_name.contains("@@") {
                    // 更新群成员
                    if let Some(members) = contact.get("MemberList") {
                        self.set_group_member(user_name.to_string(), members.clone());
                    }
                }
            }
        }

        Ok(())
    }

    /// 获取基础请求参数
    pub(crate) fn get_base_request_param(&self) -> serde_json::Map<String, Value> {
        let mut map = serde_json::Map::new();

        // BaseRequest
        let base_request = self.get_base_request();
        map.insert("BaseRequest".to_string(), Value::Object(base_request));

        map
    }

    /// 获取BaseRequest对象
    pub(crate) fn get_base_request(&self) -> serde_json::Map<String, Value> {
        let mut base = serde_json::Map::new();

        let device_id = self.get_login_info_value("deviceid")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "e123456789012345".to_string());
        let skey = self.get_login_info_value("skey")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        let wxsid = self.get_login_info_value("wxsid")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        let wxuin = self.get_login_info_value("wxuin")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        base.insert("DeviceID".to_string(), Value::String(device_id.to_string()));
        base.insert("Skey".to_string(), Value::String(skey.to_string()));
        base.insert("Sid".to_string(), Value::String(wxsid.to_string()));
        base.insert("Uin".to_string(), Value::String(wxuin.to_string()));

        base
    }
}