//! 내보내기 묶음: Spine JSON 호환 + .atlas + 페이지 PNG, PNG 시퀀스.
//! 파일 쓰기는 호출하는 쪽이 한다(앱 본체: 디스크, 화면: 다운로드). 여기서는 (파일 이름, 바이트) 목록만 만든다.
use super::{Image, PackOptions};
use crate::document::Document;
use crate::format::spine_json::SpineExportOptions;
use crate::{CoreError, Result, Warning};

/// 이미지 경로(확장자 없음, imagesPath 기준) → 픽셀.
pub trait ImageSource {
    fn load(&self, path: &str) -> Option<Image>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpineBundleOptions {
    /// 출력 파일 이름 앞부분(예: `hero` → hero.json, hero.atlas, hero.png).
    pub name: String,
    pub spine: SpineExportOptions,
    pub pack: PackOptions,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PngSequenceOptions {
    pub name: String,
    pub animation: Option<String>,
    pub skin: Option<String>,
    pub fps: f32,
    /// 월드 → 픽셀 배율.
    pub scale: f32,
    /// 0이면 셋업 포즈 경계 상자 + 여백으로 자동.
    pub width: u32,
    pub height: u32,
    pub background: [u8; 4],
}

impl Default for PngSequenceOptions {
    fn default() -> Self {
        PngSequenceOptions { name: "frame".into(), animation: None, skin: None, fps: 30.0, scale: 1.0, width: 0, height: 0, background: [0; 4] }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Bundle {
    pub files: Vec<(String, Vec<u8>)>,
    pub warnings: Vec<Warning>,
}

/// `name.json` + `name.atlas` + 페이지 PNG. 첨부물이 쓰는 이미지만 패킹한다. 없는 이미지는 경고.
pub fn export_spine_bundle(doc: &Document, images: &dyn ImageSource, opts: &SpineBundleOptions) -> Result<Bundle> {
    let _ = (doc, images, opts);
    Err(CoreError::NotImplemented("Spine 묶음 내보내기"))
}

/// `name_0000.png` … (프레임 수 = floor(길이 × fps) + 1).
pub fn export_png_sequence(doc: &mut Document, images: &dyn ImageSource, opts: &PngSequenceOptions) -> Result<Bundle> {
    let _ = (doc, images, opts);
    Err(CoreError::NotImplemented("PNG 시퀀스 내보내기"))
}
