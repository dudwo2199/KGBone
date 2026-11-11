//! 내보내기 묶음: Spine JSON 호환 + .atlas + 페이지 PNG, PNG 시퀀스.
//! 파일 쓰기는 호출하는 쪽이 한다(앱 본체: 디스크, 화면: 다운로드). 여기서는 (파일 이름, 바이트) 목록만 만든다.
use super::raster::{render_pose, RenderOptions};
use super::sequence::{sequence_times, SequenceOptions};
use super::{encode_png, pack_images, Image, PackInput, PackOptions};
use crate::document::Document;
use crate::format::spine_json::SpineExportOptions;
use crate::format::WriteResult;
use crate::model::{Attachment, Skeleton};
use crate::pose::{PhysicsStep, PoseOutput, PoseRequest};
use crate::{CoreError, Result, Warning};
use std::collections::HashMap;

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

/// 자동 크기일 때 경계 상자 둘레에 붙이는 여백(픽셀).
const AUTO_MARGIN: f32 = 10.0;

fn missing_image(path: &str) -> Warning {
    Warning::new("export.missingImage", format!("이미지를 찾을 수 없습니다: {path}"), path)
}

/// `name.json` + `name.atlas` + 페이지 PNG. 첨부물이 쓰는 이미지만 패킹한다. 없는 이미지는 경고.
pub fn export_spine_bundle(doc: &Document, images: &dyn ImageSource, opts: &SpineBundleOptions) -> Result<Bundle> {
    let json = doc.export_spine_json(&opts.spine)?;
    let paths = attachment_image_paths(&doc.project().skeleton);
    assemble_spine_bundle(json, &paths, images, opts)
}

/// 모든 스킨의 region/mesh/linkedmesh 첨부물이 쓰는 이미지 경로(처음 나온 순서, 중복 없음).
/// 시퀀스 첨부물은 경로 뒤에 번호(start부터 count개, digits 자리 0 채움)를 붙인 경로들.
fn attachment_image_paths(skel: &Skeleton) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |p: String| {
        if !out.contains(&p) {
            out.push(p);
        }
    };
    for skin in &skel.skins {
        for slot in skin.attachments.values() {
            for (key, att) in slot.iter() {
                let Some(path) = att.image_path(key) else { continue };
                let seq = match att {
                    Attachment::Region(a) => a.sequence.as_ref(),
                    Attachment::Mesh(a) => a.sequence.as_ref(),
                    Attachment::LinkedMesh(a) => a.sequence.as_ref(),
                    _ => None,
                };
                match seq {
                    Some(s) => {
                        for i in 0..s.count {
                            push(format!("{path}{:0width$}", s.start + i, width = s.digits as usize));
                        }
                    }
                    None => push(path.to_string()),
                }
            }
        }
    }
    out
}

/// JSON 결과 + 이미지 경로 목록 → 묶음(패킹·atlas·PNG). Document 없이 시험할 수 있게 나눠 둔다.
fn assemble_spine_bundle(json: WriteResult, paths: &[String], images: &dyn ImageSource, opts: &SpineBundleOptions) -> Result<Bundle> {
    let mut warnings = json.warnings;
    let mut inputs = Vec::with_capacity(paths.len());
    for p in paths {
        match images.load(p) {
            Some(image) => inputs.push(PackInput { name: p.clone(), image }),
            None => warnings.push(missing_image(p)),
        }
    }
    // 페이지 이름은 묶음 이름을 따른다(hero → hero.png, hero_2.png).
    let pack_opts = PackOptions { page_name: opts.name.clone(), ..opts.pack.clone() };
    let packed = pack_images(&inputs, &pack_opts)?;
    let atlas_text = crate::format::atlas::write(&packed.atlas);
    let mut files = vec![(format!("{}.json", opts.name), json.bytes), (format!("{}.atlas", opts.name), atlas_text.into_bytes())];
    for page in &packed.pages {
        files.push((page.file_name.clone(), encode_png(&page.image)?));
    }
    Ok(Bundle { files, warnings })
}

/// 포즈를 계산해 주는 쪽(Document). 시험에서는 가짜 계산기로 바꿔 끼운다.
trait PoseSource {
    fn skeleton(&self) -> &Skeleton;
    fn reset_physics(&mut self);
    fn pose(&mut self, req: &PoseRequest) -> PoseOutput;
}

