//! PNG 읽기·쓰기(RGBA8, 곱해지지 않은 알파).
//!
//! 읽기: 모든 표준 색 형식(회색·RGB·팔레트·회색+알파·RGBA, 1~16비트, tRNS 투명색), 필터 0~4, Adam7 인터레이스.
//! 16비트 채널은 상위 바이트만 쓴다. 쓰기: RGBA8, 줄마다 필터 고르기 + zlib(고정 허프만 + LZ77).
use super::checksum::crc32;
use super::deflate::zlib_compress;
use super::inflate::zlib_decompress;
use crate::{CoreError, Result};

/// RGBA8 이미지(행 우선, 위쪽 행부터).
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
/// 너무 큰 이미지로 메모리가 바닥나지 않게 막는 한도(픽셀 수).
const MAX_PIXELS: u64 = 1 << 26;

fn perr(m: impl Into<String>) -> CoreError {
    CoreError::Parse(format!("PNG: {}", m.into()))
}

/// IHDR 정보.
struct Header {
    width: u32,
    height: u32,
    depth: u8,
    color: u8,
    interlace: bool,
}

impl Header {
    fn channels(&self) -> usize {
        match self.color {
            0 | 3 => 1,
            2 => 3,
            4 => 2,
            _ => 4,
        }
    }
    /// 필터가 비교하는 "한 픽셀 앞" 거리(바이트, 최소 1).
    fn filter_bpp(&self) -> usize {
        (self.channels() * self.depth as usize).div_ceil(8)
    }
    fn stride(&self, w: usize) -> usize {
        (w * self.channels() * self.depth as usize).div_ceil(8)
    }
}

/// 투명도·팔레트 정보.
#[derive(Default)]
struct Palette {
    rgb: Vec<[u8; 3]>,
    alpha: Vec<u8>,
    /// 회색·RGB의 투명색(원래 비트 깊이 값).
    key: Option<[u16; 3]>,
}

