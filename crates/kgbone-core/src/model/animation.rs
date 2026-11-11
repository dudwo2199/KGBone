//! 애니메이션과 키프레임 데이터.
use super::OMap;
use crate::json::{FromJson, JValue, ToJson};
use crate::{json_object, CoreError, Result};

/// 키 사이 보간 방법.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum Curve {
    #[default]
    Linear,
    Stepped,
    /// 값 채널마다 [cx1, cy1, cx2, cy2] (절대 시간·값 좌표).
    Bezier(Vec<f32>),
}

impl ToJson for Curve {
    fn to_json(&self, full: bool) -> JValue {
        match self {
            // 선형은 "곡선 없음"이다. 전체 모드에서도 null로 써서 합치기 때 원본 curve 키가 지워지게 한다.
            Curve::Linear => JValue::Null,
            Curve::Stepped => JValue::Str("stepped".into()),
            Curve::Bezier(v) => v.to_json(full),
        }
    }
}

impl FromJson for Curve {
    fn from_json(v: &JValue) -> Result<Self> {
        match v {
            JValue::Null => Ok(Curve::Linear),
            JValue::Str(s) if s == "stepped" => Ok(Curve::Stepped),
            JValue::Str(s) if s == "linear" => Ok(Curve::Linear),
            JValue::Arr(_) => Ok(Curve::Bezier(Vec::<f32>::from_json(v)?)),
            _ => Err(CoreError::Parse("curve는 \"stepped\" 또는 숫자 배열이어야 합니다".into())),
        }
    }
}

/// 키 하나. 모든 타임라인이 같은 모양을 쓴다(FORMAT.md 3.7).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Key {
    pub time: f32,
    pub v: Vec<f32>,
    /// 문자열 값(첨부물 이름, 상속 종류, 시퀀스 모드). 첨부물 키에서 없음 = 비움.
    pub s: Option<String>,
    /// deform 키의 시작 위치(실수 번호).
    pub offset: u32,
    pub curve: Curve,
}
json_object!(Key {
    time: "time" = 0.0,
    v: "v" = Vec::new(),
    s: "s" = None,
    offset: "offset" = 0,
    curve: "curve" = Curve::Linear,
});

impl Key {
    pub fn new(time: f32, v: Vec<f32>) -> Self {
        Key { time, v, ..Default::default() }
    }
}

/// 타임라인 이름 → 키 목록(시간순).
pub type Timelines = OMap<Vec<Key>>;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct DrawOrderOffset {
    pub slot: String,
    pub offset: i32,
}
json_object!(DrawOrderOffset { slot: "slot" = String::new(), offset: "offset" = 0 });

#[derive(Clone, Debug, PartialEq, Default)]
pub struct DrawOrderKey {
    pub time: f32,
    pub offsets: Vec<DrawOrderOffset>,
}
json_object!(DrawOrderKey { time: "time" = 0.0, offsets: "offsets" = Vec::new() });

#[derive(Clone, Debug, PartialEq, Default)]
pub struct EventKey {
    pub time: f32,
    pub name: String,
    /// 없으면 이벤트 정의의 기본값.
    pub int: Option<i32>,
    pub float: Option<f32>,
    pub string: Option<String>,
    pub volume: Option<f32>,
    pub balance: Option<f32>,
}
json_object!(EventKey {
    time: "time" = 0.0,
    name: "name" = String::new(),
    int: "int" = None,
    float: "float" = None,
    string: "string" = None,
    volume: "volume" = None,
    balance: "balance" = None,
});

/// 스킨 → 슬롯 → 첨부물 → 키 목록.
pub type AttachmentKeys = OMap<OMap<OMap<Vec<Key>>>>;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Animation {
    pub name: String,
    pub bones: OMap<Timelines>,
    pub slots: OMap<Timelines>,
    pub constraints: OMap<Timelines>,
    pub deform: AttachmentKeys,
    pub sequence: AttachmentKeys,
    pub draw_order: Vec<DrawOrderKey>,
    pub events: Vec<EventKey>,
}
json_object!(Animation {
    name: "name" = String::new(),
    bones: "bones" = OMap::new(),
    slots: "slots" = OMap::new(),
    constraints: "constraints" = OMap::new(),
    deform: "deform" = OMap::new(),
    sequence: "sequence" = OMap::new(),
    draw_order: "drawOrder" = Vec::new(),
    events: "events" = Vec::new(),
});

impl Animation {
    pub fn new(name: impl Into<String>) -> Self {
        Animation { name: name.into(), ..Default::default() }
    }

    /// 마지막 키의 시간 = 애니메이션 길이.
    pub fn duration(&self) -> f32 {
        let mut d = 0.0f32;
        let mut see = |keys: &Vec<Key>| {
            if let Some(k) = keys.last() {
                d = d.max(k.time)
            }
        };
        for t in self.bones.values().chain(self.slots.values()).chain(self.constraints.values()) {
            t.values().for_each(&mut see);
        }
        for m in [&self.deform, &self.sequence] {
            for s in m.values() {
                for a in s.values() {
                    a.values().for_each(&mut see);
                }
            }
        }
        for k in &self.draw_order {
            d = d.max(k.time);
        }
        for k in &self.events {
            d = d.max(k.time);
        }
        d
    }
}

/// 타임라인 이름 목록(검증용).
pub mod timeline {
    pub const BONE: &[&str] =
        &["rotate", "translate", "translatex", "translatey", "scale", "scalex", "scaley", "shear", "shearx", "sheary", "inherit"];
    pub const SLOT: &[&str] = &["attachment", "rgba", "rgb", "alpha", "rgba2", "rgb2"];
    pub const IK: &[&str] = &["ik"];
    pub const TRANSFORM: &[&str] = &["transform"];
    pub const PATH: &[&str] = &["position", "spacing", "mix"];
    pub const PHYSICS: &[&str] = &["inertia", "strength", "damping", "mass", "wind", "gravity", "mix", "reset"];
    pub const SLIDER: &[&str] = &["time", "mix"];
}
