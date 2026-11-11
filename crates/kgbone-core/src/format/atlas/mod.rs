//! 텍스처 아틀라스(.atlas) 읽기·쓰기(공개 아틀라스 형식 문서 기준, docs/SOURCES.md S2).
mod read;
mod write;

pub use read::parse;
pub use write::write;

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Atlas {
    pub pages: Vec<AtlasPage>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AtlasPage {
    /// 페이지 이미지 파일 이름.
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// 예: `RGBA8888`. 없으면 None.
    pub format: Option<String>,
    /// (축소, 확대) 예: ("Linear", "Linear").
    pub filter: Option<(String, String)>,
    /// `x`, `y`, `xy`, `none`.
    pub repeat: Option<String>,
    pub pma: bool,
    /// 모르는 페이지 필드(이름, 원래 값 글자) — 그대로 다시 쓴다.
    pub extra: Vec<(String, String)>,
    pub regions: Vec<AtlasRegion>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AtlasRegion {
    pub name: String,
    /// 페이지 안 위치·크기(회전돼 있으면 저장된 그대로의 폭·높이가 아니라 원래 방향 폭·높이).
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    /// 잘라낸 공백: 왼쪽, 아래, 원래 폭, 원래 높이.
    pub offset_x: u32,
    pub offset_y: u32,
    pub original_width: u32,
    pub original_height: u32,
    /// 반시계 90도 회전 저장 여부(각도 값으로 저장된 경우 그 각도).
    pub degrees: u32,
    /// 시퀀스 번호(없으면 -1).
    pub index: i32,
    pub split: Option<[i32; 4]>,
    pub pad: Option<[i32; 4]>,
    /// 모르는 영역 필드.
    pub extra: Vec<(String, String)>,
}

impl AtlasRegion {
    pub fn new(name: impl Into<String>, x: u32, y: u32, width: u32, height: u32) -> Self {
        AtlasRegion {
            name: name.into(),
            x,
            y,
            width,
            height,
            offset_x: 0,
            offset_y: 0,
            original_width: width,
            original_height: height,
            degrees: 0,
            index: -1,
            split: None,
            pad: None,
            extra: vec![],
        }
    }
}