pub fn decode_png(bytes: &[u8]) -> Result<Image> {
    if bytes.len() < 8 || bytes[..8] != SIG {
        return Err(perr("PNG 시그니처가 아닙니다"));
    }
    let mut pos = 8;
    let mut header: Option<Header> = None;
    let mut pal = Palette::default();
    let mut idat = Vec::new();
    let mut ended = false;
    while pos + 12 <= bytes.len() {
        let len = u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]]) as usize;
        let kind = &bytes[pos + 4..pos + 8];
        let data = bytes.get(pos + 8..pos + 8 + len).ok_or_else(|| perr("청크가 파일 끝을 넘습니다"))?;
        let c = bytes.get(pos + 8 + len..pos + 12 + len).ok_or_else(|| perr("청크 CRC가 없습니다"))?;
        if crc32(&[kind, data]) != u32::from_be_bytes([c[0], c[1], c[2], c[3]]) {
            return Err(perr(format!("{} 청크 CRC 불일치", String::from_utf8_lossy(kind))));
        }
        pos += 12 + len;
        match kind {
            b"IHDR" => header = Some(read_header(data)?),
            b"PLTE" => pal.rgb = data.chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect(),
            b"tRNS" => {
                let h = header.as_ref().ok_or_else(|| perr("IHDR보다 tRNS가 먼저 나왔습니다"))?;
                let be = |i: usize| data.get(i..i + 2).map(|b| u16::from_be_bytes([b[0], b[1]]));
                match h.color {
                    3 => pal.alpha = data.to_vec(),
                    0 => pal.key = be(0).map(|g| [g, g, g]),
                    2 => pal.key = (|| Some([be(0)?, be(2)?, be(4)?]))(),
                    _ => {}
                }
            }
            b"IDAT" => idat.extend_from_slice(data),
            b"IEND" => {
                ended = true;
                break;
            }
            _ => {
                // 대문자로 시작하는 모르는 청크는 꼭 알아야 하는 청크다.
                if kind[0].is_ascii_uppercase() {
                    return Err(perr(format!("지원하지 않는 필수 청크: {}", String::from_utf8_lossy(kind))));
                }
            }
        }
    }
    let h = header.ok_or_else(|| perr("IHDR가 없습니다"))?;
    if !ended {
        return Err(perr("IEND가 없습니다"));
    }
    if h.color == 3 && pal.rgb.is_empty() {
        return Err(perr("팔레트(PLTE)가 없습니다"));
    }
    let raw = zlib_decompress(&idat)?;
    let (w, ht) = (h.width as usize, h.height as usize);
    let mut rgba = vec![0u8; w * ht * 4];
    let mut off = 0;
    // (시작 x, 시작 y, x 간격, y 간격). 인터레이스가 없으면 한 번에 전부.
    const ADAM7: [(usize, usize, usize, usize); 7] = [(0, 0, 8, 8), (4, 0, 8, 8), (0, 4, 4, 8), (2, 0, 4, 4), (0, 2, 2, 4), (1, 0, 2, 2), (0, 1, 1, 2)];
    let passes: &[(usize, usize, usize, usize)] = if h.interlace { &ADAM7 } else { &[(0, 0, 1, 1)] };
    for &(x0, y0, dx, dy) in passes {
        let pw = if w > x0 { (w - x0).div_ceil(dx) } else { 0 };
        let ph = if ht > y0 { (ht - y0).div_ceil(dy) } else { 0 };
        if pw == 0 || ph == 0 {
            continue;
        }
        let stride = h.stride(pw);
        let need = (stride + 1) * ph;
        let data = raw.get(off..off + need).ok_or_else(|| perr("이미지 데이터가 모자랍니다"))?;
        off += need;
        let rows = unfilter(data, stride, ph, h.filter_bpp())?;
        for py in 0..ph {
            let row = &rows[py * stride..(py + 1) * stride];
            for px in 0..pw {
                let p = pixel(&h, &pal, row, px);
                let (x, y) = (x0 + px * dx, y0 + py * dy);
                rgba[(y * w + x) * 4..(y * w + x) * 4 + 4].copy_from_slice(&p);
            }
        }
    }
    Ok(Image { width: h.width, height: h.height, rgba })
}

fn read_header(d: &[u8]) -> Result<Header> {
    if d.len() != 13 {
        return Err(perr("IHDR 길이가 잘못되었습니다"));
    }
    let be = |i: usize| u32::from_be_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]]);
    let h = Header { width: be(0), height: be(4), depth: d[8], color: d[9], interlace: d[12] == 1 };
    let ok = match h.color {
        0 => matches!(h.depth, 1 | 2 | 4 | 8 | 16),
        3 => matches!(h.depth, 1 | 2 | 4 | 8),
        2 | 4 | 6 => matches!(h.depth, 8 | 16),
        _ => false,
    };
    if !ok {
        return Err(perr(format!("지원하지 않는 색 형식(종류 {}, {}비트)", h.color, h.depth)));
    }
    if d[10] != 0 || d[11] != 0 || d[12] > 1 {
        return Err(perr("지원하지 않는 압축·필터·인터레이스 방식"));
    }
    if h.width == 0 || h.height == 0 || h.width as u64 * h.height as u64 > MAX_PIXELS {
        return Err(perr(format!("이미지 크기가 잘못되었습니다({}×{})", h.width, h.height)));
    }
    Ok(h)
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i16 + b as i16 - c as i16;
    let (pa, pb, pc) = ((p - a as i16).abs(), (p - b as i16).abs(), (p - c as i16).abs());
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// 줄마다 앞의 필터 바이트를 보고 원래 바이트로 되돌린다.
fn unfilter(data: &[u8], stride: usize, rows: usize, bpp: usize) -> Result<Vec<u8>> {
    let mut out = vec![0u8; stride * rows];
    for y in 0..rows {
        let ft = data[y * (stride + 1)];
        let src = &data[y * (stride + 1) + 1..(y + 1) * (stride + 1)];
        let (done, cur) = out.split_at_mut(y * stride);
        let prev = if y > 0 { &done[(y - 1) * stride..] } else { &[][..] };
        let cur = &mut cur[..stride];
        for i in 0..stride {
            let a = if i >= bpp { cur[i - bpp] } else { 0 };
            let b = if y > 0 { prev[i] } else { 0 };
            let c = if y > 0 && i >= bpp { prev[i - bpp] } else { 0 };
            let pred = match ft {
                0 => 0,
                1 => a,
                2 => b,
                3 => ((a as u16 + b as u16) / 2) as u8,
                4 => paeth(a, b, c),
                _ => return Err(perr(format!("모르는 필터 종류 {ft}"))),
            };
            cur[i] = src[i].wrapping_add(pred);
        }
    }
    Ok(out)
}

