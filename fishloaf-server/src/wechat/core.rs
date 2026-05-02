//! 微信核心存储类 - 对应itchat4j的Core类

use parking_lot::RwLock;
use reqwest::Client;
use serde_json::{Map, Value};

/// 核心存储类，单例模式
pub struct WechatCore {
    http_client: Client,
    /// 禁用重定向的client（用于processLoginInfo，对应Java redirect=false）
    no_redirect_client: Client,
    /// 登录状态
    alive: RwLock<bool>,
    /// UUID
    uuid: RwLock<Option<String>>,
    /// 用户名
    user_name: RwLock<Option<String>>,
    /// 昵称
    nick_name: RwLock<Option<String>>,
    /// 登录信息
    login_info: RwLock<Map<String, Value>>,
    /// 联系人列表
    contact_list: RwLock<Vec<Value>>,
    /// 群列表
    group_list: RwLock<Vec<Value>>,
    /// 群成员映射
    group_member_map: RwLock<Map<String, Value>>,
    /// 群ID列表
    group_id_list: RwLock<Vec<String>>,
    /// 用户信息映射
    user_info_map: RwLock<Map<String, Value>>,
    /// 自己的信息
    user_self: RwLock<Option<Value>>,
}

impl WechatCore {
    #[allow(dead_code)]
    pub fn new(http_client: Client, no_redirect_client: Client) -> Self {
        WechatCore {
            http_client,
            no_redirect_client,
            alive: RwLock::new(false),
            uuid: RwLock::new(None),
            user_name: RwLock::new(None),
            nick_name: RwLock::new(None),
            login_info: RwLock::new(Map::new()),
            contact_list: RwLock::new(Vec::new()),
            group_list: RwLock::new(Vec::new()),
            group_member_map: RwLock::new(Map::new()),
            group_id_list: RwLock::new(Vec::new()),
            user_info_map: RwLock::new(Map::new()),
            user_self: RwLock::new(None),
        }
    }

    pub fn http_client(&self) -> &Client {
        &self.http_client
    }

    pub fn no_redirect_client(&self) -> &Client {
        &self.no_redirect_client
    }

    pub fn is_alive(&self) -> bool {
        *self.alive.read()
    }

    pub fn set_alive(&self, alive: bool) {
        *self.alive.write() = alive;
    }

    pub fn get_uuid(&self) -> Option<String> {
        self.uuid.read().clone()
    }

    pub fn set_uuid(&self, uuid: String) {
        *self.uuid.write() = Some(uuid);
    }

    pub fn get_user_name(&self) -> Option<String> {
        self.user_name.read().clone()
    }

    pub fn set_user_name(&self, user_name: String) {
        *self.user_name.write() = Some(user_name);
    }

    pub fn get_nickname(&self) -> String {
        self.nick_name.read().clone().unwrap_or_default()
    }

    pub fn set_nickname(&self, nick_name: String) {
        *self.nick_name.write() = Some(nick_name);
    }

    #[allow(dead_code)]
    pub fn get_login_info(&self) -> Map<String, Value> {
        self.login_info.read().clone()
    }

    pub fn set_login_info(&self, key: impl Into<String>, value: Value) {
        self.login_info.write().insert(key.into(), value);
    }

    pub fn get_login_info_value(&self, key: &str) -> Option<Value> {
        self.login_info.read().get(key).cloned()
    }

    pub fn get_contact_list(&self) -> Vec<Value> {
        self.contact_list.read().clone()
    }

#[allow(dead_code)]
    pub fn set_contact_list(&self, list: Vec<Value>) {
        *self.contact_list.write() = list;
    }

    pub fn add_contact(&self, contact: Value) {
        self.contact_list.write().push(contact);
    }

    pub fn get_group_list(&self) -> Vec<Value> {
        self.group_list.read().clone()
    }

#[allow(dead_code)]
    pub fn set_group_list(&self, list: Vec<Value>) {
        *self.group_list.write() = list;
    }

    pub fn add_group(&self, group: Value) {
        self.group_list.write().push(group);
    }

    pub fn get_group_member_map(&self) -> Map<String, Value> {
        self.group_member_map.read().clone()
    }

    pub fn set_group_member(&self, group_id: String, members: Value) {
        self.group_member_map.write().insert(group_id, members);
    }

    pub fn get_group_id_list(&self) -> Vec<String> {
        self.group_id_list.read().clone()
    }

    pub fn add_group_id(&self, group_id: String) {
        self.group_id_list.write().push(group_id);
    }

    pub fn get_contact_count(&self) -> usize {
        self.contact_list.read().len()
    }

    pub fn get_user_info_map(&self) -> Map<String, Value> {
        self.user_info_map.read().clone()
    }

    #[allow(dead_code)]
    pub fn set_user_info(&self, user_id: String, info: Value) {
        self.user_info_map.write().insert(user_id, info);
    }

#[allow(dead_code)]
    pub fn get_user_self(&self) -> Option<Value> {
        self.user_self.read().clone()
    }

    pub fn set_user_self(&self, user_self: Value) {
        *self.user_self.write() = Some(user_self);
    }
}