impl PoseSource for Document {
    fn skeleton(&self) -> &Skeleton {
        &self.project().skeleton
    }
    fn reset_physics(&mut self) {
        Document::reset_physics(self)
    }
    fn pose(&mut self, req: &PoseRequest) -> PoseOutput {
        Document::pose(self, req)
    }
}

/// `name_0000.png` … (프레임 수 = floor(길이 × fps) + 1).
pub fn export_png_sequence(doc: &mut Document, images: &dyn ImageSource, opts: &PngSequenceOptions) -> Result<Bundle> {
    render_sequence(doc, images, opts)
}

fn render_sequence(src: &mut dyn PoseSource, images: &dyn ImageSource, opts: &PngSequenceOptions) -> Result<Bundle> {
    if !(opts.fps > 0.0) || !opts.fps.is_finite() {
        return Err(CoreError::InvalidArgs(format!("fps는 0보다 커야 합니다({})", opts.fps)));
    }
    if !(opts.scale > 0.0) || !opts.scale.is_finite() {
        return Err(CoreError::InvalidArgs(format!("배율은 0보다 커야 합니다({})", opts.scale)));
    }
    let duration = match &opts.animation {
        Some(name) => src.skeleton().animation(name).ok_or_else(|| CoreError::NotFound(format!("애니메이션 '{name}'")))?.duration(),
        None => 0.0,
    };
    let times = sequence_times(duration, &SequenceOptions { fps: opts.fps });

    // 크기: 셋업 포즈 그리기 목록의 경계 상자(+여백). 그림이 없으면 원점 둘레 여백만.
    src.reset_physics();
    let setup = src.pose(&PoseRequest { skin: opts.skin.clone(), physics: PhysicsStep::Reset, ..PoseRequest::default() });
    let bounds = draw_bounds(&setup);
    let (cx, cy) = bounds.map_or((0.0, 0.0), |(x0, y0, x1, y1)| ((x0 + x1) / 2.0, (y0 + y1) / 2.0));
    let auto = |extent: f32| ((extent * opts.scale).ceil() + 2.0 * AUTO_MARGIN).max(1.0) as u32;
    let width = if opts.width > 0 { opts.width } else { auto(bounds.map_or(0.0, |b| b.2 - b.0)) };
    let height = if opts.height > 0 { opts.height } else { auto(bounds.map_or(0.0, |b| b.3 - b.1)) };
    let ropts = RenderOptions { width, height, scale: opts.scale, center_x: cx, center_y: cy, background: opts.background };

    let mut cache: HashMap<String, Image> = HashMap::new();
    let mut missing: Vec<String> = Vec::new();
    let mut bundle = Bundle::default();
    src.reset_physics();
    for (i, &t) in times.iter().enumerate() {
        let physics = if i == 0 { PhysicsStep::Reset } else { PhysicsStep::Advance(1.0 / opts.fps) };
        let out = src.pose(&PoseRequest {
            animation: opts.animation.clone(),
            time: t,
            last_time: None,
            skin: opts.skin.clone(),
            physics,
            looping: false,
        });
        for item in &out.draw {
            if cache.contains_key(&item.image) || missing.contains(&item.image) {
                continue;
            }
            match images.load(&item.image) {
                Some(img) => {
                    cache.insert(item.image.clone(), img);
                }
                None => {
                    bundle.warnings.push(missing_image(&item.image));
                    missing.push(item.image.clone());
                }
            }
        }
        let frame = render_pose(&out.draw, &cache, &ropts);
        bundle.files.push((format!("{}_{:04}.png", opts.name, i), encode_png(&frame)?));
    }
    Ok(bundle)
}

/// 그리기 목록 정점 전체의 월드 경계 상자 (x0, y0, x1, y1).
fn draw_bounds(out: &PoseOutput) -> Option<(f32, f32, f32, f32)> {
    let mut b: Option<(f32, f32, f32, f32)> = None;
    for item in &out.draw {
        for p in item.positions.chunks_exact(2) {
            let (x, y) = (p[0], p[1]);
            if !x.is_finite() || !y.is_finite() {
                continue;
            }
            b = Some(match b {
                None => (x, y, x, y),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
            });
        }
    }
    b
}

#[cfg(test)]
mod tests {
    use super::super::decode_png;
    use super::*;
    use crate::model::{Animation, Key, RegionAttachment, Sequence, Skin};
    use crate::pose::DrawItem;

