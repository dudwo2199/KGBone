//! 색상. 파일에는 `"RRGGBBAA"`(어두운 색은 `"RRGGBB"`) 16진 문자열로 저장한다.
use crate::json::{FromJson, JValue, ToJson};
use crate::{CoreError, Result};

/// 0~255 정수 RGBA. 파일 왕복에서 값이 흔들리지 않도록 정수로 보관한다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const WHITE: Color = Color::rgba(255, 255, 255, 255);
    pub const BLACK: Color = Color::rgba(0, 0, 0, 255);
    pub const BONE: Color = Color::rgba(0x98, 0x98, 0x98, 255);
    pub const BOUNDING_BOX: Color = Color::rgba(0x60, 0xF0, 0x00, 255);
    pub const POINT: Color = Color::rgba(0xF1, 0xF1, 0x00, 255);
    pub const CLIPPING: Color = Color::rgba(0xCE, 0x3A, 0x3A, 255);
    pub const PATH: Color = Color::rgba(0xFF, 0x7F, 0x00, 255);

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
    }

    /// `RRGGBB` 또는 `RRGGBBAA`(대소문자 무관)를 읽는다.
    pub fn parse_hex(s: &str) -> Option<Color> {
        let h = |i: usize| u8::from_str_radix(s.get(i..i + 2)?, 16).ok();
        match s.len() {
            6 => Some(Color::rgba(h(0)?, h(2)?, h(4)?, 255)),
            8 => Some(Color::rgba(h(0)?, h(2)?, h(4)?, h(6)?)),
            _ => None,
        }
    }

    pub fn to_hex_rgba(self) -> String {
        format!("{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
    }

    pub fn to_hex_rgb(self) -> String {
        format!("{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// 0~1 실수 배열 [r, g, b, a].
    pub fn to_f32(self) -> [f32; 4] {
        [self.r as f32 / 255.0, self.g as f32 / 255.0, self.b as f32 / 255.0, self.a as f32 / 255.0]
    }

    pub fn from_f32(c: [f32; 4]) -> Color {
        let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        Color::rgba(q(c[0]), q(c[1]), q(c[2]), q(c[3]))
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::WHITE
    }
}

impl ToJson for Color {
    fn to_json(&self, _: bool) -> JValue {
        JValue::Str(self.to_hex_rgba())
    }
}

impl FromJson for Color {
    fn from_json(v: &JValue) -> Result<Self> {
        let s = v.as_str().unwrap_or_default();
        Color::parse_hex(s).ok_or_else(|| CoreError::Parse(format!("색상 형식이 잘못됨: {s}")))
    }
}

/// 어두운 색(tint black). 알파 없이 `RRGGBB`로 저장한다.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct DarkColor(pub Color);

impl ToJson for DarkColor {
    fn to_json(&self, _: bool) -> JValue {
        JValue::Str(self.0.to_hex_rgb())
    }
}

impl FromJson for DarkColor {
    fn from_json(v: &JValue) -> Result<Self> {
        let c = Color::from_json(v)?;
        Ok(DarkColor(Color { a: 255, ..c }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hex_roundtrip() {
        let c = Color::parse_hex("ff7f00cc").unwrap();
        assert_eq!(c, Color::rgba(255, 127, 0, 204));
        assert_eq!(c.to_hex_rgba(), "FF7F00CC");
        assert_eq!(Color::parse_hex("102030").unwrap().a, 255);
        assert!(Color::parse_hex("xyz").is_none());
    }
}
