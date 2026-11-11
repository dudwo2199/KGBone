//! zlib(RFC 1950) 안의 DEFLATE(RFC 1951) 풀기. 저장·고정 허프만·동적 허프만 블록을 모두 읽는다.
use super::checksum::adler32;
use super::deflate_tables::{DIST_BASE, DIST_EXTRA, LEN_BASE, LEN_EXTRA};
use crate::{CoreError, Result};

fn err(m: &str) -> CoreError {
    CoreError::Parse(format!("압축 데이터가 잘못되었습니다: {m}"))
}

/// zlib 스트림 → 원래 바이트. 끝의 Adler-32도 확인한다.
pub(super) fn zlib_decompress(data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < 6 {
        return Err(err("너무 짧음"));
    }
    let (cmf, flg) = (data[0], data[1]);
    if cmf & 0x0F != 8 {
        return Err(err("deflate 방식이 아님"));
    }
    if ((cmf as u16) << 8 | flg as u16) % 31 != 0 {
        return Err(err("zlib 머리 검사 실패"));
    }
    if flg & 0x20 != 0 {
        return Err(err("사전(FDICT)은 지원하지 않음"));
    }
    let mut r = BitReader { data: &data[2..], pos: 0, bitbuf: 0, bitcnt: 0 };
    let out = inflate(&mut r)?;
    let end = 2 + r.byte_pos();
    let tail = data.get(end..end + 4).ok_or_else(|| err("Adler-32 없음"))?;
    let want = u32::from_be_bytes([tail[0], tail[1], tail[2], tail[3]]);
    if adler32(&out) != want {
        return Err(err("Adler-32 불일치"));
    }
    Ok(out)
}

struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    bitbuf: u32,
    bitcnt: u32,
}

impl BitReader<'_> {
    /// 낮은 비트부터 n비트(n ≤ 16).
    fn bits(&mut self, n: u32) -> Result<u32> {
        while self.bitcnt < n {
            let b = *self.data.get(self.pos).ok_or_else(|| err("데이터가 일찍 끝남"))?;
            self.pos += 1;
            self.bitbuf |= (b as u32) << self.bitcnt;
            self.bitcnt += 8;
        }
        let v = self.bitbuf & ((1u32 << n) - 1);
        self.bitbuf >>= n;
        self.bitcnt -= n;
        Ok(v)
    }

    /// 남은 비트를 버리고 바이트 경계로.
    fn align(&mut self) {
        self.bitbuf = 0;
        self.bitcnt = 0;
    }

    /// 다 읽은 바이트 수(버퍼에 남은 온전한 바이트는 되돌린다).
    fn byte_pos(&self) -> usize {
        self.pos - (self.bitcnt / 8) as usize
    }
}

/// 정규(canonical) 허프만 부호: 길이별 개수와 길이·값 순으로 늘어놓은 기호.
struct Huffman {
    counts: [u16; 16],
    symbols: Vec<u16>,
}

impl Huffman {
    fn new(lengths: &[u8]) -> Result<Huffman> {
        let mut counts = [0u16; 16];
        for &l in lengths {
            counts[l as usize] += 1;
        }
        counts[0] = 0;
        // 넘치는(과포화) 부호 검사.
        let mut left = 1i32;
        for &c in &counts[1..] {
            left = (left << 1) - c as i32;
            if left < 0 {
                return Err(err("허프만 길이 과다"));
            }
        }
        let mut offs = [0u16; 16];
        for len in 1..15 {
            offs[len + 1] = offs[len] + counts[len];
        }
        let mut symbols = vec![0u16; lengths.len()];
        for (sym, &l) in lengths.iter().enumerate() {
            if l != 0 {
                symbols[offs[l as usize] as usize] = sym as u16;
                offs[l as usize] += 1;
            }
        }
        Ok(Huffman { counts, symbols })
    }

