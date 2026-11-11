//! 첨부물 데이터. `type` 값으로 종류를 구분한다.
use super::Color;
use crate::json::{FromJson, JValue, ToJson};
use crate::{json_object, CoreError, Result};

/// 프레임 넘김(시퀀스).
#[derive(Clone, Debug, PartialEq)]
pub struct Sequence {
    pub count: u32,
    pub start: u32,
    pub digits: u32,
    pub setup: u32,
}
json_object!(Sequence { count: "count" = 1, start: "start" = 1, digits: "digits" = 0, setup: "setup" = 0 });

#[derive(Clone, Debug, PartialEq)]
pub struct RegionAttachment {
    pub path: Option<String>,
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub width: f32,
    pub height: f32,
    pub color: Color,
    pub sequence: Option<Sequence>,
}
json_object!(RegionAttachment {
    path: "path" = None,
    x: "x" = 0.0,
    y: "y" = 0.0,
    rotation: "rotation" = 0.0,
    scale_x: "scaleX" = 1.0,
    scale_y: "scaleY" = 1.0,
    width: "width" = 32.0,
    height: "height" = 32.0,
    color: "color" = Color::WHITE,
    sequence: "sequence" = None,
});

impl Default for RegionAttachment {
    fn default() -> Self {
        RegionAttachment {
            path: None,
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            width: 32.0,
            height: 32.0,
            color: Color::WHITE,
            sequence: None,
        }
    }
}

/// 메시. `weighted`가 거짓이면 vertices = [x, y, …](슬롯 본 기준),
/// 참이면 정점마다 [본 수 n, (본 번호, x, y, 가중치)×n].
#[derive(Clone, Debug, PartialEq)]
pub struct MeshAttachment {
    pub path: Option<String>,
    pub width: f32,
    pub height: f32,
    pub color: Color,
    pub uvs: Vec<f32>,
    pub triangles: Vec<u32>,
    pub weighted: bool,
    pub vertices: Vec<f32>,
    /// 외곽선 정점 수(앞에서부터).
    pub hull: u32,
    pub edges: Vec<u32>,
    pub sequence: Option<Sequence>,
}
json_object!(MeshAttachment {
    path: "path" = None,
    width: "width" = 32.0,
    height: "height" = 32.0,
    color: "color" = Color::WHITE,
    uvs: "uvs" = Vec::new(),
    triangles: "triangles" = Vec::new(),
    weighted: "weighted" = false,
    vertices: "vertices" = Vec::new(),
    hull: "hull" = 0,
    edges: "edges" = Vec::new(),
    sequence: "sequence" = None,
});

impl Default for MeshAttachment {
    fn default() -> Self {
        MeshAttachment {
            path: None,
            width: 32.0,
            height: 32.0,
            color: Color::WHITE,
            uvs: vec![],
            triangles: vec![],
            weighted: false,
            vertices: vec![],
            hull: 0,
            edges: vec![],
            sequence: None,
        }
    }
}

