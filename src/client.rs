use std::{
    io::{self, Read, Write},
    net::TcpStream,
    result,
};

use tokio::sync::mpsc::{Receiver, Sender};

use crate::utils::{build_message, get_message_from_tcpstream_with_protocol};

use self::my_custom_runtime::spawn;

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
            let _ = tx.send(ret_msg);
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