/// 줄에서 `x`번 픽셀의 `ch`번 채널 원래 값(비트 깊이 그대로).
fn sample(h: &Header, row: &[u8], x: usize, ch: usize) -> u16 {
    let d = h.depth as usize;
    let idx = x * h.channels() + ch;
    match d {
        8 => row[idx] as u16,
        16 => u16::from_be_bytes([row[idx * 2], row[idx * 2 + 1]]),
        _ => {
            let bit = idx * d;
            let byte = row[bit / 8];
            let shift = 8 - d - (bit % 8);
            ((byte >> shift) & ((1u8 << d) - 1)) as u16
        }
    }
}

/// 원래 값 → 8비트(16비트는 상위 바이트, 1·2·4비트는 0~255로 늘림).
fn to8(h: &Header, v: u16) -> u8 {
    match h.depth {
        16 => (v >> 8) as u8,
        8 => v as u8,
        d => (v as u32 * 255 / ((1u32 << d) - 1)) as u8,
    }
}

fn pixel(h: &Header, pal: &Palette, row: &[u8], x: usize) -> [u8; 4] {
    let s = |c| sample(h, row, x, c);
    match h.color {
        0 => {
            let g = s(0);
            let a = if pal.key.map(|k| k[0]) == Some(g) { 0 } else { 255 };
            let g8 = to8(h, g);
            [g8, g8, g8, a]
        }
        2 => {
            let (r, g, b) = (s(0), s(1), s(2));
            let a = if pal.key == Some([r, g, b]) { 0 } else { 255 };
            [to8(h, r), to8(h, g), to8(h, b), a]
        }
        3 => {
            let i = s(0) as usize;
            // 팔레트 밖 번호는 검정 불투명으로 둔다.
            let [r, g, b] = pal.rgb.get(i).copied().unwrap_or([0, 0, 0]);
            [r, g, b, pal.alpha.get(i).copied().unwrap_or(255)]
        }
        4 => {
            let g = to8(h, s(0));
            [g, g, g, to8(h, s(1))]
        }
        _ => [to8(h, s(0)), to8(h, s(1)), to8(h, s(2)), to8(h, s(3))],
    }
}

