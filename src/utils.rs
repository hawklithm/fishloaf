use std::{io::Read, net::TcpStream, result};

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

pub fn build_message(message: &str) -> Vec<u8> {
    let byte_data = message.as_bytes();
    let len = byte_data.len();
    let mut data: Vec<u8> = Vec::with_capacity(3 + 4 + len);
    data.extend_from_slice(&MAGIC);

    let mut len_byte: [u8; 4] = [0, 0, 0, 0];
    len_byte[0] = ((len >> 24) & 0xff) as u8;
    len_byte[1] = ((len >> 16) & 0xff) as u8;
    len_byte[2] = ((len >> 8) & 0xff) as u8;
    len_byte[3] = (len & 0xff) as u8;
    data.extend_from_slice(&len_byte);
    data.extend_from_slice(&byte_data);
    return data;
}

pub fn get_message_from_tcpstream_with_protocol(stream: &mut TcpStream) -> String {
    let mut head = [0u8; 7];
    loop {
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
                        // println!("receive {} byte data, message={}", data_size, msg);
                        return msg;
                    }
                }
            }
        }
    }
}
