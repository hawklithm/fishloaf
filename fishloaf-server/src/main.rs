//! fishloaf-server 主程序
//! Rust实现的WeChat服务端

mod protocol;
mod models;
mod wechat;

use anyhow::Result;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info, warn, Level};
use tracing_appender::rolling;
use tracing_subscriber::fmt::format::FmtSpan;

use crate::models::{ActionResult, BaseRequest, ContactUserInfo, MessageSendRequest};
use crate::wechat::WechatClient;

/// 服务器端口配置
const SERVER_PORT: u16 = 9021;
const PUSH_PORT: u16 = 9022;

/// 全局消息推送通道
#[allow(dead_code)]
struct AppState {
    wechat_client: Arc<WechatClient>,
    push_tx: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    init_logging();

    info!("fishloaf-server 启动中...");

    // 创建消息通道
    let (tx, _rx) = mpsc::channel::<String>(1000);
    let (push_tx, _) = broadcast::channel::<String>(1000);

    // 创建微信客户端
    let wechat_client = Arc::new(WechatClient::new(tx.clone()));

    // 保存全局状态
    let state = Arc::new(AppState {
        wechat_client: wechat_client.clone(),
        push_tx: push_tx.clone(),
    });

    // 启动消息转发任务
    let push_tx_clone = push_tx.clone();
    let rx = _rx;
    tokio::spawn(async move {
        let mut rx = rx;
        while let Some(msg) = rx.recv().await {
            if push_tx_clone.send(msg).is_err() {
                warn!("没有客户端连接，跳过推送");
            }
        }
    });

    // 启动微信登录
    let qr_path = "login";
    std::fs::create_dir_all(qr_path)?;

    if let Err(e) = wechat_client.login(qr_path).await {
        error!("微信登录失败: {}", e);
        return Err(e);
    }

    // 启动TCP服务器
    info!("启动TCP服务器，端口: {}", SERVER_PORT);
    let listener = TcpListener::bind(("0.0.0.0", SERVER_PORT)).await?;
    info!("TCP服务器已启动，监听端口 {}", SERVER_PORT);

    // 启动推送端口
    info!("启动推送���务器，端口: {}", PUSH_PORT);
    let push_listener = TcpListener::bind(("0.0.0.0", PUSH_PORT)).await?;
    info!("推送服务器已启动，监听端口 {}", PUSH_PORT);

    // 处理请求连接
    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, addr)) => {
                        info!("新客户端连接: {}", addr);
                        let state = state.clone();
                        tokio::spawn(handle_request(stream, state));
                    }
                    Err(e) => {
                        error!("接受连接失败: {}", e);
                    }
                }
            }
            result = push_listener.accept() => {
                match result {
                    Ok((stream, addr)) => {
                        info!("新推送客户端连接: {}", addr);
                        let push_tx = push_tx.clone();
                        tokio::spawn(handle_push(stream, push_tx));
                    }
                    Err(e) => {
                        error!("接受推送连接失败: {}", e);
                    }
                }
            }
        }
    }
}

/// 处理客户端请求
async fn handle_request(mut stream: TcpStream, state: Arc<AppState>) {
    let mut buffer = Vec::new();

    // 读取数据
    match stream.read_to_end(&mut buffer).await {
        Ok(n) if n > 0 => {}
        Ok(_) => return,
        Err(e) => {
            error!("读取请求失败: {}", e);
            return;
        }
    }

    // 检查协议
    if !protocol::check_message(&buffer) {
        error!("协议校验失败");
        return;
    }

    // 解码消息
    let data = protocol::decode(&buffer);
    let message = match String::from_utf8(data) {
        Ok(m) => m,
        Err(e) => {
            error!("UTF-8解码失败: {}", e);
            return;
        }
    };

    info!("收到客户端请求: {}", message);

    // 解析请求
    let request: Result<BaseRequest, _> = serde_json::from_str(&message);

    // 分别处理两种不同的响应类型
    if let Ok(req) = &request {
        match req.method.as_str() {
            "listUserAndGroup" => {
                // 返回联系人列表
                let contacts = state.wechat_client.list_users_and_groups().await;
                let response: ActionResult<Vec<ContactUserInfo>> = match contacts {
                    Ok(list) => ActionResult::create_success(list, &req.trace_id, &req.method),
                    Err(e) => ActionResult::create_error(e.to_string(), &req.trace_id, &req.method),
                };
                let response_json = serde_json::to_string(&response).unwrap_or_default();
                let encoded = protocol::encode(&response_json);
                if let Err(e) = stream.write_all(&encoded).await {
                    error!("发送响应失败: {}", e);
                }
                return;
            }
            "sendChatMessage" => {
                // 解析完整请求并发送消息
                let send_req: Result<MessageSendRequest, _> = serde_json::from_str(&message);
                match send_req {
                    Ok(send_req) => {
                        let result = state.wechat_client.send_message(&send_req.message, &send_req.target_id).await;
                        let response: ActionResult<bool> = match result {
                            Ok(_) => ActionResult::create_success(true, &req.trace_id, &req.method),
                            Err(e) => ActionResult::create_error(e.to_string(), &req.trace_id, &req.method),
                        };
                        let response_json = serde_json::to_string(&response).unwrap_or_default();
                        let encoded = protocol::encode(&response_json);
                        if let Err(e) = stream.write_all(&encoded).await {
                            error!("发送响应失败: {}", e);
                        }
                    }
                    Err(e) => {
                        let response: ActionResult<bool> = ActionResult::create_error(format!("解析请求失败: {}", e), &req.trace_id, &req.method);
                        let response_json = serde_json::to_string(&response).unwrap_or_default();
                        let encoded = protocol::encode(&response_json);
                        if let Err(e) = stream.write_all(&encoded).await {
                            error!("发送响应失败: {}", e);
                        }
                    }
                }
                return;
            }
            _ => {
                let response: ActionResult<bool> = ActionResult::create_error("未知的请求方法", &req.trace_id, &req.method);
                let response_json = serde_json::to_string(&response).unwrap_or_default();
                let encoded = protocol::encode(&response_json);
                if let Err(e) = stream.write_all(&encoded).await {
                    error!("发送响应失败: {}", e);
                }
                return;
            }
        }
    }
}

/// 处理推送连接
async fn handle_push(mut stream: TcpStream, push_tx: broadcast::Sender<String>) {
    let mut rx = push_tx.subscribe();

    // 保持连接，接收推送消息
    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(message) => {
                        let encoded = protocol::encode(&message);
                        if let Err(e) = stream.write_all(&encoded).await {
                            error!("推送消息失败: {}", e);
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("推送消息滞后，跳过 {} 条", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
            // 检测连接是否断开（通过读取空数据）
            result = stream.readable() => {
                if result.is_err() {
                    break;
                }
            }
        }
    }
}

/// 初始化日志
fn init_logging() {
    let log_dir = dirs::home_dir()
        .map(|p| p.join("fishloaf").join("logs"))
        .unwrap_or_else(|| std::path::PathBuf::from("logs"));

    let _ = std::fs::create_dir_all(&log_dir);

    let file_appender = rolling::daily(&log_dir, "fishloaf-server.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // 保持guard存活
    std::mem::forget(_guard);

    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_writer(non_blocking)
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("日志系统初始化完成，日志目录: {:?}", log_dir);
}