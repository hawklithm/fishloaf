use arc_swap::ArcSwap;
use chashmap::CHashMap;
use json::{object::Object, JsonValue};
use lazy_static::lazy_static;
use std::{
    fmt,
    io::{self, Read, Write},
    net::TcpStream,
    result,
    str::FromStr,
    sync::Arc,
};
use tracing::error;
use uuid::Uuid;

use tokio::sync::mpsc::{Receiver, Sender};

use crate::utils::{build_message, get_message_from_tcpstream_with_protocol};

use self::my_custom_runtime::spawn;

use strum_macros::{Display, EnumString};

lazy_static! {
    pub static ref RESPONSE_WAITING_LIST: ArcSwap<CHashMap<String, String>> =
        ArcSwap::from(Arc::new(CHashMap::new()));
}

#[derive(EnumString, Display)]
pub enum ClientMethod {
    listUserAndGroup,
    sendChatMessage,
}

impl TryInto<ClientMethod> for String {
    type Error = SerializeErr;

    fn try_into(self) -> Result<ClientMethod, Self::Error> {
        Ok(ClientMethod::from_str(&self).or_else(|x| Err(SerializeErr::FormatError))?)
    }
}

pub struct SystemRequest {
    pub method: String,
    pub trace_id: String,
}

impl Into<JsonValue> for SystemRequest {
    fn into(self) -> JsonValue {
        let mut obj = Object::new();
        obj.insert("method", json::JsonValue::String(self.method));
        obj.insert("traceId", json::JsonValue::String(self.trace_id));
        JsonValue::Object(obj)
    }
}

pub struct ActionResult<T> {
    pub data: Option<T>,
    pub success: bool,
    pub message: Option<String>,
    pub trace_id: String,
    pub method: String,
}

impl<T> ActionResult<T> {
    pub fn create_success(data: T, trace_id: String, method: String) -> ActionResult<T> {
        ActionResult {
            data: Some(data),
            success: true,
            message: None,
            trace_id,
            method,
        }
    }

    pub fn create_error(error_msg: String, trace_id: String, method: String) -> ActionResult<T> {
        ActionResult {
            data: None,
            success: true,
            message: Some(error_msg),
            trace_id,
            method,
        }
    }
}

pub struct ContactUserInfo {
    pub unique_id: String,
    pub display_name: String,
    pub is_group: bool,
}

pub enum SerializeErr {
    FormatError,
    FieldMissing,
    FieldFormatError,
}

impl TryFrom<JsonValue> for ContactUserInfo {
    fn try_from(value: JsonValue) -> Result<Self, Self::Error> {
        if let JsonValue::Object(data) = value {
            return Ok(ContactUserInfo {
                unique_id: String::from(
                    data.get("uniqueId")
                        .ok_or(SerializeErr::FieldMissing)?
                        .as_str()
                        .ok_or(SerializeErr::FieldFormatError)?,
                ),
                display_name: String::from(
                    data.get("displayName")
                        .ok_or(SerializeErr::FieldMissing)?
                        .as_str()
                        .ok_or(SerializeErr::FieldFormatError)?,
                ),
                is_group: data
                    .get("isGroup")
                    .ok_or(SerializeErr::FieldMissing)?
                    .as_bool()
                    .ok_or(SerializeErr::FieldFormatError)?,
            });
        } else {
            Err(SerializeErr::FormatError)
        }
    }

    type Error = SerializeErr;
}

pub struct InputMessage {
    pub message: String,
    pub group: String,
}

pub trait ResponseChannel {
    fn response(&mut self, message: String);
}

pub struct MessageChannel {
    pub push_notification_receiver: Receiver<String>,
    pub message_sender_receiver: (Sender<String>, Receiver<String>),
}

pub fn list_user_and_group(
    channel: &MessageChannel,
    // callback: Box<dyn ResponseChannel + Send + Sync>,
) -> String {
    let uuid = Uuid::new_v4().to_string();
    channel.send_request(
        json::stringify(SystemRequest {
            method: ClientMethod::listUserAndGroup.to_string(),
            trace_id: uuid.clone(),
        }),
        // callback,
    );
    uuid
}

impl MessageChannel {
    pub fn new(address: &str, port0: u16, port1: u16) -> MessageChannel {
        let (push_notification_receiver, message_sender_receiver) = start(address, port0, port1);
        MessageChannel {
            push_notification_receiver,
            message_sender_receiver,
        }
    }

    fn send_request(
        &self,
        message: String,
        // callback: Box<dyn ResponseChannel + Send + Sync>,
    ) {
        // RESPONSE_WAITING_LIST.load().insert(uuid.clone(), callback);
        let sender = &self.message_sender_receiver.0;
        if let Err(e) = sender.blocking_send(message) {
            error!("error happen! {}", e);
            // RESPONSE_WAITING_LIST.load().remove(t);
        }
    }

    pub fn message_dispatch(&mut self) {
        let receiver = &mut self.message_sender_receiver.1;
        if let Ok(message) = receiver.try_recv() {
            if let Some(result) = parse_json_for_trace_id(message.clone()) {
                RESPONSE_WAITING_LIST.load().insert(result, message);
            }
        }
    }

    pub fn callback(&self, input: InputMessage) {
        let sender = &self.message_sender_receiver.0;
        // println!(
        //     "output message = {}, group = {}",
        //     input.message, input.group
        // );
        if let Err(e) = sender.blocking_send(input.message) {
            println!("error happend!{}", e);
        }
        // if let Err(e) = block_on_return(t) {
        //     println!("error happend!{}", e);
        // }
    }
}

