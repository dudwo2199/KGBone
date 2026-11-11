//! 순서를 지키는 이름 → 값 사전(JSON 객체로 저장).
use crate::json::{FromJson, JValue, ToJson};
use crate::{CoreError, Result};

#[derive(Clone, Debug, PartialEq)]
pub struct OMap<V>(Vec<(String, V)>);

impl<V> Default for OMap<V> {
    fn default() -> Self {
        OMap(Vec::new())
    }
}

impl<V> OMap<V> {
    pub fn new() -> Self {
        OMap(Vec::new())
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn get(&self, k: &str) -> Option<&V> {
        self.0.iter().find(|(n, _)| n == k).map(|(_, v)| v)
    }
    pub fn get_mut(&mut self, k: &str) -> Option<&mut V> {
        self.0.iter_mut().find(|(n, _)| n == k).map(|(_, v)| v)
    }
    pub fn contains_key(&self, k: &str) -> bool {
        self.get(k).is_some()
    }
    /// 넣거나 바꾼다. 이미 있으면 자리를 유지한다.
    pub fn insert(&mut self, k: impl Into<String>, v: V) -> Option<V> {
        let k = k.into();
        if let Some(slot) = self.0.iter_mut().find(|(n, _)| *n == k) {
            return Some(std::mem::replace(&mut slot.1, v));
        }
        self.0.push((k, v));
        None
    }
    pub fn remove(&mut self, k: &str) -> Option<V> {
        let i = self.0.iter().position(|(n, _)| n == k)?;
        Some(self.0.remove(i).1)
    }
    /// 키 이름을 바꾼다(자리 유지). 새 이름이 이미 있으면 false.
    pub fn rename(&mut self, from: &str, to: &str) -> bool {
        if from == to {
            return true;
        }
        if self.contains_key(to) {
            return false;
        }
        match self.0.iter_mut().find(|(n, _)| n == from) {
            Some(slot) => {
                slot.0 = to.to_string();
                true
            }
            None => false,
        }
    }
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.0.iter().map(|(k, _)| k)
    }
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.0.iter().map(|(_, v)| v)
    }
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.0.iter_mut().map(|(_, v)| v)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&String, &V)> {
        self.0.iter().map(|(k, v)| (k, v))
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&String, &mut V)> {
        self.0.iter_mut().map(|(k, v)| (&*k, v))
    }
    pub fn retain(&mut self, mut f: impl FnMut(&str, &mut V) -> bool) {
        self.0.retain_mut(|(k, v)| f(k, v));
    }
}

impl<V: Default> OMap<V> {
    /// 없으면 기본값을 넣고 그 값을 돌려준다.
    pub fn entry(&mut self, k: &str) -> &mut V {
        if !self.contains_key(k) {
            self.0.push((k.to_string(), V::default()));
        }
        self.get_mut(k).expect("방금 넣음")
    }
}

impl<V: ToJson> ToJson for OMap<V> {
    fn to_json(&self, full: bool) -> JValue {
        JValue::Obj(self.0.iter().map(|(k, v)| (k.clone(), v.to_json(full))).collect())
    }
}

impl<V: FromJson> FromJson for OMap<V> {
    fn from_json(v: &JValue) -> Result<Self> {
        match v {
            JValue::Obj(m) => Ok(OMap(m.iter().map(|(k, x)| Ok((k.clone(), V::from_json(x)?))).collect::<Result<Vec<_>>>()?)),
            _ => Err(CoreError::Parse("객체가 필요합니다".into())),
        }
    }
}
