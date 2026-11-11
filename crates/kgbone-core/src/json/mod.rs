//! 키 순서와 숫자 표기를 그대로 보존하는 JSON.
//!
//! 외부 라이브러리를 쓰지 않는다(이 PC는 스마트 앱 컨트롤이 빌드 스크립트 실행을 막으므로, 핵심 모듈은 의존성이 없어야 한다).
//! - [`parse`]: 텍스트 → [`JValue`] (숫자는 원래 글자 그대로 `Num(String)`)
//! - [`merge`]: 원본 트리 + 모델에서 새로 만든 값 → 바뀐 부분만 바뀐 트리
//! - [`write`]: 트리 → 텍스트 (FORMAT.md "표준 출력 모양")
//! - [`ToJson`] / [`FromJson`]: 모델 ↔ 트리 변환. 구조체는 [`json_object!`] 매크로로 구현한다.
mod convert;
mod merge;
mod parse;
mod writer;

pub use convert::{same, FromJson, ToJson};
pub use merge::merge;
pub use parse::parse;
pub use writer::{detect_style, write, WriteStyle};

/// 보존형 JSON 값.
#[derive(Clone, Debug, PartialEq)]
pub enum JValue {
    Null,
    Bool(bool),
    /// 원래 글자 그대로의 숫자(예: `"1.50"`, `"1e2"`).
    Num(String),
    Str(String),
    Arr(Vec<JValue>),
    /// 키 순서를 유지하는 객체.
    Obj(Vec<(String, JValue)>),
}

