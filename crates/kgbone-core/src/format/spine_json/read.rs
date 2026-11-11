//! Spine JSON → Project.
use crate::format::ReadResult;
use crate::{CoreError, Result};

pub(super) fn read(bytes: &[u8]) -> Result<ReadResult> {
    let _ = bytes;
    Err(CoreError::NotImplemented("Spine JSON 가져오기"))
}
