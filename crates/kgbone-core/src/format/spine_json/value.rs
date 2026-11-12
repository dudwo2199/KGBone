//! Spine JSON 값 읽기·만들기 도우미.
use crate::json::{FromJson, JValue, ToJson};
use crate::model::{Color, Curve, Key};
use crate::{CoreError, Result};

/// 읽기 오류에 위치를 붙인다.
pub(super) fn at(place: impl std::fmt::Display) -> impl Fn(CoreError) -> CoreError {
    move |e| match e {
        CoreError::Parse(m) => CoreError::Parse(format!("{place}: {m}")),
        other => other,
    }
}

fn typed<T: FromJson>(v: &JValue, k: &str) -> Result<Option<T>> {
    match v.get(k) {
        None | Some(JValue::Null) => Ok(None),
        Some(x) => T::from_json(x).map(Some).map_err(at(format!("'{k}'"))),
    }
}

/// 수(없으면 `d`).
pub(super) fn f(v: &JValue, k: &str, d: f32) -> Result<f32> {
    Ok(typed(v, k)?.unwrap_or(d))
}
/// 수(없으면 None).
pub(super) fn of(v: &JValue, k: &str) -> Result<Option<f32>> {
    typed(v, k)
}
pub(super) fn u(v: &JValue, k: &str, d: u32) -> Result<u32> {
    Ok(typed(v, k)?.unwrap_or(d))
}
pub(super) fn i(v: &JValue, k: &str) -> Result<Option<i32>> {
    typed(v, k)
}
pub(super) fn b(v: &JValue, k: &str, d: bool) -> Result<bool> {
    Ok(typed(v, k)?.unwrap_or(d))
}
pub(super) fn s(v: &JValue, k: &str) -> Result<Option<String>> {
    typed(v, k)
}
pub(super) fn strs(v: &JValue, k: &str) -> Result<Vec<String>> {
    Ok(typed(v, k)?.unwrap_or_default())
}
pub(super) fn floats(v: &JValue, k: &str) -> Result<Vec<f32>> {
    Ok(typed(v, k)?.unwrap_or_default())
}
pub(super) fn ints(v: &JValue, k: &str) -> Result<Vec<u32>> {
    Ok(typed(v, k)?.unwrap_or_default())
}
pub(super) fn color(v: &JValue, k: &str, d: Color) -> Result<Color> {
    Ok(typed(v, k)?.unwrap_or(d))
}

/// 객체의 (키, 값) 목록. 객체가 아니면 빈 목록.
pub(super) fn entries(v: Option<&JValue>) -> &[(String, JValue)] {
    v.and_then(JValue::as_object).map(Vec::as_slice).unwrap_or(&[])
}
/// 배열 원소 목록. 배열이 아니면 빈 목록.
pub(super) fn items(v: Option<&JValue>) -> &[JValue] {
    v.and_then(JValue::as_array).map(Vec::as_slice).unwrap_or(&[])
}

/// 키 공통 부분(time, curve)을 읽어 값 `v`와 함께 만든다.
pub(super) fn key(k: &JValue, v: Vec<f32>) -> Result<Key> {
    let curve = match k.get("curve") {
        None => Curve::Linear,
        Some(c) => Curve::from_json(c).map_err(at("'curve'"))?,
    };
    Ok(Key { time: f(k, "time", 0.0)?, v, s: None, offset: 0, curve })
}

/// 객체 만들기. `full`이면 기본값도 쓰고, 아니면 기본값과 같은 필드를 생략한다.
pub(super) struct O {
    m: Vec<(String, JValue)>,
    pub(super) full: bool,
}

impl O {
    pub(super) fn new(full: bool) -> O {
        O { m: vec![], full }
    }
    /// 항상 쓴다.
    pub(super) fn put(&mut self, k: &str, v: JValue) -> &mut Self {
        self.m.push((k.to_string(), v));
        self
    }
    /// `keep`이 참이거나 전체 모드면 쓴다.
    pub(super) fn put_if(&mut self, k: &str, keep: bool, v: JValue) -> &mut Self {
        if self.full || keep {
            self.put(k, v);
        }
        self
    }
    pub(super) fn f(&mut self, k: &str, v: f32, d: f32) -> &mut Self {
        self.put_if(k, v != d, JValue::from(v))
    }
    pub(super) fn b(&mut self, k: &str, v: bool, d: bool) -> &mut Self {
        self.put_if(k, v != d, JValue::Bool(v))
    }
    pub(super) fn s(&mut self, k: &str, v: &str, d: &str) -> &mut Self {
        self.put_if(k, v != d, JValue::str(v))
    }
    /// 있으면 쓰고, 없으면 전체 모드에서 null(합칠 때 원본 키를 지운다).
    pub(super) fn opt<T: ToJson>(&mut self, k: &str, v: Option<&T>) -> &mut Self {
        match v {
            Some(x) => self.put(k, x.to_json(self.full)),
            None => self.put_if(k, false, JValue::Null),
        }
    }
    pub(super) fn color(&mut self, k: &str, v: Color, d: Color) -> &mut Self {
        self.put_if(k, v != d, JValue::str(v.to_hex_rgba()))
    }
    /// 배열·객체: 비어 있으면 희소 모드에서 생략.
    pub(super) fn list(&mut self, k: &str, v: JValue) -> &mut Self {
        let empty = match &v {
            JValue::Arr(a) => a.is_empty(),
            JValue::Obj(m) => m.is_empty(),
            _ => false,
        };
        self.put_if(k, !empty, v)
    }
    pub(super) fn curve(&mut self, c: &Curve) -> &mut Self {
        self.put_if("curve", *c != Curve::Linear, c.to_json(self.full))
    }
    pub(super) fn done(&mut self) -> JValue {
        JValue::Obj(std::mem::take(&mut self.m))
    }
}

/// 수 목록 → JSON 배열.
pub(super) fn arr_f(v: &[f32]) -> JValue {
    JValue::Arr(v.iter().map(|x| JValue::from(*x)).collect())
}
pub(super) fn arr_u(v: &[u32]) -> JValue {
    JValue::Arr(v.iter().map(|x| JValue::Num(x.to_string())).collect())
}
pub(super) fn arr_s(v: &[String]) -> JValue {
    JValue::Arr(v.iter().map(|x| JValue::str(x.as_str())).collect())
}