pub(crate) fn parse_json(message: String) -> Option<ActionResult<Vec<ContactUserInfo>>> {
    if let Ok(jvalue) = json::parse(message.trim()) {
        if let JsonValue::Object(obj) = jvalue {
            let success = obj.get("success")?.as_bool()?;
            let method = String::from(obj.get("method")?.as_str()?);
            if success {
                let data_vec = obj.get("data")?;
                if let JsonValue::Array(array) = data_vec {
                    let mut result = Vec::new();
                    for item in array {
                        let sub = ContactUserInfo::try_from((*item).clone()).ok()?;
                        result.push(sub);
                    }
                    return Some(ActionResult::create_success(
                        result,
                        String::from(obj.get("traceId")?.as_str()?),
                        method,
                    ));
                }
            } else {
                return Some(ActionResult::create_error(
                    String::from(obj.get("message")?.as_str()?),
                    String::from(obj.get("traceId")?.as_str()?),
                    method,
                ));
            }
        }
    }
    return None;
}

pub(crate) fn parse_json_for_trace_id(message: String) -> Option<String> {
    if let Ok(jvalue) = json::parse(message.trim()) {
        if let JsonValue::Object(obj) = jvalue {
            return Some(String::from(obj.get("traceId")?.as_str()?));
        }
    }
    None
}

pub(crate) mod my_custom_runtime {
    extern crate lazy_static;
    use futures::Future;
    use once_cell::sync::Lazy;
    use tokio_util::context::TokioContext;

    pub fn spawn(f: impl Future<Output = ()> + Send + 'static) {
        EXECUTOR.spawn(f);
    }

    pub fn block_on_return<T>(f: impl Future<Output = T> + Send) -> T {
        EXECUTOR.rt.block_on(f)
    }

    struct ThreadPool {
        inner: futures::executor::ThreadPool,
        rt: tokio::runtime::Runtime,
    }

    static EXECUTOR: Lazy<ThreadPool> = Lazy::new(|| {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(if num_cpus::get() - 1 > 0 {
                num_cpus::get()
            } else {
                1
            })
            .thread_name("socket-connector")
            .enable_all()
            .build()
            .unwrap();
        let inner = futures::executor::ThreadPool::builder().create().unwrap();
        ThreadPool { inner, rt }
    });

    impl ThreadPool {
        fn spawn(&self, f: impl Future<Output = ()> + Send + 'static) {
            let handle = self.rt.handle().clone();
            self.inner.spawn_ok(TokioContext::new(f, handle));
        }
    }
}

fn createMessagePushClient(address: &str, port: u16) -> Receiver<String> {
    let mut stream = TcpStream::connect((address, port)).expect("connection failed!");
    let (tx, rx) = tokio::sync::mpsc::channel(1024);
    spawn(async move {
        loop {
            let ret_msg = get_message_from_tcpstream_with_protocol(&mut stream);
            let _ = tx.send(ret_msg).await;
        }
    });
    return rx;
}

fn createMessageSendClient(address: &str, port: u16) -> (Sender<String>, Receiver<String>) {
    let mut stream = TcpStream::connect((address, port)).expect("connection failed!");
    let (msg_tx, mut msg_rx) = tokio::sync::mpsc::channel(1024);
    let (ret_tx, ret_rx) = tokio::sync::mpsc::channel(1024);
    spawn(async move {
        loop {
            let msg_opt: Option<String> = msg_rx.recv().await;
            if let Some(input) = msg_opt {
                stream.write(&build_message(&input)).expect("write fail");
                stream.flush().expect("flush error");
            }
            let ret_msg = get_message_from_tcpstream_with_protocol(&mut stream);
            let _ = ret_tx.send(ret_msg).await;
        }
    });
    return (msg_tx, ret_rx);
}

pub fn start(
    address: &str,
    push_listener_port: u16,
    message_sender_port: u16,
) -> (Receiver<String>, (Sender<String>, Receiver<String>)) {
    //各个地方需要两个端口，一个处理server -> client的推送消息，不需要client回应，另外一个处理client->server的外发消息，需要回应
    let push_notification_receiver = createMessagePushClient(address, push_listener_port);
    let (message_sender, response_reciever) = createMessageSendClient(address, message_sender_port);
    return (
        push_notification_receiver,
        (message_sender, response_reciever),
    );
}

#[cfg(test)]
mod tests {
    use std::str;
    use std::{
        io::{self, Read, Write},
        net::TcpStream,
    };

    use super::start;

    #[test]
    fn test_socket() {
        // start();
    }

    #[test]
    fn test_socket2() {
        let mut stream = TcpStream::connect("127.0.0.1:9999").unwrap();
        //发送字符串
        stream.write("hello,rust.欢迎使用Rust".as_bytes()).unwrap();

        //创建1k的缓冲区，用于接收server发过来的内容
        let mut buffer = [0; 1024];
        //读取server发过来的内容
        stream.read(&mut buffer).unwrap();

        //打印接收到的内容(注:如果收到的实际内容小于1k,后面的部分默认全是\u{0}填充，所以要trim_matches去掉)
        println!(
            "Response from server:{:?}",
            str::from_utf8(&buffer).unwrap().trim_matches('\u{0}')
        );
    }
}