    /// 한 비트씩 읽어 길이별 첫 부호와 비교한다(정규 부호는 길이마다 연속된 값).
    fn decode(&self, r: &mut BitReader) -> Result<u16> {
        let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
        for len in 1..16 {
            code |= r.bits(1)? as i32;
            let count = self.counts[len] as i32;
            if code - first < count {
                return Ok(self.symbols[(index + code - first) as usize]);
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        Err(err("없는 허프만 부호"))
    }
}

fn inflate(r: &mut BitReader) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    loop {
        let last = r.bits(1)?;
        match r.bits(2)? {
            0 => stored(r, &mut out)?,
            1 => {
                let (lit, dist) = fixed_tables()?;
                codes(r, &mut out, &lit, &dist)?
            }
            2 => {
                let (lit, dist) = dynamic_tables(r)?;
                codes(r, &mut out, &lit, &dist)?
            }
            _ => return Err(err("모르는 블록 종류")),
        }
        if last == 1 {
            return Ok(out);
        }
    }
}

fn stored(r: &mut BitReader, out: &mut Vec<u8>) -> Result<()> {
    r.align();
    let p = r.pos;
    let h = r.data.get(p..p + 4).ok_or_else(|| err("저장 블록 머리 없음"))?;
    let len = u16::from_le_bytes([h[0], h[1]]);
    let nlen = u16::from_le_bytes([h[2], h[3]]);
    if len != !nlen {
        return Err(err("저장 블록 길이 검사 실패"));
    }
    let body = r.data.get(p + 4..p + 4 + len as usize).ok_or_else(|| err("저장 블록이 일찍 끝남"))?;
    out.extend_from_slice(body);
    r.pos = p + 4 + len as usize;
    Ok(())
}

fn fixed_tables() -> Result<(Huffman, Huffman)> {
    let mut lit = [0u8; 288];
    for (i, l) in lit.iter_mut().enumerate() {
        *l = match i {
            0..=143 => 8,
            144..=255 => 9,
            256..=279 => 7,
            _ => 8,
        };
    }
    Ok((Huffman::new(&lit)?, Huffman::new(&[5u8; 30])?))
}

fn dynamic_tables(r: &mut BitReader) -> Result<(Huffman, Huffman)> {
    const ORDER: [usize; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];
    let nlen = r.bits(5)? as usize + 257;
    let ndist = r.bits(5)? as usize + 1;
    let ncode = r.bits(4)? as usize + 4;
    if nlen > 286 || ndist > 30 {
        return Err(err("동적 블록 부호 수 초과"));
    }
    let mut cl = [0u8; 19];
    for &i in ORDER.iter().take(ncode) {
        cl[i] = r.bits(3)? as u8;
    }
    let clh = Huffman::new(&cl)?;
    let mut lengths = vec![0u8; nlen + ndist];
    let mut i = 0;
    while i < nlen + ndist {
        let sym = clh.decode(r)?;
        let (val, rep) = match sym {
            0..=15 => (sym as u8, 1),
            16 => {
                let prev = *lengths.get(i.wrapping_sub(1)).filter(|_| i > 0).ok_or_else(|| err("반복할 앞 길이 없음"))?;
                (prev, 3 + r.bits(2)? as usize)
            }
            17 => (0, 3 + r.bits(3)? as usize),
            _ => (0, 11 + r.bits(7)? as usize),
        };
        if i + rep > lengths.len() {
            return Err(err("길이 반복이 넘침"));
        }
        lengths[i..i + rep].fill(val);
        i += rep;
    }
    if lengths[256] == 0 {
        return Err(err("블록 끝 부호 없음"));
    }
    Ok((Huffman::new(&lengths[..nlen])?, Huffman::new(&lengths[nlen..])?))
}

fn codes(r: &mut BitReader, out: &mut Vec<u8>, lit: &Huffman, dist: &Huffman) -> Result<()> {
    loop {
        let sym = lit.decode(r)? as usize;
        if sym < 256 {
            out.push(sym as u8);
        } else if sym == 256 {
            return Ok(());
        } else {
            let li = sym - 257;
            if li >= LEN_BASE.len() {
                return Err(err("잘못된 길이 부호"));
            }
            let len = LEN_BASE[li] as usize + r.bits(LEN_EXTRA[li] as u32)? as usize;
            let di = dist.decode(r)? as usize;
            if di >= DIST_BASE.len() {
                return Err(err("잘못된 거리 부호"));
            }
            let d = DIST_BASE[di] as usize + r.bits(DIST_EXTRA[di] as u32)? as usize;
            if d > out.len() {
                return Err(err("거리가 앞 데이터보다 김"));
            }
            let start = out.len() - d;
            // 겹치는 복사(d < len)는 한 바이트씩 해야 반복 무늬가 된다.
            for k in 0..len {
                let b = out[start + k];
                out.push(b);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k01_inflate_stored_block() {
        // 78 01 | 저장 블록(BFINAL=1, BTYPE=00) LEN=3, NLEN=!3 | "abc" | adler32("abc") = 0x024D0127
        let data = [0x78, 0x01, 0x01, 0x03, 0x00, 0xFC, 0xFF, b'a', b'b', b'c', 0x02, 0x4D, 0x01, 0x27];
        assert_eq!(zlib_decompress(&data).unwrap(), b"abc");
    }

    #[test]
    fn k01_inflate_rejects_bad_checksum() {
        let data = [0x78, 0x01, 0x01, 0x03, 0x00, 0xFC, 0xFF, b'a', b'b', b'c', 0x02, 0x4D, 0x01, 0x28];
        assert!(zlib_decompress(&data).is_err());
        assert!(zlib_decompress(&[0x78, 0x01]).is_err());
    }
}
