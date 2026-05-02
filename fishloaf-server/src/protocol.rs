//! 协议编解码模块
//! 实现与Java服务端相同的二进制协议

/// 协议魔数
const MAGIC: &[u8; 3] = &[0xf1, 0x60, 0x6f];

/// 编码消息：添加协议头
pub fn encode(message: &str) -> Vec<u8> {
    let msg_bytes = message.as_bytes();
    encode_bytes(msg_bytes)
}

/// 编码字节数组：添加协议头
pub fn encode_bytes(msg_byte: &[u8]) -> Vec<u8> {
    let len = msg_byte.len() as u32;
    let mut data = Vec::with_capacity(3 + 4 + len as usize);

    // 添加魔数 (3字节)
    data.extend_from_slice(MAGIC);

    // 添加长度 (4字节，大端序)
    data.push(((len >> 24) & 0xff) as u8);
    data.push(((len >> 16) & 0xff) as u8);
    data.push(((len >> 8) & 0xff) as u8);
    data.push((len & 0xff) as u8);

    // 添加数据
    data.extend_from_slice(msg_byte);

    data
}

/// 检查消息是否完整
pub fn check_message(message: &[u8]) -> bool {
    if message.len() < MAGIC.len() + 4 {
        return false;
    }

    // 检查魔数
    for i in 0..MAGIC.len() {
        if message[i] != MAGIC[i] {
            return false;
        }
    }

    // 解析长度
    let mut data_len: u32 = 0;
    for i in MAGIC.len()..MAGIC.len() + 4 {
        data_len = (data_len << 8) | (message[i] & 0xff) as u32;
    }

    message.len() == MAGIC.len() + 4 + data_len as usize
}

/// 解码消息：去除协议头
pub fn decode(message: &[u8]) -> Vec<u8> {
    let mut data_len: u32 = 0;
    for i in MAGIC.len()..MAGIC.len() + 4 {
        data_len = (data_len << 8) | (message[i] & 0xff) as u32;
    }

    let start = MAGIC.len() + 4;
    let end = start + data_len as usize;
    message[start..end].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let msg = "Hello, World!";
        let encoded = encode(msg);
        assert!(check_message(&encoded));
        let decoded = decode(&encoded);
        assert_eq!(decoded, msg.as_bytes());
    }

    #[test]
    fn test_check_message_invalid() {
        let data = vec![0x01, 0x02, 0x03];
        assert!(!check_message(&data));
    }
}