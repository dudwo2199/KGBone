//! .kgbone 구버전 → 현재 버전 변환(FORMAT.md 7장).
use crate::json::JValue;
use crate::model::Project;
use crate::{CoreError, Result, Warning};

/// 변환했을 때 내는 경고 코드.
pub(crate) const MIGRATED: &str = "kgbone.migrated";

/// formatVersion 0 의 종류별 제약 배열 키(합칠 때의 종류 순서).
const LEGACY_KINDS: [&str; 4] = ["ik", "transform", "path", "physics"];

/// 원본 트리를 현재 formatVersion 구조로 바꾼다. 바꾼 내용은 경고로 알린다.
pub(crate) fn migrate(mut tree: JValue) -> Result<(JValue, Vec<Warning>)> {
    if !matches!(tree, JValue::Obj(_)) {
        return Err(CoreError::Parse("최상위가 객체가 아닙니다".into()));
    }
    let version = tree.get("formatVersion").and_then(JValue::as_f64).unwrap_or(0.0);
    let mut warnings = vec![];
    if version >= 1.0 {
        if version > Project::FORMAT_VERSION as f64 {
            warnings.push(Warning::new(
                "kgbone.newerVersion",
                format!("이 파일은 더 새로운 형식(formatVersion {version})입니다. 모르는 내용은 보존만 합니다."),
                "",
            ));
        }
        return Ok((tree, warnings));
    }
    let moved = tree.get_mut("skeleton").map(merge_legacy_constraints).unwrap_or(0);
    // formatVersion을 맨 앞에 둔다(없었으면 넣고, 있었으면 값만 바꾼다).
    if tree.get("formatVersion").is_some() {
        tree.set("formatVersion", JValue::from(Project::FORMAT_VERSION as usize));
    } else if let JValue::Obj(m) = &mut tree {
        m.insert(0, ("formatVersion".into(), JValue::from(Project::FORMAT_VERSION as usize)));
    }
    warnings.push(Warning::new(
        MIGRATED,
        format!("초기 시험판 형식(formatVersion 0)을 현재 형식으로 바꿨습니다(제약 {moved}개를 적용 순서대로 합침)."),
        "",
    ));
    Ok((tree, warnings))
}

/// `ik`·`transform`·`path`·`physics` 배열을 `order` 순으로 합쳐 `constraints`로 만든다. 옮긴 제약 수를 돌려준다.
fn merge_legacy_constraints(skeleton: &mut JValue) -> usize {
    let JValue::Obj(m) = skeleton else { return 0 };
    let first_pos = m.iter().position(|(k, _)| LEGACY_KINDS.contains(&k.as_str()));
    // (order, 종류 순서, 원래 순서, 항목)
    let mut items: Vec<(f64, usize, usize, JValue)> = vec![];
    for (kind_i, kind) in LEGACY_KINDS.iter().enumerate() {
        let Some(pos) = m.iter().position(|(k, _)| k == kind) else { continue };
        let (_, arr) = m.remove(pos);
        for (i, item) in arr.as_array().cloned().unwrap_or_default().into_iter().enumerate() {
            let JValue::Obj(fields) = item else { continue };
            let order = fields.iter().find(|(k, _)| k == "order").and_then(|(_, v)| v.as_f64()).unwrap_or(0.0);
            let mut out = vec![("type".to_string(), JValue::str(*kind))];
            out.extend(fields.into_iter().filter(|(k, _)| k != "order" && k != "type"));
            items.push((order, kind_i, i, JValue::Obj(out)));
        }
    }
    let Some(first_pos) = first_pos else { return 0 };
    items.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    let count = items.len();
    let mut list: Vec<JValue> = items.into_iter().map(|x| x.3).collect();
    if let Some(pos) = m.iter().position(|(k, _)| k == "constraints") {
        // 이미 constraints가 있으면 그 뒤에 잇는다.
        if let JValue::Arr(existing) = &mut m[pos].1 {
            existing.append(&mut list);
        }
    } else {
        m.insert(first_pos.min(m.len()), ("constraints".into(), JValue::Arr(list)));
    }
    count
}
