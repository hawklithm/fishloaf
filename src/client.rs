use std::{
    io::{self, Read, Write},
    net::TcpStream,
    result,
};

const MAGIC: [u8; 3] = [0xf1, 0x60, 0x6f];

fn check_header(head: [u8; 7]) -> result::Result<usize, String> {
    for i in 0usize..3 {
        if head[i] != MAGIC[i] {
            return Err(String::from("header not match"));
        }
    }
    let len_data = &head[3..7];
    let len = (len_data[0] as usize) << 24
        | (len_data[1] as usize) << 16
        | (len_data[2] as usize) << 8
        | len_data[3] as usize;

    Ok(len)
}

pub fn start() {
    let mut stream = TcpStream::connect("127.0.0.1:9999").expect("connection failed!");
    loop {
        let mut input = String::new();
        let size = io::stdin().read_line(&mut input).expect("read line failed");
        stream.write(&input.as_bytes()[..size]).expect("write fail");
        stream.flush().expect("flush error");
        let mut head = [0u8; 7];
        if let Ok(head_size) = stream.read(&mut head) {
            if head_size != 7 {
                continue;
            }
            if let Ok(data_size) = check_header(head) {
                if data_size > 0 {
                    let mut buffer = [0u8; 1024];
                    let mut data: Vec<u8> = Vec::new();
                    let mut remain_data = data_size;
                    while remain_data > 0 {
                        if let Ok(data_size) = stream.read(&mut buffer) {
                            if data_size > 0 {
                                remain_data -= data_size;
                                data.extend((&buffer[..data_size]).to_vec())
                            }
                        }
                    }
                    if let Ok(msg) = String::from_utf8(data) {
                        println!("receive {} byte data, message={}", data_size, msg);
                    }
                }
            }
        }
    }
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
        start();
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
