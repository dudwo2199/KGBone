//! zlib 압축: LZ77(해시 사슬, 탐욕 일치) + 고정 허프만 블록 하나.
//! 최고 압축률보다 단순함과 결정성(같은 입력 → 같은 바이트)을 우선한다.
use super::checksum::adler32;
use super::deflate_tables::{DIST_BASE, DIST_EXTRA, LEN_BASE, LEN_EXTRA};

const WINDOW: usize = 32768;
const MIN_MATCH: usize = 3;
const MAX_MATCH: usize = 258;
const HASH_BITS: u32 = 15;
/// 사슬을 따라가 볼 최대 후보 수.
const MAX_CHAIN: usize = 48;

/// 원래 바이트 → zlib 스트림.
pub(super) fn zlib_compress(data: &[u8]) -> Vec<u8> {
    let mut w = BitWriter { out: vec![0x78, 0x9C], buf: 0, cnt: 0 };
    // 블록 머리: BFINAL=1, BTYPE=01(고정 허프만).
    w.put(1, 1);
    w.put(1, 2);
    lz77(data, &mut w);
    write_lit(&mut w, 256);
    w.flush();
    let mut out = w.out;
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

struct BitWriter {
    out: Vec<u8>,
    buf: u64,
    cnt: u32,
}

impl BitWriter {
    /// 값의 낮은 비트부터 n비트.
    fn put(&mut self, v: u32, n: u32) {
        self.buf |= (v as u64) << self.cnt;
        self.cnt += n;
        while self.cnt >= 8 {
            self.out.push(self.buf as u8);
            self.buf >>= 8;
            self.cnt -= 8;
        }
    }

    /// 허프만 부호는 높은 비트부터 쓰므로 뒤집어 넣는다.
    fn put_code(&mut self, code: u32, len: u32) {
        let rev = code.reverse_bits() >> (32 - len);
        self.put(rev, len);
    }

    fn flush(&mut self) {
        if self.cnt > 0 {
            self.out.push(self.buf as u8);
        }
        self.buf = 0;
        self.cnt = 0;
    }
}

/// 고정 허프만 리터럴/길이 부호(RFC 1951 3.2.6).
fn write_lit(w: &mut BitWriter, sym: u32) {
    match sym {
        0..=143 => w.put_code(0x30 + sym, 8),
        144..=255 => w.put_code(0x190 + sym - 144, 9),
        256..=279 => w.put_code(sym - 256, 7),
        _ => w.put_code(0xC0 + sym - 280, 8),
    }
}

/// 표에서 `v` 이하인 가장 큰 기본값의 번호.
fn table_index(base: &[u16], v: usize) -> usize {
    base.iter().rposition(|&b| b as usize <= v).unwrap_or(0)
}

fn write_match(w: &mut BitWriter, len: usize, dist: usize) {
    let li = table_index(&LEN_BASE, len);
    write_lit(w, 257 + li as u32);
    w.put((len - LEN_BASE[li] as usize) as u32, LEN_EXTRA[li] as u32);
    let di = table_index(&DIST_BASE, dist);
    w.put_code(di as u32, 5);
    w.put((dist - DIST_BASE[di] as usize) as u32, DIST_EXTRA[di] as u32);
}

fn hash3(d: &[u8], i: usize) -> usize {
    let v = (d[i] as u32) << 16 | (d[i + 1] as u32) << 8 | d[i + 2] as u32;
    (v.wrapping_mul(0x9E37_79B1) >> (32 - HASH_BITS)) as usize
}

fn lz77(data: &[u8], w: &mut BitWriter) {
    const NONE: usize = usize::MAX;
    let mut head = vec![NONE; 1 << HASH_BITS];
    let mut prev = vec![NONE; WINDOW];
    let n = data.len();
    let insert = |head: &mut Vec<usize>, prev: &mut Vec<usize>, i: usize| {
        if i + MIN_MATCH <= n {
            let h = hash3(data, i);
            prev[i % WINDOW] = head[h];
            head[h] = i;
        }
    };
    let mut i = 0;
    while i < n {
        let (mut best_len, mut best_dist) = (0usize, 0usize);
        if i + MIN_MATCH <= n {
            let max_len = MAX_MATCH.min(n - i);
            let mut cand = head[hash3(data, i)];
            let mut steps = 0;
            while cand != NONE && steps < MAX_CHAIN && i - cand <= WINDOW {
                let mut l = 0;
                while l < max_len && data[cand + l] == data[i + l] {
                    l += 1;
                }
                if l > best_len {
                    best_len = l;
                    best_dist = i - cand;
                    if l == max_len {
                        break;
                    }
                }
                let next = prev[cand % WINDOW];
                // 창 밖으로 밀려난 칸이 다시 쓰였으면 더 가지 않는다.
                if next == NONE || next >= cand {
                    break;
                }
                cand = next;
                steps += 1;
            }
        }
        if best_len >= MIN_MATCH {
            write_match(w, best_len, best_dist);
            for k in i..i + best_len {
                insert(&mut head, &mut prev, k);
            }
            i += best_len;
        } else {
            write_lit(w, data[i] as u32);
            insert(&mut head, &mut prev, i);
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::inflate::zlib_decompress;
    use super::*;

    fn round_trip(data: &[u8]) {
        let z = zlib_compress(data);
        assert_eq!(zlib_decompress(&z).unwrap(), data);
    }

    #[test]
    fn k01_deflate_round_trip() {
        round_trip(b"");
        round_trip(b"a");
        round_trip(b"abcabcabcabcabcabcabcabc");
        // 긴 반복(최대 일치 길이 258을 넘는다)과 창(32KB)을 넘는 거리.
        round_trip(&vec![7u8; 100_000]);
        let mut noise = Vec::new();
        let mut s = 12345u32;
        for _ in 0..70_000 {
            s = s.wrapping_mul(1_103_515_245).wrapping_add(12345);
            noise.push((s >> 24) as u8 & 0x0F);
        }
        round_trip(&noise);
    }

    #[test]
    fn k01_deflate_compresses_repetition() {
        let z = zlib_compress(&vec![0u8; 10_000]);
        assert!(z.len() < 200, "길이 {}", z.len());
    }
}
