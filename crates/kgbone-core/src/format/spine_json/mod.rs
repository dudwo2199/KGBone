//! Spine JSON 호환 가져오기·내보내기(공개 JSON 형식 문서 기준, docs/SOURCES.md S1).
//! 4.3을 우선 지원하고 4.2(및 그 이전 4.x) 읽기를 지원한다.
mod read;
mod write;

use super::{Original, ReadResult, SkeletonFormat, WriteResult};
use crate::model::Project;
use crate::Result;

/// 내보낼 Spine 버전.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SpineVersion {
    #[default]
    V43,
    V42,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpineExportOptions {
    pub version: SpineVersion,
    /// 보기 좋게(들여쓰기) 출력.
    pub pretty: bool,
    /// `skeleton.images` 값.
    pub images_path: String,
}

impl Default for SpineExportOptions {
    fn default() -> Self {
        SpineExportOptions { version: SpineVersion::V43, pretty: true, images_path: "./images/".into() }
    }
}

/// Spine JSON 호환 형식.
#[derive(Default)]
pub struct SpineJsonFormat {
    pub options: SpineExportOptions,
}

impl SkeletonFormat for SpineJsonFormat {
    fn name(&self) -> &'static str {
        "Spine JSON 호환"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["json"]
    }
    fn read(&self, bytes: &[u8]) -> Result<ReadResult> {
        read::read(bytes)
    }
    /// 4.3 표기가 확인되지 않은 기능(슬라이더, 4.2로 표현 못 하는 트랜스폼 매핑, 부피 보존)은 경고를 낸다(GAPS G-10~12).
    fn write(&self, project: &Project, original: Option<&Original>) -> Result<WriteResult> {
        write::write(project, original, &self.options)
    }
}