    /// 이름 → 이미지 사전.
    struct MapSource(HashMap<String, Image>);
    impl ImageSource for MapSource {
        fn load(&self, path: &str) -> Option<Image> {
            self.0.get(path).cloned()
        }
    }

    fn sample_images() -> MapSource {
        let mut m = HashMap::new();
        m.insert("head".to_string(), decode_png(include_bytes!("../../../../samples/images/head.png")).unwrap());
        m.insert("body".to_string(), decode_png(include_bytes!("../../../../samples/images/body.png")).unwrap());
        m.insert("gem".to_string(), decode_png(include_bytes!("../../../../samples/images/gem.png")).unwrap());
        m.insert("arm".to_string(), decode_png(include_bytes!("../../../../samples/images/arm.png")).unwrap());
        m.insert("glow".to_string(), decode_png(include_bytes!("../../../../samples/images/glow.png")).unwrap());
        for i in 1..=3 {
            m.insert(format!("fx/spark{i:02}"), Image { width: 4, height: 4, rgba: vec![255; 64] });
        }
        MapSource(m)
    }

    fn region(path: Option<&str>, seq: Option<Sequence>) -> Attachment {
        Attachment::Region(RegionAttachment { path: path.map(String::from), sequence: seq, ..RegionAttachment::default() })
    }

    #[test]
    fn k01_bundle_collects_image_paths() {
        let mut skel = Skeleton::new_with_root();
        let mut def = Skin::new(Skin::DEFAULT);
        let mut s1 = crate::model::OMap::new();
        s1.insert("head", region(None, None));
        s1.insert("머리2", region(Some("head"), None));
        s1.insert("불꽃", region(Some("fx/spark"), Some(Sequence { count: 3, start: 1, digits: 2, setup: 0 })));
        s1.insert("점", Attachment::new_of_type("point").unwrap());
        def.attachments.insert("슬롯1", s1);
        let mut other = Skin::new("옷");
        let mut s2 = crate::model::OMap::new();
        s2.insert("body", Attachment::new_of_type("mesh").unwrap());
        s2.insert("팔", Attachment::new_of_type("linkedmesh").unwrap());
        other.attachments.insert("슬롯2", s2);
        skel.skins = vec![def, other];
        // head는 두 번 쓰여도 한 번. 시퀀스는 spark01~03. point는 이미지가 없다. linkedmesh는 경로가 없으면 첨부물 이름.
        assert_eq!(attachment_image_paths(&skel), ["head", "fx/spark01", "fx/spark02", "fx/spark03", "body", "팔"]);
    }

    #[test]
    fn k01_bundle_files_and_missing_warning() {
        let json = WriteResult { bytes: b"{\"skeleton\":{}}".to_vec(), warnings: vec![Warning::new("spine43.slider", "x", "y")] };
        let paths: Vec<String> = ["head", "body", "gem", "없는그림", "fx/spark01"].iter().map(|s| s.to_string()).collect();
        let opts = SpineBundleOptions { name: "hero".into(), spine: SpineExportOptions::default(), pack: PackOptions::default() };
        let src = sample_images();
        let b = assemble_spine_bundle(json, &paths, &src, &opts).unwrap();
        let names: Vec<_> = b.files.iter().map(|f| f.0.as_str()).collect();
        assert_eq!(names, ["hero.json", "hero.atlas", "hero.png"]);
        assert_eq!(b.files[0].1, b"{\"skeleton\":{}}");
        // JSON 쪽 경고는 그대로, 없는 이미지는 export.missingImage.
        assert_eq!(b.warnings.len(), 2);
        assert_eq!(b.warnings[1].code, "export.missingImage");
        assert_eq!(b.warnings[1].path, "없는그림");
        // 페이지 PNG를 다시 읽으면 패킹 결과와 같은 크기이고, 각 영역 픽셀이 원본과 같다.
        let page = decode_png(&b.files[2].1).unwrap();
        let packed = pack_images(
            &paths.iter().filter_map(|p| src.load(p).map(|image| PackInput { name: p.clone(), image })).collect::<Vec<_>>(),
            &PackOptions { page_name: "hero".into(), ..PackOptions::default() },
        )
        .unwrap();
        assert_eq!(packed.pages[0].image, page);
        let regions = &packed.atlas.pages[0].regions;
        assert_eq!(regions.len(), 4);
        // head(64×64 원)는 원 테두리까지 불투명이라 잘라도 64×64.
        let head = regions.iter().find(|r| r.name == "head").unwrap();
        assert_eq!((head.width, head.height, head.offset_x, head.offset_y), (64, 64, 0, 0));
    }

