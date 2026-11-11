//! 이미지 폴더 정보(크기·알파). 파일에는 저장하지 않는다.
use crate::mesh::AlphaMask;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    /// 알파 채널(행 우선, 위쪽 행부터). 메시 자동 생성에 쓴다.
    pub alpha: Option<AlphaMask>,
}

/// 이미지 경로(확장자 없음) → 정보.
#[derive(Clone, Debug, Default)]
pub struct ImageStore {
    map: HashMap<String, ImageInfo>,
}

impl ImageStore {
    pub fn set(&mut self, path: &str, width: u32, height: u32, alpha: Option<Vec<u8>>) {
        let alpha = alpha.map(|data| AlphaMask { width, height, data });
        self.map.insert(path.to_string(), ImageInfo { width, height, alpha });
    }
    pub fn get(&self, path: &str) -> Option<&ImageInfo> {
        self.map.get(path)
    }
}
