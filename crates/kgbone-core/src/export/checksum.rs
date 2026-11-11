//! 검사 값: CRC-32(PNG 청크), Adler-32(zlib 끝).

/// CRC-32(다항식 0xEDB88320, 반사형). PNG 청크의 형식 이름 + 데이터에 쓴다.
pub(super) fn crc32(parts: &[&[u8]]) -> u32 {
    let table = crc_table();
    let mut c = 0xFFFF_FFFFu32;
    for part in parts {
        for &b in part.iter() {
            c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
        }
    }
    c ^ 0xFFFF_FFFF
}

fn crc_table() -> &'static [u32; 256] {
    use std::sync::OnceLock;
    static TABLE: OnceLock<[u32; 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for (n, slot) in t.iter_mut().enumerate() {
            let mut c = n as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
            *slot = c;
        }
        t
    })
}

/// Adler-32(RFC 1950): a = 1 + Σ바이트, b = Σa (둘 다 65521로 나눈 나머지), 결과 = b << 16 | a.
pub(super) fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    let (mut a, mut b) = (1u32, 0u32);
    // 5552바이트마다 나머지를 구하면 u32가 넘치지 않는다.
    for chunk in data.chunks(5552) {
        for &x in chunk {
            a += x as u32;
            b += a;
        }
        a %= MOD;
        b %= MOD;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k01_crc32_known_values() {
        // 널리 알려진 검사 값: "123456789"의 CRC-32 = 0xCBF43926.
        assert_eq!(crc32(&[b"123456789"]), 0xCBF4_3926);
        // 나눠 넣어도 같다.
        assert_eq!(crc32(&[b"1234", b"56789"]), 0xCBF4_3926);
        // IEND 청크(데이터 없음)의 CRC = AE 42 60 82.
        assert_eq!(crc32(&[b"IEND"]), 0xAE42_6082);
    }

    #[test]
    fn k01_adler32_known_values() {
        // "Wikipedia": 바이트 합 = 87+105+107+105+112+101+100+105+97 = 919, a = 1 + 919 = 920 = 0x398, b = 각 단계 a의 합 = 4582 = 0x11E6 → 0x11E60398.
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
        assert_eq!(adler32(b""), 1);
    }
}
