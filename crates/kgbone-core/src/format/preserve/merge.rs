//! 구조 정보를 아는 합치기.
use super::Schema;
use crate::json::{self, JValue};

/// `raw`: 원본 트리, `full`: 모델을 생략 없이 바꾼 값, `sparse`: 기본값을 생략한 값.
pub(crate) fn merge(raw: &JValue, full: &JValue, sparse: &JValue, schema: &dyn Schema) -> JValue {
    let mut path = Vec::new();
    merge_at(raw, full, Some(sparse), schema, &mut path)
}

fn merge_at(raw: &JValue, full: &JValue, sparse: Option<&JValue>, schema: &dyn Schema, path: &mut Vec<String>) -> JValue {
    let sparse = sparse.unwrap_or(full);
    match (raw, full) {
        (JValue::Obj(rm), JValue::Obj(fm)) => merge_obj(rm, fm, sparse, schema, path),
        (JValue::Arr(ra), JValue::Arr(fa)) => merge_arr(ra, fa, sparse, schema, path),
        (JValue::Obj(_) | JValue::Arr(_), _) | (_, JValue::Obj(_) | JValue::Arr(_)) => sparse.clone(),
        // 원시값끼리는 공용 규칙(숫자 f32 비교, 색 대소문자 무시)을 그대로 쓴다.
        _ => json::merge(raw, full, Some(sparse)),
    }
}

fn has_key(m: &[(String, JValue)], k: &str) -> bool {
    m.iter().any(|(x, _)| x == k)
}

fn merge_obj(
    rm: &[(String, JValue)],
    fm: &[(String, JValue)],
    sparse: &JValue,
    schema: &dyn Schema,
    path: &mut Vec<String>,
) -> JValue {
    let dict = schema.is_dict(path);
    // 사전에서 같은 자리의 키가 서로 상대 쪽에 없으면 이름이 바뀐 것으로 본다.
    let rename = |i: usize, k: &str| -> Option<String> {
        if !dict || has_key(fm, k) {
            return None;
        }
        let (fk, _) = fm.get(i)?;
        (!has_key(rm, fk)).then(|| fk.clone())
    };
    let mut out: Vec<(String, JValue)> = Vec::with_capacity(fm.len().max(rm.len()));
    for (i, (k, rv)) in rm.iter().enumerate() {
        let key = rename(i, k).unwrap_or_else(|| k.clone());
        match fm.iter().find(|(fk, _)| *fk == key) {
            None => {
                if !(dict || schema.owns(path, k)) {
                    out.push((k.clone(), rv.clone()));
                }
            }
            Some((_, fv)) if fv.is_null() => {
                if rv.is_null() {
                    out.push((k.clone(), JValue::Null));
                }
            }
            Some((_, fv)) => {
                path.push(key.clone());
                // 희소 값에 키가 없으면 기본값이라는 뜻: 객체라면 "덧붙일 키 없음"(빈 객체)으로 본다.
                let empty = JValue::obj();
                let sv = sparse.get(&key).or(matches!(fv, JValue::Obj(_)).then_some(&empty));
                let v = merge_at(rv, fv, sv, schema, path);
                path.pop();
                out.push((key, v));
            }
        }
    }
    if let JValue::Obj(sm) = sparse {
        for (k, sv) in sm {
            if !has_key(&out, k) && !has_key(rm, k) && !sv.is_null() {
                out.push((k.clone(), sv.clone()));
            }
        }
    }
    JValue::Obj(out)
}

fn name_of(v: &JValue) -> Option<&str> {
    v.get("name").and_then(JValue::as_str)
}

fn merge_arr(ra: &[JValue], fa: &[JValue], sparse: &JValue, schema: &dyn Schema, path: &mut Vec<String>) -> JValue {
    let sa = sparse.as_array();
    let by_name = !ra.is_empty() && ra.iter().chain(fa.iter()).all(|x| name_of(x).is_some());
    let mut used = vec![false; ra.len()];
    let mut out = Vec::with_capacity(fa.len());
    path.push("#".into());
    for (i, fv) in fa.iter().enumerate() {
        let sv = sa.and_then(|a| a.get(i));
        let pick = if by_name {
            let name = name_of(fv);
            ra.iter().enumerate().position(|(j, r)| !used[j] && name_of(r) == name).or_else(|| {
                // 이름이 바뀐 원소: 같은 위치의 원본 이름이 새 목록에 없으면 짝짓는다.
                let r = ra.get(i)?;
                (!used[i] && !fa.iter().any(|f| name_of(f) == name_of(r))).then_some(i)
            })
        } else {
            (i < ra.len()).then_some(i)
        };
        out.push(match pick {
            Some(j) => {
                used[j] = true;
                merge_at(&ra[j], fv, sv, schema, path)
            }
            None => sv.unwrap_or(fv).clone(),
        });
    }
    path.pop();
    JValue::Arr(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::preserve::path_is;
    use crate::json::parse;

    struct S;
    impl Schema for S {
        fn is_dict(&self, path: &[String]) -> bool {
            path_is(path, &["d"])
        }
        fn owns(&self, path: &[String], key: &str) -> bool {
            path.is_empty() && key == "old"
        }
    }

    fn j(s: &str) -> JValue {
        parse(s).unwrap()
    }

    #[test]
    fn dict_drops_and_renames_in_place() {
        let raw = j(r#"{"d": {"a": 1, "b": 2, "c": 3}, "old": 1, "x-keep": 2}"#);
        // b 삭제, c → c2 이름 바꿈(같은 자리 아님 → 삭제 + 추가), a 유지.
        let full = j(r#"{"d": {"a": 1, "c2": 3}}"#);
        // 자리 0: a는 그대로, 자리 1: b ↔ c2 → 이름 바꿈으로 본다. c는 지운다.
        assert_eq!(merge(&raw, &full, &full, &S), j(r#"{"d": {"a": 1, "c2": 3}, "x-keep": 2}"#));
    }

    #[test]
    fn duplicate_names_pair_in_order_and_rename_by_position() {
        let raw = j(r#"[{"name": "a", "t": 1.0}, {"name": "a", "t": 2.0}, {"name": "b", "k": 1e2}]"#);
        let full = j(r#"[{"name": "a", "t": 1}, {"name": "a", "t": 2}, {"name": "c", "k": 100}]"#);
        assert_eq!(merge(&raw, &full, &full, &S), j(r#"[{"name": "a", "t": 1.0}, {"name": "a", "t": 2.0}, {"name": "c", "k": 1e2}]"#));
    }

    #[test]
    fn null_stays_null() {
        let raw = j(r#"{"name": null, "p": "x"}"#);
        let full = j(r#"{"name": null, "p": null}"#);
        assert_eq!(merge(&raw, &full, &j("{}"), &S), j(r#"{"name": null}"#));
    }
}
