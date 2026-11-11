//! base64(표준 알파벳, = 채움). 화면과 바이트를 JSON으로 주고받을 때 쓴다.

pub fn encode(data: &[u8]) -> String {
    let _ = data;
    String::new()
}

pub fn decode(text: &str) -> crate::Result<Vec<u8>> {
    let _ = text;
    Err(crate::CoreError::NotImplemented("base64"))
}
