//! .kgbone 구버전 → 현재 버전 변환(FORMAT.md 7장).
use crate::json::JValue;
use crate::{Result, Warning};

/// 원본 트리를 현재 formatVersion 구조로 바꾼다. 바꾼 내용은 경고로 알린다.
pub(crate) fn migrate(tree: JValue) -> Result<(JValue, Vec<Warning>)> {
    Ok((tree, vec![]))
}
