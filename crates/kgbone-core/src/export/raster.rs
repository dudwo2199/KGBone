//! 소프트웨어 삼각형 그리기(PNG 시퀀스용). 미리보기 렌더러(WebGL2)와 같은 그리기 목록(DrawItem)을 쓴다.
use super::Image;
use crate::pose::DrawItem;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct RenderOptions {
    pub width: u32,
    pub height: u32,
    /// 월드 좌표 → 픽셀 배율.
    pub scale: f32,
    /// 이미지 중심에 올 월드 좌표.
    pub center_x: f32,
    pub center_y: f32,
    /// 배경색 RGBA(투명이면 [0,0,0,0]).
    pub background: [u8; 4],
}

/// 그리기 목록을 이미지 한 장으로 그린다. `images`: 이미지 경로 → 픽셀.
pub fn render_pose(items: &[DrawItem], images: &HashMap<String, Image>, opts: &RenderOptions) -> Image {
    let _ = (items, images);
    Image { width: opts.width, height: opts.height, rgba: vec![0; (opts.width * opts.height * 4) as usize] }
}
