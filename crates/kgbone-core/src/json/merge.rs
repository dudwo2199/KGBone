//! 원본 트리와 새 값을 합친다.
//!
//! 규칙(docs/FORMAT.md 1장):
//! - 원본에 있던 키는 원래 순서대로 남긴다. `full`에 없는 키 = 모델이 모르는 키 → 원본 그대로 보존.
//! - `full`에서 null인 키 = 모델에서 없어진 값 → 지운다.
//! - 숫자는 f32로 비교해 같으면 원본 글자를 유지, 다르면 새 값.
//! - 색 문자열(16진)은 대소문자 무시 비교.
//! - 원본에 없던 키는 `sparse`(기본값 생략판)에 있을 때만 끝에 덧붙인다.
//! - 배열: 원소가 모두 `name` 문자열을 가진 객체면 이름으로 짝짓고, 아니면 위치로 짝짓는다. 순서는 새 값을 따른다.
use super::JValue;

/// `raw`: 원본 트리, `full`: 모델을 생략 없이 바꾼 값, `sparse`: 기본값을 생략한 값(없으면 full을 쓴다).
pub fn merge(raw: &JValue, full: &JValue, sparse: Option<&JValue>) -> JValue {
    let sparse = sparse.unwrap_or(full);
    match (raw, full) {
        (JValue::Obj(rm), JValue::Obj(fm)) => {
            let mut out: Vec<(String, JValue)> = Vec::with_capacity(rm.len());
            for (k, rv) in rm {
                match fm.iter().find(|(fk, _)| fk == k) {
                    None => out.push((k.clone(), rv.clone())),
                    Some((_, fv)) if fv.is_null() => {}
                    Some((_, fv)) => out.push((k.clone(), merge(rv, fv, sparse.get(k)))),
                }
            }
            if let JValue::Obj(sm) = sparse {
                for (k, sv) in sm {
                    if !rm.iter().any(|(rk, _)| rk == k) && !sv.is_null() {
                        out.push((k.clone(), sv.clone()));
                    }
                }
            }
            JValue::Obj(out)
        }
        (JValue::Arr(ra), JValue::Arr(fa)) => {
            let sa = sparse.as_array();
            let by_name = !ra.is_empty() && ra.iter().chain(fa.iter()).all(|x| x.get("name").and_then(JValue::as_str).is_some());
            let mut out = Vec::with_capacity(fa.len());
            for (i, fv) in fa.iter().enumerate() {
                let sv = sa.and_then(|a| a.get(i));
                let rv = if by_name {
                    let name = fv.get("name").and_then(JValue::as_str);
                    ra.iter().find(|r| r.get("name").and_then(JValue::as_str) == name)
                } else {
                    ra.get(i)
                };
                out.push(match rv {
                    Some(rv) => merge(rv, fv, sv),
                    None => sv.unwrap_or(fv).clone(),
                });
            }
            JValue::Arr(out)
        }
        (JValue::Num(rn), JValue::Num(fnum)) => {
            let (a, b) = (rn.parse::<f64>().unwrap_or(f64::NAN), fnum.parse::<f64>().unwrap_or(f64::NAN));
            if a == b || (a as f32) == (b as f32) {
                raw.clone()
            } else {
                full.clone()
            }
        }
        (JValue::Str(rs), JValue::Str(fs)) => {
            if rs == fs || (is_hex(rs) && is_hex(fs) && rs.eq_ignore_ascii_case(fs)) {
                raw.clone()
            } else {
                full.clone()
            }
        }
        (r, f) if r == f => raw.clone(),
        _ => sparse.clone(),
    }
}

fn is_hex(s: &str) -> bool {
    (s.len() == 6 || s.len() == 8) && s.bytes().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::parse;

    fn j(s: &str) -> JValue {
        parse(s).unwrap()
    }

    #[test]
    fn keeps_order_notation_and_unknown() {
        let raw = j(r#"{"y": 1.50, "name": "a", "x-unknown": {"k": 1}, "color": "ff0000ff"}"#);
        let full = j(r#"{"name": "a", "y": 1.5, "color": "FF0000FF", "rotation": 0}"#);
        let sparse = j(r#"{"name": "a", "y": 1.5, "color": "FF0000FF"}"#);
        let m = merge(&raw, &full, Some(&sparse));
        assert_eq!(m, raw);
    }

    #[test]
    fn changed_value_and_removed_and_added() {
        let raw = j(r#"{"name": "a", "x": 1.0, "parent": "p"}"#);
        let full = j(r#"{"name": "b", "x": 1, "parent": null, "length": 5}"#);
        let sparse = j(r#"{"name": "b", "x": 1, "length": 5}"#);
        assert_eq!(merge(&raw, &full, Some(&sparse)), j(r#"{"name": "b", "x": 1.0, "length": 5}"#));
    }

    #[test]
    fn named_arrays_match_by_name() {
        let raw = j(r#"[{"name": "a", "keep": 1}, {"name": "b", "keep": 2}]"#);
        let full = j(r#"[{"name": "b"}, {"name": "c"}]"#);
        assert_eq!(merge(&raw, &full, None), j(r#"[{"name": "b", "keep": 2}, {"name": "c"}]"#));
    }
}
