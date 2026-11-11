//! 기본 타입 ↔ JValue 변환.
use super::{format_f32, JValue};
use crate::{CoreError, Result};

/// 모델 → JSON. `full`이 참이면 기본값도 생략하지 않는다.
pub trait ToJson {
    fn to_json(&self, full: bool) -> JValue;
}

/// JSON → 모델.
pub trait FromJson: Sized {
    fn from_json(v: &JValue) -> Result<Self>;
}

/// 기본값 비교(매크로용).
pub fn same<T: PartialEq>(a: &T, b: &T) -> bool {
    a == b
}

fn num(v: &JValue) -> Result<f64> {
    match v {
        JValue::Num(s) => s.parse::<f64>().map_err(|_| CoreError::Parse(format!("숫자 형식이 잘못됨: {s}"))),
        JValue::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
        _ => Err(CoreError::Parse("숫자가 필요합니다".into())),
    }
}

impl ToJson for f32 {
    fn to_json(&self, _: bool) -> JValue {
        JValue::Num(format_f32(*self))
    }
}
impl FromJson for f32 {
    fn from_json(v: &JValue) -> Result<Self> {
        Ok(num(v)? as f32)
    }
}
impl ToJson for f64 {
    fn to_json(&self, _: bool) -> JValue {
        JValue::num(*self)
    }
}
impl FromJson for f64 {
    fn from_json(v: &JValue) -> Result<Self> {
        num(v)
    }
}
impl ToJson for u32 {
    fn to_json(&self, _: bool) -> JValue {
        JValue::Num(self.to_string())
    }
}
impl FromJson for u32 {
    fn from_json(v: &JValue) -> Result<Self> {
        Ok(num(v)?.max(0.0).round() as u32)
    }
}
impl ToJson for i32 {
    fn to_json(&self, _: bool) -> JValue {
        JValue::Num(self.to_string())
    }
}
impl FromJson for i32 {
    fn from_json(v: &JValue) -> Result<Self> {
        Ok(num(v)?.round() as i32)
    }
}
impl ToJson for usize {
    fn to_json(&self, _: bool) -> JValue {
        JValue::Num(self.to_string())
    }
}
impl FromJson for usize {
    fn from_json(v: &JValue) -> Result<Self> {
        Ok(num(v)?.max(0.0).round() as usize)
    }
}
impl ToJson for bool {
    fn to_json(&self, _: bool) -> JValue {
        JValue::Bool(*self)
    }
}
impl FromJson for bool {
    fn from_json(v: &JValue) -> Result<Self> {
        match v {
            JValue::Bool(b) => Ok(*b),
            JValue::Num(_) => Ok(num(v)? != 0.0),
            _ => Err(CoreError::Parse("참/거짓이 필요합니다".into())),
        }
    }
}
impl ToJson for String {
    fn to_json(&self, _: bool) -> JValue {
        JValue::Str(self.clone())
    }
}
impl FromJson for String {
    fn from_json(v: &JValue) -> Result<Self> {
        match v {
            JValue::Str(s) => Ok(s.clone()),
            JValue::Num(s) => Ok(s.clone()),
            _ => Err(CoreError::Parse("문자열이 필요합니다".into())),
        }
    }
}
impl<T: ToJson> ToJson for Option<T> {
    fn to_json(&self, full: bool) -> JValue {
        match self {
            Some(v) => v.to_json(full),
            None => JValue::Null,
        }
    }
}
impl<T: FromJson> FromJson for Option<T> {
    fn from_json(v: &JValue) -> Result<Self> {
        if v.is_null() {
            Ok(None)
        } else {
            T::from_json(v).map(Some)
        }
    }
}
impl<T: ToJson> ToJson for Vec<T> {
    fn to_json(&self, full: bool) -> JValue {
        JValue::Arr(self.iter().map(|x| x.to_json(full)).collect())
    }
}
impl<T: FromJson> FromJson for Vec<T> {
    fn from_json(v: &JValue) -> Result<Self> {
        match v {
            JValue::Arr(a) => a.iter().map(T::from_json).collect(),
            _ => Err(CoreError::Parse("배열이 필요합니다".into())),
        }
    }
}
impl<T: ToJson, const N: usize> ToJson for [T; N] {
    fn to_json(&self, full: bool) -> JValue {
        JValue::Arr(self.iter().map(|x| x.to_json(full)).collect())
    }
}
impl<T: FromJson + Default + Copy, const N: usize> FromJson for [T; N] {
    fn from_json(v: &JValue) -> Result<Self> {
        let a = v.as_array().ok_or_else(|| CoreError::Parse("배열이 필요합니다".into()))?;
        let mut out = [T::default(); N];
        for (i, x) in a.iter().take(N).enumerate() {
            out[i] = T::from_json(x)?;
        }
        Ok(out)
    }
}
impl ToJson for JValue {
    fn to_json(&self, _: bool) -> JValue {
        self.clone()
    }
}
impl FromJson for JValue {
    fn from_json(v: &JValue) -> Result<Self> {
        Ok(v.clone())
    }
}
