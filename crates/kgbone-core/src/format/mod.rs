//! 파일 형식별 읽기·쓰기. 모든 뼈대 형식은 [`SkeletonFormat`]을 구현한다.
pub mod atlas;
pub mod kgbone;
mod migrate;
mod preserve;
pub mod spine_json;
#[cfg(test)]
mod tests;

use crate::json::JValue;
use crate::model::Project;
use crate::{Result, Warning};

/// 읽은 결과.
#[derive(Debug, Clone)]
pub struct ReadResult {
    pub project: Project,
    /// 원본 트리(쓸 때 키 순서·숫자 표기·모르는 필드를 보존하는 데 쓴다).
    pub raw: JValue,
    pub warnings: Vec<Warning>,
}

/// 쓴 결과.
#[derive(Debug, Clone)]
pub struct WriteResult {
    pub bytes: Vec<u8>,
    pub warnings: Vec<Warning>,
}

/// 쓰기 원본 정보: 처음 읽은 바이트·트리·그때의 모델. 셋 다 있으면 편집 여부를 판단해 보존한다.
#[derive(Debug, Clone)]
pub struct Original {
    pub bytes: Vec<u8>,
    pub raw: JValue,
    pub project: Project,
}

/// 뼈대 파일 형식 공통 동작.
pub trait SkeletonFormat {
    /// 형식 이름(화면 표시용).
    fn name(&self) -> &'static str;
    /// 확장자(점 없이).
    fn extensions(&self) -> &'static [&'static str];
    fn read(&self, bytes: &[u8]) -> Result<ReadResult>;
    /// `original`이 있으면 바뀐 부분만 바꿔 쓴다.
    fn write(&self, project: &Project, original: Option<&Original>) -> Result<WriteResult>;
}