    /// 시간에 따라 오른쪽으로 움직이는 사각형 하나를 돌려주는 가짜 계산기.
    struct FakePose {
        skel: Skeleton,
        image: String,
        calls: Vec<PoseRequest>,
        resets: usize,
    }
    impl PoseSource for FakePose {
        fn skeleton(&self) -> &Skeleton {
            &self.skel
        }
        fn reset_physics(&mut self) {
            self.resets += 1;
        }
        fn pose(&mut self, req: &PoseRequest) -> PoseOutput {
            self.calls.push(req.clone());
            let dx = req.time * 10.0;
            let (x0, y0, x1, y1) = (-8.0 + dx, -8.0, 8.0 + dx, 8.0);
            let item = DrawItem {
                slot: "s".into(),
                attachment: "a".into(),
                image: self.image.clone(),
                blend: crate::model::BlendMode::Normal,
                color: [1.0; 4],
                dark: None,
                positions: vec![x0, y0, x1, y0, x1, y1, x0, y1],
                uvs: vec![0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0],
                indices: vec![0, 1, 2, 2, 3, 0],
            };
            PoseOutput { draw: vec![item], ..PoseOutput::default() }
        }
    }

    fn skel_with_anim(duration: f32) -> Skeleton {
        let mut skel = Skeleton::new_with_root();
        let mut anim = Animation::new("걷기");
        let mut tl = crate::model::OMap::new();
        tl.insert("rotate", vec![Key::new(0.0, vec![0.0]), Key::new(duration, vec![10.0])]);
        anim.bones.insert("root", tl);
        skel.animations.push(anim);
        skel
    }

    #[test]
    fn k02_sequence_frames_size_and_pixels() {
        let mut fake = FakePose { skel: skel_with_anim(0.5), image: "body".into(), calls: vec![], resets: 0 };
        let opts = PngSequenceOptions { name: "walk".into(), animation: Some("걷기".into()), fps: 10.0, ..PngSequenceOptions::default() };
        let b = render_sequence(&mut fake, &sample_images(), &opts).unwrap();
        // 0.5초 × 10fps = 5 → 6장.
        assert_eq!(b.files.len(), 6);
        assert_eq!(b.files[0].0, "walk_0000.png");
        assert_eq!(b.files[5].0, "walk_0005.png");
        assert!(b.warnings.is_empty());
        // 셋업 경계 상자 16×16 + 여백 10×2 → 36×36, 중심 (0,0).
        for (_, bytes) in &b.files {
            let img = decode_png(bytes).unwrap();
            assert_eq!((img.width, img.height), (36, 36));
        }
        // 첫 장 가운데는 body 이미지의 안쪽 색 [80,120,200], 모서리(여백)는 투명.
        let f0 = decode_png(&b.files[0].1).unwrap();
        let c = ((18 * 36 + 18) * 4) as usize;
        assert_eq!(&f0.rgba[c..c + 4], &[80, 120, 200, 255]);
        assert_eq!(f0.rgba[3], 0);
        // 마지막 장(t = 0.5)은 5만큼 오른쪽: 픽셀 x 15~30에 그려지고, 왼쪽 x 10~14는 비었다.
        let f5 = decode_png(&b.files[5].1).unwrap();
        assert_eq!(f5.rgba[((18 * 36 + 12) * 4 + 3) as usize], 0);
        assert_eq!(f5.rgba[((18 * 36 + 29) * 4 + 3) as usize], 255);
        // 물리: 셋업(Reset) 다음 프레임별로 첫 장 Reset, 나머지 Advance(1/fps).
        assert_eq!(fake.calls.len(), 7);
        assert_eq!(fake.calls[1].physics, PhysicsStep::Reset);
        assert_eq!(fake.calls[2].physics, PhysicsStep::Advance(0.1));
        assert!(fake.calls[1..].iter().all(|c| c.animation.as_deref() == Some("걷기") && !c.looping));
        assert!((fake.calls[6].time - 0.5).abs() < 1e-6);
        assert_eq!(fake.resets, 2);
    }

