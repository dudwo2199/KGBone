//! Project → Spine JSON.
use super::SpineExportOptions;
use crate::format::{Original, WriteResult};
use crate::model::Project;
use crate::{CoreError, Result};

pub(super) fn write(project: &Project, original: Option<&Original>, opts: &SpineExportOptions) -> Result<WriteResult> {
    let _ = (project, original, opts);
    Err(CoreError::NotImplemented("Spine JSON 내보내기"))
}