/// RGBA8 → PNG(색 종류 6, 8비트, 인터레이스 없음).
pub fn encode_png(img: &Image) -> Result<Vec<u8>> {
    let (w, h) = (img.width as usize, img.height as usize);
    if w == 0 || h == 0 {
        return Err(CoreError::InvalidArgs(format!("PNG 크기가 0입니다({w}×{h})")));
    }
    if img.rgba.len() != w * h * 4 {
        return Err(CoreError::InvalidArgs(format!("픽셀 수가 크기와 맞지 않습니다({w}×{h}, {}바이트)", img.rgba.len())));
    }
    let stride = w * 4;
    let mut raw = Vec::with_capacity((stride + 1) * h);
    let mut cand = vec![0u8; stride];
    let mut best = vec![0u8; stride];
    for y in 0..h {
        let cur = &img.rgba[y * stride..(y + 1) * stride];
        let prev = if y > 0 { &img.rgba[(y - 1) * stride..y * stride] } else { &[][..] };
        // 필터마다 결과 바이트를 부호 있는 값으로 본 절댓값 합이 가장 작은 것을 고른다(흔히 쓰는 경험칙).
        let mut best_score = u64::MAX;
        let mut best_ft = 0u8;
        for ft in 0..5u8 {
            let mut score = 0u64;
            for i in 0..stride {
                let a = if i >= 4 { cur[i - 4] } else { 0 };
                let b = if y > 0 { prev[i] } else { 0 };
                let c = if y > 0 && i >= 4 { prev[i - 4] } else { 0 };
                let pred = match ft {
                    0 => 0,
                    1 => a,
                    2 => b,
                    3 => ((a as u16 + b as u16) / 2) as u8,
                    _ => paeth(a, b, c),
                };
                let v = cur[i].wrapping_sub(pred);
                cand[i] = v;
                score += (v as i8).unsigned_abs() as u64;
            }
            if score < best_score {
                best_score = score;
                best_ft = ft;
                best.copy_from_slice(&cand);
            }
        }
        raw.push(best_ft);
        raw.extend_from_slice(&best);
    }
    let mut out = SIG.to_vec();
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&img.width.to_be_bytes());
    ihdr.extend_from_slice(&img.height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    write_chunk(&mut out, b"IHDR", &ihdr);
    write_chunk(&mut out, b"IDAT", &zlib_compress(&raw));
    write_chunk(&mut out, b"IEND", &[]);
    Ok(out)
}

fn write_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    out.extend_from_slice(&crc32(&[kind, data]).to_be_bytes());
}

