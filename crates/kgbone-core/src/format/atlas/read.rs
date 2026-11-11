//! .atlas 텍스트 → Atlas. 4.x 표기(bounds, offsets)와 이전 표기(xy, size, orig, offset)를 모두 읽는다.
use super::Atlas;
use crate::{CoreError, Result};

pub fn parse(text: &str) -> Result<Atlas> {
    let _ = text;
    Err(CoreError::NotImplemented("atlas 읽기"))
}
