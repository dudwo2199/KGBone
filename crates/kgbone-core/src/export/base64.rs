//! base64(표준 알파벳, = 채움). 화면과 바이트를 JSON으로 주고받을 때 쓴다.
use crate::{CoreError, Result};

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// 공백(빈칸·줄바꿈·탭)은 건너뛴다. 알파벳 밖 글자, 잘못된 채움, 남는 글자는 오류.
pub fn decode(text: &str) -> Result<Vec<u8>> {
    let bad = |m: &str| CoreError::InvalidArgs(format!("base64: {m}"));
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let mut acc = 0u32;
    let mut n = 0u32;
    let mut pad = 0u32;
    for c in text.bytes() {
        if c.is_ascii_whitespace() {
            continue;
        }
        if c == b'=' {
            pad += 1;
            continue;
        }
        if pad > 0 {
            return Err(bad("채움(=) 뒤에 글자가 있습니다"));
        }
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return Err(bad(&format!("잘못된 글자 '{}'", c as char))),
        };
        acc = acc << 6 | v as u32;
        n += 1;
        if n == 4 {
            out.extend_from_slice(&[(acc >> 16) as u8, (acc >> 8) as u8, acc as u8]);
            acc = 0;
            n = 0;
        }
    }
    // 남은 글자 2개 → 1바이트(채움 2), 3개 → 2바이트(채움 1). 채움은 생략해도 받아들인다.
    match (n, pad) {
        (0, 0) => {}
        (2, 0) | (2, 2) => out.push((acc >> 4) as u8),
        (3, 0) | (3, 1) => out.extend_from_slice(&[(acc >> 10) as u8, (acc >> 2) as u8]),
        _ => return Err(bad("길이가 맞지 않습니다")),
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k01_base64_known_values() {
        // RFC 4648 10장 시험 값.
        let cases = [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foob", "Zm9vYg=="), ("fooba", "Zm9vYmE="), ("foobar", "Zm9vYmFy")];
        for (raw, enc) in cases {
            assert_eq!(encode(raw.as_bytes()), enc);
            assert_eq!(decode(enc).unwrap(), raw.as_bytes());
        }
        // 0xFB 0xFF → 111110 111111 1111(00) → "+/8="
        assert_eq!(encode(&[0xFB, 0xFF]), "+/8=");
    }

    #[test]
    fn k01_base64_round_trip_and_errors() {
        let all: Vec<u8> = (0..=255).collect();
        assert_eq!(decode(&encode(&all)).unwrap(), all);
        assert_eq!(decode("Zm9v\nYmFy \t").unwrap(), b"foobar");
        assert!(decode("Zm9v!").is_err());
        assert!(decode("Zg==Zg").is_err());
        assert!(decode("Z").is_err());
        assert!(decode("Zm9vY===").is_err());
    }
}