/// PNG 머리(IHDR)에서 크기만 읽는다.
pub fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || bytes[..8] != SIG || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let be = |i: usize| u32::from_be_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    Some((be(16), be(20)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 시험용: 원하는 IHDR·청크·필터 줄로 PNG를 직접 만든다(압축은 우리 zlib).
    fn build_png(w: u32, h: u32, depth: u8, color: u8, interlace: u8, extra: &[(&[u8; 4], Vec<u8>)], raw: &[u8]) -> Vec<u8> {
        let mut out = SIG.to_vec();
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&w.to_be_bytes());
        ihdr.extend_from_slice(&h.to_be_bytes());
        ihdr.extend_from_slice(&[depth, color, 0, 0, interlace]);
        write_chunk(&mut out, b"IHDR", &ihdr);
        for (k, d) in extra {
            write_chunk(&mut out, k, d);
        }
        // IDAT를 둘로 나눠 이어 붙이기도 확인한다.
        let z = zlib_compress(raw);
        let mid = z.len() / 2;
        write_chunk(&mut out, b"IDAT", &z[..mid]);
        write_chunk(&mut out, b"IDAT", &z[mid..]);
        write_chunk(&mut out, b"IEND", &[]);
        out
    }

    fn test_image(w: u32, h: u32) -> Image {
        let mut rgba = Vec::new();
        for y in 0..h {
            for x in 0..w {
                rgba.extend_from_slice(&[(x * 37 % 256) as u8, (y * 91 % 256) as u8, ((x ^ y) * 13 % 256) as u8, ((x + y) * 29 % 256) as u8]);
            }
        }
        Image { width: w, height: h, rgba }
    }

    #[test]
    fn k01_png_encode_decode_round_trip() {
        for (w, h) in [(1, 1), (3, 2), (17, 9), (64, 40)] {
            let img = test_image(w, h);
            let bytes = encode_png(&img).unwrap();
            assert_eq!(png_size(&bytes), Some((w, h)));
            assert_eq!(decode_png(&bytes).unwrap(), img);
        }
        assert!(encode_png(&Image { width: 2, height: 2, rgba: vec![0; 3] }).is_err());
        assert!(encode_png(&Image { width: 0, height: 2, rgba: vec![] }).is_err());
    }

    #[test]
    fn k01_png_decode_samples() {
        // tools/gen-samples.mjs가 Node zlib로 만든 실제 PNG.
        let cases: [(&[u8], u32, u32); 3] = [
            (include_bytes!("../../../../samples/images/head.png"), 64, 64),
            (include_bytes!("../../../../samples/images/body.png"), 48, 96),
            (include_bytes!("../../../../samples/images/tail.png"), 128, 32),
        ];
        for (bytes, w, h) in cases {
            let img = decode_png(bytes).unwrap();
            assert_eq!((img.width, img.height), (w, h));
            assert_eq!(img.rgba.len() as u32, w * h * 4);
        }
        // head = 지름 64 원: 모서리(0,0)는 원 밖이라 투명, 가운데(32,32)는 원 안 색 [240,200,160,255].
        let head = decode_png(cases[0].0).unwrap();
        assert_eq!(head.rgba[3], 0);
        let c = (32 * 64 + 32) * 4;
        assert_eq!(&head.rgba[c..c + 4], &[240, 200, 160, 255]);
        // body = 48×96 사각형: 가장자리 2픽셀은 색의 절반([40,60,100]), 안쪽은 [80,120,200], 모두 불투명.
        let body = decode_png(cases[1].0).unwrap();
        assert_eq!(&body.rgba[0..4], &[40, 60, 100, 255]);
        let c = (10 * 48 + 10) * 4;
        assert_eq!(&body.rgba[c..c + 4], &[80, 120, 200, 255]);
        assert!(body.rgba.chunks(4).all(|p| p[3] == 255));
    }

    #[test]
    fn k01_png_decode_color_types() {
        // 회색 8비트 2×1 + tRNS(회색 값 0을 투명): [0, 200] → [0,0,0,0], [200,200,200,255]
        let png = build_png(2, 1, 8, 0, 0, &[(b"tRNS", vec![0, 0])], &[0, 0, 200]);
        assert_eq!(decode_png(&png).unwrap().rgba, vec![0, 0, 0, 0, 200, 200, 200, 255]);

        // 회색 1비트 3×1: 비트 101(+ 채움) = 0b1010_0000 → 255, 0, 255
        let png = build_png(3, 1, 1, 0, 0, &[], &[0, 0b1010_0000]);
        assert_eq!(decode_png(&png).unwrap().rgba, vec![255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255]);

        // 회색 2비트 2×1: 값 1, 2 → 1×255/3 = 85, 2×255/3 = 170. 바이트 = 01 10 0000
        let png = build_png(2, 1, 2, 0, 0, &[], &[0, 0b0110_0000]);
        assert_eq!(decode_png(&png).unwrap().rgba, vec![85, 85, 85, 255, 170, 170, 170, 255]);

        // RGB 8비트 1×1 + tRNS 색 키가 다르면 불투명.
        let png = build_png(1, 1, 8, 2, 0, &[(b"tRNS", vec![0, 1, 0, 2, 0, 3])], &[0, 10, 20, 30]);
        assert_eq!(decode_png(&png).unwrap().rgba, vec![10, 20, 30, 255]);
        // 같으면 투명.
        let png = build_png(1, 1, 8, 2, 0, &[(b"tRNS", vec![0, 10, 0, 20, 0, 30])], &[0, 10, 20, 30]);
        assert_eq!(decode_png(&png).unwrap().rgba, vec![10, 20, 30, 0]);

        // 팔레트 4비트 3×1: 번호 2, 0, 1 → 바이트 0010_0000, 0001_0000. tRNS는 0번만 알파 128.
        let plte = vec![255, 0, 0, 0, 255, 0, 0, 0, 255];
        let png = build_png(3, 1, 4, 3, 0, &[(b"PLTE", plte.clone()), (b"tRNS", vec![128])], &[0, 0x20, 0x10]);
        assert_eq!(decode_png(&png).unwrap().rgba, vec![0, 0, 255, 255, 255, 0, 0, 128, 0, 255, 0, 255]);
        // 팔레트 8비트, PLTE 없으면 오류.
        assert!(decode_png(&build_png(1, 1, 8, 3, 0, &[], &[0, 0])).is_err());

        // 회색+알파 8비트 1×1.
        let png = build_png(1, 1, 8, 4, 0, &[], &[0, 77, 99]);
        assert_eq!(decode_png(&png).unwrap().rgba, vec![77, 77, 77, 99]);

        // RGBA 16비트 1×1: 상위 바이트만 → [0x12, 0x34, 0x56, 0x78]
        let png = build_png(1, 1, 16, 6, 0, &[], &[0, 0x12, 0xFF, 0x34, 0x00, 0x56, 0x01, 0x78, 0x02]);
        assert_eq!(decode_png(&png).unwrap().rgba, vec![0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn k01_png_decode_filters() {
        // RGB 8비트 2×2, 줄마다 다른 필터.
        // 원래 픽셀: (10,20,30) (40,50,60) / (15,25,35) (100,110,120)
        // 0줄 필터 1(Sub): 앞 픽셀과의 차 → 10,20,30, 30,30,30
        // 1줄 필터 4(Paeth): 첫 픽셀 a=0,c=0,b=위 → 예측 = b(10,20,30) → 5,5,5
        //   둘째 픽셀 채널0: a=15, b=40, c=10 → p=45, pa=30, pb=5, pc=35 → b=40 → 100-40 = 60
        //   채널1: a=25,b=50,c=20 → p=55, pa=30,pb=5,pc=35 → 50 → 60. 채널2: a=35,b=60,c=30 → 60 → 60
        let raw = [1, 10, 20, 30, 30, 30, 30, 4, 5, 5, 5, 60, 60, 60];
        let img = decode_png(&build_png(2, 2, 8, 2, 0, &[], &raw)).unwrap();
        assert_eq!(img.rgba, vec![10, 20, 30, 255, 40, 50, 60, 255, 15, 25, 35, 255, 100, 110, 120, 255]);
        // 필터 2(Up)와 3(Average), 회색 8비트 2×2: 원래 [10, 20 / 30, 50]
        // 0줄 Average: 첫 칸 (0+0)/2=0 → 10, 둘째 (10+0)/2=5 → 15
        // 1줄 Up: 30-10=20, 50-20=30
        let raw = [3, 10, 15, 2, 20, 30];
        let img = decode_png(&build_png(2, 2, 8, 0, 0, &[], &raw)).unwrap();
        let g: Vec<u8> = img.rgba.chunks(4).map(|p| p[0]).collect();
        assert_eq!(g, vec![10, 20, 30, 50]);
    }

    #[test]
    fn k01_png_decode_adam7() {
        // 회색 8비트 3×3, 값 = y*3 + x. Adam7 패스별 픽셀:
        // 1: (0,0) / 2: x0=4 → 없음 / 3: y0=4 → 없음 / 4: (2,0) / 5: (0,2),(2,2) / 6: (1,0),(1,2) / 7: (0,1),(1,1),(2,1)
        let raw = [
            0, 0, // 패스1: 줄 하나 [0]
            0, 2, // 패스4
            0, 6, 8, // 패스5
            0, 1, 0, 7, // 패스6: 두 줄 [1], [7]
            0, 3, 4, 5, // 패스7
        ];
        let img = decode_png(&build_png(3, 3, 8, 0, 1, &[], &raw)).unwrap();
        let g: Vec<u8> = img.rgba.chunks(4).map(|p| p[0]).collect();
        assert_eq!(g, (0..9).collect::<Vec<u8>>());
    }

    #[test]
    fn k01_png_decode_errors() {
        assert!(decode_png(b"not a png").is_err());
        let mut good = encode_png(&test_image(2, 2)).unwrap();
        // IDAT 데이터 한 바이트를 바꾸면 CRC가 맞지 않는다.
        let n = good.len();
        good[n - 20] ^= 0xFF;
        assert!(decode_png(&good).is_err());
    }
}
