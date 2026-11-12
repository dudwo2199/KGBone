//! .kgbone 읽기·쓰기 (docs/FORMAT.md).
use super::preserve::{self, path_is, Schema};
use super::{migrate, Original, ReadResult, SkeletonFormat, WriteResult};
use crate::json::{self, FromJson, ToJson, WriteStyle};
use crate::model::Project;
use crate::{CoreError, Result};

/// .kgbone 형식.
pub struct KgboneFormat;

/// 읽은 결과가 구버전 변환을 거쳤는가(이때는 원래 바이트로 저장하면 안 된다).
pub(crate) fn was_migrated(r: &ReadResult) -> bool {
    r.warnings.iter().any(|w| w.code == migrate::MIGRATED)
}

impl SkeletonFormat for KgboneFormat {
    fn name(&self) -> &'static str {
        "KGBone 프로젝트"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &["kgbone"]
    }
    /// formatVersion이 낮으면 자동 변환한다.
    fn read(&self, bytes: &[u8]) -> Result<ReadResult> {
        let text = std::str::from_utf8(bytes).map_err(|_| CoreError::Parse("UTF-8 텍스트가 아닙니다".into()))?;
        let tree = json::parse(text)?;
        let (raw, warnings) = migrate::migrate(tree)?;
        let project = Project::from_json(&raw)?;
        Ok(ReadResult { project, raw, warnings })
    }
    /// 편집하지 않았으면(`project == original.project`) 원래 바이트를 그대로 돌려준다.
    /// 편집했으면 원본 트리에 합쳐 바뀐 값만 바꾼다(원본 텍스트가 있으면 그 글자 모양 그대로).
    fn write(&self, project: &Project, original: Option<&Original>) -> Result<WriteResult> {
        let Some(orig) = original else {
            let text = json::write(&project.to_json(false), &WriteStyle::default());
            return Ok(WriteResult { bytes: text.into_bytes(), warnings: vec![] });
        };
        if *project == orig.project && !orig.bytes.is_empty() {
            return Ok(WriteResult { bytes: orig.bytes.clone(), warnings: vec![] });
        }
        let merged = preserve::merge(&orig.raw, &project.to_json(true), &project.to_json(false), &KgboneSchema);
        let text = match std::str::from_utf8(&orig.bytes) {
            Ok(t) if !t.is_empty() => preserve::splice(t, &merged),
            _ => json::write(&merged, &WriteStyle::default()),
        };
        Ok(WriteResult { bytes: text.into_bytes(), warnings: vec![] })
    }
}

/// .kgbone 구조 정보: 이름 → 값 사전인 곳.
struct KgboneSchema;

impl Schema for KgboneSchema {
    fn is_dict(&self, p: &[String]) -> bool {
        const A: &str = "animations";
        path_is(p, &["skeleton", "skins", "#", "attachments"])
            || path_is(p, &["skeleton", "skins", "#", "attachments", "*"])
            || ["bones", "slots", "constraints"]
                .iter()
                .any(|k| path_is(p, &["skeleton", A, "#", k]) || path_is(p, &["skeleton", A, "#", k, "*"]))
            || ["deform", "sequence"].iter().any(|k| {
                path_is(p, &["skeleton", A, "#", k])
                    || path_is(p, &["skeleton", A, "#", k, "*"])
                    || path_is(p, &["skeleton", A, "#", k, "*", "*"])
            })
    }
    fn owns(&self, _path: &[String], _key: &str) -> bool {
        false
    }
}