impl MeshAttachment {
    /// 정점 수 = uv 수 / 2.
    pub fn vertex_count(&self) -> usize {
        self.uvs.len() / 2
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinkedMeshAttachment {
    pub path: Option<String>,
    pub width: f32,
    pub height: f32,
    pub color: Color,
    /// 부모 메시 첨부물 이름(같은 슬롯).
    pub parent: String,
    /// 부모 메시가 있는 스킨. 없으면 기본 스킨.
    pub skin: Option<String>,
    /// 부모의 변형(deform) 키를 물려받는지.
    pub timelines: bool,
    pub sequence: Option<Sequence>,
}
json_object!(LinkedMeshAttachment {
    path: "path" = None,
    width: "width" = 32.0,
    height: "height" = 32.0,
    color: "color" = Color::WHITE,
    parent: "parent" = String::new(),
    skin: "skin" = None,
    timelines: "timelines" = true,
    sequence: "sequence" = None,
});

#[derive(Clone, Debug, PartialEq)]
pub struct BoundingBoxAttachment {
    pub vertex_count: u32,
    pub weighted: bool,
    pub vertices: Vec<f32>,
    pub color: Color,
}
json_object!(BoundingBoxAttachment {
    vertex_count: "vertexCount" = 0,
    weighted: "weighted" = false,
    vertices: "vertices" = Vec::new(),
    color: "color" = Color::BOUNDING_BOX,
});

#[derive(Clone, Debug, PartialEq)]
pub struct PointAttachment {
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
    pub color: Color,
}
json_object!(PointAttachment { x: "x" = 0.0, y: "y" = 0.0, rotation: "rotation" = 0.0, color: "color" = Color::POINT });

#[derive(Clone, Debug, PartialEq)]
pub struct ClippingAttachment {
    /// 클리핑이 끝나는 슬롯 이름(그 슬롯까지 잘린다).
    pub end: Option<String>,
    pub vertex_count: u32,
    pub weighted: bool,
    pub vertices: Vec<f32>,
    /// 반전: 다각형 바깥만 보이게.
    pub inverse: bool,
    /// 볼록: 오목 다각형을 볼록 껍질로 바꿔 자른다.
    pub convex: bool,
    pub color: Color,
}
json_object!(ClippingAttachment {
    end: "end" = None,
    vertex_count: "vertexCount" = 0,
    weighted: "weighted" = false,
    vertices: "vertices" = Vec::new(),
    inverse: "inverse" = false,
    convex: "convex" = false,
    color: "color" = Color::CLIPPING,
});

#[derive(Clone, Debug, PartialEq)]
pub struct PathAttachment {
    pub closed: bool,
    pub constant_speed: bool,
    pub vertex_count: u32,
    pub weighted: bool,
    pub vertices: Vec<f32>,
    /// 곡선 구간별 누적 길이.
    pub lengths: Vec<f32>,
    pub color: Color,
}
json_object!(PathAttachment {
    closed: "closed" = false,
    constant_speed: "constantSpeed" = true,
    vertex_count: "vertexCount" = 0,
    weighted: "weighted" = false,
    vertices: "vertices" = Vec::new(),
    lengths: "lengths" = Vec::new(),
    color: "color" = Color::PATH,
});

/// 첨부물 종류.
#[derive(Clone, Debug, PartialEq)]
pub enum Attachment {
    Region(RegionAttachment),
    Mesh(MeshAttachment),
    LinkedMesh(LinkedMeshAttachment),
    BoundingBox(BoundingBoxAttachment),
    Point(PointAttachment),
    Clipping(ClippingAttachment),
    Path(PathAttachment),
}

/// `{"type": 이름, ...본문}` 모양으로 만든다.
pub(crate) fn tagged(type_name: &str, body: JValue) -> JValue {
    let mut m = vec![("type".to_string(), JValue::Str(type_name.into()))];
    if let JValue::Obj(b) = body {
        m.extend(b);
    }
    JValue::Obj(m)
}

impl ToJson for Attachment {
    fn to_json(&self, full: bool) -> JValue {
        let body = match self {
            Attachment::Region(a) => a.to_json(full),
            Attachment::Mesh(a) => a.to_json(full),
            Attachment::LinkedMesh(a) => a.to_json(full),
            Attachment::BoundingBox(a) => a.to_json(full),
            Attachment::Point(a) => a.to_json(full),
            Attachment::Clipping(a) => a.to_json(full),
            Attachment::Path(a) => a.to_json(full),
        };
        tagged(self.type_name(), body)
    }
}

impl FromJson for Attachment {
    fn from_json(v: &JValue) -> Result<Self> {
        let t = v.get("type").and_then(JValue::as_str).unwrap_or("region");
        Ok(match t {
            "region" => Attachment::Region(FromJson::from_json(v)?),
            "mesh" => Attachment::Mesh(FromJson::from_json(v)?),
            "linkedmesh" => Attachment::LinkedMesh(FromJson::from_json(v)?),
            "boundingbox" => Attachment::BoundingBox(FromJson::from_json(v)?),
            "point" => Attachment::Point(FromJson::from_json(v)?),
            "clipping" => Attachment::Clipping(FromJson::from_json(v)?),
            "path" => Attachment::Path(FromJson::from_json(v)?),
            other => return Err(CoreError::Parse(format!("모르는 첨부물 종류: {other}"))),
        })
    }
}

impl Attachment {
    pub const TYPES: [&'static str; 7] = ["region", "mesh", "linkedmesh", "boundingbox", "point", "clipping", "path"];

    pub fn type_name(&self) -> &'static str {
        match self {
            Attachment::Region(_) => "region",
            Attachment::Mesh(_) => "mesh",
            Attachment::LinkedMesh(_) => "linkedmesh",
            Attachment::BoundingBox(_) => "boundingbox",
            Attachment::Point(_) => "point",
            Attachment::Clipping(_) => "clipping",
            Attachment::Path(_) => "path",
        }
    }

    /// 이미지 경로(있는 종류만). `key`는 스킨 안에서의 첨부물 이름.
    pub fn image_path<'a>(&'a self, key: &'a str) -> Option<&'a str> {
        match self {
            Attachment::Region(a) => Some(a.path.as_deref().unwrap_or(key)),
            Attachment::Mesh(a) => Some(a.path.as_deref().unwrap_or(key)),
            Attachment::LinkedMesh(a) => Some(a.path.as_deref().unwrap_or(key)),
            _ => None,
        }
    }

    /// 정점 목록을 가진 종류의 (weighted, vertices).
    pub fn vertices(&self) -> Option<(bool, &[f32])> {
        match self {
            Attachment::Mesh(a) => Some((a.weighted, &a.vertices)),
            Attachment::BoundingBox(a) => Some((a.weighted, &a.vertices)),
            Attachment::Clipping(a) => Some((a.weighted, &a.vertices)),
            Attachment::Path(a) => Some((a.weighted, &a.vertices)),
            _ => None,
        }
    }

    /// 종류 이름으로 기본값 첨부물을 만든다.
    pub fn new_of_type(t: &str) -> Option<Attachment> {
        let square = vec![-16.0, -16.0, 16.0, -16.0, 16.0, 16.0, -16.0, 16.0];
        Some(match t {
            "region" => Attachment::Region(RegionAttachment::default()),
            "mesh" => Attachment::Mesh(MeshAttachment {
                uvs: vec![0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0],
                triangles: vec![0, 1, 2, 0, 2, 3],
                vertices: square,
                hull: 4,
                ..MeshAttachment::default()
            }),
            "linkedmesh" => Attachment::LinkedMesh(LinkedMeshAttachment {
                path: None,
                width: 32.0,
                height: 32.0,
                color: Color::WHITE,
                parent: String::new(),
                skin: None,
                timelines: true,
                sequence: None,
            }),
            "boundingbox" => {
                Attachment::BoundingBox(BoundingBoxAttachment { vertex_count: 4, weighted: false, vertices: square, color: Color::BOUNDING_BOX })
            }
            "point" => Attachment::Point(PointAttachment { x: 0.0, y: 0.0, rotation: 0.0, color: Color::POINT }),
            "clipping" => Attachment::Clipping(ClippingAttachment {
                end: None,
                vertex_count: 4,
                weighted: false,
                vertices: square,
                inverse: false,
                convex: false,
                color: Color::CLIPPING,
            }),
            "path" => Attachment::Path(PathAttachment {
                closed: false,
                constant_speed: true,
                vertex_count: 6,
                weighted: false,
                vertices: vec![-60.0, 0.0, -50.0, 0.0, -40.0, 20.0, 40.0, 20.0, 50.0, 0.0, 60.0, 0.0],
                lengths: vec![100.0],
                color: Color::PATH,
            }),
            _ => return None,
        })
    }
}
