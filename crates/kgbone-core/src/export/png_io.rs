//! PNG 읽기·쓰기(RGBA8, 곱해지지 않은 알파).
use crate::{CoreError, Result};

/// RGBA8 이미지(행 우선, 위쪽 행부터).
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub fn decode_png(bytes: &[u8]) -> Result<Image> {
    let _ = bytes;
    Err(CoreError::NotImplemented("PNG 읽기"))
}

pub fn encode_png(img: &Image) -> Result<Vec<u8>> {
    let _ = img;
    Err(CoreError::NotImplemented("PNG 쓰기"))
}

/// PNG 머리(IHDR)에서 크기만 읽는다.
pub fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes.len() < 24 || bytes[..8] != SIG || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let be = |i: usize| u32::from_be_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    Some((be(16), be(20)))
}
