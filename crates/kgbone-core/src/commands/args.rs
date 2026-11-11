//! 명령 인자(JSON 객체) 읽기 도우미.
use crate::{CoreError, Result};
use crate::json::JValue as Value;

pub struct Args<'a>(&'a Value);

impl<'a> Args<'a> {
    pub fn new(v: &'a Value) -> Self {
        Args(v)
    }
    pub fn raw(&self) -> &'a Value {
        self.0
    }
    pub fn has(&self, key: &str) -> bool {
        self.0.get(key).is_some()
    }
    pub fn get(&self, key: &str) -> Option<&'a Value> {
        self.0.get(key).filter(|v| !v.is_null())
    }
    pub fn str(&self, key: &str) -> Result<&'a str> {
        self.opt_str(key).ok_or_else(|| CoreError::InvalidArgs(format!("'{key}' 문자열이 필요합니다")))
    }
    pub fn opt_str(&self, key: &str) -> Option<&'a str> {
        self.get(key).and_then(Value::as_str)
    }
    pub fn f32(&self, key: &str) -> Result<f32> {
        self.opt_f32(key).ok_or_else(|| CoreError::InvalidArgs(format!("'{key}' 숫자가 필요합니다")))
    }
    pub fn opt_f32(&self, key: &str) -> Option<f32> {
        self.get(key).and_then(Value::as_f64).map(|v| v as f32)
    }
    pub fn opt_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(Value::as_bool)
    }
    pub fn opt_usize(&self, key: &str) -> Option<usize> {
        self.get(key).and_then(Value::as_f64).filter(|v| *v >= 0.0).map(|v| v as usize)
    }
    pub fn str_list(&self, key: &str) -> Vec<String> {
        self.get(key)
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default()
    }
    pub fn f32_list(&self, key: &str) -> Option<Vec<f32>> {
        self.get(key).and_then(Value::as_array).map(|a| a.iter().map(|x| x.as_f64().unwrap_or(0.0) as f32).collect())
    }
    pub fn u32_list(&self, key: &str) -> Option<Vec<u32>> {
        self.get(key).and_then(Value::as_array).map(|a| a.iter().map(|x| x.as_f64().unwrap_or(0.0).max(0.0) as u32).collect())
    }
    /// 객체 인자(없으면 None).
    pub fn obj(&self, key: &str) -> Option<&'a Value> {
        self.get(key).filter(|v| v.as_object().is_some())
    }
}