    #[test]
    fn k02_sequence_fixed_size_missing_image_and_errors() {
        let mut fake = FakePose { skel: skel_with_anim(1.0), image: "없음".into(), calls: vec![], resets: 0 };
        // 애니메이션 없음 → 한 장. 지정 크기. 없는 이미지는 경고 한 번.
        let opts = PngSequenceOptions { width: 50, height: 40, background: [1, 2, 3, 255], ..PngSequenceOptions::default() };
        let b = render_sequence(&mut fake, &sample_images(), &opts).unwrap();
        assert_eq!(b.files.len(), 1);
        assert_eq!(b.files[0].0, "frame_0000.png");
        let img = decode_png(&b.files[0].1).unwrap();
        assert_eq!((img.width, img.height), (50, 40));
        assert!(img.rgba.chunks(4).all(|p| p == [1, 2, 3, 255]));
        assert_eq!(b.warnings.len(), 1);
        assert_eq!(b.warnings[0].code, "export.missingImage");
        // 없는 애니메이션, fps 0은 오류.
        let bad = PngSequenceOptions { animation: Some("없음".into()), ..PngSequenceOptions::default() };
        assert!(matches!(render_sequence(&mut fake, &sample_images(), &bad), Err(CoreError::NotFound(_))));
        let bad = PngSequenceOptions { fps: 0.0, ..PngSequenceOptions::default() };
        assert!(matches!(render_sequence(&mut fake, &sample_images(), &bad), Err(CoreError::InvalidArgs(_))));
    }

    /// 실제 Document로: 다른 담당(포즈 계산, .kgbone 읽기, Spine JSON 쓰기)이 아직 자리 표시여도 견고해야 한다.
    #[test]
    fn k02_sequence_with_document() {
        // 빈 새 문서: 애니메이션이 없으니 한 장, 그림이 없으면 여백만(20×20).
        let mut doc = Document::new();
        let b = export_png_sequence(&mut doc, &sample_images(), &PngSequenceOptions::default()).unwrap();
        assert_eq!(b.files.len(), 1);
        let img = decode_png(&b.files[0].1).unwrap();
        if doc.pose(&PoseRequest::default()).draw.is_empty() {
            assert_eq!((img.width, img.height), (20, 20));
        }

        // 시험 파일(.kgbone 읽기가 구현되어 있을 때만): 프레임 수 = floor(길이 × fps) + 1, 크기 일치, 그림이 있으면 픽셀이 그려짐.
        let Ok((mut doc, _)) = Document::open_kgbone(include_bytes!("../../../../samples/kgbone/full-4.3.kgbone")) else { return };
        let Some(anim) = doc.project().skeleton.animations.first().cloned() else { return };
        let opts = PngSequenceOptions { animation: Some(anim.name.clone()), fps: 10.0, ..PngSequenceOptions::default() };
        let b = export_png_sequence(&mut doc, &sample_images(), &opts).unwrap();
        assert_eq!(b.files.len(), (anim.duration() as f64 * 10.0 + 1e-6).floor() as usize + 1);
        let first = decode_png(&b.files[0].1).unwrap();
        for (_, bytes) in &b.files {
            let img = decode_png(bytes).unwrap();
            assert_eq!((img.width, img.height), (first.width, first.height));
        }
        let drawn = !doc.pose(&PoseRequest { animation: Some(anim.name.clone()), ..PoseRequest::default() }).draw.is_empty();
        if drawn {
            assert!(first.rgba.chunks(4).any(|p| p[3] > 0), "본 이미지 픽셀이 그려져야 합니다");
        }
    }

    #[test]
    fn k01_bundle_with_document() {
        // Spine JSON 쓰기가 아직이면 그 오류를 그대로 돌려준다. 되면 json·atlas가 들어 있어야 한다.
        let doc = Document::new();
        let opts = SpineBundleOptions { name: "s".into(), spine: SpineExportOptions::default(), pack: PackOptions::default() };
        match export_spine_bundle(&doc, &sample_images(), &opts) {
            Ok(b) => {
                assert_eq!(b.files[0].0, "s.json");
                assert_eq!(b.files[1].0, "s.atlas");
            }
            Err(e) => assert!(matches!(e, CoreError::NotImplemented(_)), "{e}"),
        }
    }
}