impl JValue {
    pub fn obj() -> JValue {
        JValue::Obj(vec![])
    }
    pub fn num(f: f64) -> JValue {
        JValue::Num(format_f64(f))
    }
    pub fn str(s: impl Into<String>) -> JValue {
        JValue::Str(s.into())
    }
    pub fn get(&self, key: &str) -> Option<&JValue> {
        match self {
            JValue::Obj(m) => m.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    pub fn get_mut(&mut self, key: &str) -> Option<&mut JValue> {
        match self {
            JValue::Obj(m) => m.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    /// 객체에 키를 넣거나 바꾼다(없으면 끝에 추가). 객체가 아니면 아무것도 안 한다.
    pub fn set(&mut self, key: &str, v: JValue) {
        if let JValue::Obj(m) = self {
            if let Some(slot) = m.iter_mut().find(|(k, _)| k == key) {
                slot.1 = v;
            } else {
                m.push((key.to_string(), v));
            }
        }
    }
    pub fn remove(&mut self, key: &str) -> Option<JValue> {
        if let JValue::Obj(m) = self {
            if let Some(i) = m.iter().position(|(k, _)| k == key) {
                return Some(m.remove(i).1);
            }
        }
        None
    }
    /// 체인 편의: `v.with("a", x).with("b", y)`.
    pub fn with(mut self, key: &str, v: JValue) -> JValue {
        self.set(key, v);
        self
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            JValue::Num(s) => s.parse().ok(),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            JValue::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JValue::Bool(b) => Some(*b),
            _ => None,
        }
    }
    pub fn as_array(&self) -> Option<&Vec<JValue>> {
        match self {
            JValue::Arr(a) => Some(a),
            _ => None,
        }
    }
    pub fn as_object(&self) -> Option<&Vec<(String, JValue)>> {
        match self {
            JValue::Obj(m) => Some(m),
            _ => None,
        }
    }
    pub fn is_null(&self) -> bool {
        matches!(self, JValue::Null)
    }
    /// 한 줄 JSON 텍스트.
    pub fn to_compact(&self) -> String {
        write(self, &WriteStyle::compact())
    }
}

impl From<&str> for JValue {
    fn from(s: &str) -> Self {
        JValue::Str(s.into())
    }
}
impl From<String> for JValue {
    fn from(s: String) -> Self {
        JValue::Str(s)
    }
}
impl From<f64> for JValue {
    fn from(f: f64) -> Self {
        JValue::num(f)
    }
}
impl From<f32> for JValue {
    fn from(f: f32) -> Self {
        JValue::Num(format_f32(f))
    }
}
impl From<bool> for JValue {
    fn from(b: bool) -> Self {
        JValue::Bool(b)
    }
}
impl From<i64> for JValue {
    fn from(i: i64) -> Self {
        JValue::Num(i.to_string())
    }
}
impl From<usize> for JValue {
    fn from(i: usize) -> Self {
        JValue::Num(i.to_string())
    }
}

/// f32 값을 최단 왕복 표기로(`1.0` → `1`, `0.1` → `0.1`).
pub fn format_f32(f: f32) -> String {
    if !f.is_finite() {
        return "0".into();
    }
    if f == 0.0 {
        return "0".into();
    }
    if f.fract() == 0.0 && f.abs() < 1e15 {
        return format!("{}", f as i64);
    }
    format!("{f}")
}

/// f64 값을 표기로(정수면 소수점 없이).
pub fn format_f64(f: f64) -> String {
    if !f.is_finite() || f == 0.0 {
        return "0".into();
    }
    if f.fract() == 0.0 && f.abs() < 1e15 {
        return format!("{}", f as i64);
    }
    format!("{f}")
}

/// 구조체 ↔ JSON 객체 변환을 만든다.
///
/// ```ignore
/// json_object!(Bone {
///     name: "name" = String::new(),
///     length: "length" = 0.0,
/// });
/// ```
/// - 쓰기: 희소(full = false)면 기본값과 같은 필드를 생략, 전체(full = true)면 모두 쓴다(None은 null).
/// - 읽기: 없는 키는 기본값. 모르는 키는 무시(보존은 [`merge`]가 맡는다).
#[macro_export]
macro_rules! json_object {
    ($ty:ident { $( $field:ident : $key:literal = $default:expr ),* $(,)? }) => {
        impl $crate::json::ToJson for $ty {
            fn to_json(&self, full: bool) -> $crate::json::JValue {
                let mut m: Vec<(String, $crate::json::JValue)> = Vec::new();
                $(
                    {
                        #[allow(unused_mut)]
                        let mut write = full;
                        if !write {
                            let d = $default;
                            write = !$crate::json::same(&self.$field, &d);
                        }
                        if write {
                            m.push(($key.to_string(), $crate::json::ToJson::to_json(&self.$field, full)));
                        }
                    }
                )*
                $crate::json::JValue::Obj(m)
            }
        }
        impl $crate::json::FromJson for $ty {
            fn from_json(v: &$crate::json::JValue) -> $crate::Result<Self> {
                if !matches!(v, $crate::json::JValue::Obj(_)) {
                    return Err($crate::CoreError::Parse(format!("{}: 객체가 필요합니다", stringify!($ty))));
                }
                Ok($ty {
                    $(
                        $field: match v.get($key) {
                            Some(x) if !x.is_null() => $crate::json::FromJson::from_json(x)
                                .map_err(|e| $crate::CoreError::Parse(format!("{}.{}: {}", stringify!($ty), $key, e)))?,
                            _ => $default,
                        },
                    )*
                })
            }
        }
    };
}

/// 문자열 열거형 ↔ JSON 문자열.
#[macro_export]
macro_rules! json_string_enum {
    ($ty:ident { $( $variant:ident = $s:literal ),* $(,)? }) => {
        impl $ty {
            pub fn as_str(self) -> &'static str {
                match self { $( $ty::$variant => $s, )* }
            }
            pub fn parse(s: &str) -> Option<$ty> {
                match s { $( $s => Some($ty::$variant), )* _ => None }
            }
        }
        impl $crate::json::ToJson for $ty {
            fn to_json(&self, _full: bool) -> $crate::json::JValue {
                $crate::json::JValue::Str(self.as_str().to_string())
            }
        }
        impl $crate::json::FromJson for $ty {
            fn from_json(v: &$crate::json::JValue) -> $crate::Result<Self> {
                let s = v.as_str().ok_or_else(|| $crate::CoreError::Parse(format!("{}: 문자열이 필요합니다", stringify!($ty))))?;
                $ty::parse(s).ok_or_else(|| $crate::CoreError::Parse(format!("{}: 모르는 값 '{}'", stringify!($ty), s)))
            }
        }
    };
}
