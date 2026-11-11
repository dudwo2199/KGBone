//! .kgbone 읽기·쓰기 (docs/FORMAT.md).
use super::{Original, ReadResult, SkeletonFormat, WriteResult};
use crate::model::Project;
use crate::{CoreError, Result};

/// .kgbone 형식.
pub struct KgboneFormat;

impl SkeletonFormat for KgboneFormat {
    fn name(&self) -> &'static str {
        "KGBone 프로젝트"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["kgbone"]
    }
    /// formatVersion이 낮으면 자동 변환한다.
    fn read(&self, bytes: &[u8]) -> Result<ReadResult> {
        let _ = bytes;
        Err(CoreError::NotImplemented("kgbone 읽기"))
    }
    /// 편집하지 않았으면(`project == original.project`) 원래 바이트를 그대로 돌려준다.
    fn write(&self, project: &Project, original: Option<&Original>) -> Result<WriteResult> {
        let _ = (project, original);
        Err(CoreError::NotImplemented("kgbone 쓰기"))
    }
}
