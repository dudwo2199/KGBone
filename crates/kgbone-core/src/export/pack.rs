//! 간단한 텍스처 패킹(선반 방식, 공백 제거·회전 옵션).
use super::Image;
use crate::format::atlas::Atlas;

/// 패킹할 이미지 하나.
#[derive(Clone, Debug)]
pub struct PackInput {
    /// 영역 이름(첨부물 경로).
    pub name: String,
    pub image: Image,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PackOptions {
    pub max_width: u32,
    pub max_height: u32,
    /// 영역 사이 여백(픽셀).
    pub padding: u32,
    /// 투명한 가장자리 잘라내기.
    pub strip_whitespace: bool,
    /// 더 잘 들어가면 90도 회전 허용.
    pub rotation: bool,
    /// 크기를 2의 거듭제곱으로.
    pub power_of_two: bool,
    pub pma: bool,
    /// 페이지 이미지 이름 앞부분(예: `skeleton` → `skeleton.png`, `skeleton_2.png`).
    pub page_name: String,
}

impl Default for PackOptions {
    fn default() -> Self {
        PackOptions {
            max_width: 2048,
            max_height: 2048,
            padding: 2,
            strip_whitespace: true,
            rotation: false,
            power_of_two: true,
            pma: false,
            page_name: "skeleton".into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PackedPage {
    pub file_name: String,
    pub image: Image,
}

#[derive(Clone, Debug)]
pub struct PackResult {
    pub atlas: Atlas,
    pub pages: Vec<PackedPage>,
}

/// 영역이 서로 겹치지 않고 페이지 안에 들어가게 배치한다. 한 이미지가 최대 크기보다 크면 오류.
pub fn pack_images(inputs: &[PackInput], opts: &PackOptions) -> crate::Result<PackResult> {
    let _ = (inputs, opts);
    Err(crate::CoreError::NotImplemented("텍스처 패킹"))
}
